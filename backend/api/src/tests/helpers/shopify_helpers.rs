// =============================================================================
// Shopify Webhook Test Helpers
// =============================================================================
//
// Shared helpers for Shopify webhook testing.
// Provides functions for building Shopify webhook events and sending them to test servers.
//
// =============================================================================

#![allow(dead_code)]

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64_STANDARD};
use hmac::{Hmac, Mac};
use serde_json::json;
use sha2::Sha256;
use tower::ServiceExt;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

fn resolve_test_event_id(payload: &serde_json::Value) -> String {
    payload
        .get("testEventId")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| Uuid::now_v7().to_string())
}

/// ============================================================================
/// Shopify Webhook Event Builders
/// ============================================================================
/// Build a Shopify subscription_contracts/create webhook event
pub fn build_shopify_subscription_contracts_create_event(
    event_id: String,
    contract_id: String,
    customer_id: String,
    user_id: Uuid,
    plan_id: Uuid,
    realm_id: &str,
    client_app_id: Option<Uuid>,
) -> serde_json::Value {
    let period_end = (chrono::Utc::now() + chrono::Duration::days(30)).to_rfc3339();

    let mut payload = json!({
        "testEventId": event_id,
        "id": contract_id,
        "adminGraphqlApiId": contract_id,
        "customerId": customer_id,
        "originOrderId": format!("gid://shopify/Order/{}", Uuid::now_v7()),
        "sellingPlanId": format!("gid://shopify/SellingPlan/{}", Uuid::now_v7()),
        "currentPeriodEnd": period_end,
        "status": "ACTIVE",
        "casRealmId": realm_id,
        "casUserId": user_id.to_string(),
        "casPlanId": plan_id.to_string()
    });

    // Add optional client_app_id
    if let Some(client_id) = client_app_id {
        payload["casClientAppId"] = json!(client_id.to_string());
    }

    payload
}

/// Build a Shopify subscription_contracts/update webhook event
pub fn build_shopify_subscription_contracts_update_event(
    event_id: String,
    contract_id: String,
    customer_id: String,
    user_id: Uuid,
    old_plan_id: Uuid,
    new_plan_id: Uuid,
    realm_id: &str,
    contract_revision_id: Option<i64>,
) -> serde_json::Value {
    let period_end = (chrono::Utc::now() + chrono::Duration::days(30)).to_rfc3339();

    let mut payload = json!({
        "testEventId": event_id,
        "id": contract_id,
        "adminGraphqlApiId": contract_id,
        "customerId": customer_id,
        "sellingPlanId": format!("gid://shopify/SellingPlan/{}", Uuid::now_v7()),
        "currentPeriodEnd": period_end,
        "status": "ACTIVE",
        "casRealmId": realm_id,
        "casUserId": user_id.to_string(),
        "casPlanId": new_plan_id.to_string(),
        "previousAttributes": {
            "casPlanId": old_plan_id.to_string()
        }
    });

    if let Some(revision_id) = contract_revision_id {
        payload["contractRevisionId"] = json!(revision_id);
    }

    payload
}

/// Build a Shopify subscription_billing_attempts/success webhook event
pub fn build_shopify_billing_attempt_success_event(
    event_id: String,
    billing_attempt_id: String,
    contract_id: String,
    order_id: String,
    _realm_id: &str,
) -> serde_json::Value {
    json!({
        "testEventId": event_id,
        "id": billing_attempt_id,
        "subscriptionContractId": contract_id,
        "orderId": order_id,
        "success": true,
        "errorCode": null,
        "errorMessage": null
    })
}

/// Build a Shopify subscription_billing_attempts/failure webhook event
pub fn build_shopify_billing_attempt_failure_event(
    event_id: String,
    billing_attempt_id: String,
    contract_id: String,
    error_code: String,
    error_message: String,
) -> serde_json::Value {
    json!({
        "testEventId": event_id,
        "id": billing_attempt_id,
        "subscriptionContractId": contract_id,
        "orderId": null,
        "success": false,
        "errorCode": error_code,
        "errorMessage": error_message
    })
}

/// Build a Shopify refunds/create webhook event
pub fn build_shopify_refunds_create_event(
    event_id: String,
    refund_id: String,
    order_id: String,
    refund_amount: i32,
    currency: &str,
    reason: Option<&str>,
) -> serde_json::Value {
    json!({
        "testEventId": event_id,
        "id": refund_id,
        "orderId": order_id,
        "createdAt": chrono::Utc::now().to_rfc3339(),
        "refundAmount": refund_amount,
        "currency": currency,
        "reason": reason
    })
}

/// ============================================================================
/// Shopify Webhook Sending Helpers
/// ============================================================================
/// Calculate Shopify HMAC-SHA256 signature
pub fn calculate_shopify_hmac(body: &str, client_secret: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(client_secret.as_bytes()).unwrap();
    mac.update(body.as_bytes());
    let hmac_bytes = mac.finalize().into_bytes();
    BASE64_STANDARD.encode(hmac_bytes)
}

/// Send a Shopify webhook event with valid HMAC signature
pub async fn send_shopify_webhook_with_signature(
    app: &axum::Router,
    realm_id: &str,
    topic: &str,
    payload: serde_json::Value,
    client_secret: &str,
) -> axum::response::Response {
    let payload_str = serde_json::to_string(&payload).unwrap();
    let signature = calculate_shopify_hmac(&payload_str, client_secret);
    let event_id = resolve_test_event_id(&payload);

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/third/pay/{}/shopify/webhooks", realm_id))
                .header("x-shopify-topic", topic)
                .header("x-shopify-event-id", event_id)
                .header("x-shopify-hmac-sha256", signature)
                .header("Content-Type", "application/json")
                .body(Body::from(payload_str))
                .unwrap(),
        )
        .await
        .unwrap()
}

/// Send a Shopify webhook event with invalid HMAC signature
pub async fn send_shopify_webhook_with_invalid_signature(
    app: &axum::Router,
    realm_id: &str,
    topic: &str,
    payload: serde_json::Value,
) -> axum::response::Response {
    let payload_str = serde_json::to_string(&payload).unwrap();
    let invalid_signature = "invalid_hmac_signature_base64";
    let event_id = resolve_test_event_id(&payload);

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/third/pay/{}/shopify/webhooks", realm_id))
                .header("x-shopify-topic", topic)
                .header("x-shopify-event-id", event_id)
                .header("x-shopify-hmac-sha256", invalid_signature)
                .header("Content-Type", "application/json")
                .body(Body::from(payload_str))
                .unwrap(),
        )
        .await
        .unwrap()
}

/// Send a Shopify webhook event without HMAC signature
pub async fn send_shopify_webhook_without_signature(
    app: &axum::Router,
    realm_id: &str,
    topic: &str,
    payload: serde_json::Value,
) -> axum::response::Response {
    let payload_str = serde_json::to_string(&payload).unwrap();
    let event_id = resolve_test_event_id(&payload);

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/third/pay/{}/shopify/webhooks", realm_id))
                .header("x-shopify-topic", topic)
                .header("x-shopify-event-id", event_id)
                .header("Content-Type", "application/json")
                .body(Body::from(payload_str))
                .unwrap(),
        )
        .await
        .unwrap()
}

/// ============================================================================
/// Shopify Test Data Generators
/// ============================================================================
/// Generate a unique Shopify contract ID for testing
pub fn generate_shopify_contract_id() -> String {
    format!("gid://shopify/SubscriptionContract/{}", Uuid::now_v7())
}

/// Generate a unique Shopify customer ID for testing
pub fn generate_shopify_customer_id() -> String {
    format!("gid://shopify/Customer/{}", Uuid::now_v7())
}

/// Generate a unique Shopify order ID for testing
pub fn generate_shopify_order_id() -> String {
    format!("gid://shopify/Order/{}", Uuid::now_v7())
}

/// Generate a unique Shopify billing attempt ID for testing
pub fn generate_shopify_billing_attempt_id() -> String {
    format!("gid://shopify/BillingAttempt/{}", Uuid::now_v7())
}

/// ============================================================================
/// Shopify Database Setup Helpers
/// ============================================================================
/// Insert Shopify configuration for a realm
/// Insert a single Shopify configuration value
async fn insert_shopify_config(
    ctx: &crate::tests::schema_test_context::SchemaTestContext,
    realm_id: &str,
    config_key: &str,
    config_value: &str,
    is_secret: bool,
) {
    sqlx::query(
        "INSERT INTO realm_config (realm_id, config_type, config_key, config_value, is_secret, enabled, metadata)
         VALUES ($1, 'shopify', $2, $3, $4, true, null)
         ON CONFLICT (realm_id, config_type, config_key)
         DO UPDATE SET config_value = EXCLUDED.config_value, updated_at = CURRENT_TIMESTAMP"
    )
    .bind(realm_id)
    .bind(config_key)
    .bind(config_value)
    .bind(is_secret)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to insert Shopify config");
}

pub async fn setup_shopify_config(
    ctx: &crate::tests::schema_test_context::SchemaTestContext,
    realm_id: &str,
    shop_domain: &str,
    admin_access_token: &str,
    storefront_access_token: &str,
    app_client_secret: &str,
    api_version: &str,
) {
    insert_shopify_config(ctx, realm_id, "shop_domain", shop_domain, false).await;
    insert_shopify_config(
        ctx,
        realm_id,
        "admin_access_token",
        admin_access_token,
        true,
    )
    .await;
    insert_shopify_config(
        ctx,
        realm_id,
        "storefront_access_token",
        storefront_access_token,
        true,
    )
    .await;
    insert_shopify_config(ctx, realm_id, "app_client_secret", app_client_secret, true).await;
    insert_shopify_config(ctx, realm_id, "api_version", api_version, false).await;

    tracing::debug!(
        realm_id = %realm_id,
        "Inserted Shopify config for realm"
    );
}

/// Create a Shopify subscription binding for testing
pub async fn create_shopify_binding(
    ctx: &crate::tests::schema_test_context::SchemaTestContext,
    subscription_id: Uuid,
    realm_id: &str,
    shop_domain: &str,
    contract_id: &str,
    customer_id: &str,
) -> i64 {
    // The realm_id parameter is already the UUID string, use it directly
    // shopify_subscription_binding.realm_id is TEXT type, same as realm.id
    let binding_id: i64 = sqlx::query_scalar(
        "INSERT INTO shopify_subscription_binding
         (subscription_id, realm_id, shop_domain, contract_id, contract_gid,
          contract_revision_id, customer_id, last_order_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         RETURNING id",
    )
    .bind(subscription_id)
    .bind(realm_id) // Use realm_id directly (it's already a UUID string)
    .bind(shop_domain)
    .bind(contract_id)
    .bind(contract_id) // Use same as contract_id for GID
    .bind(1i64) // Initial revision ID
    .bind(customer_id)
    .bind::<Option<String>>(None) // No order ID yet
    .fetch_one(&ctx.app_state.pool)
    .await
    .expect("Failed to create Shopify binding");

    binding_id
}

/// ============================================================================
/// Shopify Webhook Assertion Helpers
/// ============================================================================
/// Assert Shopify webhook response is successful (202 Accepted)
pub fn assert_shopify_webhook_success(response: &axum::response::Response) {
    assert_eq!(
        response.status(),
        StatusCode::ACCEPTED,
        "Expected Shopify webhook success (202), got {}",
        response.status()
    );
}

/// Assert Shopify webhook response is unauthorized (401)
pub fn assert_shopify_webhook_unauthorized(response: &axum::response::Response) {
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Expected Shopify webhook unauthorized (401), got {}",
        response.status()
    );
}

/// ============================================================================
/// Shopify Payment Event Helpers
/// ============================================================================
/// Check if a payment event exists for a given event ID
pub async fn payment_event_exists(
    ctx: &crate::tests::schema_test_context::SchemaTestContext,
    external_event_id: &str,
) -> bool {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM payment_event
         WHERE external_event_id = $1 AND payment_provider = 'shopify')",
    )
    .bind(external_event_id)
    .fetch_one(&ctx.app_state.pool)
    .await
    .unwrap_or(false)
}

/// Get payment event by external event ID
pub async fn get_payment_event(
    ctx: &crate::tests::schema_test_context::SchemaTestContext,
    external_event_id: &str,
) -> Option<serde_json::Value> {
    let (event_type, processed, processing_started_at): (
        String,
        bool,
        Option<chrono::DateTime<chrono::Utc>>,
    ) = sqlx::query_as(
        "SELECT event_type, processed, processing_started_at FROM payment_event
         WHERE external_event_id = $1 AND payment_provider = 'shopify'",
    )
    .bind(external_event_id)
    .fetch_optional(&ctx.app_state.pool)
    .await
    .ok()??;

    Some(json!({
        "eventType": event_type,
        "processed": processed,
        "processingStartedAt": processing_started_at
    }))
}

/// ============================================================================
/// Shopify Subscription Helpers
/// ============================================================================
/// Get Shopify binding by contract ID
pub async fn get_shopify_binding_by_contract(
    ctx: &crate::tests::schema_test_context::SchemaTestContext,
    contract_id: &str,
) -> Option<(i64, Uuid, i64)> {
    sqlx::query_as(
        "SELECT id, subscription_id, contract_revision_id
         FROM shopify_subscription_binding
         WHERE contract_id = $1",
    )
    .bind(contract_id)
    .fetch_optional(&ctx.app_state.pool)
    .await
    .ok()?
}

/// Get Shopify binding by order ID
pub async fn get_shopify_binding_by_order(
    ctx: &crate::tests::schema_test_context::SchemaTestContext,
    order_id: &str,
) -> Option<(i64, Uuid)> {
    sqlx::query_as(
        "SELECT id, subscription_id
         FROM shopify_subscription_binding
         WHERE last_order_id = $1",
    )
    .bind(order_id)
    .fetch_optional(&ctx.app_state.pool)
    .await
    .ok()?
}

/// Get subscription status
pub async fn get_subscription_status(
    ctx: &crate::tests::schema_test_context::SchemaTestContext,
    subscription_id: Uuid,
) -> Option<String> {
    sqlx::query_scalar("SELECT status FROM subscription WHERE id = $1")
        .bind(subscription_id)
        .fetch_optional(&ctx.app_state.pool)
        .await
        .ok()?
}

/// ============================================================================
/// User Creation Helpers
/// ============================================================================
/// Create a user via API for testing
///
/// This function is intentionally simplified - tests should use
/// create_test_user from test_setup_helpers instead.
pub async fn create_user_via_api(
    _app: &axum::Router,
    _realm_id: &str,
    _user_id: &str,
    _email: &str,
    _password: &str,
) {
    // Placeholder - tests should use create_test_user from test_setup_helpers
    unimplemented!("Use create_test_user from test_setup_helpers instead");
}
