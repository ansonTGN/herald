use crate::tests::helpers::billing_helpers::{
    setup_billing_admin_session, setup_stripe_config, setup_test_entitlement_mapping_full,
};
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::json;
use test_context::test_context;
use tower::ServiceExt;
use uuid::Uuid;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path},
};

async fn create_client_app(ctx: &TestContext, realm_id: &str) -> Uuid {
    let client_app_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO client_app (id, realm_id, client_id, name, enabled)
         VALUES ($1, $2, $3, 'Checkout Test App', true)",
    )
    .bind(client_app_id)
    .bind(realm_id)
    .bind(format!("checkout-client-{client_app_id}"))
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create client app");

    client_app_id
}

async fn set_mock_base_url(ctx: &TestContext, realm_id: &str, provider: &str, base_url: &str) {
    sqlx::query(
        "INSERT INTO realm_config (realm_id, config_type, config_key, config_value, enabled, created_at, updated_at)
         VALUES ($1, $2, 'mock_base_url', $3, true, NOW(), NOW())
         ON CONFLICT (realm_id, config_type, config_key)
         DO UPDATE SET config_value = $3, enabled = true, updated_at = NOW()",
    )
    .bind(realm_id)
    .bind(provider)
    .bind(base_url)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to set provider mock base URL");
}

async fn create_subscription_mapping(
    ctx: &mut TestContext,
    realm_id: &str,
    entitlement_key: &str,
    provider: &str,
    enabled: bool,
) -> Uuid {
    setup_test_entitlement_mapping_full(
        ctx,
        realm_id,
        provider,
        &format!("prod_{provider}_{entitlement_key}"),
        None,
        entitlement_key,
        Some("recurring"),
        Some("monthly"),
        None,
        None,
        None,
        false,
        None,
        enabled,
        Some(json!({
            "price": 1200,
            "currency": "usd"
        })),
    )
    .await
}

async fn post_checkout(
    ctx: &mut TestContext,
    client_app_id: Uuid,
    token: &str,
    entitlement_key: &str,
    payment_provider: &str,
) -> (StatusCode, serde_json::Value, String) {
    let app = ctx.create_unified_test_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/bill/{}/client/{}/checkout",
                    ctx._realm_id, client_app_id
                ))
                .header("Content-Type", "application/json")
                .header("cookie", format!("X-Auth={token}"))
                .body(Body::from(
                    json!({
                        "entitlementKey": entitlement_key,
                        "paymentProvider": payment_provider
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_text = String::from_utf8(body.to_vec()).unwrap();
    let body_json = serde_json::from_str(&body_text).unwrap_or(serde_json::Value::Null);
    (status, body_json, body_text)
}

#[test_context(TestContext)]
#[tokio::test]
async fn test_stripe_checkout_session_returns_provider_url(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();
    let client_app_id = create_client_app(ctx, &realm_id).await;
    let token = setup_billing_admin_session(ctx, "checkout-stripe@test.com").await;
    let entitlement_key = "checkout_stripe_monthly";
    create_subscription_mapping(ctx, &realm_id, entitlement_key, "stripe", true).await;

    let mock_server = MockServer::start().await;
    let session_id = format!("cs_test_{}", Uuid::now_v7());
    let checkout_url = format!("https://checkout.stripe.com/c/pay/{session_id}");

    Mock::given(method("POST"))
        .and(path("/v1/checkout/sessions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": session_id,
            "url": checkout_url,
            "status": "open",
            "metadata": {}
        })))
        .mount(&mock_server)
        .await;

    setup_stripe_config(ctx, &realm_id, "sk_test_fake_key", "whsec_test").await;
    set_mock_base_url(ctx, &realm_id, "stripe", &mock_server.uri()).await;

    let (status, body, body_text) =
        post_checkout(ctx, client_app_id, &token, entitlement_key, "stripe").await;

    assert_eq!(
        status,
        StatusCode::OK,
        "Expected 200, got {status}: {body_text}"
    );
    assert_eq!(body["checkoutUrl"].as_str(), Some(checkout_url.as_str()));
    assert!(
        body["checkoutId"]
            .as_str()
            .and_then(|id| Uuid::parse_str(id).ok())
            .is_some(),
        "checkoutId should be a UUID, got: {body_text}"
    );
}

#[test_context(TestContext)]
#[tokio::test]
async fn test_checkout_session_returns_404_when_mapping_missing(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();
    let client_app_id = create_client_app(ctx, &realm_id).await;
    let token = setup_billing_admin_session(ctx, "checkout-missing@test.com").await;
    setup_stripe_config(ctx, &realm_id, "sk_test_fake_key", "whsec_test").await;

    let (status, _body, body_text) =
        post_checkout(ctx, client_app_id, &token, "nonexistent", "stripe").await;

    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "Expected 404, got {status}: {body_text}"
    );
    assert!(
        body_text.contains("Entitlement mapping not found"),
        "Error should explain missing mapping, got: {body_text}"
    );
}

#[test_context(TestContext)]
#[tokio::test]
async fn test_checkout_session_rejects_disabled_mapping(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();
    let client_app_id = create_client_app(ctx, &realm_id).await;
    let token = setup_billing_admin_session(ctx, "checkout-disabled@test.com").await;
    let entitlement_key = "checkout_disabled_monthly";
    create_subscription_mapping(ctx, &realm_id, entitlement_key, "stripe", false).await;
    setup_stripe_config(ctx, &realm_id, "sk_test_fake_key", "whsec_test").await;

    let (status, _body, body_text) =
        post_checkout(ctx, client_app_id, &token, entitlement_key, "stripe").await;

    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "Expected 400, got {status}: {body_text}"
    );
    assert!(
        body_text.contains("is not enabled"),
        "Error should explain disabled mapping, got: {body_text}"
    );
}

#[test_context(TestContext)]
#[tokio::test]
async fn test_checkout_session_rejects_provider_mismatch(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();
    let client_app_id = create_client_app(ctx, &realm_id).await;
    let token = setup_billing_admin_session(ctx, "checkout-provider-mismatch@test.com").await;
    let entitlement_key = "checkout_provider_mismatch";
    create_subscription_mapping(ctx, &realm_id, entitlement_key, "stripe", true).await;

    let (status, _body, body_text) =
        post_checkout(ctx, client_app_id, &token, entitlement_key, "creem").await;

    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "Expected 400, got {status}: {body_text}"
    );
    assert!(
        body_text.contains("does not match mapping provider"),
        "Error should explain provider mismatch, got: {body_text}"
    );
}

#[test_context(TestContext)]
#[tokio::test]
async fn test_checkout_session_reports_missing_stripe_config(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();
    let client_app_id = create_client_app(ctx, &realm_id).await;
    let token = setup_billing_admin_session(ctx, "checkout-no-stripe-config@test.com").await;
    let entitlement_key = "checkout_no_stripe_config";
    create_subscription_mapping(ctx, &realm_id, entitlement_key, "stripe", true).await;

    let (status, _body, body_text) =
        post_checkout(ctx, client_app_id, &token, entitlement_key, "stripe").await;

    assert_eq!(
        status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "Expected current get_stripe_client_for_realm missing-config mapping, got {status}: {body_text}"
    );
    assert!(
        body_text.contains("Stripe not configured"),
        "Error should explain missing Stripe config, got: {body_text}"
    );
}

#[test_context(TestContext)]
#[tokio::test]
async fn test_creem_checkout_session_returns_provider_url(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();
    let client_app_id = create_client_app(ctx, &realm_id).await;
    let token = setup_billing_admin_session(ctx, "checkout-creem@test.com").await;
    let entitlement_key = "checkout_creem_monthly";
    create_subscription_mapping(ctx, &realm_id, entitlement_key, "creem", true).await;

    let mock_server = MockServer::start().await;
    let session_id = format!("co_test_{}", Uuid::now_v7());
    let checkout_url = format!("https://checkout.test.creem.io/{session_id}");

    Mock::given(method("POST"))
        .and(path("/v1/checkouts"))
        .and(header("x-api-key", "test_creem_api_key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": session_id,
            "checkout_url": checkout_url,
            "status": "pending"
        })))
        .mount(&mock_server)
        .await;

    ctx.with_creem_config(
        &realm_id,
        Some("test_creem_api_key"),
        Some("test_webhook_secret"),
        None,
    )
    .await;
    set_mock_base_url(ctx, &realm_id, "creem", &mock_server.uri()).await;

    let (status, body, body_text) =
        post_checkout(ctx, client_app_id, &token, entitlement_key, "creem").await;

    assert_eq!(
        status,
        StatusCode::OK,
        "Expected 200, got {status}: {body_text}"
    );
    assert_eq!(body["checkoutUrl"].as_str(), Some(checkout_url.as_str()));
    assert!(
        body["checkoutId"]
            .as_str()
            .and_then(|id| Uuid::parse_str(id).ok())
            .is_some(),
        "checkoutId should be a UUID, got: {body_text}"
    );
}
