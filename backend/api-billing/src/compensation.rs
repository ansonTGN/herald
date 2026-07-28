use std::pin::Pin;

use herald_api_base::application::http::state::AppState;
use herald_core::domain::billing::compensation::WebhookEventProcessor;
use herald_core::domain::common::entities::app_errors::CoreError;
use serde_json::Value;

pub struct WebhookEventProcessorImpl {
    app_state: AppState,
}

impl WebhookEventProcessorImpl {
    pub fn new(app_state: AppState) -> Self {
        Self { app_state }
    }
}

impl WebhookEventProcessor for WebhookEventProcessorImpl {
    fn reprocess_event<'a>(
        &'a self,
        realm_id: &'a str,
        payment_provider: &'a str,
        event_type: &'a str,
        payload: &'a Value,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<(), CoreError>> + Send + 'a>> {
        match payment_provider {
            "stripe" => Box::pin(crate::stripe_webhook_handlers::reprocess_stripe_event(
                self.app_state.clone(),
                realm_id,
                payload,
                event_type,
            )),
            "creem" => Box::pin(crate::webhook_handlers::reprocess_creem_event(
                self.app_state.clone(),
                realm_id,
                payload,
                event_type,
            )),
            // IAP compensation (design support-iap §5.7). BE-D03 ships only the
            // skeleton signatures; the full "lookup + replay" implementation
            // (Apple getNotificationHistory / Google subscriptionsv2.get polling)
            // is BE-D04. The signatures here are the contract BE-D04 must
            // preserve (job + compensation both depend on them).
            "apple" => Box::pin(crate::iap_handlers::reprocess_apple_event(
                self.app_state.clone(),
                realm_id.to_string(),
                payload.clone(),
                event_type.to_string(),
            )),
            "google" => Box::pin(crate::iap_handlers::reprocess_google_event(
                self.app_state.clone(),
                realm_id.to_string(),
                payload.clone(),
                event_type.to_string(),
            )),
            _ => Box::pin(async move {
                Err(CoreError::BadRequest(format!(
                    "unsupported payment provider: {}",
                    payment_provider
                )))
            }),
        }
    }
}
