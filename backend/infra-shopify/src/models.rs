//! Shopify webhook and API data models
//!
//! Defines structures for deserializing Shopify webhook payloads.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Shopify Subscription Contract Webhook Payload
///
/// This structure represents the payload sent by Shopify when a subscription
/// contract is created or updated.
#[derive(Debug, Deserialize, Serialize)]
pub struct ShopifySubscriptionContractWebhook {
    pub id: String,
    #[serde(default)]
    #[serde(rename = "contractRevisionId", alias = "contract_revision_id")]
    pub contract_revision_id: Option<i64>,
    #[serde(rename = "adminGraphqlApiId")]
    #[serde(alias = "admin_graphql_api_id")]
    pub admin_graphql_api_id: String,
    #[serde(rename = "customerId")]
    #[serde(alias = "customer_id")]
    pub customer_id: String,
    #[serde(rename = "originOrderId")]
    #[serde(alias = "origin_order_id")]
    pub origin_order_id: Option<String>,
    #[serde(rename = "sellingPlanId")]
    #[serde(alias = "selling_plan_id")]
    pub selling_plan_id: String,
    #[serde(rename = "currentPeriodEnd")]
    #[serde(alias = "current_period_end")]
    pub current_period_end: DateTime<Utc>,
    pub status: String,
    /// Herald identifiers (from contract attributes)
    #[serde(default)]
    #[serde(rename = "casRealmId")]
    pub herald_realm_id: Option<String>,
    #[serde(default)]
    #[serde(rename = "casUserId")]
    pub herald_user_id: Option<Uuid>,
    #[serde(default)]
    #[serde(rename = "casClientAppId")]
    pub herald_client_app_id: Option<Uuid>,
    /// Primary entitlement key from contract attributes.
    /// Falls back to casPlanId for backward compatibility.
    #[serde(default)]
    #[serde(rename = "herald_entitlement_key")]
    pub herald_entitlement_key: Option<String>,
    /// Legacy plan ID - kept for backward compatibility.
    /// Used as fallback when herald_entitlement_key is not present.
    #[serde(default)]
    #[serde(rename = "casPlanId")]
    pub herald_plan_id: Option<Uuid>,
}

impl ShopifySubscriptionContractWebhook {
    /// Resolve the entitlement key from the contract payload.
    /// Uses herald_entitlement_key first, falls back to casPlanId as string.
    pub fn resolve_entitlement_key(&self) -> Option<String> {
        self.herald_entitlement_key
            .clone()
            .filter(|s| !s.is_empty())
            .or_else(|| self.herald_plan_id.map(|id| id.to_string()))
    }
}

/// Shopify Billing Attempt Webhook Payload
///
/// Represents a billing attempt (success or failure) for a subscription.
#[derive(Debug, Deserialize, Serialize)]
pub struct ShopifyBillingAttemptWebhook {
    pub id: String,
    #[serde(rename = "subscriptionContractId")]
    #[serde(alias = "subscription_contract_id")]
    pub subscription_contract_id: String,
    #[serde(rename = "orderId")]
    #[serde(alias = "order_id")]
    pub order_id: Option<String>,
    #[serde(default)]
    #[serde(rename = "currentPeriodEnd", alias = "current_period_end")]
    pub current_period_end: Option<DateTime<Utc>>,
    pub success: bool,
    #[serde(rename = "errorCode")]
    #[serde(alias = "error_code")]
    pub error_code: Option<String>,
    #[serde(rename = "errorMessage")]
    #[serde(alias = "error_message")]
    pub error_message: Option<String>,
}

/// Shopify Refund Webhook Payload
///
/// Represents a refund event for an order.
#[derive(Debug, Deserialize, Serialize)]
pub struct ShopifyRefundWebhook {
    pub id: String,
    #[serde(rename = "orderId")]
    #[serde(alias = "order_id")]
    pub order_id: String,
    #[serde(rename = "createdAt")]
    #[serde(alias = "created_at")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "refundAmount")]
    #[serde(alias = "refund_amount")]
    pub refund_amount: i32,
    pub currency: String,
    pub reason: Option<String>,
}

/// Helper function to parse subscription contract webhook payload
pub fn parse_subscription_contract_payload(
    value: &serde_json::Value,
) -> Result<ShopifySubscriptionContractWebhook, serde_json::Error> {
    serde_json::from_value(value.clone())
}

/// Helper function to parse billing attempt webhook payload
pub fn parse_billing_attempt_payload(
    value: &serde_json::Value,
) -> Result<ShopifyBillingAttemptWebhook, serde_json::Error> {
    serde_json::from_value(value.clone())
}

/// Helper function to parse refund webhook payload
pub fn parse_refund_payload(
    value: &serde_json::Value,
) -> Result<ShopifyRefundWebhook, serde_json::Error> {
    serde_json::from_value(value.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_subscription_contract() {
        let json = r#"
        {
            "id": "gid://shopify/SubscriptionContract/12345",
            "contractRevisionId": 7,
            "adminGraphqlApiId": "gid://shopify/SubscriptionContract/12345",
            "customerId": "gid://shopify/Customer/67890",
            "originOrderId": "gid://shopify/Order/11111",
            "sellingPlanId": "gid://shopify/SellingPlan/22222",
            "currentPeriodEnd": "2026-05-01T00:00:00Z",
            "status": "ACTIVE",
            "casRealmId": "realm-1",
            "casUserId": "550e8400-e29b-41d4-a716-446655440000",
            "casClientAppId": "550e8400-e29b-41d4-a716-446655440001",
            "casPlanId": "550e8400-e29b-41d4-a716-446655440002"
        }
        "#;

        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        let contract = parse_subscription_contract_payload(&value).unwrap();

        assert_eq!(contract.id, "gid://shopify/SubscriptionContract/12345");
        assert_eq!(contract.contract_revision_id, Some(7));
        assert_eq!(contract.customer_id, "gid://shopify/Customer/67890");
        assert_eq!(contract.status, "ACTIVE");
        assert!(contract.herald_user_id.is_some());
        assert!(contract.herald_plan_id.is_some());
        // casPlanId used as fallback when herald_entitlement_key missing
        assert_eq!(
            contract.resolve_entitlement_key(),
            Some("550e8400-e29b-41d4-a716-446655440002".to_string())
        );
    }

    #[test]
    fn test_resolve_entitlement_key_prefers_herald_key() {
        let json = r#"
        {
            "id": "gid://shopify/SubscriptionContract/12345",
            "adminGraphqlApiId": "gid://shopify/SubscriptionContract/12345",
            "customerId": "gid://shopify/Customer/67890",
            "sellingPlanId": "gid://shopify/SellingPlan/22222",
            "currentPeriodEnd": "2026-05-01T00:00:00Z",
            "status": "ACTIVE",
            "herald_entitlement_key": "pro-monthly",
            "casPlanId": "550e8400-e29b-41d4-a716-446655440002"
        }
        "#;

        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        let contract = parse_subscription_contract_payload(&value).unwrap();

        // herald_entitlement_key takes priority
        assert_eq!(
            contract.resolve_entitlement_key(),
            Some("pro-monthly".to_string())
        );
    }

    #[test]
    fn test_resolve_entitlement_key_falls_back_to_casplanid() {
        let json = r#"
        {
            "id": "gid://shopify/SubscriptionContract/12345",
            "adminGraphqlApiId": "gid://shopify/SubscriptionContract/12345",
            "customerId": "gid://shopify/Customer/67890",
            "sellingPlanId": "gid://shopify/SellingPlan/22222",
            "currentPeriodEnd": "2026-05-01T00:00:00Z",
            "status": "ACTIVE",
            "casPlanId": "550e8400-e29b-41d4-a716-446655440002"
        }
        "#;

        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        let contract = parse_subscription_contract_payload(&value).unwrap();

        // Falls back to casPlanId as string
        assert_eq!(
            contract.resolve_entitlement_key(),
            Some("550e8400-e29b-41d4-a716-446655440002".to_string())
        );
    }

    #[test]
    fn test_parse_billing_attempt() {
        let json = r#"
        {
            "id": "gid://shopify/BillingAttempt/99999",
            "subscriptionContractId": "gid://shopify/SubscriptionContract/12345",
            "orderId": "gid://shopify/Order/88888",
            "currentPeriodEnd": "2026-05-01T00:00:00Z",
            "success": true,
            "errorCode": null,
            "errorMessage": null
        }
        "#;

        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        let attempt = parse_billing_attempt_payload(&value).unwrap();

        assert_eq!(attempt.id, "gid://shopify/BillingAttempt/99999");
        assert_eq!(
            attempt.subscription_contract_id,
            "gid://shopify/SubscriptionContract/12345"
        );
        assert!(attempt.current_period_end.is_some());
        assert!(attempt.success);
        assert!(attempt.error_code.is_none());
    }

    #[test]
    fn test_parse_refund() {
        let json = r#"
        {
            "id": "gid://shopify/Refund/77777",
            "orderId": "gid://shopify/Order/88888",
            "createdAt": "2026-04-01T12:00:00Z",
            "refundAmount": 1000,
            "currency": "USD",
            "reason": "Customer request"
        }
        "#;

        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        let refund = parse_refund_payload(&value).unwrap();

        assert_eq!(refund.id, "gid://shopify/Refund/77777");
        assert_eq!(refund.order_id, "gid://shopify/Order/88888");
        assert_eq!(refund.refund_amount, 1000);
        assert_eq!(refund.currency, "USD");
        assert_eq!(refund.reason, Some("Customer request".to_string()));
    }

    #[test]
    fn test_subscription_without_herald_identifiers() {
        let json = r#"
        {
            "id": "gid://shopify/SubscriptionContract/12345",
            "adminGraphqlApiId": "gid://shopify/SubscriptionContract/12345",
            "customerId": "gid://shopify/Customer/67890",
            "originOrderId": null,
            "sellingPlanId": "gid://shopify/SellingPlan/22222",
            "currentPeriodEnd": "2026-05-01T00:00:00Z",
            "status": "ACTIVE"
        }
        "#;

        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        let contract = parse_subscription_contract_payload(&value).unwrap();

        assert!(contract.herald_realm_id.is_none());
        assert!(contract.herald_user_id.is_none());
        assert!(contract.herald_client_app_id.is_none());
        assert!(contract.herald_plan_id.is_none());
        assert!(contract.herald_entitlement_key.is_none());
        assert!(contract.resolve_entitlement_key().is_none());
    }
}
