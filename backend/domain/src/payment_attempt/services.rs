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

        // Update status to Cancelled
        attempt.status = target_status;
        attempt.updated_at = chrono::Utc::now();

        self.repository.update_payment_attempt(attempt).await
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

        // Update status to Succeeded
        attempt.status = target_status;
        attempt.provider_status = Some(provider_status);
        attempt.provider_reference = Some(provider_transaction_id);
        attempt.completed_at = Some(completed_at);
        attempt.updated_at = chrono::Utc::now();

        self.repository.update_payment_attempt(attempt).await
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

        // Update status to Failed
        attempt.status = target_status;
        attempt.provider_status = Some(provider_status);
        attempt.completed_at = Some(completed_at);
        attempt.updated_at = chrono::Utc::now();

        self.repository.update_payment_attempt(attempt).await
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
            attempt.status = PaymentAttemptStatus::Expired;
            attempt.updated_at = chrono::Utc::now();

            let updated_attempt = self.repository.update_payment_attempt(attempt).await?;
            updated.push(updated_attempt);
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
    }

    fn create_test_attempt(status: PaymentAttemptStatus) -> PaymentAttempt {
        let now = Utc::now();
        PaymentAttempt {
            id: Uuid::now_v7(),
            realm_id: "test-realm".to_string(),
            user_id: Uuid::now_v7(),
            payment_provider: "stripe".to_string(),
            target_type: PurchasableTarget::PointsPackage,
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
}
