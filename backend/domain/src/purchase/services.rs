// Purchase domain value types
// PurchaseService implementation moved to infrastructure/purchase/purchase_service.rs

use uuid::Uuid;

use crate::billing::entities::BillingType;
use crate::payment_attempt::PurchasableTarget;
use crate::payment_attempt::entities::{PaymentAttempt, PaymentContext};

#[derive(Clone, Debug, PartialEq)]
pub struct PreparePaymentAttemptInput {
    pub realm_id: String,
    pub user_id: Uuid,
    pub user_email: Option<String>,
    pub payment_provider: String,
    pub target_type: String,
    pub target_id: Uuid,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PurchaseTargetSnapshot {
    pub target_type: PurchasableTarget,
    pub target_id: Uuid,
    pub amount: i64,
    pub currency: String,
    pub title: String,
    pub provider_external_product_id: Option<String>,
    pub billing_period: Option<String>,
    pub billing_type: Option<BillingType>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreparedPaymentAttempt {
    pub attempt: PaymentAttempt,
    pub target: PurchaseTargetSnapshot,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreatedPaymentAttempt {
    pub attempt: PaymentAttempt,
    pub context: PaymentContext,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PaymentCompletionSource {
    InternalApi,
    ProviderWebhook { provider: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompletePaymentAttemptInput {
    pub attempt_id: Uuid,
    pub provider_status: String,
    pub provider_transaction_id: String,
    pub completed_at: chrono::DateTime<chrono::Utc>,
    pub source: PaymentCompletionSource,
    /// Override the billing_type read from the entitlement mapping.
    /// Used when the provider webhook carries billing_type metadata that
    /// should take precedence over the mapping's stored billing_type.
    pub billing_type_override: Option<BillingType>,
}
