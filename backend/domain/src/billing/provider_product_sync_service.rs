use std::sync::Arc;

use crate::authentication::Identity;
use crate::billing::entities::EntitlementMapping;
use crate::billing::policies::BillingPolicy;
use crate::billing::ports::BillingRepository;
use crate::common::entities::app_errors::CoreError;
use crate::common::policies::ensure_policy;

/// External provider product info returned by ProviderApiPort
#[derive(Debug, Clone)]
pub struct ProviderProduct {
    pub external_product_id: String,
    pub external_price_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub price: Option<i64>,
    pub currency: Option<String>,
    pub billing_type: Option<String>,
    pub billing_period: Option<String>,
}

/// Result of a full provider sync operation
#[derive(Debug, Clone)]
pub struct SyncResult {
    pub products_synced: usize,
    pub prices_synced: usize,
    pub sync_status: SyncStatus,
    pub error: Option<String>,
    pub partial_errors: Vec<PartialSyncError>,
}

/// Sync status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncStatus {
    Completed,
    Partial,
    Failed,
}

/// Partial sync error detail
#[derive(Debug, Clone)]
pub struct PartialSyncError {
    pub external_id: String,
    pub reason: String,
}

/// Port for accessing external provider APIs (Stripe, Creem, etc.)
/// Concrete implementations live in the infra layer (BE-D05).
pub trait ProviderApiPort: Send + Sync {
    /// Fetch all products from the provider for a given realm
    fn fetch_products(
        &self,
        realm_id: &str,
        payment_provider: &str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<ProviderProduct>, CoreError>> + Send + '_>,
    >;
}

/// ProviderProductSyncService - Syncs provider products into local entitlement mappings
pub struct ProviderProductSyncService<R, P, A>
where
    R: BillingRepository,
    P: BillingPolicy,
    A: ProviderApiPort,
{
    repository: Arc<R>,
    policy: Arc<P>,
    provider_api: Arc<A>,
}

impl<R, P, A> ProviderProductSyncService<R, P, A>
where
    R: BillingRepository + Send + Sync,
    P: BillingPolicy,
    A: ProviderApiPort,
{
    pub fn new(repository: Arc<R>, policy: Arc<P>, provider_api: Arc<A>) -> Self {
        Self {
            repository,
            policy,
            provider_api,
        }
    }

    /// Sync provider products into local entitlement mappings
    pub async fn sync_provider_products(
        &self,
        identity: Identity,
        realm_id: &str,
        payment_provider: &str,
    ) -> Result<SyncResult, CoreError> {
        ensure_policy(
            self.policy.can_manage_billing(identity.clone()).await,
            "Insufficient permissions to manage billing",
        )?;

        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot access billing from a different realm".to_string(),
            ));
        }

        let products = self
            .provider_api
            .fetch_products(realm_id, payment_provider)
            .await?;

        let mut products_synced = 0usize;
        let mut prices_synced = 0usize;
        let mut partial_errors = Vec::new();

        for product in products {
            let entitlement_key = format!(
                "{}-{}",
                payment_provider,
                &product.external_product_id[..product.external_product_id.len().min(32)]
            );

            let mapping = EntitlementMapping {
                id: uuid::Uuid::now_v7(),
                realm_id: realm_id.to_string(),
                payment_provider: payment_provider.to_string(),
                external_product_id: product.external_product_id.clone(),
                external_price_id: product.external_price_id.clone(),
                entitlement_key: entitlement_key.clone(),
                billing_type: product
                    .billing_type
                    .as_deref()
                    .and_then(|s: &str| s.parse().ok()),
                billing_period: product.billing_period.clone(),
                points_per_period: None,
                grant_period_type: None,
                validity_days: None,
                grant_on_subscribe: false,
                max_periods: None,
                enabled: false,
                provider_product_info: Some(serde_json::json!({
                    "name": product.name,
                    "description": product.description,
                    "price": product.price,
                    "currency": product.currency,
                    "billing_type": product.billing_type,
                    "billing_period": product.billing_period,
                })),
                synced_at: Some(chrono::Utc::now()),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };

            match self.repository.upsert_entitlement_mapping(mapping).await {
                Ok(_) => {
                    products_synced += 1;
                    if product.external_price_id.is_some() {
                        prices_synced += 1;
                    }
                }
                Err(e) => {
                    partial_errors.push(PartialSyncError {
                        external_id: product.external_product_id.clone(),
                        reason: e.to_string(),
                    });
                }
            }
        }

        let (sync_status, error) = if partial_errors.is_empty() {
            (SyncStatus::Completed, None)
        } else if products_synced > 0 {
            (SyncStatus::Partial, None)
        } else {
            (
                SyncStatus::Failed,
                Some("All products failed to sync".to_string()),
            )
        };

        Ok(SyncResult {
            products_synced,
            prices_synced,
            sync_status,
            error,
            partial_errors,
        })
    }
}
