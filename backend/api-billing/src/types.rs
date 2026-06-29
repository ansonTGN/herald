use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

/// Response from checkout session creation
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateCheckoutResponse {
    pub checkout_url: String,
    pub checkout_id: Uuid,
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
    pub grant_period_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validity_days: Option<i64>,
    pub grant_on_subscribe: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_periods: Option<i64>,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_product_info: Option<serde_json::Value>,
    /// Subscription quota window config (design §4.3.2). `None` ⟺ no
    /// window-model grant. Each window carries the stable display `key`
    /// (derived from `windowSeconds`), the limit, and the window length.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota_windows: Option<Vec<QuotaWindowViewDto>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synced_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Read-side quota window view (response DTO). Mirrors the domain `QuotaWindow`
/// snapshot carried on an entitlement mapping.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuotaWindowViewDto {
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
    pub grant_period_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validity_days: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grant_on_subscribe: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_periods: Option<i64>,
    /// Subscription quota window config (design §4.3.2). `None` ⟺ leave the
    /// stored value untouched; `Some([])` ⟺ clear (no window grant);
    /// `Some([...])` ⟺ replace. Same validation as batch update.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota_windows: Option<Vec<QuotaWindowInputDto>>,
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
    pub partial_errors: Vec<PartialSyncErrorDto>,
}

/// Partial sync error detail
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PartialSyncErrorDto {
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
    pub provider_product_info: Option<serde_json::Value>,
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

/// Request to create checkout session.
///
/// Purchase target is price-level: callers pass the entitlement **mapping** id.
/// The checkout handler resolves the mapping to its
/// entitlement key / provider product / price and routes to the payment
/// provider. Replaces the former `entitlement_key`-only contract.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateCheckoutSessionRequest {
    pub mapping_id: Uuid,
    pub payment_provider: String,
}

/// Input DTO for one quota window in a mapping batch save (design §4.3.2).
///
/// Carries only the editable fields; the stable display `key` is derived by
/// the backend from `windowSeconds` (via `derive_window_key`) before
/// persistence, so callers cannot drift window identity. Mirrors the
/// points-domain `QuotaWindowInputDto` shape; kept local to billing so
/// api-billing does not depend on api-points.
#[derive(Debug, Deserialize, ToSchema, validator::Validate)]
#[serde(rename_all = "camelCase")]
pub struct QuotaWindowInputDto {
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
    pub entitlement_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_period: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub points_per_period: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grant_period_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validity_days: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grant_on_subscribe: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_periods: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Quota window config (design §4.3.2). `None` ⟺ leave unchanged;
    /// `Some([])` ⟺ clear (no window grant); `Some(non-empty)` ⟺ set.
    /// Non-empty triggers the `points.manage` credit-field permission gate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota_windows: Option<Vec<QuotaWindowInputDto>>,
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
}

/// GET `/api/bill/{realmId}/client/{clientAppId}/purchase-options` response.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseOptionListResponse {
    pub items: Vec<PurchaseOptionView>,
}
