use std::sync::Arc;

use crate::authentication::Identity;
use crate::billing::entities::EntitlementMapping;
use crate::billing::policies::BillingPolicy;
use crate::billing::ports::BillingRepository;
use crate::common::entities::app_errors::CoreError;
use crate::common::policies::ensure_policy;

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
    /// Non-renewing service-period length (days). Required (`>= 1`) when
    pub service_duration_days: Option<i64>,
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

        //   - service_duration_days must be present and >= 1 (US-PM-002 scene 2 → 400)
        //   - billing_period must be empty (mutually exclusive billing semantics → 400)
        validate_non_renewing(
            &input.billing_type,
            input.service_duration_days,
            input.billing_period.as_deref(),
        )?;

        let now = chrono::Utc::now();
        // Only non_renewing carries a service duration; other types store None
        // regardless of the input (the field is ignored for them). Resolve
        // before moving `input.billing_type` into the struct below.
        let service_duration_days = match input.billing_type {
            crate::billing::entities::BillingType::NonRenewing => input.service_duration_days,
            _ => None,
        };
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
            service_duration_days,
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

/// Validate the non-renewing billing-type invariants
///   - when `billing_type == NonRenewing`, `service_duration_days` must be
///     `Some(>= 1)` (US-PM-002 scene 2 → 400) and `billing_period` must be
///     empty (mutually exclusive billing semantics → 400);
///   - for other billing types this is a no-op (their duration/period rules
///     are enforced separately).
///
/// Pure/free function so the rule is unit-testable without a repository or
/// policy mock (avoids requiring the service's `R`/`P` generics to be inferred).
/// `create_mapping` routes through it. The PATCH handler applies the equivalent
/// resolved check inline against the stored mapping (3-state input), since it
/// must reconcile `service_duration_days` against the existing value.
fn validate_non_renewing(
    billing_type: &crate::billing::entities::BillingType,
    service_duration_days: Option<i64>,
    billing_period: Option<&str>,
) -> Result<(), CoreError> {
    if !matches!(
        billing_type,
        crate::billing::entities::BillingType::NonRenewing
    ) {
        return Ok(());
    }
    match service_duration_days {
        Some(days) if days >= 1 => {}
        _ => {
            return Err(CoreError::BadRequest(
                "service_duration_days is required and must be >= 1 for non_renewing billing_type"
                    .to_string(),
            ));
        }
    }
    if !billing_period.unwrap_or("").is_empty() {
        return Err(CoreError::BadRequest(
            "billing_period must be empty for non_renewing billing_type".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::billing::entities::BillingType;

    #[test]
    fn non_renewing_requires_service_duration_days() {
        // Missing duration → BadRequest (US-PM-002 scene 2 → 400).
        let err = validate_non_renewing(&BillingType::NonRenewing, None, None).unwrap_err();
        assert!(matches!(err, CoreError::BadRequest(_)), "got {err:?}");
    }

    #[test]
    fn non_renewing_rejects_zero_or_negative_duration() {
        for bad in [0_i64, -1, -5] {
            let err =
                validate_non_renewing(&BillingType::NonRenewing, Some(bad), None).unwrap_err();
            assert!(
                matches!(err, CoreError::BadRequest(_)),
                "value {bad} accepted"
            );
        }
    }

    #[test]
    fn non_renewing_accepts_positive_duration_without_period() {
        assert!(validate_non_renewing(&BillingType::NonRenewing, Some(30), None,).is_ok());
        assert!(validate_non_renewing(&BillingType::NonRenewing, Some(1), Some(""),).is_ok());
    }

    #[test]
    fn non_renewing_rejects_billing_period_as_mutually_exclusive() {
        // A non-renewing mapping carries a fixed service period; a recurring
        // billing_period is a conflicting billing semantics → 400.
        let err = validate_non_renewing(&BillingType::NonRenewing, Some(30), Some("monthly"))
            .unwrap_err();
        assert!(matches!(err, CoreError::BadRequest(_)), "got {err:?}");
    }

    #[test]
    fn non_renewing_validation_is_noop_for_other_billing_types() {
        // recurring / one_time are not subject to the non-renewing invariants
        // here (their own rules are enforced separately), so the duration and
        // period arguments are ignored.
        assert!(validate_non_renewing(&BillingType::Recurring, None, Some("monthly"),).is_ok());
        assert!(validate_non_renewing(&BillingType::OneTime, None, None,).is_ok());
    }

    #[test]
    fn billing_type_stays_immutable_on_update_path() {
        // The update path resolves the duration against the EXISTING billing_type
        // (never the request), so a non_renewing mapping cannot be silently
        // downgraded. This pins the resolved-type branch: a recurring mapping
        // with a cleared duration must still pass (duration is meaningless for
        // recurring), while a non_renewing mapping with the same input must fail.
        // (Execised via validate_non_renewing with the resolved type.)
        assert!(validate_non_renewing(&BillingType::Recurring, None, None,).is_ok());
        assert!(validate_non_renewing(&BillingType::NonRenewing, None, None,).is_err());
    }
}
