use crate::models::{
    CheckoutSession, CreateCheckoutRequest, CreatePaymentIntentRequest, PaymentIntent,
};
use herald_domain::common::entities::app_errors::CoreError;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::Duration;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
pub struct StripeClient {
    pub(crate) http: reqwest::Client,
    pub(crate) api_key: String,
    pub(crate) base_url: String,
}

impl StripeClient {
    /// Create a new Stripe API client
    ///
    /// # Arguments
    ///
    /// * `api_key` - Stripe API key (sk_test_... or sk_live_...)
    /// * `timeout_seconds` - HTTP request timeout in seconds
    ///
    /// # Note
    ///
    /// Uses Stripe API endpoint: https://api.stripe.com
    pub fn new(api_key: String, timeout_seconds: u64) -> Result<Self, CoreError> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()
            .map_err(|e| {
                CoreError::InternalServerError(format!("Failed to create HTTP client: {e}"))
            })?;

        Ok(Self {
            http,
            api_key,
            base_url: "https://api.stripe.com".to_string(),
        })
    }

    /// Create a new Stripe API client with a custom base URL
    ///
    /// This is primarily useful for testing with mock servers.
    ///
    /// # Arguments
    ///
    /// * `api_key` - Stripe API key
    /// * `base_url` - Custom base URL for the API
    /// * `timeout_seconds` - HTTP request timeout in seconds
    pub fn with_base_url(
        api_key: String,
        base_url: String,
        timeout_seconds: u64,
    ) -> Result<Self, CoreError> {
        let http = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()
            .map_err(|e| {
                CoreError::InternalServerError(format!("Failed to create HTTP client: {e}"))
            })?;

        Ok(Self {
            http,
            api_key,
            base_url,
        })
    }

    /// Create a checkout session for a product
    ///
    /// # Arguments
    ///
    /// * `request` - Checkout session creation request
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The API request fails
    /// - Invalid API key
    /// - Network connectivity issues
    /// - Stripe returns an error response
    pub async fn create_checkout_session(
        &self,
        request: &CreateCheckoutRequest,
    ) -> Result<CheckoutSession, CoreError> {
        if self.base_url == "mock://stripe" {
            let plan_id = &request.plan_id.to_string();
            let short_id = &plan_id[plan_id.len().saturating_sub(8)..];
            let id = format!("cs_mock_{short_id}");
            return Ok(CheckoutSession {
                id: id.clone(),
                url: format!("mock://stripe/checkout/{id}"),
                customer: None,
                status: Some("open".to_string()),
                payment_intent: Some(format!("pi_mock_{short_id}")),
                subscription: None,
                metadata: serde_json::to_value(&request.metadata).unwrap_or_default(),
            });
        }

        let url = format!("{}/v1/checkout/sessions", self.base_url);

        // Build form-encoded fields (Stripe requires application/x-www-form-urlencoded)
        let mut form_fields: Vec<(String, String)> = vec![
            ("success_url".to_string(), request.success_url.clone()),
            ("cancel_url".to_string(), request.cancel_url.clone()),
            ("mode".to_string(), "subscription".to_string()),
            // Metadata fields
            ("metadata[realm_id]".to_string(), request.realm_id.clone()),
            (
                "metadata[client_app_id]".to_string(),
                request.client_app_id.to_string(),
            ),
            ("metadata[plan_id]".to_string(), request.plan_id.to_string()),
            (
                "metadata[billing_period]".to_string(),
                request.billing_period.clone(),
            ),
            ("metadata[plan_name]".to_string(), request.plan_name.clone()),
        ];

        if let Some(user_id) = request.user_id {
            form_fields.push(("metadata[user_id]".to_string(), user_id.to_string()));
        }

        if let Some(extra_metadata) = &request.metadata {
            for (key, value) in extra_metadata {
                form_fields.push((format!("metadata[{key}]"), value.clone()));
            }
        }

        if let Some(customer_email) = &request.customer_email {
            form_fields.push(("customer_email".to_string(), customer_email.clone()));
        }

        // Line items[0] fields
        form_fields.push((
            "line_items[0][price_data][currency]".to_string(),
            request.currency.clone(),
        ));
        form_fields.push((
            "line_items[0][price_data][product_data][name]".to_string(),
            request.plan_name.clone(),
        ));
        form_fields.push((
            "line_items[0][price_data][product_data][metadata][plan_id]".to_string(),
            request.plan_id.to_string(),
        ));
        form_fields.push((
            "line_items[0][price_data][unit_amount]".to_string(),
            request.price_amount.to_string(),
        ));
        let interval = if request.billing_period == "monthly" {
            "month"
        } else {
            "year"
        };
        form_fields.push((
            "line_items[0][price_data][recurring][interval]".to_string(),
            interval.to_string(),
        ));
        form_fields.push(("line_items[0][quantity]".to_string(), "1".to_string()));

        // Add trial period if specified
        if let Some(trial_days) = request.trial_days
            && trial_days > 0
        {
            form_fields.push((
                "subscription_data[trial_period_days]".to_string(),
                trial_days.to_string(),
            ));
        }

        tracing::info!(
            "Creating Stripe checkout session for plan: {}",
            request.plan_id
        );

        let response = self
            .http
            .post(&url)
            .bearer_auth(&self.api_key)
            .form(&form_fields)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            tracing::error!("Stripe API error: {} - {}", status, text);
            return Err(CoreError::InternalServerError(format!(
                "Stripe API error: {} - {}",
                status.as_u16(),
                text
            )));
        }

        let stripe_response: serde_json::Value = response.json().await.map_err(|e| {
            tracing::error!("Failed to parse Stripe response: {}", e);
            CoreError::InternalServerError(format!("Invalid Stripe response: {}", e))
        })?;

        Ok(CheckoutSession {
            id: stripe_response["id"]
                .as_str()
                .ok_or_else(|| {
                    CoreError::InternalServerError(
                        "Missing 'id' in Stripe checkout session response".to_string(),
                    )
                })?
                .to_string(),
            url: stripe_response["url"]
                .as_str()
                .ok_or_else(|| {
                    CoreError::InternalServerError(
                        "Missing 'url' in Stripe checkout session response".to_string(),
                    )
                })?
                .to_string(),
            customer: stripe_response["customer"].as_str().map(|s| s.to_string()),
            status: stripe_response["status"].as_str().map(|s| s.to_string()),
            payment_intent: stripe_response["payment_intent"]
                .as_str()
                .map(|s| s.to_string()),
            subscription: stripe_response["subscription"]
                .as_str()
                .map(|s| s.to_string()),
            metadata: serde_json::from_value(stripe_response["metadata"].clone())
                .unwrap_or_default(),
        })
    }

    /// Create a payment intent for one-off payments such as points package purchases.
    pub async fn create_payment_intent(
        &self,
        request: &CreatePaymentIntentRequest,
    ) -> Result<PaymentIntent, CoreError> {
        if request.amount <= 0 {
            return Err(CoreError::BadRequest(
                "Payment intent amount must be greater than 0".to_string(),
            ));
        }

        if request.currency.len() != 3 {
            return Err(CoreError::BadRequest(
                "Payment intent currency must be a 3-letter ISO code".to_string(),
            ));
        }

        if self.base_url == "mock://stripe" {
            let attempt_id = request
                .metadata
                .get("attemptId")
                .cloned()
                .unwrap_or_else(|| uuid::Uuid::now_v7().to_string());
            let id = format!("pi_mock_{attempt_id}");
            return Ok(PaymentIntent {
                id: id.clone(),
                client_secret: format!("{id}_secret_mock"),
                amount: request.amount,
                currency: request.currency.clone(),
                status: Some("requires_payment_method".to_string()),
                metadata: serde_json::to_value(&request.metadata).unwrap_or_default(),
            });
        }

        let url = format!("{}/v1/payment_intents", self.base_url);

        let mut form_fields = vec![
            ("amount".to_string(), request.amount.to_string()),
            ("currency".to_string(), request.currency.clone()),
            (
                "automatic_payment_methods[enabled]".to_string(),
                "true".to_string(),
            ),
        ];
        if let Some(receipt_email) = &request.receipt_email {
            form_fields.push(("receipt_email".to_string(), receipt_email.clone()));
        }
        form_fields.extend(
            request
                .metadata
                .iter()
                .map(|(key, value)| (format!("metadata[{key}]"), value.clone())),
        );

        let response = self
            .http
            .post(&url)
            .bearer_auth(&self.api_key)
            .form(&form_fields)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            tracing::error!("Stripe payment intent API error: {} - {}", status, text);
            return Err(CoreError::InternalServerError(format!(
                "Stripe payment intent API error: {} - {}",
                status.as_u16(),
                text
            )));
        }

        let stripe_response: serde_json::Value = response.json().await.map_err(|e| {
            tracing::error!("Failed to parse Stripe payment intent response: {}", e);
            CoreError::InternalServerError(format!("Invalid Stripe response: {}", e))
        })?;

        Ok(PaymentIntent {
            id: stripe_response["id"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            client_secret: stripe_response["client_secret"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            amount: stripe_response["amount"].as_i64().unwrap_or_default(),
            currency: stripe_response["currency"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            status: stripe_response["status"].as_str().map(str::to_string),
            metadata: stripe_response["metadata"].clone(),
        })
    }

    /// Verify a Stripe webhook signature
    ///
    /// # Arguments
    ///
    /// * `payload` - Raw webhook payload bytes
    /// * `signature` - Stripe-Signature header value
    /// * `secret` - Webhook signing secret
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Signature is invalid
    /// - Timestamp is too old (replay attack protection)
    /// - Secret is invalid
    ///
    /// # Implementation Details
    ///
    /// Stripe uses HMAC-SHA256 to sign webhook payloads.
    /// The signature is sent in the `stripe-signature` header with format: "t=...,v1=..."
    ///
    /// # Note
    ///
    /// This is a static method (doesn't require &self) because webhook signature verification
    /// doesn't need any client state (api_key, http, etc.). This allows per-realm webhook
    /// verification without creating a StripeClient instance.
    pub fn verify_webhook_signature(
        payload: &[u8],
        signature: &str,
        secret: &str,
    ) -> Result<(), CoreError> {
        // Parse signature header
        let signature_elements: Vec<&str> = signature.split(',').collect();
        let mut timestamp = None;
        let mut expected_signature = None;

        for element in signature_elements {
            let parts: Vec<&str> = element.split('=').collect();
            if parts.len() != 2 {
                continue;
            }
            match parts[0] {
                "t" => timestamp = Some(parts[1]),
                "v1" => expected_signature = Some(parts[1]),
                _ => {}
            }
        }

        let timestamp = timestamp.ok_or_else(|| {
            CoreError::BadRequest("Missing timestamp in webhook signature".to_string())
        })?;

        let expected_signature = expected_signature.ok_or_else(|| {
            CoreError::BadRequest("Missing signature in webhook signature".to_string())
        })?;

        // Check timestamp age (replay attack protection - 15 minutes)
        let timestamp_i64: i64 = timestamp.parse().map_err(|_| {
            CoreError::BadRequest("Invalid timestamp in webhook signature".to_string())
        })?;

        let now = chrono::Utc::now().timestamp();
        let age_seconds = now - timestamp_i64;

        if age_seconds > 900 {
            // 15 minutes = 900 seconds
            return Err(CoreError::BadRequest(format!(
                "Webhook timestamp is too old: {} seconds",
                age_seconds
            )));
        }

        if age_seconds < -900 {
            return Err(CoreError::BadRequest(
                "Webhook timestamp is in the future".to_string(),
            ));
        }

        // Build signed payload
        let signed_payload = format!("{}.{}", timestamp, String::from_utf8_lossy(payload));

        // Compute HMAC-SHA256
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .map_err(|_| CoreError::InternalServerError("Invalid webhook secret".to_string()))?;

        mac.update(signed_payload.as_bytes());
        let computed_signature = hex::encode(mac.finalize().into_bytes());

        // Compare signatures
        if computed_signature == expected_signature {
            Ok(())
        } else {
            Err(CoreError::BadRequest(
                "Invalid webhook signature".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::any;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_verify_webhook_signature_valid() {
        let payload = b"test_payload";
        let secret = "whsec_test_secret";

        // Create a valid signature
        let timestamp = chrono::Utc::now().timestamp().to_string();
        let signed_payload = format!("{}.{}", timestamp, String::from_utf8_lossy(payload));

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(signed_payload.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        let signature_header = format!("t={},v1={}", timestamp, signature);

        let result = StripeClient::verify_webhook_signature(payload, &signature_header, secret);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_webhook_signature_invalid() {
        let payload = b"test_payload";
        let secret = "whsec_test_secret";

        let timestamp = chrono::Utc::now().timestamp().to_string();
        let signature_header = format!("t={},v1=invalid_signature", timestamp);

        let result = StripeClient::verify_webhook_signature(payload, &signature_header, secret);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_webhook_signature_old_timestamp() {
        let payload = b"test_payload";
        let secret = "whsec_test_secret";

        // Use a timestamp from 20 minutes ago
        let old_timestamp = (chrono::Utc::now().timestamp() - 1200).to_string();
        let signature_header = format!("t={},v1=some_signature", old_timestamp);

        let result = StripeClient::verify_webhook_signature(payload, &signature_header, secret);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_payment_intent_sends_form_encoded_metadata() {
        let mock_server = MockServer::start().await;
        let client =
            StripeClient::with_base_url("sk_test_123".to_string(), mock_server.uri(), 30).unwrap();

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "pi_test_123",
                "client_secret": "pi_test_123_secret_abc",
                "amount": 1299,
                "currency": "usd",
                "status": "requires_payment_method",
                "metadata": {
                    "attemptId": "attempt-123",
                    "targetType": "points_package"
                }
            })))
            .mount(&mock_server)
            .await;

        let result = client
            .create_payment_intent(&CreatePaymentIntentRequest {
                amount: 1299,
                currency: "usd".to_string(),
                receipt_email: Some("buyer@example.com".to_string()),
                metadata: std::collections::HashMap::from([
                    ("attemptId".to_string(), "attempt-123".to_string()),
                    ("targetType".to_string(), "points_package".to_string()),
                ]),
            })
            .await
            .expect("payment intent should be created");

        assert_eq!(result.id, "pi_test_123");
        assert_eq!(result.client_secret, "pi_test_123_secret_abc");
        assert_eq!(result.amount, 1299);
        assert_eq!(result.currency, "usd");
        assert_eq!(result.metadata["attemptId"], "attempt-123");

        let requests = mock_server
            .received_requests()
            .await
            .expect("request recording should be enabled");
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method.as_str(), "POST");
        assert_eq!(requests[0].url.path(), "/v1/payment_intents");
        let form: std::collections::HashMap<_, _> = url::form_urlencoded::parse(&requests[0].body)
            .into_owned()
            .collect();

        assert_eq!(
            requests[0]
                .headers
                .get("authorization")
                .and_then(|value| value.to_str().ok()),
            Some("Bearer sk_test_123")
        );
        assert_eq!(form.get("amount"), Some(&"1299".to_string()));
        assert_eq!(form.get("currency"), Some(&"usd".to_string()));
        assert_eq!(
            form.get("receipt_email"),
            Some(&"buyer@example.com".to_string())
        );
        assert_eq!(
            form.get("automatic_payment_methods[enabled]"),
            Some(&"true".to_string())
        );
        assert_eq!(
            form.get("metadata[attemptId]"),
            Some(&"attempt-123".to_string())
        );
        assert_eq!(
            form.get("metadata[targetType]"),
            Some(&"points_package".to_string())
        );
    }

    #[tokio::test]
    async fn test_create_payment_intent_supports_demo_mock_base_url() {
        let client = StripeClient::with_base_url(
            "sk_test_demo".to_string(),
            "mock://stripe".to_string(),
            30,
        )
        .unwrap();

        let result = client
            .create_payment_intent(&CreatePaymentIntentRequest {
                amount: 500,
                currency: "USD".to_string(),
                receipt_email: Some("buyer@example.com".to_string()),
                metadata: std::collections::HashMap::from([(
                    "attemptId".to_string(),
                    "attempt-123".to_string(),
                )]),
            })
            .await
            .expect("mock payment intent should be created");

        assert_eq!(result.id, "pi_mock_attempt-123");
        assert_eq!(result.client_secret, "pi_mock_attempt-123_secret_mock");
        assert_eq!(result.amount, 500);
        assert_eq!(result.currency, "USD");
        assert_eq!(result.status.as_deref(), Some("requires_payment_method"));
        assert_eq!(result.metadata["attemptId"], "attempt-123");
    }

    #[tokio::test]
    async fn test_create_checkout_session_supports_demo_mock_base_url() {
        let client = StripeClient::with_base_url(
            "sk_test_demo".to_string(),
            "mock://stripe".to_string(),
            30,
        )
        .unwrap();

        let plan_id = uuid::Uuid::now_v7();
        let plan_id_str = plan_id.to_string();
        let short_id = &plan_id_str[plan_id_str.len() - 8..];

        let result = client
            .create_checkout_session(&CreateCheckoutRequest {
                client_app_id: uuid::Uuid::now_v7(),
                plan_id,
                user_id: Some(uuid::Uuid::now_v7()),
                customer_email: Some("buyer@example.com".to_string()),
                success_url: "https://example.com/success".to_string(),
                cancel_url: "https://example.com/cancel".to_string(),
                billing_period: "monthly".to_string(),
                trial_days: None,
                price_amount: 999,
                currency: "usd".to_string(),
                plan_name: "Pro Plan".to_string(),
                realm_id: "realm-1".to_string(),
                webhook_url: None,
                metadata: Some(std::collections::HashMap::from([(
                    "source".to_string(),
                    "demo".to_string(),
                )])),
            })
            .await
            .expect("mock checkout session should be created");

        assert_eq!(result.id, format!("cs_mock_{short_id}"));
        assert_eq!(
            result.url,
            format!("mock://stripe/checkout/cs_mock_{short_id}")
        );
        assert!(result.customer.is_none());
        assert_eq!(result.status.as_deref(), Some("open"));
        assert_eq!(result.payment_intent, Some(format!("pi_mock_{short_id}")));
        assert!(result.subscription.is_none());
        assert_eq!(result.metadata["source"], "demo");
    }

    /// Verifies that create_checkout_session sends form-encoded data (not JSON),
    /// matching Stripe's requirement for application/x-www-form-urlencoded.
    #[tokio::test]
    async fn test_create_checkout_session_sends_form_encoded() {
        let mock_server = MockServer::start().await;
        let client =
            StripeClient::with_base_url("sk_test_123".to_string(), mock_server.uri(), 30).unwrap();

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "cs_test_123",
                "url": "https://checkout.stripe.com/test",
                "customer": null,
                "status": "open",
                "payment_intent": "pi_test_123",
                "subscription": null,
                "metadata": { "realm_id": "realm-1", "source": "web" }
            })))
            .mount(&mock_server)
            .await;

        let plan_id = uuid::Uuid::now_v7();
        let result = client
            .create_checkout_session(&CreateCheckoutRequest {
                client_app_id: uuid::Uuid::now_v7(),
                plan_id,
                user_id: Some(uuid::Uuid::now_v7()),
                customer_email: Some("buyer@example.com".to_string()),
                success_url: "https://example.com/success".to_string(),
                cancel_url: "https://example.com/cancel".to_string(),
                billing_period: "monthly".to_string(),
                trial_days: Some(14),
                price_amount: 1999,
                currency: "usd".to_string(),
                plan_name: "Pro Plan".to_string(),
                realm_id: "realm-1".to_string(),
                webhook_url: None,
                metadata: Some(std::collections::HashMap::from([(
                    "source".to_string(),
                    "web".to_string(),
                )])),
            })
            .await
            .expect("checkout session should be created");

        assert_eq!(result.id, "cs_test_123");
        assert_eq!(result.url, "https://checkout.stripe.com/test");
        assert_eq!(result.status.as_deref(), Some("open"));

        let requests = mock_server
            .received_requests()
            .await
            .expect("request recording should be enabled");
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method.as_str(), "POST");
        assert_eq!(requests[0].url.path(), "/v1/checkout/sessions");

        // Parse form body to verify form-encoding (not JSON)
        let form: std::collections::HashMap<_, _> = url::form_urlencoded::parse(&requests[0].body)
            .into_owned()
            .collect();

        assert_eq!(
            requests[0]
                .headers
                .get("authorization")
                .and_then(|value| value.to_str().ok()),
            Some("Bearer sk_test_123")
        );
        // Verify key fields are form-encoded
        assert_eq!(form.get("mode"), Some(&"subscription".to_string()));
        assert_eq!(
            form.get("success_url"),
            Some(&"https://example.com/success".to_string())
        );
        assert_eq!(
            form.get("cancel_url"),
            Some(&"https://example.com/cancel".to_string())
        );
        assert_eq!(
            form.get("customer_email"),
            Some(&"buyer@example.com".to_string())
        );
        // Metadata fields
        assert_eq!(form.get("metadata[realm_id]"), Some(&"realm-1".to_string()));
        assert_eq!(
            form.get("metadata[plan_name]"),
            Some(&"Pro Plan".to_string())
        );
        assert_eq!(form.get("metadata[source]"), Some(&"web".to_string()));
        // Line items
        assert_eq!(
            form.get("line_items[0][price_data][currency]"),
            Some(&"usd".to_string())
        );
        assert_eq!(
            form.get("line_items[0][price_data][unit_amount]"),
            Some(&"1999".to_string())
        );
        assert_eq!(
            form.get("line_items[0][price_data][recurring][interval]"),
            Some(&"month".to_string())
        );
        assert_eq!(form.get("line_items[0][quantity]"), Some(&"1".to_string()));
        // Trial period
        assert_eq!(
            form.get("subscription_data[trial_period_days]"),
            Some(&"14".to_string())
        );
    }

    #[tokio::test]
    async fn test_create_payment_intent_rejects_invalid_amount_before_request() {
        let mock_server = MockServer::start().await;
        let client =
            StripeClient::with_base_url("sk_test_123".to_string(), mock_server.uri(), 30).unwrap();

        let result = client
            .create_payment_intent(&CreatePaymentIntentRequest {
                amount: 0,
                currency: "usd".to_string(),
                receipt_email: None,
                metadata: std::collections::HashMap::new(),
            })
            .await;

        assert!(matches!(result, Err(CoreError::BadRequest(_))));
    }
}
