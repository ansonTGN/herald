// =============================================================================
// Billing Test Helpers
// =============================================================================
//
// Consolidated test helpers for billing-related tests.
// Reduces duplication across postgres_repository_test.rs, entities_test.rs,
// and services_test.rs.
//
// =============================================================================

use crate::billing::{
    BillingPeriod, ClientAppSubscriptionPlan, CreateSubscriptionPlanInput, PaymentEvent,
    Subscription, SubscriptionPlan, SubscriptionPlanType, SubscriptionStatus, SubscriptionTier,
    UpdateSubscriptionPlanInput,
};
use chrono::Utc;
use serde_json;
use uuid::Uuid;

// =============================================================================
// Builders
// =============================================================================

/// Builder for creating test SubscriptionPlan instances
#[derive(Debug, Clone)]
pub struct SubscriptionPlanBuilder {
    id: Option<Uuid>,
    realm_id: String,
    name: String,
    title: String,
    description: Option<String>,
    plan_type: SubscriptionPlanType,
    price: i32,
    currency: String,
    checkout_url: Option<String>,
    active: bool,
    trial_days: i32,
    sort_order: i32,
    product_id: Uuid,
}

impl Default for SubscriptionPlanBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SubscriptionPlanBuilder {
    pub fn new() -> Self {
        Self {
            id: None,
            realm_id: "test-realm".to_string(),
            name: "test_plan".to_string(),
            title: "Test Plan".to_string(),
            description: Some("Test Description".to_string()),
            plan_type: SubscriptionPlanType::Monthly,
            price: 1000,
            currency: "USD".to_string(),
            checkout_url: Some("https://app.example.com/billing/checkout?plan_id=test".to_string()),
            active: true,
            trial_days: 0,
            sort_order: 1,
            product_id: Uuid::now_v7(),
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

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        self.name = name.clone();
        self.title.clone_from(&name);
        self
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_no_description(mut self) -> Self {
        self.description = None;
        self
    }

    pub fn with_type(mut self, plan_type: SubscriptionPlanType) -> Self {
        self.plan_type = plan_type;
        self
    }

    pub fn with_price(mut self, price: i32) -> Self {
        self.price = price;
        self
    }

    pub fn with_currency(mut self, currency: impl Into<String>) -> Self {
        self.currency = currency.into();
        self
    }

    pub fn with_checkout_url(mut self, url: impl Into<String>) -> Self {
        self.checkout_url = Some(url.into());
        self
    }

    pub fn with_no_checkout_url(mut self) -> Self {
        self.checkout_url = None;
        self
    }

    pub fn with_active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn with_trial_days(mut self, days: i32) -> Self {
        self.trial_days = days;
        self
    }

    pub fn with_sort_order(mut self, order: i32) -> Self {
        self.sort_order = order;
        self
    }

    pub fn with_product_id(mut self, product_id: Uuid) -> Self {
        self.product_id = product_id;
        self
    }

    pub fn build(self) -> SubscriptionPlan {
        SubscriptionPlan {
            id: self.id.unwrap_or_else(Uuid::now_v7),
            realm_id: self.realm_id,
            name: self.name,
            title: self.title,
            description: self.description,
            r#type: self.plan_type,
            price: self.price,
            currency: self.currency,
            checkout_url: self.checkout_url,
            active: self.active,
            trial_days: self.trial_days,
            sort_order: self.sort_order,
            product_id: self.product_id,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

/// Builder for creating test Subscription instances
#[derive(Debug, Clone)]
pub struct SubscriptionBuilder {
    id: Option<Uuid>,
    realm_id: String,
    user_id: Option<Uuid>,
    external_subscription_id: String,
    external_product_id: String,
    payment_provider: String,
    status: SubscriptionStatus,
    tier: SubscriptionTier,
    current_period_start: Option<chrono::DateTime<chrono::Utc>>,
    current_period_end: Option<chrono::DateTime<chrono::Utc>>,
    cancel_at_period_end: bool,
    client_app_id: Option<Uuid>,
    plan_id: Option<Uuid>,
    billing_period: BillingPeriod,
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
            user_id: None,
            external_subscription_id: "sub_test_123".to_string(),
            external_product_id: "prod_starter_monthly".to_string(),
            payment_provider: "creem".to_string(),
            status: SubscriptionStatus::Active,
            tier: SubscriptionTier::Starter,
            current_period_start: Some(Utc::now()),
            current_period_end: Some(Utc::now() + chrono::Duration::days(30)),
            cancel_at_period_end: false,
            client_app_id: None,
            plan_id: None,
            billing_period: BillingPeriod::Monthly,
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
        self.user_id = Some(user_id);
        self
    }

    pub fn with_no_user_id(mut self) -> Self {
        self.user_id = None;
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

    pub fn with_tier(mut self, tier: SubscriptionTier) -> Self {
        self.tier = tier;
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

    pub fn with_plan_id(mut self, id: Uuid) -> Self {
        self.plan_id = Some(id);
        self
    }

    pub fn with_no_plan_id(mut self) -> Self {
        self.plan_id = None;
        self
    }

    pub fn with_billing_period(mut self, period: BillingPeriod) -> Self {
        self.billing_period = period;
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
            tier: self.tier,
            current_period_start: self.current_period_start,
            current_period_end: self.current_period_end,
            cancel_at_period_end: self.cancel_at_period_end,
            client_app_id: self.client_app_id,
            plan_id: self.plan_id,
            billing_period: self.billing_period,
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

/// Builder for creating test CreateSubscriptionPlanInput instances
#[derive(Debug, Clone)]
pub struct CreateSubscriptionPlanInputBuilder {
    realm_id: String,
    name: String,
    title: String,
    description: Option<String>,
    plan_type: SubscriptionPlanType,
    price: i32,
    currency: String,
    checkout_url: Option<String>,
    trial_days: Option<i32>,
    sort_order: Option<i32>,
    product_id: Uuid,
}

impl Default for CreateSubscriptionPlanInputBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl CreateSubscriptionPlanInputBuilder {
    pub fn new() -> Self {
        Self {
            realm_id: "test-realm".to_string(),
            name: "test_plan".to_string(),
            title: "Test Plan".to_string(),
            description: Some("Test Description".to_string()),
            plan_type: SubscriptionPlanType::Monthly,
            price: 1000,
            currency: "USD".to_string(),
            checkout_url: Some("https://app.example.com/billing/checkout?plan_id=test".to_string()),
            trial_days: Some(0),
            sort_order: Some(1),
            product_id: Uuid::now_v7(),
        }
    }

    pub fn with_realm_id(mut self, realm_id: impl Into<String>) -> Self {
        self.realm_id = realm_id.into();
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        self.name = name.clone();
        self.title.clone_from(&name);
        self
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_no_description(mut self) -> Self {
        self.description = None;
        self
    }

    pub fn with_type(mut self, plan_type: SubscriptionPlanType) -> Self {
        self.plan_type = plan_type;
        self
    }

    pub fn with_price(mut self, price: i32) -> Self {
        self.price = price;
        self
    }

    pub fn with_currency(mut self, currency: impl Into<String>) -> Self {
        self.currency = currency.into();
        self
    }

    pub fn with_checkout_url(mut self, url: impl Into<String>) -> Self {
        self.checkout_url = Some(url.into());
        self
    }

    pub fn with_trial_days(mut self, days: i32) -> Self {
        self.trial_days = Some(days);
        self
    }

    pub fn with_no_trial_days(mut self) -> Self {
        self.trial_days = None;
        self
    }

    pub fn with_sort_order(mut self, order: i32) -> Self {
        self.sort_order = Some(order);
        self
    }

    pub fn with_no_sort_order(mut self) -> Self {
        self.sort_order = None;
        self
    }

    pub fn with_product_id(mut self, product_id: Uuid) -> Self {
        self.product_id = product_id;
        self
    }

    pub fn build(self) -> CreateSubscriptionPlanInput {
        CreateSubscriptionPlanInput {
            realm_id: self.realm_id,
            name: self.name,
            title: self.title,
            description: self.description,
            r#type: self.plan_type,
            price: self.price,
            currency: self.currency,
            checkout_url: self.checkout_url,
            trial_days: self.trial_days,
            sort_order: self.sort_order,
            product_id: self.product_id,
        }
    }
}

/// Builder for creating test UpdateSubscriptionPlanInput instances
#[derive(Debug, Clone, Default)]
pub struct UpdateSubscriptionPlanInputBuilder {
    name: Option<String>,
    title: Option<String>,
    description: Option<String>,
    plan_type: Option<SubscriptionPlanType>,
    price: Option<i32>,
    currency: Option<String>,
    checkout_url: Option<String>,
    active: Option<bool>,
    trial_days: Option<i32>,
    sort_order: Option<i32>,
    product_id: Option<Uuid>,
}

impl UpdateSubscriptionPlanInputBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_no_name(mut self) -> Self {
        self.name = None;
        self
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_no_title(mut self) -> Self {
        self.title = None;
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_no_description(mut self) -> Self {
        self.description = None;
        self
    }

    pub fn with_type(mut self, plan_type: SubscriptionPlanType) -> Self {
        self.plan_type = Some(plan_type);
        self
    }

    pub fn with_no_type(mut self) -> Self {
        self.plan_type = None;
        self
    }

    pub fn with_price(mut self, price: i32) -> Self {
        self.price = Some(price);
        self
    }

    pub fn with_no_price(mut self) -> Self {
        self.price = None;
        self
    }

    pub fn with_currency(mut self, currency: impl Into<String>) -> Self {
        self.currency = Some(currency.into());
        self
    }

    pub fn with_no_currency(mut self) -> Self {
        self.currency = None;
        self
    }

    pub fn with_checkout_url(mut self, url: impl Into<String>) -> Self {
        self.checkout_url = Some(url.into());
        self
    }

    pub fn with_no_checkout_url(mut self) -> Self {
        self.checkout_url = None;
        self
    }

    pub fn with_active(mut self, active: bool) -> Self {
        self.active = Some(active);
        self
    }

    pub fn with_no_active(mut self) -> Self {
        self.active = None;
        self
    }

    pub fn with_trial_days(mut self, days: i32) -> Self {
        self.trial_days = Some(days);
        self
    }

    pub fn with_no_trial_days(mut self) -> Self {
        self.trial_days = None;
        self
    }

    pub fn with_sort_order(mut self, order: i32) -> Self {
        self.sort_order = Some(order);
        self
    }

    pub fn with_no_sort_order(mut self) -> Self {
        self.sort_order = None;
        self
    }

    pub fn with_product_id(mut self, product_id: Uuid) -> Self {
        self.product_id = Some(product_id);
        self
    }

    pub fn with_no_product_id(mut self) -> Self {
        self.product_id = None;
        self
    }

    pub fn build(self) -> UpdateSubscriptionPlanInput {
        UpdateSubscriptionPlanInput {
            name: self.name,
            title: self.title,
            description: self.description,
            r#type: self.plan_type,
            price: self.price,
            currency: self.currency,
            checkout_url: self.checkout_url,
            active: self.active,
            trial_days: self.trial_days,
            sort_order: self.sort_order,
            product_id: self.product_id,
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

/// Create a basic test subscription plan with the given realm_id and name
pub fn test_plan(realm_id: impl Into<String>, name: impl Into<String>) -> SubscriptionPlan {
    SubscriptionPlanBuilder::new()
        .with_realm_id(realm_id)
        .with_name(name)
        .build()
}

/// Create a basic test client app subscription plan
pub fn test_client_app_plan(
    client_app_id: Uuid,
    plan_id: Uuid,
    enabled: bool,
) -> ClientAppSubscriptionPlan {
    ClientAppSubscriptionPlan {
        id: Uuid::now_v7(),
        client_app_id,
        plan_id,
        enabled,
        created_at: Utc::now(),
    }
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

/// Assert that a subscription has the expected tier
pub fn assert_subscription_tier(subscription: &Subscription, expected: SubscriptionTier) {
    assert_eq!(
        subscription.tier, expected,
        "Expected subscription tier {:?}, got {:?}",
        expected, subscription.tier
    );
}

/// Assert that a subscription plan has the expected price
pub fn assert_plan_price(plan: &SubscriptionPlan, expected_price: i32) {
    assert_eq!(
        plan.price, expected_price,
        "Expected plan price {}, got {}",
        expected_price, plan.price
    );
}

/// Assert that a subscription plan is active or inactive
pub fn assert_plan_active(plan: &SubscriptionPlan, expected_active: bool) {
    assert_eq!(
        plan.active, expected_active,
        "Expected plan.active={}, got {}",
        expected_active, plan.active
    );
}
