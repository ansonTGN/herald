use std::sync::Arc;

use crate::authentication::Identity;
use crate::billing::entities::EntitlementMapping;
use crate::billing::policies::BillingPolicy;
use crate::billing::ports::BillingRepository;
use crate::common::entities::app_errors::CoreError;
use crate::common::policies::ensure_policy;
use crate::points::entities::QuotaWindow;

#[derive(Debug, Clone)]
pub struct UpdateEntitlementMappingInput {
    pub entitlement_key: Option<String>,
    pub enabled: Option<bool>,
    pub points_per_period: Option<Option<i64>>,
    pub validity_days: Option<Option<i64>>,
    pub grant_on_subscribe: Option<bool>,
    pub bucket_id: Option<uuid::Uuid>,
    /// `None` = leave unchanged; `Some(None)` = clear; `Some(Some(vec))` = replace.
    pub quota_windows: Option<Option<Vec<QuotaWindow>>>,
}

/// Input for creating an entitlement mapping (design support-iap §4.2.2 / A2).
///
/// The generic `POST /api/bill/{realmId}/entitlement-mappings` endpoint accepts
/// this shape for any provider (IAP, Stripe, Creem). Required identity fields
/// (`payment_provider`, `external_product_id`, `entitlement_key`, `bucket_id`,
/// `billing_type`) are non-optional; everything else defaults. The
/// `uq_pem_realm_provider_product_price` unique constraint is enforced by the
/// repository and surfaces as a 409 conflict at the HTTP layer.
#[derive(Debug, Clone)]
pub struct CreateEntitlementMappingInput {
    pub payment_provider: String,
    pub external_product_id: String,
    /// Stripe Price ID for Stripe; `None` for IAP / Creem (price-less).
    pub external_price_id: Option<String>,
    pub entitlement_key: String,
    pub bucket_id: uuid::Uuid,
    pub billing_type: crate::billing::entities::BillingType,
    /// Required when `billing_type == Recurring`.
    pub billing_period: Option<String>,
    /// Credit-strategy field (requires `points.manage`).
    pub points_per_period: Option<i64>,
    /// Credit-strategy field.
    pub grant_on_subscribe: Option<bool>,
    /// One-time validity window (days).
    pub validity_days: Option<i64>,
    /// Roles auto-granted on payment success.
    pub granted_role_ids: Vec<uuid::Uuid>,
    pub enabled: bool,
}

pub struct EntitlementMappingService<R, P>
where
    R: BillingRepository,
    P: BillingPolicy,
{
    repository: Arc<R>,
    policy: Arc<P>,
}

impl<R, P> EntitlementMappingService<R, P>
where
    R: BillingRepository + Send + Sync,
    P: BillingPolicy,
{
    pub fn new(repository: Arc<R>, policy: Arc<P>) -> Self {
        Self { repository, policy }
    }

    pub async fn list_mappings(
        &self,
        identity: Identity,
        realm_id: &str,
        payment_provider: Option<&str>,
        enabled: Option<bool>,
        page: Option<u64>,
        page_size: Option<u64>,
    ) -> Result<(Vec<EntitlementMapping>, u64), CoreError> {
        ensure_policy(
            self.policy.can_view_billing(identity.clone()).await,
            "Insufficient permissions to view billing",
        )?;

        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot access billing from a different realm".to_string(),
            ));
        }

        self.repository
            .list_entitlement_mappings(realm_id, payment_provider, enabled, page, page_size)
            .await
    }

    pub async fn get_mapping(
        &self,
        identity: Identity,
        realm_id: &str,
        mapping_id: uuid::Uuid,
    ) -> Result<EntitlementMapping, CoreError> {
        ensure_policy(
            self.policy.can_view_billing(identity.clone()).await,
            "Insufficient permissions to view billing",
        )?;

        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot access billing from a different realm".to_string(),
            ));
        }

        let mapping = self
            .repository
            .find_entitlement_mapping_by_id(mapping_id)
            .await?
            .ok_or(CoreError::EntitlementMappingNotFound)?;

        if mapping.realm_id != realm_id {
            return Err(CoreError::EntitlementMappingNotFound);
        }

        Ok(mapping)
    }

    /// Create a new entitlement mapping (design support-iap §4.2.2 / A2).
    ///
    /// Caller is responsible for:
    /// - enforcing `billing.manage` (and `points.manage` when credit-strategy
    ///   fields are present) before invoking,
    /// - validating `granted_role_ids` realm membership,
    /// - `billing_period` presence when `billing_type == Recurring`.
    ///
    /// The `uq_pem_realm_provider_product_price` unique constraint is enforced
    /// by the repository; a violation surfaces as `CoreError::Conflict` and the
    /// handler maps it to HTTP 409.
    pub async fn create_mapping(
        &self,
        identity: Identity,
        realm_id: &str,
        input: CreateEntitlementMappingInput,
    ) -> Result<EntitlementMapping, CoreError> {
        ensure_policy(
            self.policy.can_manage_billing(identity.clone()).await,
            "Insufficient permissions to manage billing",
        )?;

        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot access billing from a different realm".to_string(),
            ));
        }

        Self::validate_entitlement_key(&input.entitlement_key)?;
        if !matches!(
            input.payment_provider.as_str(),
            "stripe" | "creem" | "apple" | "google"
        ) {
            return Err(CoreError::BadRequest(format!(
                "Unsupported payment provider: {}",
                input.payment_provider
            )));
        }
        if matches!(
            input.billing_type,
            crate::billing::entities::BillingType::Recurring
        ) && input.billing_period.as_deref().unwrap_or("").is_empty()
        {
            return Err(CoreError::BadRequest(
                "billing_period is required for recurring billing_type".to_string(),
            ));
        }

        let now = chrono::Utc::now();
        let mapping = EntitlementMapping {
            id: uuid::Uuid::now_v7(),
            realm_id: realm_id.to_string(),
            payment_provider: input.payment_provider,
            external_product_id: input.external_product_id,
            external_price_id: input.external_price_id,
            bucket_id: input.bucket_id,
            entitlement_key: input.entitlement_key,
            billing_type: Some(input.billing_type),
            billing_period: input.billing_period,
            points_per_period: input.points_per_period,
            grant_period_type: None,
            validity_days: input.validity_days,
            grant_on_subscribe: input.grant_on_subscribe.unwrap_or(false),
            max_periods: None,
            enabled: input.enabled,
            provider_product_info: None,
            quota_windows: None,
            granted_role_ids: input.granted_role_ids,
            synced_at: None,
            created_at: now,
            updated_at: now,
        };

        self.repository.create_entitlement_mapping(mapping).await
    }

    pub async fn update_mapping(
        &self,
        identity: Identity,
        realm_id: &str,
        mapping_id: uuid::Uuid,
        input: UpdateEntitlementMappingInput,
    ) -> Result<EntitlementMapping, CoreError> {
        ensure_policy(
            self.policy.can_manage_billing(identity.clone()).await,
            "Insufficient permissions to manage billing",
        )?;

        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot access billing from a different realm".to_string(),
            ));
        }

        let existing = self
            .repository
            .find_entitlement_mapping_by_id(mapping_id)
            .await?
            .ok_or(CoreError::EntitlementMappingNotFound)?;

        if existing.realm_id != realm_id {
            return Err(CoreError::EntitlementMappingNotFound);
        }

        if let Some(ref key) = input.entitlement_key {
            Self::validate_entitlement_key(key)?;
        }

        let updated = EntitlementMapping {
            id: existing.id,
            realm_id: existing.realm_id,
            payment_provider: existing.payment_provider,
            external_product_id: existing.external_product_id,
            external_price_id: existing.external_price_id,
            bucket_id: input.bucket_id.unwrap_or(existing.bucket_id),
            entitlement_key: input.entitlement_key.unwrap_or(existing.entitlement_key),
            billing_type: existing.billing_type,
            billing_period: existing.billing_period,
            points_per_period: match input.points_per_period {
                Some(v) => v,
                None => existing.points_per_period,
            },
            grant_period_type: existing.grant_period_type,
            validity_days: match input.validity_days {
                Some(v) => v,
                None => existing.validity_days,
            },
            grant_on_subscribe: input
                .grant_on_subscribe
                .unwrap_or(existing.grant_on_subscribe),
            max_periods: existing.max_periods,
            enabled: input.enabled.unwrap_or(existing.enabled),
            provider_product_info: existing.provider_product_info,
            quota_windows: match input.quota_windows {
                Some(v) => v,
                None => existing.quota_windows,
            },
            granted_role_ids: existing.granted_role_ids,
            synced_at: existing.synced_at,
            created_at: existing.created_at,
            updated_at: chrono::Utc::now(),
        };

        self.repository.update_entitlement_mapping(updated).await
    }

    pub async fn find_mapping_by_provider_product(
        &self,
        realm_id: &str,
        payment_provider: &str,
        external_product_id: &str,
    ) -> Result<Option<EntitlementMapping>, CoreError> {
        self.repository
            .find_entitlement_mapping_by_provider_product_price(
                realm_id,
                payment_provider,
                external_product_id,
                None,
            )
            .await
    }

    pub async fn find_mapping_by_entitlement_key(
        &self,
        realm_id: &str,
        entitlement_key: &str,
    ) -> Result<Option<EntitlementMapping>, CoreError> {
        self.repository
            .find_entitlement_mapping_by_key(realm_id, entitlement_key)
            .await
    }

    /// Validate entitlement_key format: [a-z0-9-], length 1-64
    fn validate_entitlement_key(key: &str) -> Result<(), CoreError> {
        if key.is_empty() || key.len() > 64 {
            return Err(CoreError::BadRequest(
                "entitlement_key must be 1-64 characters".to_string(),
            ));
        }
        if !key
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(CoreError::BadRequest(
                "entitlement_key must match [a-z0-9-]".to_string(),
            ));
        }
        Ok(())
    }
}
