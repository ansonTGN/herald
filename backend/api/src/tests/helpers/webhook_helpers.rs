// =============================================================================
// Webhook Test Helpers
// =============================================================================
//
// Shared helpers for webhook testing.
// Provides functions for building webhook events and sending them to test servers.
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
/// Webhook Event Builders
/// ============================================================================
/// Build a subscription.paid webhook event
pub fn build_subscription_paid_event(
    event_id: String,
    user_id: Uuid,
    plan_id: Uuid,
    is_renewal: bool,
    realm_id: &str,
) -> serde_json::Value {
    build_subscription_paid_event_with_client(
        event_id, user_id, plan_id, is_renewal, realm_id, None, // client_app_id is optional
    )
}

/// Build a subscription.paid webhook event with client_app_id
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
        "active", // Default status
    )
}

/// Build a subscription.paid webhook event with client_app_id and custom status
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
        "monthly", // Default billing period
    )
}

/// Build a subscription.paid webhook event with full customization
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
    // Calculate period_end based on billing period
    let period_end = match billing_period {
        "yearly" => chrono::Utc::now() + chrono::Duration::days(365),
        "quarterly" => chrono::Utc::now() + chrono::Duration::days(90),
        _ => chrono::Utc::now() + chrono::Duration::days(30), // monthly
    };

    let amount = match billing_period {
        "yearly" => 25000,
        "quarterly" => 7500,
        _ => 2500, // monthly
    };

    let mut metadata = json!({
        "realmId": realm_id
    });

    // Add clientAppId to metadata if provided
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

/// Build a subscription.update webhook event
pub fn build_subscription_updated_event(
    event_id: String,
    user_id: Uuid,
    previous_plan_id: Uuid,
    current_plan_id: Uuid,
    realm_id: &str,
) -> serde_json::Value {
    // Create a date 30 days in the future for currentPeriodEnd
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
                "currentPeriodEnd": period_end
            },
            "previousAttributes": {
                "planId": previous_plan_id.to_string()
            }
        },
        "metadata": {
            "realmId": realm_id
        }
    })
}

/// Build a subscription.canceled webhook event
pub fn build_subscription_canceled_event(
    event_id: String,
    user_id: Uuid,
    cancel_at_period_end: bool,
    realm_id: &str,
) -> serde_json::Value {
    // Create a date 30 days in the future for currentPeriodEnd
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

/// Build a refund.created webhook event
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
        Uuid::now_v7(), // Generate a default user ID
    )
}

/// Build a refund.created webhook event with explicit user ID
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
        "topup", // Default to topup refund type
    )
}

/// Build a refund.created webhook event with explicit user ID and refund type
pub fn build_refund_created_event_with_user_and_type(
    event_id: String,
    refund_id: String,
    payment_id: String,
    amount: i64,
    original_amount: i64,
    realm_id: &str,
    user_id: Uuid,
    refund_type: &str, // "topup" or "subscription"
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
/// Database Setup Helpers
/// ============================================================================
/// Create a test plan and its config in the database
///
/// This is needed for webhook tests that require plan configs to exist.
/// Creates both the plan record (to satisfy foreign key constraints) and
/// the points_plan_configs record.
pub async fn setup_test_plan_config(
    ctx: &crate::tests::schema_test_context::SchemaTestContext,
    realm_id: &str,
    plan_id: Uuid,
) {
    setup_test_plan_config_with_points(ctx, realm_id, plan_id, 1000).await;
}

/// Create a test plan and config with custom point values
///
/// This is needed for webhook tests that require different point values for different plans.
/// Creates both the plan record (to satisfy foreign key constraints) and
/// the points_plan_configs record with the specified point amounts.
pub async fn setup_test_plan_config_with_points(
    ctx: &crate::tests::schema_test_context::SchemaTestContext,
    realm_id: &str,
    plan_id: Uuid,
    points_per_period: i64,
) {
    // Ensure default product exists for this realm
    let product_id: Uuid = sqlx::query_scalar(
        "INSERT INTO products (id, realm_id, name, title, description, sort_order, enabled, created_at, updated_at)
         VALUES ($1, $2, 'default', 'Default Product', 'Default test product', 0, true, NOW(), NOW())
         ON CONFLICT (realm_id, name) DO UPDATE SET updated_at = products.updated_at
         RETURNING id"
    )
    .bind(Uuid::now_v7())
    .bind(realm_id)
    .fetch_one(&ctx.app_state.pool)
    .await
    .expect("Failed to ensure default product");

    // First, create the plan record to satisfy foreign key constraint
    let plan_name = format!("Test Plan {}", plan_id);
    sqlx::query(
        "INSERT INTO plan (id, realm_id, name, description, title, type, price, currency,
                          active, trial_days, sort_order, product_id, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, NOW(), NOW())",
    )
    .bind(plan_id)
    .bind(realm_id)
    .bind(&plan_name)
    .bind(format!("{} description", plan_name))
    .bind(&plan_name) // title
    .bind("monthly") // type
    .bind(2500) // price in cents
    .bind("USD") // currency
    .bind(true)
    .bind(0)
    .bind(1)
    .bind(product_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create test plan");

    // Then create the points plan config with custom points
    sqlx::query(
        "INSERT INTO points_plan_configs (id, realm_id, plan_id, grant_period_type, points_per_period,
         validity_days, grant_on_subscribe, max_periods, active, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW(), NOW())"
    )
    .bind(Uuid::now_v7())
    .bind(realm_id)
    .bind(plan_id)
    .bind("monthly") // grant_period_type
    .bind(points_per_period) // points_per_period - amount granted each period
    .bind(0i64) // validity_days - 0 means permanent (matches old behavior)
    .bind(true) // grant_on_subscribe - always grant on subscribe
    .bind::<Option<i64>>(None) // max_periods - NULL means unlimited (matches old behavior)
    .bind(true) // active
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create test plan config");
}

/// ============================================================================
/// Stripe Webhook Helpers
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

/// Build a Stripe checkout.session.completed webhook event
pub fn build_stripe_checkout_session_completed_event(
    event_id: String,
    client_app_id: Uuid,
    plan_id: Uuid,
    realm_id: &str,
    user_id: Option<Uuid>,
    trial_days: Option<u64>,
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
                "subscription": format!("sub_test_{}", Uuid::now_v7()),
                "payment_intent": format!("pi_test_{}", Uuid::now_v7()),
                "metadata": {
                    "realmId": realm_id,
                    "clientAppId": client_app_id.to_string(),
                    "planId": plan_id.to_string(),
                    "userId": user_id.map(|u| u.to_string()).unwrap_or_default(),
                    "billingPeriod": "monthly",
                    "planName": "Test Plan",
                    "trialDays": trial_days.unwrap_or(0)
                },
                "display_items": [{
                    "price": {
                        "product": format!("prod_test_{}", plan_id)
                    }
                }]
            }
        }
    })
}

/// Build a Stripe invoice.payment_succeeded webhook event (subscription renewal)
pub fn build_stripe_invoice_payment_succeeded_event(
    event_id: String,
    subscription_id: String,
    realm_id: &str,
    user_id: &str,
    plan_id: &str,
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
                "subscription": subscription_id,
                "amount_paid": amount_paid,
                "currency": "usd",
                "current_period_start": period_start,
                "current_period_end": period_end,
                "metadata": {
                    "realmId": realm_id,
                    "userId": user_id,
                    "planId": plan_id
                }
            }
        }
    })
}

/// Build a Stripe customer.subscription.updated webhook event (plan upgrade/downgrade)
pub fn build_stripe_subscription_updated_event(
    event_id: String,
    stripe_subscription_id: String,
    realm_id: &str,
    previous_plan_id: Option<String>,
    new_plan_id: String,
) -> serde_json::Value {
    let period_end = (chrono::Utc::now() + chrono::Duration::days(30)).to_rfc3339();

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
                            "product": new_plan_id,
                            "metadata": {
                                "planId": new_plan_id
                            }
                        }
                    }]
                },
                "metadata": {
                    "realmId": realm_id,
                    "userId": "test_user_id"
                }
            },
            "previous_attributes": {
                "items": {
                    "data": [{
                        "price": {
                            "product": previous_plan_id.clone().unwrap_or_else(|| "old_plan".to_string()),
                            "metadata": {
                                "planId": previous_plan_id.unwrap_or_else(|| "old_plan".to_string())
                            }
                        }
                    }]
                }
            }
        }
    })
}

/// Build a Stripe customer.subscription.deleted webhook event (cancellation)
pub fn build_stripe_subscription_deleted_event(
    event_id: String,
    stripe_subscription_id: String,
    realm_id: &str,
    cancel_at_period_end: bool,
) -> serde_json::Value {
    let period_end = (chrono::Utc::now() + chrono::Duration::days(30)).to_rfc3339();

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
                "status": if cancel_at_period_end { "active" } else { "canceled" },
                "cancel_at_period_end": cancel_at_period_end,
                "current_period_end": period_end,
                "metadata": {
                    "realmId": realm_id,
                    "userId": "test_user_id"
                }
            }
        }
    })
}

/// Build a Stripe charge.refunded webhook event
pub fn build_stripe_charge_refunded_event(
    event_id: String,
    charge_id: String,
    realm_id: &str,
    user_id: &str,
    amount_refunded: i64,
    original_amount: i64,
) -> serde_json::Value {
    json!({
        "id": event_id,
        "object": "event",
        "type": "charge.refunded",
        "api_version": "2020-08-27",
        "created": chrono::Utc::now().timestamp(),
        "data": {
            "object": {
                "id": charge_id,
                "object": "charge",
                "amount": amount_refunded,
                "amount_refunded": amount_refunded,
                "currency": "usd",
                "paid": true,
                "refunded": amount_refunded == original_amount,
                "metadata": {
                    "realmId": realm_id,
                    "userId": user_id,
                    "refundType": "subscription"
                }
            }
        }
    })
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
