// =============================================================================
// Billing Test Helpers
// =============================================================================
//
// Shared helpers for billing-related API tests (subscription_scenarios, plan_scenarios)
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
use serde_json::json;
use sha2::Sha256;
use tower::ServiceExt;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

/// Ensure billing encryption has a deterministic test key before handlers encrypt secrets.
pub fn ensure_billing_test_encryption_key() {
    unsafe {
        std::env::set_var(
            "ENCRYPTION_KEY",
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
        );
    }
}

/// ============================================================================
/// Billing Test Setup Helpers
/// ============================================================================
///
/// Setup admin session for billing tests
pub async fn setup_billing_admin_session(ctx: &mut TestContext, email: &str) -> String {
    ensure_billing_test_encryption_key();

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
    ensure_billing_test_encryption_key();

    let (admin_token, user_id) =
        crate::tests::helpers::create_admin_session_with_user(ctx, email, 1800).await;

    // Grant Realm Admin role
    crate::tests::helpers::grant_realm_admin_role(ctx, &user_id).await;

    let user_uuid = Uuid::parse_str(&user_id).expect("Invalid user_id format");
    (admin_token, user_uuid)
}

/// ============================================================================
/// Test Data Creation Helpers
/// ============================================================================
///
/// Ensure a default product exists for the given realm and return its ID.
///
/// Creates a default product if one does not already exist for the realm.
/// This is needed because the plan table has a NOT NULL product_id FK column.
pub async fn ensure_default_product(ctx: &mut TestContext, realm_id: &str) -> Uuid {
    // Check if default product already exists for this realm
    let existing: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM products WHERE realm_id = $1 AND code = 'default' LIMIT 1",
    )
    .bind(realm_id)
    .fetch_optional(&ctx.app_state.pool)
    .await
    .unwrap();

    if let Some(id) = existing {
        return id;
    }

    // Create default product
    let product_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO products (id, realm_id, code, title, description, enabled, created_at, updated_at)
         VALUES ($1, $2, 'default', 'Default Product', 'Default test product', true, NOW(), NOW())"
    )
    .bind(product_id)
    .bind(realm_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create default product");

    product_id
}

/// Create a test plan config via direct SQL insertion
///
/// Uses the new flexible grant period schema (grant_period_type, points_per_period, etc.)
///
/// Returns the config_id
pub async fn create_test_plan_config(
    ctx: &mut TestContext,
    realm_id: &str,
    plan_id: Uuid,
    points_per_period: i64,
    validity_days: i64,
) -> Uuid {
    let config_id = Uuid::now_v7();

    sqlx::query(
        "INSERT INTO points_plan_configs (id, realm_id, plan_id, grant_period_type, points_per_period, validity_days, grant_on_subscribe, max_periods, active)
         VALUES ($1, $2, $3, 'monthly', $4, $5, true, NULL, true)"
    )
    .bind(config_id)
    .bind(realm_id)
    .bind(plan_id)
    .bind(points_per_period)
    .bind(validity_days)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create plan config");

    config_id
}

/// Create a test plan via direct SQL insertion
///
/// Returns the plan_id
pub async fn create_test_plan(ctx: &mut TestContext, realm_id: &str, name: &str) -> Uuid {
    create_test_plan_with_attrs(ctx, realm_id, name, "monthly", 2500).await
}

/// Create a test plan with custom attributes via direct SQL insertion
pub async fn create_test_plan_with_attrs(
    ctx: &mut TestContext,
    realm_id: &str,
    name: &str,
    plan_type: &str,
    price_cents: i64,
) -> Uuid {
    let plan_id = Uuid::now_v7();
    let product_id = ensure_default_product(ctx, realm_id).await;

    sqlx::query(
        "INSERT INTO subscription_plan (id, realm_id, name, description, title, type, price, currency,
                          active, trial_days, sort_order, product_id, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, NOW(), NOW())",
    )
    .bind(plan_id)
    .bind(realm_id)
    .bind(name)
    .bind(format!("{} description", name))
    .bind(name) // title
    .bind(plan_type) // type
    .bind(price_cents) // price in cents
    .bind("USD") // currency
    .bind(true)
    .bind(0)
    .bind(1)
    .bind(product_id) // product_id
    .execute(&ctx.app_state.pool)
    .await
    .unwrap();

    // Create a default points plan config with 1000 points per period, 30-day validity
    create_test_plan_config(ctx, realm_id, plan_id, 1000, 30).await;

    plan_id
}

/// Create a test subscription via direct SQL insertion
///
/// Returns the subscription_id
pub async fn create_test_subscription(
    ctx: &mut TestContext,
    realm_id: &str,
    client_app_id: Uuid,
    plan_id: Uuid,
    billing_period: &str,
) -> Uuid {
    let subscription_id = Uuid::now_v7();
    let external_subscription_id = format!("sub_test_{}", subscription_id);

    sqlx::query(
        "INSERT INTO subscription (id, realm_id, plan_id, client_app_id, status, billing_period, tier,
                                 external_product_id, external_subscription_id, payment_provider,
                                 current_period_start, current_period_end,
                                 cancel_at_period_end, created_at, updated_at)
         VALUES ($1, $2, $3, $4, 'active', $5, 'free', $6, $7, 'creem', NOW(), NOW() + INTERVAL '30 days',
                 false, NOW(), NOW())",
    )
    .bind(subscription_id)
    .bind(realm_id)
    .bind(plan_id)
    .bind(client_app_id)
    .bind(billing_period)
    .bind(format!("prod_{}", subscription_id))  // external_product_id
    .bind(&external_subscription_id)  // external_subscription_id
    .execute(&ctx.app_state.pool)
    .await
    .unwrap();

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
/// ============================================================================
///
/// JSON payload for creating a basic client app
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
/// Create a checkout session via API
///
/// Returns the checkout session response JSON
pub async fn create_checkout_session(
    _ctx: &TestContext,
    app: &axum::Router,
    realm_id: &str,
    client_app_id: Uuid,
    plan_id: Uuid,
    billing_period: &str,
    admin_token: &str,
) -> serde_json::Value {
    let request_payload = json!({
        "planId": plan_id.to_string(),
        "billingPeriod": billing_period,
        "successUrl": "https://example.com/success",
        "cancelUrl": "https://example.com/cancel"
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/bill/{}/client/{}/checkout",
                    realm_id, client_app_id
                ))
                .header("Content-Type", "application/json")
                .header("cookie", format!("X-Auth={}", admin_token))
                .body(Body::from(request_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Failed to create checkout session"
    );

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&body).unwrap()
}

/// Send a webhook event to the system
///
/// # Arguments
/// * `app` - The axum router
/// * `realm_id` - The realm ID
/// * `payload` - Webhook payload
/// * `webhook_secret` - Webhook secret for signature generation
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
///
/// # Arguments
/// * `ctx` - Test context
/// * `subscription_id` - Subscription ID to check
/// * `expected_status` - Expected subscription status
///
/// Panics if subscription doesn't exist or status doesn't match
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
///
/// # Arguments
/// * `ctx` - Test context
/// * `creem_event_id` - Creem event ID to check
///
/// Returns true if payment event exists
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
///
/// # Arguments
/// * `ctx` - Test context
/// * `client_app_id` - Client app ID
///
/// Returns the subscription ID if exists
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
/// Multi-Period Plan Creation Helpers
/// ============================================================================
/// Create a yearly test plan
///
/// Returns the plan_id
pub async fn create_test_plan_yearly(ctx: &mut TestContext, realm_id: &str, name: &str) -> Uuid {
    create_test_plan_with_attrs(ctx, realm_id, name, "yearly", 25000).await
}

/// Create a quarterly test plan
///
/// Returns the plan_id
pub async fn create_test_plan_quarterly(ctx: &mut TestContext, realm_id: &str, name: &str) -> Uuid {
    create_test_plan_with_attrs(ctx, realm_id, name, "quarterly", 7500).await
}

/// Create a test plan with trial days
///
/// # Arguments
/// * `ctx` - Test context
/// * `realm_id` - Realm ID
/// * `name` - Plan name
/// * `plan_type` - Plan type (monthly, yearly, quarterly)
/// * `price_cents` - Price in cents
/// * `trial_days` - Number of trial days
///
/// Returns the plan_id
pub async fn create_test_plan_with_trial(
    ctx: &mut TestContext,
    realm_id: &str,
    name: &str,
    plan_type: &str,
    price_cents: i64,
    trial_days: i32,
) -> Uuid {
    let plan_id = Uuid::now_v7();
    let product_id = ensure_default_product(ctx, realm_id).await;

    sqlx::query(
        "INSERT INTO subscription_plan (id, realm_id, name, description, title, type, price, currency,
                          active, trial_days, sort_order, product_id, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, NOW(), NOW())",
    )
    .bind(plan_id)
    .bind(realm_id)
    .bind(name)
    .bind(format!("{} description", name))
    .bind(name) // title
    .bind(plan_type) // type
    .bind(price_cents) // price in cents
    .bind("USD") // currency
    .bind(true)
    .bind(trial_days)
    .bind(1)
    .bind(product_id) // product_id
    .execute(&ctx.app_state.pool)
    .await
    .unwrap();

    // Create a default points plan config with 1000 points per period, 30-day validity
    create_test_plan_config(ctx, realm_id, plan_id, 1000, 30).await;

    plan_id
}

/// ============================================================================
/// Subscription Status Transition Helpers
/// ============================================================================
/// Update subscription status directly via SQL
///
/// # Arguments
/// * `ctx` - Test context
/// * `subscription_id` - Subscription ID
/// * `new_status` - New status
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
///
/// # Arguments
/// * `ctx` - Test context
/// * `subscription_id` - Subscription ID
/// * `period_start` - New period start
/// * `period_end` - New period end
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
/// ============================================================================
/// Setup Stripe configuration for a test realm
///
/// This helper inserts Stripe configuration into the realm_config table,
/// which is where the Stripe client reads it from.
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

/// Create a test plan with Stripe as the payment provider
///
/// # Arguments
/// * `ctx` - Test context
/// * `realm_id` - Realm ID
/// * `name` - Plan name
/// * `plan_type` - Plan type (monthly, yearly, quarterly)
/// * `price_cents` - Price in cents
/// * `stripe_product_id` - Stripe product ID
///
/// Returns the plan_id
pub async fn create_stripe_test_plan(
    ctx: &mut TestContext,
    realm_id: &str,
    name: &str,
    plan_type: &str,
    price_cents: i64,
    _stripe_product_id: &str,
) -> Uuid {
    let plan_id = Uuid::now_v7();
    let product_id = ensure_default_product(ctx, realm_id).await;

    sqlx::query(
        "INSERT INTO subscription_plan (id, realm_id, name, description, title, type, price, currency,
                          active, trial_days, sort_order, product_id, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, NOW(), NOW())",
    )
    .bind(plan_id)
    .bind(realm_id)
    .bind(name)
    .bind(format!("{} description", name))
    .bind(name) // title
    .bind(plan_type) // type
    .bind(price_cents) // price in cents
    .bind("USD") // currency
    .bind(true)
    .bind(0)
    .bind(1)
    .bind(product_id) // product_id
    .execute(&ctx.app_state.pool)
    .await
    .unwrap();

    // Create a default points plan config with 1000 points per period, 30-day validity
    create_test_plan_config(ctx, realm_id, plan_id, 1000, 30).await;

    plan_id
}

/// Create a test plan with Stripe and trial days
///
/// # Arguments
/// * `ctx` - Test context
/// * `realm_id` - Realm ID
/// * `name` - Plan name
/// * `plan_type` - Plan type (monthly, yearly, quarterly)
/// * `price_cents` - Price in cents
/// * `stripe_product_id` - Stripe product ID
/// * `trial_days` - Number of trial days
///
/// Returns the plan_id
pub async fn create_stripe_test_plan_with_trial(
    ctx: &mut TestContext,
    realm_id: &str,
    name: &str,
    plan_type: &str,
    price_cents: i64,
    _stripe_product_id: &str,
    trial_days: i32,
) -> Uuid {
    let plan_id = Uuid::now_v7();
    let product_id = ensure_default_product(ctx, realm_id).await;

    sqlx::query(
        "INSERT INTO subscription_plan (id, realm_id, name, description, title, type, price, currency,
                          active, trial_days, sort_order, product_id, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, NOW(), NOW())",
    )
    .bind(plan_id)
    .bind(realm_id)
    .bind(name)
    .bind(format!("{} description", name))
    .bind(name) // title
    .bind(plan_type) // type
    .bind(price_cents) // price in cents
    .bind("USD") // currency
    .bind(true)
    .bind(trial_days)
    .bind(1)
    .bind(product_id) // product_id
    .execute(&ctx.app_state.pool)
    .await
    .unwrap();

    // Create a default points plan config with 1000 points per period, 30-day validity
    create_test_plan_config(ctx, realm_id, plan_id, 1000, 30).await;

    plan_id
}

/// Verify payment event exists with Stripe event ID
///
/// # Arguments
/// * `ctx` - Test context
/// * `stripe_event_id` - Stripe event ID to check
///
/// Returns true if payment event exists
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
///
/// # Arguments
/// * `ctx` - Test context
/// * `stripe_subscription_id` - Stripe subscription ID
///
/// Returns the subscription ID if exists
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
/// List user-visible points packages via the external API endpoint.
///
/// Uses API Key authentication (X-API-Key header).
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
