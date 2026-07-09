// Purchase-specific error types

use crate::common::entities::app_errors::CoreError;

/// Marker prefix carried on `CoreError::Conflict` to signal that a one-time+role
/// entitlement is already owned by the buyer (M3 anti-repeat, design §5.4).
///
/// The codebase has no dedicated `PurchaseError` enum — `PurchaseResult<T> =
/// Result<T, CoreError>` — so the already-owned signal rides on
/// `CoreError::Conflict("<MARKER><entitlement_key>")`. The API handler parses
/// this prefix to emit a structured 409 body `{ "code": "already_owned",
/// "entitlementKey": <key> }` instead of the generic conflict message.
pub const ALREADY_OWNED_MARKER: &str = "already_owned:";

/// Trait for Purchase-specific error extensions
pub trait PurchaseErrorExt {
    /// Subscription plan not found
    fn plan_not_found(plan_id: &str) -> Self;

    /// Fulfillment already completed for payment attempt
    fn already_fulfilled(attempt_id: &str) -> Self;

    /// One-time role entitlement already owned by the buyer (M3 anti-repeat).
    /// Carries `ALREADY_OWNED_MARKER` + `entitlement_key` on `CoreError::Conflict`
    /// so the API handler can map it to a structured 409 `already_owned` body.
    fn already_owned(entitlement_key: &str) -> Self;

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

    fn already_owned(entitlement_key: &str) -> Self {
        tracing::debug!(
            "One-time role entitlement already owned: {}",
            entitlement_key
        );
        CoreError::Conflict(format!("{ALREADY_OWNED_MARKER}{entitlement_key}"))
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
