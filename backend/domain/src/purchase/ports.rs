// Purchase fulfillment ports

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::common::entities::app_errors::CoreError;
use crate::payment_attempt::PaymentAttempt;

/// Result of a fulfillment operation
#[derive(Clone, Debug, PartialEq)]
pub struct FulfillmentResult {
    pub fulfillment_type: FulfillmentType,
    pub subscription_id: Option<Uuid>,
    pub points_granted: Option<PointsGrant>,
    pub granted_at: DateTime<Utc>,
}

/// Type of fulfillment
#[derive(Clone, Debug, PartialEq)]
pub enum FulfillmentType {
    SubscriptionCreated,
    SubscriptionUpdated,
    PointsGranted,
}

/// Points grant information
#[derive(Clone, Debug, PartialEq)]
pub struct PointsGrant {
    pub transaction_id: Uuid,
    pub points_type: String, // "subscription_credit" or "topup_credit"
    pub points: i64,
    pub description: String,
}

/// Service for fulfilling payment attempts
#[allow(async_fn_in_trait)]
pub trait FulfillmentService: Send + Sync {
    /// Fulfill a subscription purchase
    async fn fulfill_subscription_purchase(
        &self,
        attempt: &PaymentAttempt,
        provider_transaction_id: String,
    ) -> Result<FulfillmentResult, CoreError>;

    /// Fulfill a one-time purchase (e.g. topup_credit via entitlement mapping)
    async fn fulfill_one_time_purchase(
        &self,
        attempt: &PaymentAttempt,
        provider_transaction_id: String,
    ) -> Result<FulfillmentResult, CoreError>;

    /// (`current_period_end = now + mapping.service_duration_days`) that does
    /// not auto-renew. Idempotent on `external_subscription_id`.
    async fn fulfill_non_renewing_purchase(
        &self,
        attempt: &PaymentAttempt,
        provider_transaction_id: String,
    ) -> Result<FulfillmentResult, CoreError>;
}
