use std::sync::Arc;

use crate::authentication::Identity;
use crate::billing::entities::EntitlementMapping;
use crate::billing::policies::BillingPolicy;
use crate::billing::ports::BillingRepository;
use crate::common::entities::app_errors::CoreError;
use crate::common::policies::ensure_policy;
use crate::points::entities::QuotaWindow;

/// Input for updating an entitlement mapping
#[derive(Debug, Clone)]
pub struct UpdateEntitlementMappingInput {
    pub entitlement_key: Option<String>,
    pub enabled: Option<bool>,
    pub points_per_period: Option<Option<i64>>,
    pub grant_period_type: Option<Option<String>>,
    pub validity_days: Option<Option<i64>>,
    pub grant_on_subscribe: Option<bool>,
    pub max_periods: Option<Option<i64>>,
    pub bucket_id: Option<uuid::Uuid>,
    /// `None` = leave unchanged; `Some(None)` = clear; `Some(Some(vec))` = replace.
    pub quota_windows: Option<Option<Vec<QuotaWindow>>>,
}

/// EntitlementMappingService - Business logic for entitlement mapping management
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

    /// List entitlement mappings for a realm
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

    /// Get a single entitlement mapping by ID
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

    /// Update an entitlement mapping
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
            grant_period_type: match input.grant_period_type {
                Some(v) => v,
                None => existing.grant_period_type,
            },
            validity_days: match input.validity_days {
                Some(v) => v,
                None => existing.validity_days,
            },
            grant_on_subscribe: input
                .grant_on_subscribe
                .unwrap_or(existing.grant_on_subscribe),
            max_periods: match input.max_periods {
                Some(v) => v,
                None => existing.max_periods,
            },
            enabled: input.enabled.unwrap_or(existing.enabled),
            provider_product_info: existing.provider_product_info,
            quota_windows: match input.quota_windows {
                Some(v) => v,
                None => existing.quota_windows,
            },
            synced_at: existing.synced_at,
            created_at: existing.created_at,
            updated_at: chrono::Utc::now(),
        };

        self.repository.update_entitlement_mapping(updated).await
    }

    /// Find mapping by provider and external product ID
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

    /// Find mapping by entitlement key
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
