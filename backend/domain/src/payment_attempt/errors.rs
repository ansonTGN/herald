// Payment Attempt-specific error types

use crate::common::entities::app_errors::CoreError;

/// Trait for PaymentAttempt-specific error extensions
pub trait PaymentAttemptErrorExt {
    /// Payment attempt not found
    fn attempt_not_found(attempt_id: &str) -> Self;

    /// Payment attempt expired
    fn attempt_expired(attempt_id: &str) -> Self;

    /// Payment attempt already completed
    fn already_completed(attempt_id: &str) -> Self;

    /// Invalid payment attempt status transition
    fn invalid_status_transition(from: &str, to: &str) -> Self;

    /// Purchasable target not found
    fn target_not_found(target_type: &str, target_id: &str) -> Self;

    /// Payment provider not configured for target
    fn provider_not_configured(provider: &str) -> Self;

    /// Fulfillment failed
    fn fulfillment_failed(reason: &str) -> Self;
}

/// PaymentAttempt-specific error variants that extend CoreError
impl PaymentAttemptErrorExt for CoreError {
    fn attempt_not_found(attempt_id: &str) -> Self {
        tracing::debug!("Payment attempt not found: {}", attempt_id);
        CoreError::NotFound
    }

    fn attempt_expired(attempt_id: &str) -> Self {
        tracing::debug!("Payment attempt expired: {}", attempt_id);
        CoreError::BadRequest(format!("Payment attempt {} has expired", attempt_id))
    }

    fn already_completed(attempt_id: &str) -> Self {
        tracing::debug!("Payment attempt already completed: {}", attempt_id);
        CoreError::Conflict(format!(
            "Payment attempt {} is already completed",
            attempt_id
        ))
    }

    fn invalid_status_transition(from: &str, to: &str) -> Self {
        tracing::debug!(
            "Invalid payment attempt status transition: {} -> {}",
            from,
            to
        );
        CoreError::BadRequest(format!("Invalid status transition: {} -> {}", from, to))
    }

    fn target_not_found(target_type: &str, target_id: &str) -> Self {
        tracing::debug!(
            "Purchasable target not found: {}:{}",
            target_type,
            target_id
        );
        CoreError::NotFound
    }

    fn provider_not_configured(provider: &str) -> Self {
        tracing::debug!("Payment provider not configured: {}", provider);
        CoreError::BadRequest(format!(
            "Payment provider '{}' not configured for this target",
            provider
        ))
    }

    fn fulfillment_failed(reason: &str) -> Self {
        tracing::debug!("Fulfillment failed: {}", reason);
        CoreError::InternalServerError(format!("Payment fulfillment failed: {}", reason))
    }
}

/// Result type for PaymentAttempt operations
pub type PaymentAttemptResult<T> = Result<T, CoreError>;
