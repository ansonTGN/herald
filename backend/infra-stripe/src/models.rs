use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Stripe Checkout Session response
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CheckoutSession {
    /// Stripe checkout session ID
    pub id: String,
    /// URL to redirect user to for payment
    pub url: String,
    /// Customer ID (if created)
    pub customer: Option<String>,
    /// Payment status
    pub status: Option<String>,
    /// Payment intent ID
    pub payment_intent: Option<String>,
    /// Subscription ID (for subscription checkout)
    pub subscription: Option<String>,
    /// Metadata
    pub metadata: serde_json::Value,
}

/// Request to create a Stripe checkout session
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateCheckoutRequest {
    /// Client app ID
    #[serde(rename = "clientAppId")]
    pub client_app_id: Uuid,
    /// Herald entitlement mapping ID
    #[serde(rename = "mappingId")]
    pub mapping_id: Uuid,
    /// User ID (for subscription tracking)
    #[serde(rename = "userId")]
    pub user_id: Option<Uuid>,
    /// Customer email for checkout prefill and payment provider requirements
    #[serde(rename = "customerEmail", skip_serializing_if = "Option::is_none")]
    pub customer_email: Option<String>,
    /// Success URL
    #[serde(rename = "successUrl")]
    pub success_url: String,
    /// Cancel URL
    #[serde(rename = "cancelUrl")]
    pub cancel_url: String,
    /// Billing period (monthly/yearly)
    #[serde(rename = "billingPeriod")]
    pub billing_period: String,
    /// Trial days (optional)
    #[serde(rename = "trialDays")]
    pub trial_days: Option<u32>,
    /// Plan price amount (in cents)
    #[serde(rename = "priceAmount")]
    pub price_amount: i64,
    /// Plan currency (e.g., "usd")
    #[serde(rename = "currency")]
    pub currency: String,
    /// Plan name
    #[serde(rename = "planName")]
    pub plan_name: String,
    /// Realm ID (for webhook routing)
    #[serde(rename = "realmId")]
    pub realm_id: String,
    /// Webhook URL (optional, for realm-specific webhooks)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook_url: Option<String>,
    /// Extra metadata to merge into the Stripe checkout session metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

/// Request to create a Stripe payment intent.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreatePaymentIntentRequest {
    /// Amount in the smallest currency unit (for example cents)
    pub amount: i64,
    /// ISO 4217 currency code
    pub currency: String,
    /// Customer email for receipt and payment provider requirements
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt_email: Option<String>,
    /// Provider metadata stored on the payment intent
    pub metadata: std::collections::HashMap<String, String>,
}

/// Stripe Payment Intent response.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PaymentIntent {
    pub id: String,
    pub client_secret: String,
    pub amount: i64,
    pub currency: String,
    pub status: Option<String>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// Stripe webhook event envelope
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StripeWebhookEvent {
    /// Event ID
    pub id: String,
    /// Event object (contains the actual data)
    pub object: serde_json::Value,
    /// Event type (e.g., "checkout.session.completed")
    #[serde(rename = "type")]
    pub event_type: String,
    /// API version
    #[serde(rename = "api_version")]
    pub api_version: Option<String>,
    /// Created timestamp
    pub created: i64,
    /// Request ID (for idempotency)
    pub request: Option<serde_json::Value>,
}
