// Purchase-specific error types

use crate::common::entities::app_errors::CoreError;

/// Trait for Purchase-specific error extensions
pub trait PurchaseErrorExt {
    /// Subscription plan not found
    fn plan_not_found(plan_id: &str) -> Self;

    /// Points package not found
    fn package_not_found(package_id: &str) -> Self;

    /// Fulfillment already completed for payment attempt
    fn already_fulfilled(attempt_id: &str) -> Self;

    /// Failed to grant points
    fn points_grant_failed(reason: &str) -> Self;

    /// Failed to create subscription
    fn subscription_creation_failed(reason: &str) -> Self;
}

/// Purchase-specific error variants that extend CoreError
impl PurchaseErrorExt for CoreError {
    fn plan_not_found(plan_id: &str) -> Self {
        tracing::debug!("Subscription plan not found: {}", plan_id);
        CoreError::NotFound
    }

    fn package_not_found(package_id: &str) -> Self {
        tracing::debug!("Points package not found: {}", package_id);
        CoreError::NotFound
    }

    fn already_fulfilled(attempt_id: &str) -> Self {
        tracing::debug!(
            "Fulfillment already completed for payment attempt: {}",
            attempt_id
        );
        CoreError::Conflict(format!(
            "Payment attempt {} is already fulfilled",
            attempt_id
        ))
    }

    fn points_grant_failed(reason: &str) -> Self {
        tracing::debug!("Failed to grant points: {}", reason);
        CoreError::InternalServerError(format!("Points grant failed: {}", reason))
    }

    fn subscription_creation_failed(reason: &str) -> Self {
        tracing::debug!("Failed to create subscription: {}", reason);
        CoreError::InternalServerError(format!("Subscription creation failed: {}", reason))
    }
}

/// Result type for Purchase operations
pub type PurchaseResult<T> = Result<T, CoreError>;
