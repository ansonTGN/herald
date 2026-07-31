// Purchase fulfillment ports

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::common::entities::app_errors::CoreError;
use crate::payment_attempt::PaymentAttempt;

/// Result of a fulfillment operation.
///
/// `point_grants` is the multi-wallet grant set produced by the distribution
/// rule executor: zero to many grants, one per matched rule. Each entry carries
/// the rule id, the target bucket, the concrete result id (ledger / quota
/// entitlement / schedule), and the credit type. Quota grants report `points =
/// None` (their value is a rolling window, surfaced separately); an attempt
/// that matched no rules reports an empty array.
#[derive(Clone, Debug, PartialEq)]
pub struct FulfillmentResult {
    pub fulfillment_type: FulfillmentType,
    pub subscription_id: Option<Uuid>,
    pub point_grants: Vec<PointsGrant>,
    pub granted_at: DateTime<Utc>,
}

/// Type of fulfillment
#[derive(Clone, Debug, PartialEq)]
pub enum FulfillmentType {
    SubscriptionCreated,
    SubscriptionUpdated,
    PointsGranted,
}

/// One grant within a fulfillment's `point_grants` set. The `result_id` is the
/// concrete persisted artifact of the rule (a credit ledger id for fixed, a
/// quota entitlement id for quota, or a grant schedule id for free-periodic
/// fixed). `points` is `None` for quota grants and `Some(amount)` for fixed.
#[derive(Clone, Debug, PartialEq)]
pub struct PointsGrant {
    pub rule_id: Uuid,
    pub bucket_id: Uuid,
    pub result_id: Uuid,
    pub points_type: String, // "subscription_credit" / "topup_credit" / ...
    pub points: Option<i64>,
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
