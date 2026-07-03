//! Batch price-mapping write types.
//!
//! Single-transaction upsert of ALL price rows for a product; shared-key rename
//! consistency (group-wide); any row violating the active-subscription lock
//! rolls back the whole transaction and surfaces a structured 409.

use uuid::Uuid;

use crate::billing::entities::EntitlementMapping;

/// Billing-side input DTO for one quota window in a mapping batch update.
///
/// Carries only the editable fields (`window_seconds`, `limit`); the stable
/// display `key` is derived by the API layer (via `derive_window_key`) before
/// persistence, so callers cannot drift window identity. Mirrors the
/// points-domain request shape (`api-points::types::QuotaWindowInputDto`) — kept
/// local to billing so api-billing does not depend on api-points.
#[derive(Debug, Clone)]
pub struct QuotaWindowInput {
    /// Sliding window length in seconds. Must be > 0.
    pub window_seconds: i64,
    /// Quota limit. Must be >= 0 (0 = window grants nothing but is a valid
    /// config edge case).
    pub limit: i64,
}

/// One price-mapping row within a batch save (domain input mirror of the API
/// `PriceMappingUpdate` DTO). Credit-strategy fields are `Option<Option<T>>` so
/// the caller can distinguish "leave unchanged" (`None`) from "clear" (outer
/// `Some`, inner `None`) where that matters; for simple optional fields the
/// outer `Option` carries the new value or "leave unchanged".
#[derive(Debug, Clone)]
pub struct PriceMappingUpdateInput {
    pub mapping_id: Uuid,
    pub entitlement_key: String,
    pub billing_type: Option<String>,
    pub points_per_period: Option<i64>,
    pub grant_period_type: Option<String>,
    pub validity_days: Option<i64>,
    pub grant_on_subscribe: Option<bool>,
    pub max_periods: Option<i32>,
    pub enabled: Option<bool>,
    /// Quota window config (design §4.3.2). `None` ⟺ leave unchanged;
    /// `Some(vec![])` ⟺ clear (no window grant); `Some(non-empty)` ⟺ set.
    /// Non-empty triggers the `points.manage` credit-field permission gate.
    pub quota_windows: Option<Vec<QuotaWindowInput>>,
}

/// Request payload for a batch price-mapping save scoped to one product.
#[derive(Debug, Clone)]
pub struct BatchUpdateMappingsInput {
    pub realm_id: String,
    pub payment_provider: String,
    pub external_product_id: String,
    pub updates: Vec<PriceMappingUpdateInput>,
}

/// Result of a successful batch save: the count of rows written and the
/// product's full latest set of price rows.
#[derive(Debug, Clone)]
pub struct BatchUpdateResult {
    pub saved: u32,
    pub prices: Vec<EntitlementMapping>,
}

/// Structured errors from a batch price-mapping save. The handler maps these to
/// HTTP statuses (400 / 409); infra/database failures stay `Other(CoreError)`.
#[derive(Debug, thiserror::Error)]
pub enum BatchMappingError {
    /// A `mapping_id` in the request does not belong to the
    /// `(realm, provider, product)` group — cross-product/realm tampering
    /// attempt. → 400.
    #[error(
        "mapping {mapping_id} does not belong to provider '{provider}' product '{product}' in this realm"
    )]
    MappingNotInGroup {
        mapping_id: Uuid,
        provider: String,
        product: String,
    },

    /// Renaming the shared `entitlement_key` for this product's group would also
    /// affect mappings of OTHER products that currently share the same key
    /// (cross-product rename leak). → 400.
    #[error(
        "shared-key rename would affect {affected_count} mapping(s) outside provider '{provider}' product '{product}'"
    )]
    CrossProductSharedKeyRename {
        provider: String,
        product: String,
        affected_count: i64,
    },

    /// One or more rows transition `enabled` true→false while their mapping has
    /// active (access-granting) subscriptions. The WHOLE transaction is rolled
    /// back. → 409 with `{ activeSubscriptions }`.
    #[error(
        "cannot disable mapping(s) with active subscriptions (provider '{provider}', product '{product}')"
    )]
    ActiveSubscriptionLock {
        provider: String,
        product: String,
        active_subscriptions: i64,
    },

    /// Infrastructure / unexpected failure. Preserves the wrapped `CoreError`
    /// status mapping (404 / 500 / …).
    #[error(transparent)]
    Other(#[from] crate::common::entities::app_errors::CoreError),
}
