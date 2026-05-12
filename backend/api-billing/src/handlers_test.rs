// =============================================================================
// Billing Utility Function Tests
// =============================================================================

use crate::handlers::{determine_tier_from_product, extract_realm_id};
use chrono::Utc;
use herald_core::domain::billing::{
    BillingPeriod, PaymentEvent, Subscription, SubscriptionStatus, SubscriptionTier,
};
use herald_core::domain::common::entities::app_errors::CoreError;
use herald_core::infrastructure::creem::CreemWebhookEvent;
use uuid::Uuid;

// =============================================================================
// extract_realm_id Tests
// =============================================================================

#[test]
fn test_handler_extract_realm_id_valid() {
    let event = CreemWebhookEvent {
        id: "evt_test123".to_string(),
        event_type: "subscription.paid".to_string(),
        object: serde_json::json!({
            "id": "sub_123",
            "metadata": {
                "realmId": "realm_test456"
            }
        }),
    };

    let result = extract_realm_id(&event);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "realm_test456");
}

#[test]
fn test_handler_extract_realm_id_missing_metadata() {
    let event = CreemWebhookEvent {
        id: "evt_test123".to_string(),
        event_type: "subscription.paid".to_string(),
        object: serde_json::json!({
            "id": "sub_123"
        }),
    };

    let result = extract_realm_id(&event);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), CoreError::BadRequest(_)));
}

#[test]
fn test_handler_extract_realm_id_empty_metadata() {
    let event = CreemWebhookEvent {
        id: "evt_test123".to_string(),
        event_type: "subscription.paid".to_string(),
        object: serde_json::json!({
            "id": "sub_123",
            "metadata": {}
        }),
    };

    let result = extract_realm_id(&event);
    assert!(result.is_err());
}

#[test]
fn test_handler_extract_realm_id_null_metadata() {
    let event = CreemWebhookEvent {
        id: "evt_null".to_string(),
        event_type: "subscription.paid".to_string(),
        object: serde_json::json!({
            "id": "sub_123",
            "metadata": null
        }),
    };

    let result = extract_realm_id(&event);
    assert!(result.is_err());
}

#[test]
fn test_handler_extract_realm_id_invalid_type() {
    let event = CreemWebhookEvent {
        id: "evt_invalid".to_string(),
        event_type: "subscription.paid".to_string(),
        object: serde_json::json!({
            "id": "sub_123",
            "metadata": {
                "realmId": 12345
            }
        }),
    };

    let result = extract_realm_id(&event);
    assert!(result.is_err());
}

#[test]
fn test_handler_extract_realm_id_nested() {
    let event = CreemWebhookEvent {
        id: "evt_nested".to_string(),
        event_type: "subscription.paid".to_string(),
        object: serde_json::json!({
            "id": "sub_123",
            "customer": {
                "email": "test@example.com"
            },
            "metadata": {
                "realmId": "realm_nested",
                "user_id": "user_456"
            }
        }),
    };

    let result = extract_realm_id(&event);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "realm_nested");
}

// =============================================================================
// determine_tier_from_product Tests
// =============================================================================

#[test]
fn test_handler_determine_tier_from_product() {
    assert_eq!(
        determine_tier_from_product("prod_starter_monthly"),
        SubscriptionTier::Starter
    );
    assert_eq!(
        determine_tier_from_product("prod_starter_yearly"),
        SubscriptionTier::Starter
    );
    assert_eq!(
        determine_tier_from_product("prod_professional_monthly"),
        SubscriptionTier::Professional
    );
    assert_eq!(
        determine_tier_from_product("prod_pro_monthly"),
        SubscriptionTier::Professional
    );
    assert_eq!(
        determine_tier_from_product("prod_enterprise_monthly"),
        SubscriptionTier::Enterprise
    );
    assert_eq!(
        determine_tier_from_product("prod_free"),
        SubscriptionTier::Free
    );
    assert_eq!(
        determine_tier_from_product("unknown_product"),
        SubscriptionTier::Free
    );
}

#[test]
fn test_handler_determine_tier_case_sensitivity() {
    assert_eq!(
        determine_tier_from_product("prod_STARTER_monthly"),
        SubscriptionTier::Free
    );
    assert_eq!(
        determine_tier_from_product("PROD_STARTER_MONTHLY"),
        SubscriptionTier::Free
    );
}

#[test]
fn test_handler_determine_tier_partial_matches() {
    assert_eq!(
        determine_tier_from_product("super_starter_plan"),
        SubscriptionTier::Starter
    );
    assert_eq!(
        determine_tier_from_product("my_pro_plan"),
        SubscriptionTier::Professional
    );
    assert_eq!(
        determine_tier_from_product("big_enterprise_bundle"),
        SubscriptionTier::Enterprise
    );
}

// =============================================================================
// Serialization Tests
// =============================================================================

#[test]
fn test_webhook_event_serialization() {
    let json = serde_json::json!({
        "id": "evt_test123",
        "eventType": "subscription.paid",
        "object": {
            "id": "sub_123",
            "status": "active",
            "metadata": {
                "realmId": "realm_test"
            }
        }
    });

    let event: CreemWebhookEvent = serde_json::from_value(json).unwrap();
    assert_eq!(event.id, "evt_test123");
    assert_eq!(event.event_type, "subscription.paid");
}

// =============================================================================
// Entity Creation/Validation Tests
// =============================================================================

#[test]
fn test_payment_event_creation() {
    let event = PaymentEvent {
        id: Uuid::now_v7(),
        realm_id: "realm_test".to_string(),
        external_event_id: "evt_123".to_string(),
        payment_provider: "creem".to_string(),
        event_type: "subscription.paid".to_string(),
        subscription_id: None,
        payload: serde_json::json!({"test": "data"}),
        processed: false,
        processing_started_at: None,
        created_at: Utc::now(),
    };

    assert_eq!(event.realm_id, "realm_test");
    assert_eq!(event.external_event_id, "evt_123");
    assert_eq!(event.payment_provider, "creem");
    assert_eq!(event.event_type, "subscription.paid");
    assert!(!event.processed);
}

#[test]
fn test_subscription_status_transitions() {
    let sub = Subscription {
        id: Uuid::now_v7(),
        realm_id: "realm_test".to_string(),
        user_id: None,
        external_subscription_id: "sub_123".to_string(),
        external_product_id: "prod_starter".to_string(),
        payment_provider: "creem".to_string(),
        status: SubscriptionStatus::Pending,
        tier: SubscriptionTier::Starter,
        current_period_start: None,
        current_period_end: None,
        cancel_at_period_end: false,
        client_app_id: None,
        plan_id: None,
        billing_period: BillingPeriod::Monthly,
        cancel_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    assert_eq!(sub.status, SubscriptionStatus::Pending);
    assert_eq!(sub.status.as_str(), "pending");

    let mut active_sub = sub.clone();
    active_sub.status = SubscriptionStatus::Active;
    assert_eq!(active_sub.status.as_str(), "active");

    let mut canceled_sub = active_sub.clone();
    canceled_sub.status = SubscriptionStatus::Canceled;
    assert_eq!(canceled_sub.status.as_str(), "canceled");
}

// =============================================================================
// Enum Parsing Tests
// =============================================================================

#[test]
fn test_subscription_tier_parsing() {
    assert_eq!(
        "free".parse::<SubscriptionTier>().unwrap(),
        SubscriptionTier::Free
    );
    assert_eq!(
        "starter".parse::<SubscriptionTier>().unwrap(),
        SubscriptionTier::Starter
    );
    assert_eq!(
        "professional".parse::<SubscriptionTier>().unwrap(),
        SubscriptionTier::Professional
    );
    assert_eq!(
        "enterprise".parse::<SubscriptionTier>().unwrap(),
        SubscriptionTier::Enterprise
    );
    assert!("invalid".parse::<SubscriptionTier>().is_err());
}

#[test]
fn test_subscription_status_parsing() {
    assert_eq!(
        "active".parse::<SubscriptionStatus>().unwrap(),
        SubscriptionStatus::Active
    );
    assert_eq!(
        "canceled".parse::<SubscriptionStatus>().unwrap(),
        SubscriptionStatus::Canceled
    );
    assert_eq!(
        "expired".parse::<SubscriptionStatus>().unwrap(),
        SubscriptionStatus::Expired
    );
    assert_eq!(
        "pending".parse::<SubscriptionStatus>().unwrap(),
        SubscriptionStatus::Pending
    );
    assert_eq!(
        "trialing".parse::<SubscriptionStatus>().unwrap(),
        SubscriptionStatus::Trialing
    );
    assert_eq!(
        "paused".parse::<SubscriptionStatus>().unwrap(),
        SubscriptionStatus::Paused
    );
    assert!("invalid".parse::<SubscriptionStatus>().is_err());
}
