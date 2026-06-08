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

// ===== Entitlement Mapping Types =====

/// Response for a single entitlement mapping
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EntitlementMappingResponse {
    pub id: Uuid,
    pub payment_provider: String,
    pub external_product_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_price_id: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synced_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
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

// ===== One-Time Mapping Types =====

/// Single one-time mapping item for frontend display
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OneTimeMappingItem {
    pub id: String,
    pub entitlement_key: String,
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

// ===== Subscription Types =====

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

/// Request to create checkout session
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateCheckoutSessionRequest {
    pub entitlement_key: String,
    pub payment_provider: String,
}
