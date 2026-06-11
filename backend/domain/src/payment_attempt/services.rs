// Payment Attempt domain service

use std::sync::Arc;

use super::entities::{PaymentAttempt, PaymentAttemptStatus, PaymentContext};
use super::errors::{PaymentAttemptErrorExt, PaymentAttemptResult};
use super::ports::{CreatePaymentAttemptInput, PaymentAttemptRepository};
use crate::common::entities::app_errors::CoreError;

/// Payment Attempt service
pub struct PaymentAttemptService<R: PaymentAttemptRepository> {
    repository: Arc<R>,
}

impl<R: PaymentAttemptRepository> PaymentAttemptService<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    /// Create a new payment attempt
    pub async fn create_payment_attempt(
        &self,
        input: CreatePaymentAttemptInput,
        payment_context: PaymentContext,
    ) -> PaymentAttemptResult<(PaymentAttempt, PaymentContext)> {
        // Create payment attempt with 2-hour expiration
        let now = chrono::Utc::now();
        let _expires_at = now + chrono::Duration::hours(2);

        let attempt_input = CreatePaymentAttemptInput {
            realm_id: input.realm_id.clone(),
            user_id: input.user_id,
            payment_provider: input.payment_provider.clone(),
            target_type: input.target_type.clone(),
            target_id: input.target_id,
            amount: input.amount,
            currency: input.currency.clone(),
            provider_reference: input.provider_reference,
            metadata: input.metadata,
        };

        let attempt = self
            .repository
            .create_payment_attempt(attempt_input)
            .await?;

        Ok((attempt, payment_context))
    }

    /// Get payment attempt status
    pub async fn get_payment_attempt_status(
        &self,
        realm_id: &str,
        attempt_id: uuid::Uuid,
        requesting_user_id: uuid::Uuid,
    ) -> PaymentAttemptResult<PaymentAttempt> {
        let attempt = self
            .repository
            .find_payment_attempt_by_id(realm_id, attempt_id)
            .await?
            .ok_or_else(|| CoreError::attempt_not_found(&attempt_id.to_string()))?;

        // Verify user owns this attempt
        if attempt.user_id != requesting_user_id {
            return Err(CoreError::Forbidden(
                "You can only view your own payment attempts".to_string(),
            ));
        }

        Ok(attempt)
    }

    /// Find a payment attempt by ID within a specific realm (no ownership check).
    /// Used for webhook handlers that have realm_id but cannot verify user ownership.
    pub async fn find_payment_attempt(
        &self,
        realm_id: &str,
        attempt_id: uuid::Uuid,
    ) -> PaymentAttemptResult<Option<PaymentAttempt>> {
        self.repository
            .find_payment_attempt_by_id(realm_id, attempt_id)
            .await
    }

    /// Get payment attempt by ID only (without realm filter)
    /// Used for webhook handlers where realm is not known upfront
    pub async fn get_payment_attempt_by_id_only(
        &self,
        attempt_id: uuid::Uuid,
    ) -> PaymentAttemptResult<PaymentAttempt> {
        self.repository
            .find_payment_attempt_by_id_only(attempt_id)
            .await?
            .ok_or_else(|| CoreError::attempt_not_found(&attempt_id.to_string()))
    }

    /// Find a payment attempt by provider reference for provider-originated callbacks.
    pub async fn get_payment_attempt_by_provider_reference(
        &self,
        provider: &str,
        reference: &str,
    ) -> PaymentAttemptResult<Option<PaymentAttempt>> {
        self.repository
            .find_payment_attempt_by_provider_reference(provider, reference)
            .await
    }

    /// Cancel a payment attempt
    pub async fn cancel_payment_attempt(
        &self,
        realm_id: &str,
        attempt_id: uuid::Uuid,
        user_id: uuid::Uuid,
    ) -> PaymentAttemptResult<PaymentAttempt> {
        let mut attempt = self
            .get_payment_attempt_status(realm_id, attempt_id, user_id)
            .await?;

        // Validate state transition using can_transition_to
        let target_status = PaymentAttemptStatus::Cancelled;
        if !attempt.status.can_transition_to(&target_status) {
            return Err(CoreError::invalid_status_transition(
                &attempt.status.to_string(),
                &target_status.to_string(),
            ));
        }

        let original_status = attempt.status.clone();

        // Update status to Cancelled
        attempt.status = target_status;
        attempt.updated_at = chrono::Utc::now();

        self.repository
            .update_payment_attempt_with_status_guard(attempt, original_status)
            .await
    }

    /// Mark payment as succeeded (for webhook processing)
    pub async fn mark_payment_succeeded(
        &self,
        realm_id: &str,
        attempt_id: uuid::Uuid,
        provider_status: String,
        provider_transaction_id: String,
        completed_at: chrono::DateTime<chrono::Utc>,
    ) -> PaymentAttemptResult<PaymentAttempt> {
        let mut attempt = self
            .repository
            .find_payment_attempt_by_id(realm_id, attempt_id)
            .await?
            .ok_or_else(|| CoreError::attempt_not_found(&attempt_id.to_string()))?;

        // Validate state transition using can_transition_to
        let target_status = PaymentAttemptStatus::Succeeded;
        if !attempt.status.can_transition_to(&target_status) {
            return Err(CoreError::invalid_status_transition(
                &attempt.status.to_string(),
                &target_status.to_string(),
            ));
        }

        let original_status = attempt.status.clone();

        // Update status to Succeeded
        attempt.status = target_status;
        attempt.provider_status = Some(provider_status);
        attempt.provider_reference = Some(provider_transaction_id);
        attempt.completed_at = Some(completed_at);
        attempt.updated_at = chrono::Utc::now();

        self.repository
            .update_payment_attempt_with_status_guard(attempt, original_status)
            .await
    }

    /// Mark payment as failed (for webhook processing)
    pub async fn mark_payment_failed(
        &self,
        realm_id: &str,
        attempt_id: uuid::Uuid,
        provider_status: String,
        completed_at: chrono::DateTime<chrono::Utc>,
    ) -> PaymentAttemptResult<PaymentAttempt> {
        let mut attempt = self
            .repository
            .find_payment_attempt_by_id(realm_id, attempt_id)
            .await?
            .ok_or_else(|| CoreError::attempt_not_found(&attempt_id.to_string()))?;

        // Validate state transition using can_transition_to
        let target_status = PaymentAttemptStatus::Failed;
        if !attempt.status.can_transition_to(&target_status) {
            return Err(CoreError::invalid_status_transition(
                &attempt.status.to_string(),
                &target_status.to_string(),
            ));
        }

        let original_status = attempt.status.clone();

        // Update status to Failed
        attempt.status = target_status;
        attempt.provider_status = Some(provider_status);
        attempt.completed_at = Some(completed_at);
        attempt.updated_at = chrono::Utc::now();

        self.repository
            .update_payment_attempt_with_status_guard(attempt, original_status)
            .await
    }

    /// Mark a succeeded payment attempt as failed for async payment recovery.
    /// This is used when eager strategy issued points on checkout.session.completed
    /// but async_payment_failed arrives — the attempt must transition Succeeded -> Failed
    /// to enable points revocation downstream.
    pub async fn mark_failed_for_async_recovery(
        &self,
        realm_id: &str,
        attempt_id: uuid::Uuid,
        provider_status: String,
        completed_at: chrono::DateTime<chrono::Utc>,
    ) -> PaymentAttemptResult<PaymentAttempt> {
        let mut attempt = self
            .repository
            .find_payment_attempt_by_id(realm_id, attempt_id)
            .await?
            .ok_or_else(|| CoreError::attempt_not_found(&attempt_id.to_string()))?;

        if !attempt.status.can_transition_to_failed_for_async_recovery() {
            return Err(CoreError::invalid_status_transition(
                &attempt.status.to_string(),
                &PaymentAttemptStatus::Failed.to_string(),
            ));
        }

        let original_status = attempt.status.clone();

        attempt.status = PaymentAttemptStatus::Failed;
        attempt.provider_status = Some(provider_status);
        attempt.completed_at = Some(completed_at);
        attempt.updated_at = chrono::Utc::now();

        self.repository
            .update_payment_attempt_with_status_guard(attempt, original_status)
            .await
    }

    /// Update provider reference after the upstream payment object has been created.
    pub async fn update_provider_reference(
        &self,
        realm_id: &str,
        attempt_id: uuid::Uuid,
        provider_reference: Option<String>,
    ) -> PaymentAttemptResult<PaymentAttempt> {
        let mut attempt = self
            .repository
            .find_payment_attempt_by_id(realm_id, attempt_id)
            .await?
            .ok_or_else(|| CoreError::attempt_not_found(&attempt_id.to_string()))?;

        attempt.provider_reference = provider_reference;
        attempt.updated_at = chrono::Utc::now();

        self.repository.update_payment_attempt(attempt).await
    }

    /// Mark expired payment attempts
    pub async fn mark_expired_attempts(
        &self,
        before: chrono::DateTime<chrono::Utc>,
    ) -> PaymentAttemptResult<Vec<PaymentAttempt>> {
        let expired_attempts = self.repository.list_expired_attempts(before).await?;

        let mut updated = Vec::new();
        for mut attempt in expired_attempts {
            let target_status = PaymentAttemptStatus::Expired;
            if !attempt.status.can_transition_to(&target_status) {
                tracing::warn!(
                    attempt_id = %attempt.id,
                    current_status = %attempt.status,
                    "Skipping expired attempt: concurrent status change detected"
                );
                continue;
            }

            let original_status = attempt.status.clone();
            let attempt_id = attempt.id;
            attempt.status = target_status;
            attempt.updated_at = chrono::Utc::now();

            match self
                .repository
                .update_payment_attempt_with_status_guard(attempt, original_status)
                .await
            {
                Ok(updated_attempt) => updated.push(updated_attempt),
                Err(e) => {
                    tracing::warn!(
                        attempt_id = %attempt_id,
                        error = %e,
                        "Skipping expired attempt: status guard conflict"
                    );
                    continue;
                }
            }
        }

        Ok(updated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::payment_attempt::entities::{
        PaymentAttempt, PaymentAttemptStatus, PurchasableTarget,
    };
    use crate::payment_attempt::ports::{CreatePaymentAttemptInput, PaymentAttemptRepository};
    use chrono::Utc;
    use std::sync::Arc;
    use uuid::Uuid;

    // Mock repository for testing
    struct MockPaymentAttemptRepository {
        attempts: std::sync::Mutex<Vec<PaymentAttempt>>,
    }

    impl MockPaymentAttemptRepository {
        fn new() -> Self {
            Self {
                attempts: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn add_attempt(&self, attempt: PaymentAttempt) {
            self.attempts.lock().unwrap().push(attempt);
        }

        fn set_status(&self, attempt_id: Uuid, status: PaymentAttemptStatus) {
            let mut attempts = self.attempts.lock().unwrap();
            let attempt = attempts
                .iter_mut()
                .find(|a| a.id == attempt_id)
                .expect("test attempt should exist");
            attempt.status = status;
        }
    }

    impl PaymentAttemptRepository for MockPaymentAttemptRepository {
        async fn create_payment_attempt(
            &self,
            input: CreatePaymentAttemptInput,
        ) -> PaymentAttemptResult<PaymentAttempt> {
            let attempt = PaymentAttempt {
                id: Uuid::now_v7(),
                realm_id: input.realm_id,
                user_id: input.user_id,
                payment_provider: input.payment_provider,
                target_type: input.target_type.parse()?, // Parse String to PurchasableTarget
                target_id: input.target_id,
                amount: input.amount,
                currency: input.currency,
                status: PaymentAttemptStatus::Pending,
                provider_reference: input.provider_reference,
                provider_status: None,
                metadata: input.metadata,
                expires_at: Utc::now() + chrono::Duration::hours(2),
                completed_at: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };
            self.attempts.lock().unwrap().push(attempt.clone());
            Ok(attempt)
        }

        async fn find_payment_attempt_by_id(
            &self,
            _realm_id: &str,
            attempt_id: Uuid,
        ) -> PaymentAttemptResult<Option<PaymentAttempt>> {
            Ok(self
                .attempts
                .lock()
                .unwrap()
                .iter()
                .find(|a| a.id == attempt_id)
                .cloned())
        }

        async fn find_payment_attempt_by_id_only(
            &self,
            attempt_id: Uuid,
        ) -> PaymentAttemptResult<Option<PaymentAttempt>> {
            Ok(self
                .attempts
                .lock()
                .unwrap()
                .iter()
                .find(|a| a.id == attempt_id)
                .cloned())
        }

        async fn update_payment_attempt(
            &self,
            attempt: PaymentAttempt,
        ) -> PaymentAttemptResult<PaymentAttempt> {
            let mut attempts = self.attempts.lock().unwrap();
            if let Some(existing) = attempts.iter_mut().find(|a| a.id == attempt.id) {
                *existing = attempt.clone();
            }
            Ok(attempt)
        }

        async fn update_payment_attempt_with_status_guard(
            &self,
            attempt: PaymentAttempt,
            expected_status: PaymentAttemptStatus,
        ) -> PaymentAttemptResult<PaymentAttempt> {
            let mut attempts = self.attempts.lock().unwrap();
            let existing = attempts
                .iter_mut()
                .find(|a| a.id == attempt.id)
                .ok_or_else(|| CoreError::attempt_not_found(&attempt.id.to_string()))?;

            if existing.status != expected_status {
                if existing.status == attempt.status {
                    return Ok(existing.clone());
                }

                return Err(CoreError::invalid_status_transition(
                    &expected_status.to_string(),
                    &existing.status.to_string(),
                ));
            }

            // Mirror production SQL: only update status + provider fields
            existing.status = attempt.status.clone();
            existing.provider_reference = attempt.provider_reference.clone();
            existing.provider_status = attempt.provider_status.clone();
            existing.completed_at = attempt.completed_at;
            existing.updated_at = attempt.updated_at;
            Ok(existing.clone())
        }

        async fn list_expired_attempts(
            &self,
            _before: chrono::DateTime<chrono::Utc>,
        ) -> PaymentAttemptResult<Vec<PaymentAttempt>> {
            Ok(Vec::new())
        }

        async fn find_payment_attempts_by_user(
            &self,
            _realm_id: &str,
            _user_id: Uuid,
            _limit: u64,
        ) -> PaymentAttemptResult<Vec<PaymentAttempt>> {
            Ok(Vec::new())
        }

        async fn find_payment_attempt_by_provider_reference(
            &self,
            _provider: &str,
            _reference: &str,
        ) -> PaymentAttemptResult<Option<PaymentAttempt>> {
            Ok(None)
        }

        async fn list_purchase_history(
            &self,
            _realm_id: &str,
            _user_id: uuid::Uuid,
            _payment_provider: Option<&str>,
            _start_date: Option<&str>,
            _end_date: Option<&str>,
            _page: u64,
            _page_size: u64,
        ) -> PaymentAttemptResult<(
            Vec<crate::payment_attempt::entities::PurchaseHistoryRow>,
            i64,
        )> {
            Ok((Vec::new(), 0))
        }
    }

    fn create_test_attempt(status: PaymentAttemptStatus) -> PaymentAttempt {
        let now = Utc::now();
        PaymentAttempt {
            id: Uuid::now_v7(),
            realm_id: "test-realm".to_string(),
            user_id: Uuid::now_v7(),
            payment_provider: "stripe".to_string(),
            target_type: PurchasableTarget::EntitlementMapping,
            target_id: Uuid::now_v7(),
            amount: 1000,
            currency: "USD".to_string(),
            status,
            provider_reference: None,
            provider_status: None,
            metadata: None,
            expires_at: now + chrono::Duration::hours(2),
            completed_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn test_valid_status_transitions_from_pending() {
        let pending = PaymentAttemptStatus::Pending;

        // All transitions from Pending should be valid
        assert!(pending.can_transition_to(&PaymentAttemptStatus::RequiresAction));
        assert!(pending.can_transition_to(&PaymentAttemptStatus::Succeeded));
        assert!(pending.can_transition_to(&PaymentAttemptStatus::Failed));
        assert!(pending.can_transition_to(&PaymentAttemptStatus::Cancelled));
        assert!(pending.can_transition_to(&PaymentAttemptStatus::Expired));

        // Idempotent transition
        assert!(pending.can_transition_to(&PaymentAttemptStatus::Pending));
    }

    #[test]
    fn test_valid_status_transitions_from_requires_action() {
        let requires_action = PaymentAttemptStatus::RequiresAction;

        // Can transition to terminal states
        assert!(requires_action.can_transition_to(&PaymentAttemptStatus::Succeeded));
        assert!(requires_action.can_transition_to(&PaymentAttemptStatus::Failed));
        assert!(requires_action.can_transition_to(&PaymentAttemptStatus::Cancelled));
        assert!(requires_action.can_transition_to(&PaymentAttemptStatus::Expired));

        // Idempotent transition
        assert!(requires_action.can_transition_to(&PaymentAttemptStatus::RequiresAction));

        // Cannot go back to Pending
        assert!(!requires_action.can_transition_to(&PaymentAttemptStatus::Pending));
    }

    #[test]
    fn test_invalid_status_transitions_from_terminal_states() {
        // Terminal states can only transition to themselves (idempotent)
        let terminal_states = vec![
            PaymentAttemptStatus::Succeeded,
            PaymentAttemptStatus::Failed,
            PaymentAttemptStatus::Cancelled,
            PaymentAttemptStatus::Expired,
        ];

        for terminal_state in terminal_states {
            // Can transition to itself
            assert!(terminal_state.can_transition_to(&terminal_state));

            // Cannot transition to any other state
            assert!(!terminal_state.can_transition_to(&PaymentAttemptStatus::Pending));
            assert!(!terminal_state.can_transition_to(&PaymentAttemptStatus::RequiresAction));

            // Cannot transition to other terminal states
            for other_state in &[
                PaymentAttemptStatus::Succeeded,
                PaymentAttemptStatus::Failed,
                PaymentAttemptStatus::Cancelled,
                PaymentAttemptStatus::Expired,
            ] {
                if terminal_state != *other_state {
                    assert!(
                        !terminal_state.can_transition_to(other_state),
                        "Terminal state {:?} should not be able to transition to {:?}",
                        terminal_state,
                        other_state
                    );
                }
            }
        }
    }

    #[tokio::test]
    async fn test_mark_failed_from_succeeded_fails() {
        let repo = Arc::new(MockPaymentAttemptRepository::new());
        let service = PaymentAttemptService::new(repo.clone());

        let mut attempt = create_test_attempt(PaymentAttemptStatus::Succeeded);
        attempt.updated_at = Utc::now();
        repo.add_attempt(attempt.clone());

        // Cannot mark succeeded attempt as failed
        let result = service
            .mark_payment_failed(
                &attempt.realm_id,
                attempt.id,
                "failed".to_string(),
                Utc::now(),
            )
            .await;

        assert!(result.is_err());
        match result {
            Err(CoreError::BadRequest(msg)) => {
                assert!(msg.contains("Invalid status transition"));
                assert!(msg.contains("Succeeded"));
                assert!(msg.contains("Failed"));
            }
            _ => panic!("Expected BadRequest error for invalid transition"),
        }
    }

    #[tokio::test]
    async fn test_cancel_succeeded_fails() {
        let repo = Arc::new(MockPaymentAttemptRepository::new());
        let service = PaymentAttemptService::new(repo.clone());

        let mut attempt = create_test_attempt(PaymentAttemptStatus::Succeeded);
        attempt.updated_at = Utc::now();
        repo.add_attempt(attempt.clone());

        // Cannot cancel succeeded attempt
        let result = service
            .cancel_payment_attempt(&attempt.realm_id, attempt.id, attempt.user_id)
            .await;

        assert!(result.is_err());
        match result {
            Err(CoreError::BadRequest(msg)) => {
                assert!(msg.contains("Invalid status transition"));
                assert!(msg.contains("Succeeded"));
                assert!(msg.contains("Cancelled"));
            }
            _ => panic!("Expected BadRequest error for invalid transition"),
        }
    }

    #[tokio::test]
    async fn test_mark_succeeded_from_expired_fails() {
        let repo = Arc::new(MockPaymentAttemptRepository::new());
        let service = PaymentAttemptService::new(repo.clone());

        let mut attempt = create_test_attempt(PaymentAttemptStatus::Expired);
        attempt.updated_at = Utc::now();
        repo.add_attempt(attempt.clone());

        // Cannot mark expired attempt as succeeded
        let result = service
            .mark_payment_succeeded(
                &attempt.realm_id,
                attempt.id,
                "completed".to_string(),
                "txn_123".to_string(),
                Utc::now(),
            )
            .await;

        assert!(result.is_err());
        match result {
            Err(CoreError::BadRequest(msg)) => {
                assert!(msg.contains("Invalid status transition"));
                assert!(msg.contains("Expired"));
                assert!(msg.contains("Succeeded"));
            }
            _ => panic!("Expected BadRequest error for invalid transition"),
        }
    }

    #[tokio::test]
    async fn test_mark_succeeded_updates_with_status_guard() {
        let repo = Arc::new(MockPaymentAttemptRepository::new());
        let service = PaymentAttemptService::new(repo.clone());

        let attempt = create_test_attempt(PaymentAttemptStatus::Pending);
        repo.add_attempt(attempt.clone());

        let completed_at = Utc::now();
        let result = service
            .mark_payment_succeeded(
                &attempt.realm_id,
                attempt.id,
                "paid".to_string(),
                "txn_guarded".to_string(),
                completed_at,
            )
            .await
            .expect("pending attempt should transition to succeeded");

        assert_eq!(result.status, PaymentAttemptStatus::Succeeded);
        assert_eq!(result.provider_status.as_deref(), Some("paid"));
        assert_eq!(result.provider_reference.as_deref(), Some("txn_guarded"));
        assert_eq!(result.completed_at, Some(completed_at));
    }

    #[tokio::test]
    async fn test_status_guard_rejects_concurrent_different_status() {
        let repo = MockPaymentAttemptRepository::new();
        let mut attempt = create_test_attempt(PaymentAttemptStatus::Pending);
        repo.add_attempt(attempt.clone());

        repo.set_status(attempt.id, PaymentAttemptStatus::Failed);

        attempt.status = PaymentAttemptStatus::Succeeded;
        let result = repo
            .update_payment_attempt_with_status_guard(attempt, PaymentAttemptStatus::Pending)
            .await;

        assert!(result.is_err());
        match result {
            Err(CoreError::BadRequest(msg)) => {
                assert!(msg.contains("Invalid status transition"));
                assert!(msg.contains("Pending"));
                assert!(msg.contains("Failed"));
            }
            _ => panic!("Expected BadRequest error for guarded update conflict"),
        }
    }

    #[tokio::test]
    async fn test_status_guard_treats_same_target_status_as_idempotent() {
        let repo = MockPaymentAttemptRepository::new();
        let mut attempt = create_test_attempt(PaymentAttemptStatus::Pending);
        repo.add_attempt(attempt.clone());

        repo.set_status(attempt.id, PaymentAttemptStatus::Succeeded);

        attempt.status = PaymentAttemptStatus::Succeeded;
        let result = repo
            .update_payment_attempt_with_status_guard(attempt, PaymentAttemptStatus::Pending)
            .await
            .expect("same target status should be idempotent");

        assert_eq!(result.status, PaymentAttemptStatus::Succeeded);
    }
}
