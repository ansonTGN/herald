// =============================================================================
// Billing Utility Function Tests
// =============================================================================

use crate::handlers::extract_realm_id;
use herald_core::domain::common::entities::app_errors::CoreError;
use herald_core::infrastructure::creem::CreemWebhookEvent;

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
