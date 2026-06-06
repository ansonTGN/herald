// =============================================================================
// Billing Test Helpers
// =============================================================================
//
// Shared helpers for billing-related API tests.
// Adapted for product_reduce: subscription uses entitlement_key instead of
// plan_id/tier/billing_period; Product/Plan helpers removed; entitlement
// mapping helpers added.
//
// =============================================================================

#![allow(dead_code)]

use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use herald_core::domain::billing::entities::SubscriptionStatus;
use hex;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use tower::ServiceExt;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

/// ============================================================================
/// Billing Test Setup Helpers
/// ============================================================================
///
/// Setup admin session for billing tests
pub async fn setup_billing_admin_session(ctx: &mut TestContext, email: &str) -> String {
    let (admin_token, user_id) =
        crate::tests::helpers::create_admin_session_with_user(ctx, email, 1800).await;

    // Grant Realm Admin role
    crate::tests::helpers::grant_realm_admin_role(ctx, &user_id).await;

    admin_token
}

/// Setup admin session for billing tests and return both token and user_id
pub async fn setup_billing_admin_session_with_user(
    ctx: &mut TestContext,
    email: &str,
) -> (String, Uuid) {
    let (admin_token, user_id) =
        crate::tests::helpers::create_admin_session_with_user(ctx, email, 1800).await;

    // Grant Realm Admin role
    crate::tests::helpers::grant_realm_admin_role(ctx, &user_id).await;

    let user_uuid = Uuid::parse_str(&user_id).expect("Invalid user_id format");
    (admin_token, user_uuid)
}

/// ============================================================================
/// Entitlement Mapping Test Data Creation Helpers
/// =============================================================================
///
/// Create a test entitlement mapping via direct SQL insertion.
/// Returns the mapping ID.
pub async fn setup_test_entitlement_mapping(
    ctx: &mut TestContext,
    realm_id: &str,
    payment_provider: &str,
    external_product_id: &str,
    entitlement_key: &str,
) -> Uuid {
    let mapping_id = Uuid::now_v7();

    sqlx::query(
        "INSERT INTO provider_entitlement_mappings
            (id, realm_id, payment_provider, external_product_id, entitlement_key,
             grant_on_subscribe, enabled, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, false, false, NOW(), NOW())",
    )
    .bind(mapping_id)
    .bind(realm_id)
    .bind(payment_provider)
    .bind(external_product_id)
    .bind(entitlement_key)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create test entitlement mapping");

    mapping_id
}

/// Create a test entitlement mapping with full points policy via direct SQL insertion.
///
/// Returns the mapping ID.
#[allow(clippy::too_many_arguments)]
pub async fn setup_test_entitlement_mapping_with_points(
    ctx: &mut TestContext,
    realm_id: &str,
    payment_provider: &str,
    external_product_id: &str,
    entitlement_key: &str,
    points_per_period: i64,
    grant_on_subscribe: bool,
    enabled: bool,
) -> Uuid {
    let mapping_id = Uuid::now_v7();

    sqlx::query(
        "INSERT INTO provider_entitlement_mappings
            (id, realm_id, payment_provider, external_product_id, entitlement_key,
             points_per_period, grant_on_subscribe, enabled, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), NOW())",
    )
    .bind(mapping_id)
    .bind(realm_id)
    .bind(payment_provider)
    .bind(external_product_id)
    .bind(entitlement_key)
    .bind(points_per_period)
    .bind(grant_on_subscribe)
    .bind(enabled)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create test entitlement mapping with points");

    mapping_id
}

/// Create a full entitlement mapping with all optional fields via direct SQL.
#[allow(clippy::too_many_arguments)]
pub async fn setup_test_entitlement_mapping_full(
    ctx: &mut TestContext,
    realm_id: &str,
    payment_provider: &str,
    external_product_id: &str,
    external_price_id: Option<&str>,
    entitlement_key: &str,
    billing_type: Option<&str>,
    billing_period: Option<&str>,
    points_per_period: Option<i64>,
    grant_period_type: Option<&str>,
    validity_days: Option<i64>,
    grant_on_subscribe: bool,
    max_periods: Option<i64>,
    enabled: bool,
    provider_product_info: Option<serde_json::Value>,
) -> Uuid {
    let mapping_id = Uuid::now_v7();

    sqlx::query(
        "INSERT INTO provider_entitlement_mappings
            (id, realm_id, payment_provider, external_product_id, external_price_id, entitlement_key,
             billing_type, billing_period, points_per_period, grant_period_type, validity_days,
             grant_on_subscribe, max_periods, enabled, provider_product_info, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, NOW(), NOW())",
    )
    .bind(mapping_id)
    .bind(realm_id)
    .bind(payment_provider)
    .bind(external_product_id)
    .bind(external_price_id)
    .bind(entitlement_key)
    .bind(billing_type)
    .bind(billing_period)
    .bind(points_per_period)
    .bind(grant_period_type)
    .bind(validity_days)
    .bind(grant_on_subscribe)
    .bind(max_periods)
    .bind(enabled)
    .bind(provider_product_info)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create full test entitlement mapping");

    mapping_id
}

/// ============================================================================
/// Subscription Test Data Creation Helpers
/// =============================================================================
///
/// Create a test subscription with entitlement_key via direct SQL insertion.
/// Uses the new schema (entitlement_key, external_price_id, provider_metadata).
/// Returns the subscription ID.
pub async fn create_test_subscription_with_entitlement(
    ctx: &mut TestContext,
    realm_id: &str,
    client_app_id: Uuid,
    entitlement_key: &str,
    external_price_id: &str,
) -> Uuid {
    let subscription_id = Uuid::now_v7();
    let external_subscription_id = format!("sub_test_{}", subscription_id);

    sqlx::query(
        "INSERT INTO subscription
            (id, realm_id, client_app_id, status, entitlement_key, external_price_id,
             external_subscription_id, external_product_id, payment_provider,
             current_period_start, current_period_end,
             cancel_at_period_end, created_at, updated_at)
         VALUES ($1, $2, $3, 'active', $4, $5,
                 $6, $7, 'creem', NOW(), NOW() + INTERVAL '30 days',
                 false, NOW(), NOW())",
    )
    .bind(subscription_id)
    .bind(realm_id)
    .bind(client_app_id)
    .bind(entitlement_key)
    .bind(external_price_id)
    .bind(&external_subscription_id)
    .bind(format!("prod_{}", subscription_id))
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create test subscription with entitlement");

    subscription_id
}

/// Delete a subscription via SQL (for cleanup)
pub async fn delete_test_subscription(ctx: &mut TestContext, subscription_id: Uuid) {
    sqlx::query("DELETE FROM subscription WHERE id = $1")
        .bind(subscription_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();
}

/// Delete subscriptions by client app ID (for cleanup)
pub async fn delete_subscriptions_by_client_app(ctx: &mut TestContext, client_app_id: Uuid) {
    sqlx::query("DELETE FROM subscription WHERE client_app_id = $1")
        .bind(client_app_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();
}

/// ============================================================================
/// Client App Creation Helper
/// =============================================================================
///
pub fn client_app_create_json(client_id: &str, name: &str, redirect_uris: &[&str]) -> String {
    use serde_json::json;

    let payload = json!({
        "clientId": client_id,
        "name": name,
        "redirectUris": redirect_uris,
        "enabled": true
    });

    payload.to_string()
}

/// ============================================================================
/// Payment Flow Helpers
/// ============================================================================
///
/// Send a webhook event to the system (Creem)
///
/// Returns the HTTP response
pub async fn send_webhook_event(
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

/// Verify subscription status in database
pub async fn verify_subscription_status(
    ctx: &TestContext,
    subscription_id: Uuid,
    expected_status: SubscriptionStatus,
) {
    let status_str: String = sqlx::query_scalar("SELECT status FROM subscription WHERE id = $1")
        .bind(subscription_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .expect("Subscription not found");

    let actual_status: SubscriptionStatus = status_str
        .parse()
        .expect("Invalid subscription status in database");

    assert_eq!(
        actual_status, expected_status,
        "Expected status {:?}, got {:?}",
        expected_status, actual_status
    );
}

/// Verify payment event exists in database
pub async fn verify_payment_event_exists(ctx: &TestContext, creem_event_id: &str) -> bool {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM payment_event WHERE external_event_id = $1 AND payment_provider = 'creem'")
            .bind(creem_event_id)
            .fetch_one(&ctx.app_state.pool)
            .await
            .unwrap();

    count > 0
}

/// Get subscription by client app ID
pub async fn get_subscription_by_client_app(
    ctx: &TestContext,
    client_app_id: Uuid,
) -> Option<Uuid> {
    sqlx::query_scalar("SELECT id FROM subscription WHERE client_app_id = $1")
        .bind(client_app_id)
        .fetch_optional(&ctx.app_state.pool)
        .await
        .unwrap()
}

/// ============================================================================
/// Subscription Status Transition Helpers
/// ============================================================================
///
/// Update subscription status directly via SQL
pub async fn update_subscription_status(
    ctx: &mut TestContext,
    subscription_id: Uuid,
    new_status: &str,
) {
    // When canceling, also set cancel_at to now
    if new_status == "canceled" {
        sqlx::query(
            "UPDATE subscription SET status = $1, cancel_at = NOW(), updated_at = NOW() WHERE id = $2"
        )
        .bind(new_status)
        .bind(subscription_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();
    } else {
        sqlx::query("UPDATE subscription SET status = $1, updated_at = NOW() WHERE id = $2")
            .bind(new_status)
            .bind(subscription_id)
            .execute(&ctx.app_state.pool)
            .await
            .unwrap();
    }
}

/// Update subscription period dates
pub async fn update_subscription_period(
    ctx: &mut TestContext,
    subscription_id: Uuid,
    period_start: chrono::DateTime<chrono::Utc>,
    period_end: chrono::DateTime<chrono::Utc>,
) {
    sqlx::query(
        "UPDATE subscription SET current_period_start = $1, current_period_end = $2, updated_at = NOW() WHERE id = $3"
    )
    .bind(period_start)
    .bind(period_end)
    .bind(subscription_id)
    .execute(&ctx.app_state.pool)
    .await
    .unwrap();
}

/// ============================================================================
/// Cleanup Helpers
/// ============================================================================
///
/// Clean up payment events for a specific subscription
pub async fn cleanup_payment_events(ctx: &mut TestContext, subscription_id: Uuid) {
    sqlx::query("DELETE FROM payment_event WHERE subscription_id = $1")
        .bind(subscription_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();
}

/// Clean up payment events by Creem event ID
pub async fn cleanup_payment_event_by_creem_id(ctx: &mut TestContext, creem_event_id: &str) {
    sqlx::query(
        "DELETE FROM payment_event WHERE external_event_id = $1 AND payment_provider = 'creem'",
    )
    .bind(creem_event_id)
    .execute(&ctx.app_state.pool)
    .await
    .unwrap();
}

/// ============================================================================
/// Stripe Configuration Helpers
/// =============================================================================
///
/// Setup Stripe configuration for a test realm
pub async fn setup_stripe_config(
    ctx: &TestContext,
    realm_id: &str,
    api_key: &str,
    webhook_secret: &str,
) {
    // Insert Stripe API key
    sqlx::query(
        "INSERT INTO realm_config (realm_id, config_type, config_key, config_value, enabled, created_at, updated_at)
         VALUES ($1, 'stripe', $2, $3, true, NOW(), NOW())
         ON CONFLICT (realm_id, config_type, config_key) DO UPDATE SET config_value = $3, enabled = true, updated_at = NOW()"
    )
    .bind(realm_id)
    .bind("api_key")
    .bind(api_key)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to insert Stripe API key");

    // Insert Stripe webhook secret
    sqlx::query(
        "INSERT INTO realm_config (realm_id, config_type, config_key, config_value, enabled, created_at, updated_at)
         VALUES ($1, 'stripe', $2, $3, true, NOW(), NOW())
         ON CONFLICT (realm_id, config_type, config_key) DO UPDATE SET config_value = $3, enabled = true, updated_at = NOW()"
    )
    .bind(realm_id)
    .bind("webhook_secret")
    .bind(webhook_secret)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to insert Stripe webhook secret");

    // Insert Stripe timeout (default 30 seconds)
    sqlx::query(
        "INSERT INTO realm_config (realm_id, config_type, config_key, config_value, enabled, created_at, updated_at)
         VALUES ($1, 'stripe', $2, $3, true, NOW(), NOW())
         ON CONFLICT (realm_id, config_type, config_key) DO UPDATE SET config_value = $3, enabled = true, updated_at = NOW()"
    )
    .bind(realm_id)
    .bind("timeout")
    .bind("30")
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to insert Stripe timeout");
}

/// Verify payment event exists with Stripe event ID
pub async fn verify_stripe_payment_event_exists(ctx: &TestContext, stripe_event_id: &str) -> bool {
    let count: i64 =
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM payment_event WHERE external_event_id = $1 AND payment_provider = 'stripe'"
        )
        .bind(stripe_event_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap();

    count > 0
}

/// Get subscription by Stripe subscription ID
pub async fn get_subscription_by_stripe_id(
    ctx: &TestContext,
    stripe_subscription_id: &str,
) -> Option<Uuid> {
    sqlx::query_scalar(
        "SELECT id FROM subscription WHERE external_subscription_id = $1 AND payment_provider = 'stripe'"
    )
    .bind(stripe_subscription_id)
    .fetch_optional(&ctx.app_state.pool)
    .await
    .unwrap()
}

/// ============================================================================
/// User-Facing Ext API Helpers
/// ============================================================================
///
/// List user-visible points packages via the external API endpoint.
///
/// Returns (StatusCode, response body as serde_json::Value)
pub async fn list_user_visible_points_packages_via_ext_api(
    app: &axum::Router,
    realm_id: &str,
    api_key: &str,
) -> (StatusCode, serde_json::Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/ext/{}/points-packages", realm_id))
                .header("X-API-Key", api_key)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_json: serde_json::Value =
        serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
    (status, body_json)
}
