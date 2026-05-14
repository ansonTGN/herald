//! Webhook Service
//!
//! Provides high-level webhook processing logic with idempotency handling
//! and transaction management. This service encapsulates the common pattern
//! of processing webhooks with automatic duplicate detection and error handling.

use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::common::entities::app_errors::CoreError;
use crate::infrastructure::webhook::{IdempotencyResult, WebhookEventRepository};

/// Webhook processing context
///
/// Contains all information needed to process a webhook event.
#[derive(Debug, Clone)]
pub struct WebhookContext {
    /// Realm ID for the webhook
    pub realm_id: String,
    /// External event ID from payment provider
    pub external_event_id: String,
    /// Payment provider (e.g., "shopify", "stripe", "creem")
    pub payment_provider: String,
    /// Event type (e.g., "subscription_contracts/create")
    pub event_type: String,
    /// Event payload as JSON
    pub payload: Value,
}

/// Result of webhook processing
#[derive(Debug)]
pub enum WebhookProcessResult {
    /// Event was processed successfully
    Processed { event_id: Uuid },
    /// Event was already processed, skipped (idempotent)
    Skipped { event_id: Uuid },
    /// Event is currently being processed by another request
    InProgress { event_id: Uuid },
}

/// Webhook Service
///
/// High-level service for processing webhooks with built-in idempotency
/// and transaction management.
pub struct WebhookService {
    event_repository: Arc<WebhookEventRepository>,
}

impl WebhookService {
    pub fn new(event_repository: Arc<WebhookEventRepository>) -> Self {
        Self { event_repository }
    }

    /// Process a webhook event with idempotency handling
    ///
    /// This method:
    /// 1. Creates a payment event record (or finds existing one)
    /// 2. Checks if the event has already been processed
    /// 3. If claimable, acquires a short processing lease
    /// 4. Executes the provided handler function outside the transaction
    /// 5. Marks the event as processed or releases the lease on failure
    ///
    /// # Arguments
    /// * `context` - The webhook context containing event information
    /// * `handler` - The business logic to execute for new events
    ///
    /// # Returns
    /// * `WebhookProcessResult::Processed` if the event was claimed and processed
    /// * `WebhookProcessResult::Skipped` if the event was already processed
    /// * `WebhookProcessResult::InProgress` if another request already claimed the event
    ///
    /// # Errors
    /// Returns an error if database operations fail or the handler fails.
    pub async fn process_webhook_with_idempotency<F, Fut>(
        &self,
        context: WebhookContext,
        handler: F,
    ) -> Result<WebhookProcessResult, CoreError>
    where
        F: Fn(&WebhookContext) -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<(), CoreError>> + Send,
    {
        // Begin transaction
        let mut tx = self.event_repository.begin_transaction().await?;

        // Create event and check idempotency
        let idempotency_result = self
            .event_repository
            .create_event_with_idempotency_check(
                &mut tx,
                &context.realm_id,
                &context.external_event_id,
                &context.payment_provider,
                &context.event_type,
                &context.payload,
            )
            .await?;

        match idempotency_result {
            IdempotencyResult::Claimed { event_id } => {
                tx.commit()
                    .await
                    .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

                let handler_result = handler(&context).await;

                match handler_result {
                    Ok(()) => {
                        self.event_repository.mark_event_processed(event_id).await?;
                        Ok(WebhookProcessResult::Processed { event_id })
                    }
                    Err(err) => {
                        let _ = self.event_repository.mark_event_failed(event_id).await;
                        Err(err)
                    }
                }
            }
            IdempotencyResult::AlreadyProcessed { event_id } => {
                tx.commit()
                    .await
                    .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

                Ok(WebhookProcessResult::Skipped { event_id })
            }
            IdempotencyResult::InProgress { event_id } => {
                tx.commit()
                    .await
                    .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

                Ok(WebhookProcessResult::InProgress { event_id })
            }
        }
    }

    /// Check if an event has been processed
    ///
    /// This is a convenience method for checking event processing status
    /// without starting a transaction.
    ///
    /// # Arguments
    /// * `external_event_id` - The external event ID from payment provider
    /// * `payment_provider` - The payment provider (e.g., "shopify", "stripe", "creem")
    ///
    /// # Returns
    /// * `true` if the event has been processed
    /// * `false` if the event has not been processed or does not exist
    pub async fn is_event_processed(
        &self,
        external_event_id: &str,
        payment_provider: &str,
    ) -> Result<bool, CoreError> {
        self.event_repository
            .is_event_processed(external_event_id, payment_provider)
            .await
    }
}
