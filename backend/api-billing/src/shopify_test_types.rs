//! Shopify Test Data Types
//!
//! Types for test data creation endpoints.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// Request to create an unclaimed Shopify subscription
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateUnclaimedSubscriptionRequest {
    /// Shopify shop domain (e.g., "demo-store.myshopify.com")
    #[validate(length(min = 1))]
    pub shop_domain: String,

    /// Shopify customer ID (numeric or GID)
    #[validate(length(min = 1))]
    pub shopify_customer_id: String,

    /// Shopify customer GID (optional, defaults to customer_id)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shopify_customer_gid: Option<String>,

    /// Shopify subscription contract ID (GID format)
    #[validate(length(min = 1))]
    pub contract_id: String,

    /// Last order ID (optional, for order-based claims)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_order_id: Option<String>,

    /// Billing plan ID (UUID)
    #[validate(length(min = 1))]
    pub plan_id: String,

    /// Subscription status
    #[validate(length(min = 1))]
    pub status: String,
}

/// Response for unclaimed subscription creation
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateUnclaimedSubscriptionResponse {
    /// Created subscription ID
    pub subscription_id: String,

    /// Realm ID
    pub realm_id: String,

    /// Shopify customer ID
    pub shopify_customer_id: String,

    /// Contract ID
    pub contract_id: String,

    /// Subscription status
    pub status: String,

    /// Current period end date
    pub current_period_end: String,
}

#[cfg(test)]
mod tests {
    use super::CreateUnclaimedSubscriptionRequest;
    use validator::Validate;

    #[test]
    fn validate_create_unclaimed_subscription_request() {
        let request = CreateUnclaimedSubscriptionRequest {
            shop_domain: "demo-store.myshopify.com".to_string(),
            shopify_customer_id: "gid://shopify/Customer/123".to_string(),
            shopify_customer_gid: Some("gid://shopify/Customer/123".to_string()),
            contract_id: "gid://shopify/SubscriptionContract/123".to_string(),
            last_order_id: Some("gid://shopify/Order/456".to_string()),
            plan_id: "plan-uuid-123".to_string(),
            status: "active".to_string(),
        };

        assert!(request.validate().is_ok());
    }

    #[test]
    fn reject_invalid_create_unclaimed_subscription_request() {
        let request = CreateUnclaimedSubscriptionRequest {
            shop_domain: "".to_string(),         // Invalid: empty
            shopify_customer_id: "".to_string(), // Invalid: empty
            shopify_customer_gid: None,
            contract_id: "".to_string(), // Invalid: empty
            last_order_id: None,
            plan_id: "".to_string(), // Invalid: empty
            status: "".to_string(),  // Invalid: empty
        };

        assert!(request.validate().is_err());
    }
}
