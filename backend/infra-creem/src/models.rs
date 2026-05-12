use serde::{Deserialize, Serialize};

/// Request to create a Creem checkout session
#[derive(Debug, Clone, Serialize)]
pub struct CreateCheckoutRequest {
    pub product_id: String,
    pub success_url: String,
    pub cancel_url: String,
    pub customer_email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,
    /// Webhook URL (optional, for realm-specific webhooks)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook_url: Option<String>,
}

/// Response from Creem checkout session creation
#[derive(Debug, Clone, Deserialize)]
pub struct CheckoutSession {
    pub id: String,
    pub url: String,
    pub status: String,
}

/// Creem webhook event
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreemWebhookEvent {
    pub id: String,
    #[serde(rename = "eventType")]
    pub event_type: String,
    pub object: serde_json::Value,
}

/// Creem subscription object from webhook
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreemSubscription {
    pub id: String,
    pub customer: CreemCustomer,
    pub product: CreemProduct,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_period_start: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_period_end: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel_at_period_end: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreemCustomer {
    pub email: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreemProduct {
    pub id: String,
    pub name: String,
    pub price: i64,
    pub currency: String,
    #[serde(rename = "billing_type")]
    pub billing_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_period: Option<String>,
}

/// Creem dispute object from webhook
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreemDispute {
    pub id: String,
    pub subscription_id: String,
    pub status: String,
    pub amount: i64,
    pub currency: String,
    pub reason: String,
    pub created_at: i64,
}

/// Creem refund object from webhook
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreemRefund {
    pub id: String,
    pub subscription_id: String,
    pub amount: i64,
    pub currency: String,
    pub reason: Option<String>,
    pub created_at: i64,
}
