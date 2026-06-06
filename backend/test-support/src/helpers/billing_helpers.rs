// =============================================================================
// Billing Test Helpers
// =============================================================================
//
// Shared helpers for billing-related API tests.
// Adapted for product_reduce: subscription uses entitlement_key instead of
// plan_id/tier/billing_period; Product/Plan helpers removed.
//
// =============================================================================

#![allow(dead_code)]

use crate::schema_test_context::SchemaTestContext as TestContext;
use uuid::Uuid;

/// ============================================================================
/// Billing Test Setup Helpers
/// ============================================================================
///
/// Setup admin session for billing tests
pub async fn setup_billing_admin_session(ctx: &mut TestContext, email: &str) -> String {
    let (admin_token, user_id) =
        crate::helpers::create_admin_session_with_user(ctx, email, 1800).await;

    // Grant Realm Admin role
    crate::helpers::grant_realm_admin_role(ctx, &user_id).await;

    admin_token
}

/// ============================================================================
/// Entitlement Mapping Test Data Creation Helpers
/// ============================================================================
///
/// Create a test entitlement mapping via direct SQL insertion.
///
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
/// Points Package Promo Test Helpers
/// ============================================================================
///
/// Create a points package via direct SQL insertion with promo fields.
///
/// Returns the package_id.
#[allow(clippy::too_many_arguments)]
pub async fn create_test_points_package_via_sql(
    ctx: &mut TestContext,
    realm_id: &str,
    name: &str,
    points: i64,
    price: i64,
    currency: &str,
    package_type: &str,
    original_price: Option<i64>,
    promo_start: Option<chrono::DateTime<chrono::Utc>>,
    promo_end: Option<chrono::DateTime<chrono::Utc>>,
) -> Uuid {
    let package_id = Uuid::now_v7();

    sqlx::query(
        "INSERT INTO points_packages (id, realm_id, name, title, description, points, price, currency,
                          sort_order, enabled, package_type, original_price, promo_start_time, promo_end_time,
                          created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 0, true, $9, $10, $11, $12, NOW(), NOW())",
    )
    .bind(package_id)
    .bind(realm_id)
    .bind(name)
    .bind(name) // title
    .bind(format!("{} description", name))
    .bind(points)
    .bind(price)
    .bind(currency)
    .bind(package_type)
    .bind(original_price)
    .bind(promo_start)
    .bind(promo_end)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create points package via SQL");

    package_id
}

/// Create a points package via the HTTP API.
///
/// Returns (StatusCode, response body as serde_json::Value).
pub async fn create_points_package_via_api(
    router: &axum::Router,
    realm_id: &str,
    session_token: &str,
    payload: serde_json::Value,
) -> (axum::http::StatusCode, serde_json::Value) {
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use tower::ServiceExt;

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/bill/{}/points-packages", realm_id))
                .header("Content-Type", "application/json")
                .header("cookie", format!("X-Auth={}", session_token))
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
    (status, json)
}

/// Update a points package via the HTTP API.
///
/// Returns (StatusCode, response body as serde_json::Value).
pub async fn update_points_package_via_api(
    router: &axum::Router,
    realm_id: &str,
    package_id: Uuid,
    session_token: &str,
    payload: serde_json::Value,
) -> (axum::http::StatusCode, serde_json::Value) {
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use tower::ServiceExt;

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!(
                    "/api/bill/{}/points-packages/{}",
                    realm_id, package_id
                ))
                .header("Content-Type", "application/json")
                .header("cookie", format!("X-Auth={}", session_token))
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
    (status, json)
}

/// List points packages via the HTTP API.
///
/// Returns (StatusCode, response body as serde_json::Value).
pub async fn list_points_packages_via_api(
    router: &axum::Router,
    realm_id: &str,
    session_token: &str,
) -> (axum::http::StatusCode, serde_json::Value) {
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use tower::ServiceExt;

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/bill/{}/points-packages", realm_id))
                .header("cookie", format!("X-Auth={}", session_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
    (status, json)
}
