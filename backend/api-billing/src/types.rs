use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::provider_common_types::validate_payment_provider_value;

/// Structured view of the `provider_product_info` JSONB synced from the
/// provider (design §5.7).
///
/// Field names are intentionally snake_case (no `rename_all`) so they match
/// the stored JSONB keys written by `build_provider_product_info` — the value
/// round-trips through `serde_json::from_value` without renaming. Every field
/// is optional because the backend stores the union of what each provider
/// exposes. `product_metadata` / `price_metadata` are strict string→string
/// maps (coerced at sync time in the provider adapter), matching how Stripe
/// and Creem model metadata.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProviderProductInfo {
    pub name: Option<String>,
    pub description: Option<String>,
    pub price: Option<i64>,
    pub currency: Option<String>,
    pub billing_type: Option<String>,
    pub billing_period: Option<String>,
    pub product_metadata: Option<HashMap<String, String>>,
    pub price_metadata: Option<HashMap<String, String>>,
}

/// Response for a single entitlement mapping
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EntitlementMappingResponse {
    pub id: Uuid,
    pub payment_provider: String,
    pub external_product_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_price_id: Option<String>,
    /// Bound credit bucket (non-null; matches domain entity).
    pub bucket_id: Uuid,
    pub entitlement_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_period: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub points_per_period: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validity_days: Option<i64>,
    pub grant_on_subscribe: bool,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_product_info: Option<ProviderProductInfo>,
    /// Subscription quota window config (design §4.3.2). `None` ⟺ no
    /// window-model grant. Each window carries the stable display `key`
    /// (derived from `windowSeconds`), the limit, and the window length.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota_windows: Option<Vec<EntitlementQuotaWindowResponse>>,
    /// Role IDs auto-granted on payment success (design §5.2). Always present
    /// (not skip-serializing) so the frontend always sees the field; empty
    /// array when no role grant is configured.
    pub granted_role_ids: Vec<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synced_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Read-side quota window view. Mirrors the domain `QuotaWindow`
/// snapshot carried on an entitlement mapping.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EntitlementQuotaWindowResponse {
    /// Sliding window length in seconds.
    pub window_seconds: i64,
    /// Quota limit.
    pub limit: i64,
    /// Stable display key (derived from `windowSeconds`, e.g. "5h"/"week"/"month").
    pub key: String,
}

/// Response for listing entitlement mappings
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EntitlementMappingListResponse {
    pub items: Vec<EntitlementMappingResponse>,
    pub total: i64,
}

/// Query parameters for listing entitlement mappings
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct EntitlementMappingQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<u64>,
}

/// Request to update an entitlement mapping
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEntitlementMappingRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entitlement_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub points_per_period: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validity_days: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grant_on_subscribe: Option<bool>,
    /// Subscription quota window config (design §4.3.2). `None` ⟺ leave the
    /// stored value untouched; `Some([])` ⟺ clear (no window grant);
    /// `Some([...])` ⟺ replace. Same validation as batch update.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota_windows: Option<Vec<QuotaWindowInput>>,
}

/// Request to create an entitlement mapping (design support-iap §4.2.2 / A2).
///
/// Generic over provider (IAP, Stripe, Creem). The
/// `uq_pem_realm_provider_product_price` unique constraint is enforced by the
/// repository and surfaces as HTTP 409. Credit-strategy fields
/// (`pointsPerPeriod` / `grantOnSubscribe` / `validityDays`) additionally
/// require the `points.manage` permission.
#[derive(Debug, Deserialize, ToSchema, validator::Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateEntitlementMappingRequest {
    /// `"apple"` / `"google"` / `"stripe"` / `"creem"`.
    #[validate(custom(function = "validate_payment_provider_value"))]
    pub payment_provider: String,
    pub external_product_id: String,
    /// Stripe Price ID for Stripe; `None` for IAP / Creem.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_price_id: Option<String>,
    pub entitlement_key: String,
    pub bucket_id: Uuid,
    /// `"recurring"` / `"one_time"`.
    #[validate(custom(function = "validate_create_billing_type"))]
    pub billing_type: String,
    /// Required when `billing_type == "recurring"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_period: Option<String>,
    /// Credit-strategy field (requires `points.manage`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub points_per_period: Option<i64>,
    /// Credit-strategy field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grant_on_subscribe: Option<bool>,
    /// One-time validity window (days).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validity_days: Option<i64>,
    /// Roles auto-granted on payment success.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub granted_role_ids: Vec<Uuid>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

fn validate_create_billing_type(billing_type: &str) -> Result<(), validator::ValidationError> {
    if matches!(billing_type, "recurring" | "one_time") {
        Ok(())
    } else {
        Err(validator::ValidationError::new("invalid_billing_type"))
    }
}

/// Request to sync provider products
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SyncProviderRequest {
    pub payment_provider: String,
}

/// Response from provider product sync
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SyncProviderResponse {
    pub products_synced: i64,
    pub prices_synced: i64,
    pub sync_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub partial_errors: Vec<PartialSyncError>,
}

/// Partial sync error detail
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PartialSyncError {
    pub external_id: String,
    pub reason: String,
}

/// Single one-time mapping item for frontend display
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OneTimeMappingItem {
    pub id: String,
    pub entitlement_key: String,
    /// Bound credit bucket (non-null; matches domain entity).
    pub bucket_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_product_info: Option<ProviderProductInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub points_per_period: Option<i64>,
    pub payment_provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validity_days: Option<i64>,
}

/// Response for listing one-time mappings
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OneTimeMappingListResponse {
    pub items: Vec<OneTimeMappingItem>,
}

/// Response for subscription detail
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionDetailResponse {
    pub id: Uuid,
    pub client_app_id: Option<Uuid>,
    pub entitlement_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_price_id: Option<String>,
    pub payment_provider: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_period_start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_period_end: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel_at_period_end: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_metadata: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synced_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Response for subscription list item
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionListItemResponse {
    pub id: Uuid,
    pub client_app_id: Option<Uuid>,
    pub entitlement_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_price_id: Option<String>,
    pub payment_provider: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_period_start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_period_end: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synced_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Response for listing subscriptions
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionListResponse {
    pub items: Vec<SubscriptionListItemResponse>,
    pub total: i64,
}

/// Query parameters for listing subscriptions
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionListQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entitlement_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<u64>,
}

/// Request to cancel subscription
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CancelSubscriptionRequest {
    pub cancel_at_period_end: bool,
}

/// Response for subscription cancellation
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CancelSubscriptionResponse {
    pub subscription_id: String,
    pub canceled_at: String,
    pub message: String,
}

/// Input for one quota window in a mapping batch save (design §4.3.2).
///
/// Carries only the editable fields; the stable display `key` is derived by
/// the backend from `windowSeconds` (via `derive_window_key`) before
/// persistence, so callers cannot drift window identity. Mirrors the
/// points-domain quota-window input shape; kept local to billing so
/// api-billing does not depend on api-points.
#[derive(Debug, Deserialize, ToSchema, validator::Validate)]
#[serde(rename_all = "camelCase")]
pub struct QuotaWindowInput {
    /// Sliding window length in seconds. Must be > 0.
    #[validate(range(min = 1))]
    pub window_seconds: i64,
    /// Quota limit. Must be >= 0 (0 = window grants nothing but is a valid
    /// config edge case).
    #[validate(range(min = 0))]
    pub limit: i64,
}

/// Single price-mapping row within a batch save.
///
/// One row per price of a product; `entitlement_key` is shared across the
/// product's prices. Credit-strategy fields (`points_per_period` etc.)
/// require `points.manage` server-side.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PriceMappingUpdate {
    pub mapping_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub points_per_period: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validity_days: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grant_on_subscribe: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Quota window config (design §4.3.2). `None` ⟺ leave unchanged;
    /// `Some([])` ⟺ clear (no window grant); `Some(non-empty)` ⟺ set.
    /// Non-empty triggers the `points.manage` credit-field permission gate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota_windows: Option<Vec<QuotaWindowInput>>,
    /// Role-grant config dimension (design §5.2). `None` ⟺ leave unchanged;
    /// `Some([])` ⟺ clear (no role grant); `Some(non-empty)` ⟺ set.
    /// Non-empty triggers realm-membership validation server-side (does NOT
    /// require `roles.manage`).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub granted_role_ids: Option<Vec<Uuid>>,
}

/// PUT `/api/bill/{realmId}/entitlement-mappings/batch` request body.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BatchUpdateEntitlementMappingsRequest {
    pub payment_provider: String,
    pub external_product_id: String,
    pub updates: Vec<PriceMappingUpdate>,
}

/// PUT `/api/bill/{realmId}/entitlement-mappings/batch` response body.
///
/// `prices` returns the product's full latest set of price rows.
/// Reuses `EntitlementMappingResponse` as the per-price view.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BatchUpdateEntitlementMappingsResponse {
    pub saved: u32,
    pub prices: Vec<EntitlementMappingResponse>,
}

/// A purchasable price-level option for the purchase page.
///
/// Flat list; the frontend groups by `external_product_id` / billing period.
/// Field set evolves `OneTimeMappingItem` down to price granularity.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseOptionView {
    pub mapping_id: Uuid,
    pub external_product_id: String,
    /// Stripe real price id; `None` for price-less providers (Creem).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_price_id: Option<String>,
    pub payment_provider: String,
    pub entitlement_key: String,
    /// `recurring` | `one_time`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_type: Option<String>,
    /// `month` / `year` / …
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_period: Option<String>,
    /// Product / plan name (card title).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Price amount (minor units).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub points_per_period: Option<i64>,
    pub enabled: bool,
    /// Whether purchasing this option grants a role that is one-per-user
    /// (design §4.2.2). True only for the gated combo
    /// `billing_type=one_time` + non-empty `granted_role_ids`; points packages
    /// and subscriptions are never gated, so `grants_role` is `false` for them
    /// even if they happen to carry role grants.
    pub grants_role: bool,
    /// Whether the authenticated user already owns this one-time+role
    /// entitlement (design §4.2.2). Computed only for the gated combo
    /// (`grants_role == true`); `false` otherwise. Lets the frontend disable the
    /// card pre-purchase. Ownership predicate:
    /// `user_has_any_role(granted_role_ids) || has_succeeded_attempt(target_id)`.
    pub already_owned: bool,
}

/// GET `/api/bill/{realmId}/client/{clientAppId}/purchase-options` response.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseOptionListResponse {
    pub items: Vec<PurchaseOptionView>,
}
