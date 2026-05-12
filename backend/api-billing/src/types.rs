use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// Response from checkout session creation
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateCheckoutResponse {
    pub checkout_url: String,
    pub checkout_id: Uuid,
}

// ===== Plan Management Types =====

/// Request to create a plan
#[derive(Debug, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreatePlanRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    #[validate(length(min = 1))]
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(max = 500))]
    pub description: Option<String>,
    // Keep field-level rename for Rust keyword conflict
    #[serde(rename = "type")]
    #[validate(custom(function = "validate_billing_type"))]
    pub r#type: String, // "monthly" | "yearly"
    #[validate(range(min = 0))]
    pub price: i32, // Price in cents
    #[validate(custom(function = "validate_currency"))]
    pub currency: String, // USD, EUR, CNY
    // NOTE: Payment provider fields removed - use PlanPaymentProvider API instead
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(url)]
    pub checkout_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 0))]
    pub trial_days: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 0))]
    pub sort_order: Option<i32>,
    pub product_id: Uuid,
}

/// Custom validator for billing type
fn validate_billing_type(type_str: &str) -> Result<(), validator::ValidationError> {
    if ["monthly", "yearly"].contains(&type_str) {
        Ok(())
    } else {
        Err(validator::ValidationError::new("invalid billing type"))
    }
}

/// Custom validator for currency
fn validate_currency(currency: &str) -> Result<(), validator::ValidationError> {
    if ["USD", "EUR", "CNY"].contains(&currency) {
        Ok(())
    } else {
        Err(validator::ValidationError::new("invalid currency"))
    }
}

/// Request to update a plan
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePlanRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    // Keep field-level rename for Rust keyword conflict
    #[serde(rename = "type")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    // NOTE: Payment provider fields removed - use PlanPaymentProvider API instead
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkout_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trial_days: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_id: Option<Uuid>,
}

/// Response for plan query
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlanResponse {
    pub id: Uuid,
    pub realm_id: String,
    pub name: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    // Keep field-level rename for Rust keyword conflict
    #[serde(rename = "type")]
    pub r#type: String,
    pub price: i32,
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkout_url: Option<String>,
    pub active: bool,
    pub trial_days: i32,
    pub sort_order: i32,
    pub product_id: Uuid,
    /// Payment providers configured for this plan (summary view)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub payment_providers: Vec<PaymentProviderSummary>,
    pub created_at: String,
    pub updated_at: String,
}

/// Summary of payment provider configuration for a plan
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaymentProviderSummary {
    pub id: Uuid,
    pub payment_provider: String,
    pub external_product_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_price_id: Option<String>,
    pub enabled: bool,
}

/// Response for listing plans
#[derive(Debug, Serialize, ToSchema)]
pub struct ListPlansResponse {
    pub plans: Vec<PlanResponse>,
}

// ===== Plan Assignment Types =====

/// Request to assign a plan to a client app
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssignPlanRequest {
    pub plan_id: Uuid,
}

/// Response for plan assignment
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlanAssignmentResponse {
    pub id: Uuid,
    pub client_app_id: Uuid,
    pub plan_id: Uuid,
    pub enabled: bool,
    pub created_at: String,
}

/// Response for listing plan assignments
#[derive(Debug, Serialize, ToSchema)]
pub struct ListPlanAssignmentsResponse {
    pub assignments: Vec<PlanAssignmentResponse>,
}

/// Request to toggle plan assignment
#[derive(Debug, Deserialize, ToSchema)]
pub struct TogglePlanAssignmentRequest {
    pub enabled: bool,
}

// ===== Subscription Types (Simplified) =====

/// Response for subscription detail (includes plan info)
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionDetailResponse {
    pub id: Uuid,
    pub client_app_id: Option<Uuid>,
    pub plan_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<PlanResponse>,
    pub status: String,
    pub billing_period: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_period_start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_period_end: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel_at_period_end: Option<bool>,
    pub created_at: String,
    pub updated_at: String,
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

/// Request to create checkout session for a plan
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateCheckoutSessionRequest {
    pub plan_id: Uuid,
    pub payment_provider: String,
    pub billing_period: String,
}

/// Plan summary in subscription history responses
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlanSummary {
    pub id: Uuid,
    pub name: String,
    pub title: String,
}

// Product Management Types

/// Request to create a product
#[derive(Debug, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateProductRequest {
    #[validate(length(min = 3, max = 50))]
    pub name: String,
    #[validate(length(min = 1, max = 100))]
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(max = 500))]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 0))]
    pub sort_order: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

/// Request to update a product
#[derive(Debug, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProductRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 1, max = 100))]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(max = 500))]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

/// Response for product query
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProductResponse {
    pub id: Uuid,
    pub realm_id: String,
    pub name: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub sort_order: i32,
    pub enabled: bool,
    pub plans_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// Response for listing products
#[derive(Debug, Serialize, ToSchema)]
pub struct ListProductsResponse {
    pub products: Vec<ProductResponse>,
}

/// Response for product detail (includes plan summary and plans_count)
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProductDetailResponse {
    pub id: Uuid,
    pub realm_id: String,
    pub name: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub sort_order: i32,
    pub enabled: bool,
    pub plans_count: i64,
    pub plans: Vec<PlanSummaryForProduct>,
    pub created_at: String,
    pub updated_at: String,
}

/// Plan summary within product detail response
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlanSummaryForProduct {
    pub id: Uuid,
    pub name: String,
    pub title: String,
    pub r#type: String,
    pub price: i32,
    pub currency: String,
    pub active: bool,
    pub sort_order: i32,
}

// ===== Plan Payment Provider Mapping Types =====

/// Request to add a payment provider to a plan
#[derive(Debug, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreatePlanPaymentProviderRequest {
    #[validate(length(min = 1))]
    pub payment_provider: String,
    #[validate(length(min = 1))]
    pub external_product_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_price_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

/// Response for plan payment provider mapping
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlanPaymentProviderResponse {
    pub id: Uuid,
    pub plan_id: Uuid,
    pub payment_provider: String,
    pub external_product_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_price_id: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Request to update a payment provider mapping
#[derive(Debug, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePlanPaymentProviderRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 1))]
    pub external_product_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_price_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

/// Request to toggle payment provider enabled status
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TogglePlanPaymentProviderRequest {
    pub enabled: bool,
}
