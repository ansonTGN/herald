// Payment Attempt repository ports

use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use super::entities::PurchaseHistoryRow;
use super::entities::{PaymentAttempt, PaymentAttemptStatus};
use crate::common::entities::app_errors::CoreError;

/// Input for creating a payment attempt
#[derive(Debug, Clone)]
pub struct CreatePaymentAttemptInput {
    pub realm_id: String,
    pub user_id: Uuid,
    pub payment_provider: String,
    pub target_type: String, // "entitlement_mapping" (legacy values "subscription_entitlement" and "points_package" are accepted)
    pub target_id: Uuid,
    pub bucket_id: Uuid,
    pub amount: i64,
    pub currency: String,
    pub provider_reference: Option<String>,
    pub metadata: Option<Value>,
}

/// Repository trait for PaymentAttempt operations
#[allow(async_fn_in_trait)]
pub trait PaymentAttemptRepository: Send + Sync {
    /// Create a new payment attempt
    async fn create_payment_attempt(
        &self,
        input: CreatePaymentAttemptInput,
    ) -> Result<PaymentAttempt, CoreError>;

    /// Find a payment attempt by ID
    async fn find_payment_attempt_by_id(
        &self,
        realm_id: &str,
        attempt_id: Uuid,
    ) -> Result<Option<PaymentAttempt>, CoreError>;

    /// Find a payment attempt by ID only (without realm filter)
    /// Used for webhook handlers where realm is not known upfront
    async fn find_payment_attempt_by_id_only(
        &self,
        attempt_id: Uuid,
    ) -> Result<Option<PaymentAttempt>, CoreError>;

    /// Find payment attempts by user (paginated)
    async fn find_payment_attempts_by_user(
        &self,
        realm_id: &str,
        user_id: Uuid,
        limit: u64,
    ) -> Result<Vec<PaymentAttempt>, CoreError>;

    /// Find a payment attempt by provider reference (for webhooks)
    async fn find_payment_attempt_by_provider_reference(
        &self,
        provider: &str,
        reference: &str,
    ) -> Result<Option<PaymentAttempt>, CoreError>;

    /// Update a payment attempt
    async fn update_payment_attempt(
        &self,
        attempt: PaymentAttempt,
    ) -> Result<PaymentAttempt, CoreError>;

    /// Update a payment attempt only if its current status still matches the
    /// status observed by the caller.
    async fn update_payment_attempt_with_status_guard(
        &self,
        attempt: PaymentAttempt,
        expected_status: PaymentAttemptStatus,
    ) -> Result<PaymentAttempt, CoreError>;

    /// List expired attempts (for cleanup)
    async fn list_expired_attempts(
        &self,
        before: DateTime<Utc>,
    ) -> Result<Vec<PaymentAttempt>, CoreError>;

    /// List purchase history for a user with filters and pagination
    async fn list_purchase_history(
        &self,
        realm_id: &str,
        user_id: Uuid,
        payment_provider: Option<&str>,
        start_date: Option<&str>,
        end_date: Option<&str>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<PurchaseHistoryRow>, i64), CoreError>;
}
