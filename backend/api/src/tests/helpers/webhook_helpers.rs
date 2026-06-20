// =============================================================================
// Webhook Test Helpers
// =============================================================================
//
// Shared helpers for webhook testing.
// Provides functions for building webhook events and sending them to test servers.
//
// Updated for product_reduce: herald_* metadata builders replace plan_id builders.
// Old plan_id-based builders have been removed.
//
// =============================================================================

#![allow(dead_code)]

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use hex;
use hmac::{Hmac, Mac};
use serde_json::json;
use sha2::Sha256;
use tower::ServiceExt;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

/// ============================================================================
/// Creem Webhook Event Builders (herald_* metadata)
/// ============================================================================
///
/// Build a Creem checkout.completed webhook event with all herald_* metadata keys.
pub fn build_creem_checkout_completed_with_herald_metadata(
    event_id: &str,
    entitlement_key: &str,
    realm_id: &str,
    user_id: Uuid,
    client_app_id: Uuid,
) -> serde_json::Value {
    json!({
        "id": event_id,
        "eventType": "checkout.completed",
        "object": {
            "id": format!("checkout_{}", event_id),
            "status": "completed",
            "product": {
                "id": format!("prod_creem_{}", entitlement_key),
            },
            "metadata": {
                "herald_entitlement_key": entitlement_key,
                "herald_realm_id": realm_id,
                "herald_user_id": user_id.to_string(),
                "herald_client_app_id": client_app_id.to_string(),
                "clientAppId": client_app_id.to_string(),
            }
        }
    })
}

/// Build a Creem checkout.completed webhook without herald_entitlement_key
/// (for fallback chain testing).
pub fn build_creem_checkout_completed_without_entitlement_key(
    event_id: &str,
    realm_id: &str,
    user_id: Uuid,
    client_app_id: Uuid,
) -> serde_json::Value {
    json!({
        "id": event_id,
        "eventType": "checkout.completed",
        "object": {
            "id": format!("checkout_{}", event_id),
            "status": "completed",
            "product": {
                "id": format!("prod_creem_fallback_{}", event_id),
            },
            "metadata": {
                "herald_realm_id": realm_id,
                "herald_user_id": user_id.to_string(),
                "herald_client_app_id": client_app_id.to_string(),
                "clientAppId": client_app_id.to_string(),
            }
        }
    })
}

/// Build a Creem subscription.paid webhook event with herald_* metadata.
#[allow(clippy::too_many_arguments)]
pub fn build_creem_subscription_paid_with_herald_metadata(
    event_id: &str,
    entitlement_key: &str,
    realm_id: &str,
    user_id: Uuid,
    client_app_id: Option<Uuid>,
    external_subscription_id: &str,
    external_product_id: &str,
    is_renewal: bool,
) -> serde_json::Value {
    let period_end = chrono::Utc::now() + chrono::Duration::days(30);
    let mut metadata = json!({
        "herald_entitlement_key": entitlement_key,
        "herald_realm_id": realm_id,
        "herald_user_id": user_id.to_string(),
    });
    if let Some(caid) = client_app_id {
        metadata["herald_client_app_id"] = json!(caid.to_string());
        metadata["clientAppId"] = json!(caid.to_string());
    }

    json!({
        "id": event_id,
        "eventType": "subscription.paid",
        "data": {
            "object": {
                "subscriptionId": external_subscription_id,
                "productId": external_product_id,
                "userId": user_id.to_string(),
                "isRenewal": is_renewal,
                "currentPeriodEnd": period_end.to_rfc3339(),
                "status": "active",
                "cancelAtPeriodEnd": false,
                "metadata": metadata,
            }
        },
        "herald_entitlement_key": entitlement_key,
    })
}

/// Build a Creem subscription.paid webhook without herald_entitlement_key
/// (for fallback chain testing).
#[allow(clippy::too_many_arguments)]
pub fn build_creem_subscription_paid_without_entitlement_key(
    event_id: &str,
    realm_id: &str,
    user_id: Uuid,
    client_app_id: Option<Uuid>,
    external_subscription_id: &str,
    external_product_id: &str,
    is_renewal: bool,
) -> serde_json::Value {
    let period_end = chrono::Utc::now() + chrono::Duration::days(30);
    let mut metadata = json!({
        "herald_realm_id": realm_id,
        "herald_user_id": user_id.to_string(),
    });
    if let Some(caid) = client_app_id {
        metadata["herald_client_app_id"] = json!(caid.to_string());
        metadata["clientAppId"] = json!(caid.to_string());
    }

    json!({
        "id": event_id,
        "eventType": "subscription.paid",
        "data": {
            "object": {
                "subscriptionId": external_subscription_id,
                "productId": external_product_id,
                "userId": user_id.to_string(),
                "isRenewal": is_renewal,
                "currentPeriodEnd": period_end.to_rfc3339(),
                "status": "active",
                "cancelAtPeriodEnd": false,
                "metadata": metadata,
            }
        },
    })
}

/// Build a Creem subscription.canceled webhook with entitlement_key.
#[allow(clippy::too_many_arguments)]
pub fn build_creem_subscription_canceled_with_entitlement(
    event_id: &str,
    entitlement_key: &str,
    realm_id: &str,
    user_id: Uuid,
    external_subscription_id: &str,
    external_product_id: &str,
    cancel_at_period_end: bool,
) -> serde_json::Value {
    let period_end = chrono::Utc::now() + chrono::Duration::days(30);
    json!({
        "id": event_id,
        "eventType": "subscription.canceled",
        "data": {
            "object": {
                "subscriptionId": external_subscription_id,
                "productId": external_product_id,
                "userId": user_id.to_string(),
                "cancelAtPeriodEnd": cancel_at_period_end,
                "currentPeriodEnd": period_end.to_rfc3339(),
                "status": if cancel_at_period_end { "active" } else { "canceled" },
            }
        },
        "herald_entitlement_key": entitlement_key,
        "metadata": {
            "herald_entitlement_key": entitlement_key,
            "herald_realm_id": realm_id,
            "herald_user_id": user_id.to_string(),
        },
    })
}

/// Build a Creem checkout.completed webhook missing herald_realm_id.
pub fn build_creem_checkout_missing_realm_id(
    event_id: &str,
    entitlement_key: &str,
    user_id: Uuid,
    client_app_id: Uuid,
) -> serde_json::Value {
    json!({
        "id": event_id,
        "eventType": "checkout.completed",
        "object": {
            "id": format!("checkout_{}", event_id),
            "status": "completed",
            "product": {
                "id": format!("prod_creem_{}", entitlement_key),
            },
            "metadata": {
                "herald_entitlement_key": entitlement_key,
                "herald_user_id": user_id.to_string(),
                "herald_client_app_id": client_app_id.to_string(),
                "clientAppId": client_app_id.to_string(),
            }
        }
    })
}

/// Build a Creem checkout.completed webhook with empty herald_entitlement_key.
pub fn build_creem_checkout_empty_entitlement_key(
    event_id: &str,
    realm_id: &str,
    user_id: Uuid,
    client_app_id: Uuid,
) -> serde_json::Value {
    json!({
        "id": event_id,
        "eventType": "checkout.completed",
        "object": {
            "id": format!("checkout_{}", event_id),
            "status": "completed",
            "product": {
                "id": format!("prod_creem_empty_{}", event_id),
            },
            "metadata": {
                "herald_entitlement_key": "",
                "herald_realm_id": realm_id,
                "herald_user_id": user_id.to_string(),
                "herald_client_app_id": client_app_id.to_string(),
                "clientAppId": client_app_id.to_string(),
            }
        }
    })
}

/// Build a Creem checkout.completed webhook with invalid entitlement_key format.
pub fn build_creem_checkout_invalid_entitlement_key(
    event_id: &str,
    realm_id: &str,
    user_id: Uuid,
    client_app_id: Uuid,
    invalid_key: &str,
) -> serde_json::Value {
    json!({
        "id": event_id,
        "eventType": "checkout.completed",
        "object": {
            "id": format!("checkout_{}", event_id),
            "status": "completed",
            "product": {
                "id": format!("prod_creem_invalid_{}", event_id),
            },
            "metadata": {
                "herald_entitlement_key": invalid_key,
                "herald_realm_id": realm_id,
                "herald_user_id": user_id.to_string(),
                "herald_client_app_id": client_app_id.to_string(),
                "clientAppId": client_app_id.to_string(),
            }
        }
    })
}

/// ============================================================================
/// Stripe Webhook Event Builders (herald_* metadata)
/// ============================================================================
///
/// Build a Stripe checkout.session.completed webhook with herald_* metadata.
#[allow(clippy::too_many_arguments)]
pub fn build_stripe_checkout_completed_with_herald_metadata(
    event_id: &str,
    entitlement_key: &str,
    realm_id: &str,
    user_id: Uuid,
    client_app_id: Uuid,
) -> serde_json::Value {
    json!({
        "id": event_id,
        "object": "event",
        "type": "checkout.session.completed",
        "api_version": "2020-08-27",
        "created": chrono::Utc::now().timestamp(),
        "data": {
            "object": {
                "id": format!("cs_test_{}", Uuid::now_v7()),
                "object": "checkout.session",
                "status": "complete",
                "payment_status": "paid",
                "customer": format!("cus_test_{}", Uuid::now_v7()),
                "subscription": format!("sub_test_{}", event_id),
                "payment_intent": format!("pi_test_{}", Uuid::now_v7()),
                "metadata": {
                    "herald_entitlement_key": entitlement_key,
                    "herald_realm_id": realm_id,
                    "herald_user_id": user_id.to_string(),
                    "herald_client_app_id": client_app_id.to_string(),
                    "clientAppId": client_app_id.to_string(),
                    "userId": user_id.to_string(),
                },
                "display_items": [{
                    "price": {
                        "product": format!("prod_stripe_{}", entitlement_key)
                    }
                }]
            }
        }
    })
}

/// Build a Stripe checkout.session.completed webhook without herald_entitlement_key
/// (for fallback chain testing).
pub fn build_stripe_checkout_completed_without_entitlement_key(
    event_id: &str,
    realm_id: &str,
    user_id: Uuid,
    client_app_id: Uuid,
) -> serde_json::Value {
    json!({
        "id": event_id,
        "object": "event",
        "type": "checkout.session.completed",
        "api_version": "2020-08-27",
        "created": chrono::Utc::now().timestamp(),
        "data": {
            "object": {
                "id": format!("cs_test_{}", Uuid::now_v7()),
                "object": "checkout.session",
                "status": "complete",
                "payment_status": "paid",
                "customer": format!("cus_test_{}", Uuid::now_v7()),
                "subscription": format!("sub_test_{}", event_id),
                "payment_intent": format!("pi_test_{}", Uuid::now_v7()),
                "metadata": {
                    "herald_realm_id": realm_id,
                    "herald_user_id": user_id.to_string(),
                    "herald_client_app_id": client_app_id.to_string(),
                    "clientAppId": client_app_id.to_string(),
                    "userId": user_id.to_string(),
                },
                "display_items": [{
                    "price": {
                        "product": format!("prod_stripe_fallback_{}", event_id)
                    }
                }]
            }
        }
    })
}

/// Build a Stripe customer.subscription.updated webhook with herald_entitlement_key.
#[allow(clippy::too_many_arguments)]
pub fn build_stripe_subscription_updated_with_entitlement(
    event_id: &str,
    stripe_subscription_id: &str,
    realm_id: &str,
    user_id: Uuid,
    previous_entitlement_key: &str,
    current_entitlement_key: &str,
    external_product_id: &str,
) -> serde_json::Value {
    let period_end = (chrono::Utc::now() + chrono::Duration::days(30)).timestamp();
    json!({
        "id": event_id,
        "object": "event",
        "type": "customer.subscription.updated",
        "api_version": "2020-08-27",
        "created": chrono::Utc::now().timestamp(),
        "data": {
            "object": {
                "id": stripe_subscription_id,
                "object": "subscription",
                "status": "active",
                "current_period_end": period_end,
                "items": {
                    "data": [{
                        "price": {
                            "product": external_product_id,
                            "metadata": {
                                "herald_entitlement_key": current_entitlement_key,
                            }
                        }
                    }]
                },
                "metadata": {
                    "herald_realm_id": realm_id,
                    "herald_user_id": user_id.to_string(),
                    "userId": user_id.to_string(),
                }
            },
            "previous_attributes": {
                "items": {
                    "data": [{
                        "price": {
                            "product": external_product_id,
                            "metadata": {
                                "herald_entitlement_key": previous_entitlement_key,
                            }
                        }
                    }]
                }
            }
        }
    })
}

/// Build a Stripe customer.subscription.deleted webhook with entitlement_key.
pub fn build_stripe_subscription_deleted_with_entitlement(
    event_id: &str,
    stripe_subscription_id: &str,
    realm_id: &str,
    user_id: Uuid,
    entitlement_key: &str,
) -> serde_json::Value {
    let period_end = (chrono::Utc::now() + chrono::Duration::days(30)).timestamp();
    json!({
        "id": event_id,
        "object": "event",
        "type": "customer.subscription.deleted",
        "api_version": "2020-08-27",
        "created": chrono::Utc::now().timestamp(),
        "data": {
            "object": {
                "id": stripe_subscription_id,
                "object": "subscription",
                "status": "canceled",
                "cancel_at_period_end": false,
                "current_period_end": period_end,
                "metadata": {
                    "herald_realm_id": realm_id,
                    "herald_user_id": user_id.to_string(),
                    "herald_entitlement_key": entitlement_key,
                    "userId": user_id.to_string(),
                }
            }
        }
    })
}

/// Build a Stripe invoice.payment_succeeded webhook with herald_* metadata.
#[allow(clippy::too_many_arguments)]
pub fn build_stripe_invoice_with_herald_metadata(
    event_id: &str,
    stripe_subscription_id: &str,
    realm_id: &str,
    user_id: Uuid,
    entitlement_key: &str,
    amount_paid: i64,
) -> serde_json::Value {
    let period_start = chrono::Utc::now().timestamp();
    let period_end = (chrono::Utc::now() + chrono::Duration::days(30)).timestamp();
    json!({
        "id": event_id,
        "object": "event",
        "type": "invoice.payment_succeeded",
        "api_version": "2020-08-27",
        "created": chrono::Utc::now().timestamp(),
        "data": {
            "object": {
                "id": format!("in_test_{}", Uuid::now_v7()),
                "object": "invoice",
                "status": "paid",
                "subscription": stripe_subscription_id,
                "amount_paid": amount_paid,
                "currency": "usd",
                "current_period_start": period_start,
                "current_period_end": period_end,
                "metadata": {
                    "herald_realm_id": realm_id,
                    "herald_user_id": user_id.to_string(),
                    "herald_entitlement_key": entitlement_key,
                    "userId": user_id.to_string(),
                }
            }
        }
    })
}

/// Build a Stripe checkout.session.completed webhook missing herald_realm_id.
pub fn build_stripe_checkout_missing_realm_id(
    event_id: &str,
    entitlement_key: &str,
    user_id: Uuid,
    client_app_id: Uuid,
) -> serde_json::Value {
    json!({
        "id": event_id,
        "object": "event",
        "type": "checkout.session.completed",
        "api_version": "2020-08-27",
        "created": chrono::Utc::now().timestamp(),
        "data": {
            "object": {
                "id": format!("cs_test_{}", Uuid::now_v7()),
                "object": "checkout.session",
                "status": "complete",
                "payment_status": "paid",
                "customer": format!("cus_test_{}", Uuid::now_v7()),
                "subscription": format!("sub_test_{}", event_id),
                "payment_intent": format!("pi_test_{}", Uuid::now_v7()),
                "metadata": {
                    "herald_entitlement_key": entitlement_key,
                    "herald_user_id": user_id.to_string(),
                    "herald_client_app_id": client_app_id.to_string(),
                    "clientAppId": client_app_id.to_string(),
                    "userId": user_id.to_string(),
                },
                "display_items": [{
                    "price": {
                        "product": format!("prod_stripe_{}", entitlement_key)
                    }
                }]
            }
        }
    })
}

/// ============================================================================
/// Webhook Sending Helpers
/// ============================================================================
/// Send a webhook event with signature
pub async fn send_webhook_with_signature(
    app: &axum::Router,
    realm_id: &str,
    payload: serde_json::Value,
    webhook_secret: &str,
) -> axum::response::Response {
    let payload_str = serde_json::to_string(&payload).unwrap();

    // Generate signature
    let mut mac = HmacSha256::new_from_slice(webhook_secret.as_bytes()).unwrap();
    mac.update(payload_str.as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/third/pay/{}/creem/webhooks", realm_id))
                .header("creem-signature", signature)
                .header("Content-Type", "application/json")
                .body(Body::from(payload_str))
                .unwrap(),
        )
        .await
        .unwrap()
}

/// Send a webhook event with invalid signature
pub async fn send_webhook_with_invalid_signature(
    app: &axum::Router,
    realm_id: &str,
    payload: serde_json::Value,
) -> axum::response::Response {
    let payload_str = serde_json::to_string(&payload).unwrap();
    let invalid_signature = "aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabb";

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/third/pay/{}/creem/webhooks", realm_id))
                .header("creem-signature", invalid_signature)
                .header("Content-Type", "application/json")
                .body(Body::from(payload_str))
                .unwrap(),
        )
        .await
        .unwrap()
}

/// Send a webhook event without signature
pub async fn send_webhook_without_signature(
    app: &axum::Router,
    realm_id: &str,
    payload: serde_json::Value,
) -> axum::response::Response {
    let payload_str = serde_json::to_string(&payload).unwrap();

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/third/pay/{}/creem/webhooks", realm_id))
                .header("Content-Type", "application/json")
                .body(Body::from(payload_str))
                .unwrap(),
        )
        .await
        .unwrap()
}

/// ============================================================================
/// Webhook Assertion Helpers
/// ============================================================================
/// Assert webhook response is successful (200 OK or 202 Accepted)
pub fn assert_webhook_success(response: &axum::response::Response) {
    let status = response.status();
    assert!(
        status == StatusCode::OK || status == StatusCode::ACCEPTED,
        "Expected webhook success (200 or 202), got {}",
        status
    );
}

/// Assert webhook response is an error
pub fn assert_webhook_error(response: &axum::response::Response, expected_status: StatusCode) {
    assert_eq!(
        response.status(),
        expected_status,
        "Expected webhook error {}, got {}",
        expected_status,
        response.status()
    );
}

/// ============================================================================
/// Idempotency Helpers
/// ============================================================================
/// Generate idempotency key for Creem webhook
pub fn generate_webhook_idempotency_key(event_id: &str) -> String {
    format!("creem_{}", event_id)
}

/// ============================================================================
/// Test Data Generators
/// ============================================================================
/// Generate a unique event ID for testing
pub fn generate_test_event_id() -> String {
    format!("evt_test_{}", Uuid::now_v7())
}

/// Generate a unique user ID for testing
pub fn generate_test_user_id() -> Uuid {
    Uuid::now_v7()
}

/// Generate a unique plan ID for testing
pub fn generate_test_plan_id() -> Uuid {
    Uuid::now_v7()
}

/// ============================================================================
/// Legacy Builders (retained for existing tests that have not yet been migrated)
/// ============================================================================
/// These builders reference the pre-product_reduce schema (plan_id, planId).
/// New tests should use the herald_* metadata builders above.
/// BE-T03 / BE-T04 will migrate or remove the tests that depend on these.
///
/// Build a subscription.paid webhook event (legacy, uses plan_id)
pub fn build_subscription_paid_event(
    event_id: String,
    user_id: Uuid,
    plan_id: Uuid,
    is_renewal: bool,
    realm_id: &str,
) -> serde_json::Value {
    build_subscription_paid_event_with_client(
        event_id, user_id, plan_id, is_renewal, realm_id, None,
    )
}

/// Build a subscription.paid webhook event with client_app_id (legacy)
pub fn build_subscription_paid_event_with_client(
    event_id: String,
    user_id: Uuid,
    plan_id: Uuid,
    is_renewal: bool,
    realm_id: &str,
    client_app_id: Option<Uuid>,
) -> serde_json::Value {
    build_subscription_paid_event_with_client_and_status(
        event_id,
        user_id,
        plan_id,
        is_renewal,
        realm_id,
        client_app_id,
        "active",
    )
}

/// Build a subscription.paid webhook event with client_app_id and custom status (legacy)
pub fn build_subscription_paid_event_with_client_and_status(
    event_id: String,
    user_id: Uuid,
    plan_id: Uuid,
    is_renewal: bool,
    realm_id: &str,
    client_app_id: Option<Uuid>,
    status: &str,
) -> serde_json::Value {
    build_subscription_paid_event_full(
        event_id,
        user_id,
        plan_id,
        is_renewal,
        realm_id,
        client_app_id,
        status,
        "monthly",
    )
}

/// Build a subscription.paid webhook event with full customization (legacy)
pub fn build_subscription_paid_event_full(
    event_id: String,
    user_id: Uuid,
    plan_id: Uuid,
    is_renewal: bool,
    realm_id: &str,
    client_app_id: Option<Uuid>,
    status: &str,
    billing_period: &str,
) -> serde_json::Value {
    let period_end = match billing_period {
        "yearly" => chrono::Utc::now() + chrono::Duration::days(365),
        "quarterly" => chrono::Utc::now() + chrono::Duration::days(90),
        _ => chrono::Utc::now() + chrono::Duration::days(30),
    };

    let amount = match billing_period {
        "yearly" => 25000,
        "quarterly" => 7500,
        _ => 2500,
    };

    let mut metadata = json!({
        "realmId": realm_id
    });

    if let Some(client_id) = client_app_id {
        metadata["clientAppId"] = json!(client_id.to_string());
    }

    json!({
        "id": event_id,
        "eventType": "subscription.paid",
        "data": {
            "object": {
                "subscriptionId": format!("sub_{}", event_id),
                "productId": format!("prod_test_{}", billing_period),
                "userId": user_id.to_string(),
                "planId": plan_id.to_string(),
                "entitlementKey": plan_id.to_string(),
                "isRenewal": is_renewal,
                "currentPeriodEnd": period_end.to_rfc3339(),
                "amount": amount,
                "currency": "USD",
                "status": status,
                "cancelAtPeriodEnd": false,
                "billingPeriod": billing_period,
                "metadata": metadata
            }
        },
        "metadata": metadata
    })
}

/// Build a subscription.canceled webhook event (legacy)
pub fn build_subscription_canceled_event(
    event_id: String,
    user_id: Uuid,
    cancel_at_period_end: bool,
    realm_id: &str,
) -> serde_json::Value {
    let period_end = (chrono::Utc::now() + chrono::Duration::days(30)).to_rfc3339();

    json!({
        "id": event_id,
        "eventType": "subscription.canceled",
        "data": {
            "object": {
                "subscriptionId": format!("sub_{}", event_id),
                "productId": "prod_test_monthly",
                "userId": user_id.to_string(),
                "cancelAtPeriodEnd": cancel_at_period_end,
                "currentPeriodEnd": period_end
            }
        },
        "metadata": {
            "realmId": realm_id
        }
    })
}

/// Build a subscription.update webhook event (legacy)
pub fn build_subscription_updated_event(
    event_id: String,
    user_id: Uuid,
    previous_plan_id: Uuid,
    current_plan_id: Uuid,
    realm_id: &str,
) -> serde_json::Value {
    let period_end = (chrono::Utc::now() + chrono::Duration::days(30)).to_rfc3339();

    json!({
        "id": event_id,
        "eventType": "subscription.update",
        "data": {
            "object": {
                "subscriptionId": format!("sub_{}", event_id),
                "productId": "prod_test_monthly",
                "userId": user_id.to_string(),
                "planId": current_plan_id.to_string(),
                "entitlementKey": current_plan_id.to_string(),
                "currentPeriodEnd": period_end
            },
            "previousAttributes": {
                "planId": previous_plan_id.to_string(),
                "entitlementKey": previous_plan_id.to_string()
            }
        },
        "metadata": {
            "realmId": realm_id
        }
    })
}

/// Build a refund.created webhook event (legacy)
pub fn build_refund_created_event(
    event_id: String,
    refund_id: String,
    payment_id: String,
    amount: i64,
    original_amount: i64,
    realm_id: &str,
) -> serde_json::Value {
    build_refund_created_event_with_user(
        event_id,
        refund_id,
        payment_id,
        amount,
        original_amount,
        realm_id,
        Uuid::now_v7(),
    )
}

/// Build a refund.created webhook event with explicit user ID (legacy)
pub fn build_refund_created_event_with_user(
    event_id: String,
    refund_id: String,
    payment_id: String,
    amount: i64,
    original_amount: i64,
    realm_id: &str,
    user_id: Uuid,
) -> serde_json::Value {
    build_refund_created_event_with_user_and_type(
        event_id,
        refund_id,
        payment_id,
        amount,
        original_amount,
        realm_id,
        user_id,
        "topup",
    )
}

/// Build a refund.created webhook event with explicit user ID and refund type (legacy)
pub fn build_refund_created_event_with_user_and_type(
    event_id: String,
    refund_id: String,
    payment_id: String,
    amount: i64,
    original_amount: i64,
    realm_id: &str,
    user_id: Uuid,
    refund_type: &str,
) -> serde_json::Value {
    json!({
        "id": event_id,
        "eventType": "refund.created",
        "data": {
            "object": {
                "id": refund_id,
                "paymentId": payment_id,
                "amount": amount,
                "originalAmount": original_amount,
                "currency": "USD",
                "reason": "customer_request",
                "metadata": {
                    "userId": user_id.to_string(),
                    "refundType": refund_type
                }
            }
        },
        "metadata": {
            "realmId": realm_id
        }
    })
}

/// Setup a test plan config by creating a provider_entitlement_mappings entry.
pub async fn setup_test_plan_config(
    ctx: &crate::tests::schema_test_context::SchemaTestContext,
    realm_id: &str,
    plan_id: Uuid,
) {
    setup_test_plan_config_with_points(ctx, realm_id, plan_id, 1000).await;
}

/// Setup a test plan config with custom points_per_period by creating a provider_entitlement_mappings entry.
pub async fn setup_test_plan_config_with_points(
    ctx: &crate::tests::schema_test_context::SchemaTestContext,
    realm_id: &str,
    plan_id: Uuid,
    points_per_period: i64,
) {
    let external_product_id = format!("prod_test_{}", plan_id);
    let entitlement_key = plan_id.to_string();

    // Credit Buckets model (design §5.5): a subscription grant routes to the
    // bucket bound on the entitlement mapping. Bind both mappings below to the
    // realm's legacy test bucket so the lazy subscription-bucket resolution in
    // the Creem webhook handler succeeds (case "mapping present + bucket").
    let bucket_id = crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm(
        &ctx.app_state.pool,
        realm_id,
    )
    .await;

    // Create mapping with plan-specific external_product_id (for events that include entitlementKey)
    sqlx::query(
        "INSERT INTO provider_entitlement_mappings
            (id, realm_id, payment_provider, external_product_id, entitlement_key,
             billing_type, billing_period, points_per_period, grant_on_subscribe,
             validity_days, enabled, bucket_id, created_at, updated_at)
         VALUES ($1, $2, 'creem', $3, $4, 'recurring', 'monthly', $5, true, 30, true, $6, NOW(), NOW())
         ON CONFLICT DO NOTHING",
    )
    .bind(plan_id)
    .bind(realm_id)
    .bind(&external_product_id)
    .bind(&entitlement_key)
    .bind(points_per_period)
    .bind(bucket_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create test entitlement mapping");

    // Create additional mapping with generic "prod_test_monthly" for update/cancel events
    // that resolve entitlement_key via product ID lookup
    let generic_mapping_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO provider_entitlement_mappings
            (id, realm_id, payment_provider, external_product_id, entitlement_key,
             billing_type, billing_period, points_per_period, grant_on_subscribe,
             validity_days, enabled, bucket_id, created_at, updated_at)
         VALUES ($1, $2, 'creem', 'prod_test_monthly', $3, 'recurring', 'monthly', $4, true, 30, true, $5, NOW(), NOW())
         ON CONFLICT DO NOTHING",
    )
    .bind(generic_mapping_id)
    .bind(realm_id)
    .bind(&entitlement_key)
    .bind(points_per_period)
    .bind(bucket_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create generic test entitlement mapping");
}

/// ============================================================================
/// Database Setup Helpers (entitlement mapping for webhook tests)
/// ============================================================================
///
/// Create a test entitlement mapping for webhook tests via direct SQL.
///
/// This is self-contained in webhook_helpers.rs (not dependent on billing_helpers.rs)
/// because BE-T01 and BE-T02 run in parallel.
///
/// Returns the mapping ID.
#[allow(clippy::too_many_arguments)]
pub async fn setup_test_entitlement_mapping_for_webhook(
    ctx: &crate::tests::schema_test_context::SchemaTestContext,
    realm_id: &str,
    provider: &str,
    external_product_id: &str,
    entitlement_key: &str,
    points_per_period: i64,
    grant_on_subscribe: bool,
    enabled: bool,
) -> Uuid {
    let mapping_id = Uuid::now_v7();

    // Credit Buckets model: bucket_id is NOT NULL — bind the realm's legacy
    // test bucket (matches the bucket-bound mappings created elsewhere).
    let bucket_id = crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm(
        &ctx.app_state.pool,
        realm_id,
    )
    .await;

    sqlx::query(
        "INSERT INTO provider_entitlement_mappings
            (id, realm_id, payment_provider, external_product_id, entitlement_key,
             points_per_period, grant_on_subscribe, enabled, bucket_id, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW(), NOW())",
    )
    .bind(mapping_id)
    .bind(realm_id)
    .bind(provider)
    .bind(external_product_id)
    .bind(entitlement_key)
    .bind(points_per_period)
    .bind(grant_on_subscribe)
    .bind(enabled)
    .bind(bucket_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create test entitlement mapping for webhook");

    mapping_id
}

/// ============================================================================
/// Stripe Webhook Sending Helpers
/// ============================================================================
/// Build a Stripe webhook signature for testing
///
/// Follows Stripe's webhook signature format: t={timestamp},v1={signature}
pub fn build_stripe_webhook_signature(payload: &str, secret: &str) -> String {
    let timestamp = chrono::Utc::now().timestamp();
    let signed_payload = format!("{}.{}", timestamp, payload);

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(signed_payload.as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());

    format!("t={},v1={}", timestamp, signature)
}

/// Send a Stripe webhook event with signature
pub async fn send_stripe_webhook_with_signature(
    app: &axum::Router,
    realm_id: &str,
    payload: serde_json::Value,
    webhook_secret: &str,
) -> axum::response::Response {
    let payload_str = serde_json::to_string(&payload).unwrap();
    let signature = build_stripe_webhook_signature(&payload_str, webhook_secret);
    let uri = &format!("/api/third/pay/{}/stripe/webhooks", realm_id);

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("stripe-signature", signature)
                .header("Content-Type", "application/json")
                .body(Body::from(payload_str))
                .unwrap(),
        )
        .await
        .unwrap()
}

/// Send a Stripe webhook event with invalid signature
pub async fn send_stripe_webhook_with_invalid_signature(
    app: &axum::Router,
    realm_id: &str,
    payload: serde_json::Value,
) -> axum::response::Response {
    let payload_str = serde_json::to_string(&payload).unwrap();
    let invalid_signature = "t=1234567890,v1=invalid_signature_here_aabbccddeeff";
    let uri = &format!("/api/third/pay/{}/stripe/webhooks", realm_id);

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("stripe-signature", invalid_signature)
                .header("Content-Type", "application/json")
                .body(Body::from(payload_str))
                .unwrap(),
        )
        .await
        .unwrap()
}

/// Send a Stripe webhook event without signature
pub async fn send_stripe_webhook_without_signature(
    app: &axum::Router,
    realm_id: &str,
    payload: serde_json::Value,
) -> axum::response::Response {
    let payload_str = serde_json::to_string(&payload).unwrap();
    let uri = &format!("/api/third/pay/{}/stripe/webhooks", realm_id);

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("Content-Type", "application/json")
                .body(Body::from(payload_str))
                .unwrap(),
        )
        .await
        .unwrap()
}
