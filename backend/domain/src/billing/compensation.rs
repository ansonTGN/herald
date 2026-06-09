use serde_json::Value;

use crate::common::entities::app_errors::CoreError;

/// Processor for compensating missed webhook events.
///
/// Implemented by api-billing, consumed by Worker via trait object.
pub trait WebhookEventProcessor: Send + Sync {
    /// Reprocess a single webhook event that Herald missed.
    ///
    /// The implementation must:
    /// - Skip Redis idempotency checks (rely on DB `payment_event` only)
    /// - Skip HTTP-layer concerns (signature verification, header parsing)
    /// - Route to the correct handler based on payment_provider and event_type
    fn reprocess_event<'a>(
        &'a self,
        realm_id: &'a str,
        payment_provider: &'a str,
        event_type: &'a str,
        payload: &'a Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), CoreError>> + Send + 'a>>;
}
