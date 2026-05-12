// =============================================================================
// Creem API Mock Server
// =============================================================================
//
// Mock server for Creem payment API using wiremock.
// Provides predefined scenarios for testing payment flows.
//
// =============================================================================

#![allow(dead_code)]
#![allow(clippy::let_underscore_future)]

use serde_json::json;
use uuid::Uuid;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path},
};

/// Creem API Mock Server
///
/// Provides wiremock-based mocking for Creem payment API endpoints.
pub struct CreemMockServer {
    /// The underlying wiremock server
    pub server: MockServer,
    /// API key to use for authentication
    pub api_key: String,
    /// Webhook secret for signature verification
    pub webhook_secret: String,
}

impl CreemMockServer {
    /// Create a new CreemMockServer
    pub async fn start() -> Self {
        let server = MockServer::start().await;
        Self {
            server,
            api_key: "test_creem_api_key".to_string(),
            webhook_secret: "test_webhook_secret".to_string(),
        }
    }

    /// Get the base URL for the mock server
    pub fn base_url(&self) -> String {
        self.server.uri()
    }

    /// Mock successful checkout session creation
    ///
    /// # Arguments
    /// * `session_id` - The session ID to return
    /// * `product_id` - The product ID that should be in the request
    pub fn mock_checkout_success(&self, session_id: &str, product_id: &str) {
        let _ = Mock::given(method("POST"))
            .and(path("/v1/checkout"))
            .and(header("Authorization", format!("Bearer {}", self.api_key)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": session_id,
                "url": format!("https://checkout.test.creem.io/{}", session_id),
                "status": "pending",
                "product_id": product_id
            })))
            .mount(&self.server);
    }

    /// Mock checkout session creation with metadata
    ///
    /// # Arguments
    /// * `session_id` - The session ID to return
    /// * `product_id` - The product ID that should be in the request
    /// * `metadata` - Optional metadata to include in response
    pub fn mock_checkout_success_with_metadata(
        &self,
        session_id: &str,
        product_id: &str,
        metadata: Option<serde_json::Value>,
    ) {
        let mut response = json!({
            "id": session_id,
            "url": format!("https://checkout.test.creem.io/{}", session_id),
            "status": "pending",
            "product_id": product_id
        });

        if let Some(meta) = metadata {
            response["metadata"] = meta;
        }

        let _ = Mock::given(method("POST"))
            .and(path("/v1/checkout"))
            .and(header("Authorization", format!("Bearer {}", self.api_key)))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .mount(&self.server);
    }

    /// Mock checkout session creation failure
    ///
    /// # Arguments
    /// * `error_code` - Error code to return (e.g., "invalid_product", "authentication_failed")
    pub fn mock_checkout_failure(&self, error_code: &str) {
        let status_code = match error_code {
            "authentication_failed" | "invalid_api_key" => 401,
            "invalid_product" | "bad_request" => 400,
            _ => 500,
        };

        let _ = Mock::given(method("POST"))
            .and(path("/v1/checkout"))
            .respond_with(ResponseTemplate::new(status_code).set_body_json(json!({
                "error": error_code,
                "message": format!("Checkout failed: {}", error_code)
            })))
            .mount(&self.server);
    }

    /// Mock API timeout
    ///
    /// Simulates a timeout when calling Creem API.
    /// Note: wiremock doesn't support actual delays, so this returns a 504 Gateway Timeout.
    pub fn mock_api_timeout(&self) {
        let _ = Mock::given(method("POST"))
            .and(path("/v1/checkout"))
            .respond_with(ResponseTemplate::new(504).set_body_json(json!({
                "error": "timeout",
                "message": "Gateway timeout"
            })))
            .mount(&self.server);
    }

    /// Mock API error with custom status code
    ///
    /// # Arguments
    /// * `status_code` - HTTP status code to return
    /// * `error_msg` - Error message to return
    pub fn mock_api_error(&self, status_code: u16, error_msg: &str) {
        let _ = Mock::given(method("POST"))
            .and(path("/v1/checkout"))
            .respond_with(ResponseTemplate::new(status_code).set_body_json(json!({
                "error": "api_error",
                "message": error_msg
            })))
            .mount(&self.server);
    }

    /// Mock subscription retrieval
    ///
    /// # Arguments
    /// * `subscription_id` - The subscription ID
    /// * `status` - The subscription status
    pub fn mock_get_subscription(&self, subscription_id: &str, status: &str) {
        let _ = Mock::given(method("GET"))
            .and(path(format!("/v1/subscriptions/{}", subscription_id)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": subscription_id,
                "status": status,
                "current_period_start": chrono::Utc::now().to_rfc3339(),
                "current_period_end": chrono::Utc::now().to_rfc3339(),
            })))
            .mount(&self.server);
    }

    /// Mock subscription cancellation
    ///
    /// # Arguments
    /// * `subscription_id` - The subscription ID to cancel
    pub fn mock_cancel_subscription(&self, subscription_id: &str) {
        let _ = Mock::given(method("POST"))
            .and(path(format!(
                "/v1/subscriptions/{}/cancel",
                subscription_id
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": subscription_id,
                "status": "canceled",
                "canceled_at": chrono::Utc::now().to_rfc3339(),
            })))
            .mount(&self.server);
    }

    /// Reset all mocks
    ///
    /// Clears all registered mocks from the server.
    pub async fn reset(&self) {
        self.server.reset().await;
    }
}

/// Helper to generate a webhook payload
///
/// # Arguments
/// * `event_type` - Type of webhook event (e.g., "checkout.completed")
/// * `event_id` - Unique event ID
/// * `metadata` - Optional metadata to include
pub fn generate_webhook_payload(
    event_type: &str,
    event_id: &str,
    metadata: Option<serde_json::Value>,
) -> serde_json::Value {
    let mut payload = json!({
        "id": event_id,
        "eventType": event_type,
        "object": {
            "id": format!("{}_test_123", event_type.split('.').next().unwrap_or("evt")),
            "status": "completed",
            "amount": 2500,
            "currency": "USD",
            "product": {
                "id": "prod_test_monthly",
                "name": "Test Plan"
            },
            "customer": {
                "email": "test@example.com"
            }
        }
    });

    if let Some(meta) = metadata {
        payload["object"]["metadata"] = meta;
    }

    payload
}

/// Helper to generate checkout completed webhook
pub fn generate_checkout_completed_webhook(
    event_id: &str,
    realm_id: &str,
    client_app_id: &str,
    plan_id: &str,
    billing_period: &str,
) -> serde_json::Value {
    generate_checkout_completed_webhook_with_trial(
        event_id,
        realm_id,
        client_app_id,
        plan_id,
        billing_period,
        None,
    )
}

/// Helper to generate checkout completed webhook with trial days
pub fn generate_checkout_completed_webhook_with_trial(
    event_id: &str,
    realm_id: &str,
    client_app_id: &str,
    plan_id: &str,
    billing_period: &str,
    trial_days: Option<u32>,
) -> serde_json::Value {
    let mut metadata = json!({
        "realmId": realm_id,
        "clientAppId": client_app_id,
        "planId": plan_id,
        "billingPeriod": billing_period
    });

    // Add trial days if specified
    if let Some(days) = trial_days {
        metadata["trialDays"] = json!(days);
    }

    generate_webhook_payload("checkout.completed", event_id, Some(metadata))
}

/// Helper to generate subscription paid webhook
pub fn generate_subscription_paid_webhook(
    event_id: &str,
    subscription_id: &str,
    amount: i32,
) -> serde_json::Value {
    json!({
        "id": event_id,
        "eventType": "subscription.paid",
        "object": {
            "id": subscription_id,
            "status": "active",
            "amount": amount,
            "currency": "USD",
            "current_period_start": chrono::Utc::now().to_rfc3339(),
            "current_period_end": chrono::Utc::now()
                .checked_add_signed(chrono::Duration::days(30))
                .unwrap()
                .to_rfc3339()
        }
    })
}

/// Helper to generate complete subscription paid webhook with all required fields
/// This creates a subscription.paid webhook that will actually create/update a subscription
pub fn generate_subscription_paid_webhook_full(
    event_id: &str,
    realm_id: &str,
    client_app_id: &str,
    plan_id: &str,
    billing_period: &str,
    subscription_id: &str,
    is_trial: bool,
) -> serde_json::Value {
    let (duration_days, status) = if is_trial {
        (14, "trialing")
    } else {
        (if billing_period == "yearly" { 365 } else { 30 }, "active")
    };

    json!({
        "id": event_id,
        "eventType": "subscription.paid",
        "data": {
            "object": {
                "id": subscription_id,
                "subscriptionId": subscription_id,
                "userId": realm_id,
                "planId": plan_id,
                "clientAppId": client_app_id,
                "productId": format!("prod_{}", billing_period),
                "product": {
                    "id": format!("prod_{}", billing_period)
                },
                "status": status,
                "billingPeriod": billing_period,
                "cancelAtPeriodEnd": false,
                "amount": if billing_period == "yearly" { 25000 } else { 2500 },
                "currency": "USD",
                "current_period_start": chrono::Utc::now().to_rfc3339(),
                "current_period_end": chrono::Utc::now()
                    .checked_add_signed(chrono::Duration::days(duration_days))
                    .unwrap()
                    .to_rfc3339(),
                "metadata": {
                    "realmId": realm_id,
                    "clientAppId": client_app_id,
                    "planId": plan_id,
                    "billingPeriod": billing_period
                }
            }
        }
    })
}

/// Helper to generate refund webhook
pub fn generate_refund_webhook(
    event_id: &str,
    subscription_id: &str,
    refund_id: &str,
    amount: i32,
    is_full_refund: bool,
) -> serde_json::Value {
    json!({
        "id": event_id,
        "eventType": if is_full_refund { "refund.full" } else { "refund.partial" },
        "object": {
            "id": refund_id,
            "subscription_id": subscription_id,
            "amount": amount,
            "currency": "USD",
            "status": "succeeded"
        }
    })
}

/// Helper to generate dispute webhook
pub fn generate_dispute_webhook(event_id: &str, subscription_id: &str) -> serde_json::Value {
    json!({
        "id": event_id,
        "eventType": "dispute.created",
        "object": {
            "id": format!("disp_{}", Uuid::now_v7()),
            "subscription_id": subscription_id,
            "status": "needs_response",
            "amount": 2500,
            "currency": "USD"
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_creem_mock_server_creation() {
        let mock = CreemMockServer::start().await;
        assert!(!mock.base_url().is_empty());
    }

    #[tokio::test]
    async fn test_generate_webhook_payload() {
        let payload = generate_checkout_completed_webhook(
            "evt_test123",
            "realm_test",
            "client_app_test",
            "plan_test",
            "monthly",
        );

        assert_eq!(payload["id"], "evt_test123");
        assert_eq!(payload["eventType"], "checkout.completed");
        assert_eq!(payload["object"]["metadata"]["realmId"], "realm_test");
    }
}
