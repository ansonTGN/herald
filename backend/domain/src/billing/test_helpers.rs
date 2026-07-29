// =============================================================================
// Billing Test Helpers
// =============================================================================
//
// Test helpers for billing-related tests.
//
// =============================================================================

use crate::billing::entities::BillingType;
use crate::billing::{PaymentEvent, Subscription, SubscriptionStatus};
use chrono::Utc;
use serde_json;
use uuid::Uuid;

// =============================================================================
// Builders
// =============================================================================

/// Builder for creating test Subscription instances
#[derive(Debug, Clone)]
pub struct SubscriptionBuilder {
    id: Option<Uuid>,
    realm_id: String,
    user_id: Uuid,
    external_subscription_id: String,
    external_product_id: String,
    payment_provider: String,
    status: SubscriptionStatus,
    entitlement_key: String,
    billing_type: BillingType,
    external_price_id: Option<String>,
    provider_metadata: Option<serde_json::Value>,
    synced_at: Option<chrono::DateTime<chrono::Utc>>,
    current_period_start: Option<chrono::DateTime<chrono::Utc>>,
    current_period_end: Option<chrono::DateTime<chrono::Utc>>,
    cancel_at_period_end: bool,
    client_app_id: Option<Uuid>,
    cancel_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for SubscriptionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SubscriptionBuilder {
    pub fn new() -> Self {
        Self {
            id: None,
            realm_id: "test-realm".to_string(),
            user_id: Uuid::now_v7(),
            external_subscription_id: "sub_test_123".to_string(),
            external_product_id: "prod_starter_monthly".to_string(),
            payment_provider: "creem".to_string(),
            status: SubscriptionStatus::Active,
            entitlement_key: "starter-plan".to_string(),
            billing_type: BillingType::Recurring,
            external_price_id: None,
            provider_metadata: None,
            synced_at: None,
            current_period_start: Some(Utc::now()),
            current_period_end: Some(Utc::now() + chrono::Duration::days(30)),
            cancel_at_period_end: false,
            client_app_id: None,
            cancel_at: None,
        }
    }

    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = Some(id);
        self
    }

    pub fn with_realm_id(mut self, realm_id: impl Into<String>) -> Self {
        self.realm_id = realm_id.into();
        self
    }

    pub fn with_external_subscription_id(mut self, id: impl Into<String>) -> Self {
        self.external_subscription_id = id.into();
        self
    }

    pub fn with_user_id(mut self, user_id: Uuid) -> Self {
        self.user_id = user_id;
        self
    }

    pub fn with_external_product_id(mut self, id: impl Into<String>) -> Self {
        self.external_product_id = id.into();
        self
    }

    pub fn with_payment_provider(mut self, provider: impl Into<String>) -> Self {
        self.payment_provider = provider.into();
        self
    }

    pub fn with_status(mut self, status: SubscriptionStatus) -> Self {
        self.status = status;
        self
    }

    pub fn with_entitlement_key(mut self, key: impl Into<String>) -> Self {
        self.entitlement_key = key.into();
        self
    }

    pub fn with_billing_type(mut self, billing_type: BillingType) -> Self {
        self.billing_type = billing_type;
        self
    }

    pub fn with_external_price_id(mut self, id: impl Into<String>) -> Self {
        self.external_price_id = Some(id.into());
        self
    }

    pub fn with_provider_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.provider_metadata = Some(metadata);
        self
    }

    pub fn with_period_start(mut self, start: chrono::DateTime<chrono::Utc>) -> Self {
        self.current_period_start = Some(start);
        self
    }

    pub fn with_no_period_start(mut self) -> Self {
        self.current_period_start = None;
        self
    }

    pub fn with_period_end(mut self, end: chrono::DateTime<chrono::Utc>) -> Self {
        self.current_period_end = Some(end);
        self
    }

    pub fn with_no_period_end(mut self) -> Self {
        self.current_period_end = None;
        self
    }

    pub fn with_cancel_at_period_end(mut self, cancel: bool) -> Self {
        self.cancel_at_period_end = cancel;
        self
    }

    pub fn with_client_app_id(mut self, id: Uuid) -> Self {
        self.client_app_id = Some(id);
        self
    }

    pub fn with_no_client_app_id(mut self) -> Self {
        self.client_app_id = None;
        self
    }

    pub fn with_cancel_at(mut self, cancel_at: chrono::DateTime<chrono::Utc>) -> Self {
        self.cancel_at = Some(cancel_at);
        self
    }

    pub fn with_no_cancel_at(mut self) -> Self {
        self.cancel_at = None;
        self
    }

    pub fn build(self) -> Subscription {
        Subscription {
            id: self.id.unwrap_or_else(Uuid::now_v7),
            realm_id: self.realm_id,
            user_id: self.user_id,
            external_subscription_id: self.external_subscription_id,
            external_product_id: self.external_product_id,
            payment_provider: self.payment_provider,
            status: self.status,
            entitlement_key: self.entitlement_key,
            billing_type: self.billing_type,
            external_price_id: self.external_price_id,
            bucket_id: Uuid::now_v7(),
            provider_metadata: self.provider_metadata,
            synced_at: self.synced_at,
            current_period_start: self.current_period_start,
            current_period_end: self.current_period_end,
            cancel_at_period_end: self.cancel_at_period_end,
            client_app_id: self.client_app_id,
            cancel_at: self.cancel_at,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

/// Builder for creating test PaymentEvent instances
#[derive(Debug, Clone)]
pub struct PaymentEventBuilder {
    id: Option<Uuid>,
    realm_id: String,
    external_event_id: Option<String>,
    payment_provider: Option<String>,
    event_type: String,
    subscription_id: Option<Uuid>,
    payload: serde_json::Value,
    processed: bool,
    processing_started_at: Option<chrono::DateTime<chrono::Utc>>,
    created_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for PaymentEventBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl PaymentEventBuilder {
    pub fn new() -> Self {
        Self {
            id: None,
            realm_id: "test-realm".to_string(),
            external_event_id: Some("evt_test".to_string()),
            payment_provider: Some("creem".to_string()),
            event_type: "subscription.paid".to_string(),
            subscription_id: None,
            payload: serde_json::json!({"test": "data"}),
            processed: false,
            processing_started_at: None,
            created_at: None,
        }
    }

    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = Some(id);
        self
    }

    pub fn with_realm_id(mut self, realm_id: impl Into<String>) -> Self {
        self.realm_id = realm_id.into();
        self
    }

    pub fn with_external_event_id(mut self, id: impl Into<String>) -> Self {
        self.external_event_id = Some(id.into());
        self
    }

    pub fn with_payment_provider(mut self, provider: impl Into<String>) -> Self {
        self.payment_provider = Some(provider.into());
        self
    }

    pub fn with_event_type(mut self, event_type: impl Into<String>) -> Self {
        self.event_type = event_type.into();
        self
    }

    pub fn with_subscription_id(mut self, id: Uuid) -> Self {
        self.subscription_id = Some(id);
        self
    }

    pub fn with_no_subscription_id(mut self) -> Self {
        self.subscription_id = None;
        self
    }

    pub fn with_payload(mut self, payload: serde_json::Value) -> Self {
        self.payload = payload;
        self
    }

    pub fn with_processed(mut self, processed: bool) -> Self {
        self.processed = processed;
        self
    }

    pub fn with_created_at(mut self, created_at: chrono::DateTime<chrono::Utc>) -> Self {
        self.created_at = Some(created_at);
        self
    }

    pub fn with_processing_started_at(
        mut self,
        processing_started_at: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        self.processing_started_at = Some(processing_started_at);
        self
    }

    pub fn build(self) -> PaymentEvent {
        PaymentEvent {
            id: self.id.unwrap_or_else(Uuid::now_v7),
            realm_id: self.realm_id,
            external_event_id: self
                .external_event_id
                .unwrap_or_else(|| "evt_test".to_string()),
            payment_provider: self.payment_provider.unwrap_or_else(|| "creem".to_string()),
            event_type: self.event_type,
            subscription_id: self.subscription_id,
            payload: self.payload,
            processed: self.processed,
            processing_started_at: self.processing_started_at,
            created_at: self.created_at.unwrap_or_else(Utc::now),
        }
    }
}

// =============================================================================
// Convenience Functions
// =============================================================================

/// Create a basic test subscription with the given realm_id
pub fn test_subscription(realm_id: impl Into<String>) -> Subscription {
    SubscriptionBuilder::new().with_realm_id(realm_id).build()
}

/// Create a test payment event with the given realm_id and event_id
pub fn test_payment_event(
    realm_id: impl Into<String>,
    external_event_id: impl Into<String>,
) -> PaymentEvent {
    PaymentEventBuilder::new()
        .with_realm_id(realm_id)
        .with_external_event_id(external_event_id)
        .with_payment_provider("creem")
        .build()
}

// =============================================================================
// Assertion Helpers
// =============================================================================

/// Assert that a subscription has the expected status
pub fn assert_subscription_status(subscription: &Subscription, expected: SubscriptionStatus) {
    assert_eq!(
        subscription.status, expected,
        "Expected subscription status {:?}, got {:?}",
        expected, subscription.status
    );
}
