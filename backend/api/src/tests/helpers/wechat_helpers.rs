// =============================================================================
// WeChat Pay Test Helpers
// =============================================================================
//
// Shared helpers for WeChat Pay testing.
// Provides functions for building WeChat webhooks, creating test orders,
// and managing WeChat configuration in tests.
//
// =============================================================================

#![allow(dead_code)]

use axum::{body::Body, http::Request};
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

use crate::tests::schema_test_context::SchemaTestContext as TestContext;

/// ============================================================================
/// WeChat Configuration Setup Helpers
/// ============================================================================
/// Setup WeChat config in realm_config table
pub async fn setup_wechat_config(ctx: &mut TestContext, realm_id: &str) {
    let test_v3_key = "test_v3_key_32_bytes_long_xxxxxx";

    setup_wechat_config_with_keys(ctx, realm_id, test_v3_key, None).await;
}

/// Read the test RSA key from fixtures
fn load_test_rsa_key() -> String {
    // TODO: Replace with real WeChat Pay sandbox credentials when available
    // For now, providing a minimal valid PEM format for validation tests
    include_str!("../../fixtures/rsa_test_key.pem").to_string()
}

/// Read the test RSA public key from fixtures
fn load_test_rsa_pubkey() -> String {
    // TODO: Replace with real WeChat Pay sandbox credentials when available
    // For now, providing a minimal valid PEM format for validation tests
    include_str!("../../fixtures/rsa_test_pubkey.pem").to_string()
}

/// Setup WeChat config with specific keys
pub async fn setup_wechat_config_with_keys(
    ctx: &mut TestContext,
    realm_id: &str,
    v3_key: &str,
    _mock_base_url: Option<&str>, // Parameter kept for compatibility but unused
) {
    let notify_url = format!(
        "https://example.com/api/third/pay/{}/wechat/webhooks",
        realm_id
    );
    let private_key_pem = load_test_rsa_key();
    let platform_cert_pem = load_test_rsa_pubkey();

    let keys: Vec<(&str, &str)> = vec![
        ("app_id", "wx1234567890abcdef"),
        ("mch_id", "1234567890"),
        ("private_key", &private_key_pem),
        ("serial_no", "1A2B3C4D5E6F"),
        ("v3_key", v3_key),
        ("notify_url", notify_url.as_str()),
        ("platform_public_key", platform_cert_pem.as_str()),
    ];

    for (key, value) in keys {
        sqlx::query(
            "INSERT INTO realm_config (realm_id, config_type, config_key, config_value)
             VALUES ($1, 'wechat', $2, $3)",
        )
        .bind(realm_id)
        .bind(key)
        .bind(value)
        .execute(&ctx._app_state.pool)
        .await
        .expect("Failed to insert WeChat config");
    }
}

/// Generate a valid WeChat config payload
pub fn valid_wechat_config_payload() -> serde_json::Value {
    let private_key_pem = load_test_rsa_key();
    let platform_public_key_pem = load_test_rsa_pubkey();
    json!({
        "appId": "wx1234567890abcdef",
        "mchId": "1234567890",
        "privateKey": private_key_pem,
        "serialNo": "1A2B3C4D5E6F",
        "v3Key": "abcd1234567890abcdef1234567890ab",
        "platformPublicKey": platform_public_key_pem,
        "notifyUrl": "https://example.com/api/third/pay/realm-1/wechat/webhooks"
    })
}

/// Generate a valid payload with one field overridden
pub fn valid_wechat_config_payload_with_override(field: &str, value: &str) -> serde_json::Value {
    let mut payload = valid_wechat_config_payload();
    payload[field] = json!(value);
    payload
}

/// ============================================================================
/// HTTP Request Helpers
/// ============================================================================
/// Send create WeChat config request
pub async fn send_create_wechat_config(
    app: &axum::Router,
    realm_id: &str,
    token: &str,
    payload: &serde_json::Value,
) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/third/pay/{}/providers/wechat", realm_id))
                .header("Content-Type", "application/json")
                .header("cookie", format!("X-Auth={}", token))
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

/// Send get WeChat config request
pub async fn send_get_wechat_config(
    app: &axum::Router,
    realm_id: &str,
    token: &str,
) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/third/pay/{}/providers/wechat", realm_id))
                .header("Content-Type", "application/json")
                .header("cookie", format!("X-Auth={}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

/// Send update WeChat config request
pub async fn send_update_wechat_config(
    app: &axum::Router,
    realm_id: &str,
    token: &str,
    payload: &serde_json::Value,
) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/third/pay/{}/providers/wechat", realm_id))
                .header("Content-Type", "application/json")
                .header("cookie", format!("X-Auth={}", token))
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

/// Send delete WeChat config request
pub async fn send_delete_wechat_config(
    app: &axum::Router,
    realm_id: &str,
    token: &str,
) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/third/pay/{}/providers/wechat", realm_id))
                .header("Content-Type", "application/json")
                .header("cookie", format!("X-Auth={}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

/// Send create WeChat order request
pub async fn send_create_wechat_order(
    app: &axum::Router,
    realm_id: &str,
    token: &str,
    payload: &serde_json::Value,
) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/third/pay/{}/wechat/create-order", realm_id))
                .header("Content-Type", "application/json")
                .header("cookie", format!("X-Auth={}", token))
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

/// Send query WeChat order request
pub async fn send_query_wechat_order(
    app: &axum::Router,
    realm_id: &str,
    token: &str,
    order_id: &str,
) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/third/pay/{}/wechat/order-status/{}",
                    realm_id, order_id
                ))
                .header("Content-Type", "application/json")
                .header("cookie", format!("X-Auth={}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

/// Send close WeChat order request
pub async fn send_close_wechat_order(
    app: &axum::Router,
    realm_id: &str,
    token: &str,
    order_id: &str,
) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/third/pay/{}/wechat/close-order/{}",
                    realm_id, order_id
                ))
                .header("Content-Type", "application/json")
                .header("cookie", format!("X-Auth={}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

/// ============================================================================
/// Webhook Helpers (Simplified - crypto handled by SDK)
/// ============================================================================
/// Build WeChat Pay notification JSON (simplified for testing)
pub fn build_wechat_pay_notification_json(
    out_trade_no: &str,
    trade_state: &str,
    amount: i64,
) -> serde_json::Value {
    json!({
        "id": format!("EV_{}", Uuid::now_v7()),
        "createTime": chrono::Utc::now().to_rfc3339(),
        "resourceType": "encrypt-resource",
        "eventType": "TRANSACTION.SUCCESS",
        "resource": {
            "outTradeNo": out_trade_no,
            "tradeState": trade_state,
            "transactionId": format!("wx_tx_{}", Uuid::now_v7()),
            "amount": {
                "total": amount,
                "currency": "CNY"
            }
        },
        "summary": "支付成功"
    })
}

/// Send WeChat webhook with proper headers (simplified - uses test signature)
pub async fn send_wechat_webhook_with_valid_signature_and_encryption(
    app: &axum::Router,
    realm_id: &str,
    decrypted_data: &serde_json::Value,
    private_key_pem: &str,
    v3_key: &str,
) -> axum::response::Response {
    use aes_gcm::aead::Payload;
    use aes_gcm::aead::{Aead, KeyInit};
    use base64::Engine;

    let timestamp = chrono::Utc::now().timestamp().to_string();
    let nonce_str = Uuid::now_v7().to_string()[..12].to_string();

    // Encrypt the data using AES-256-GCM (same as WeChat Pay)
    let associated_data = "transaction";
    let plaintext = serde_json::to_string(decrypted_data).unwrap();

    // Create cipher and generate nonce
    let cipher = aes_gcm::Aes256Gcm::new_from_slice(v3_key.as_bytes()).unwrap();
    let nonce_bytes = aes_gcm::Nonce::from_slice(nonce_str.as_bytes());

    let ciphertext = cipher
        .encrypt(
            nonce_bytes,
            Payload {
                msg: plaintext.as_bytes(),
                aad: associated_data.as_bytes(),
            },
        )
        .unwrap();

    // Encode to base64
    let ciphertext_b64 = base64::engine::general_purpose::STANDARD.encode(&ciphertext);

    // Build notification payload with encrypted resource
    let payload = json!({
        "id": format!("EV_{}", Uuid::now_v7()),
        "createTime": chrono::Utc::now().to_rfc3339(),
        "resourceType": "encrypt-resource",
        "eventType": "TRANSACTION.SUCCESS",
        "resource": {
            "ciphertext": ciphertext_b64,
            "nonce": nonce_str,
            "associatedData": associated_data
        },
        "summary": "支付成功"
    });

    let body_str = serde_json::to_string(&payload).unwrap();

    // Generate valid SHA256-RSA signature for testing
    let signature = generate_wechat_signature(&timestamp, &nonce_str, &body_str, private_key_pem);

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/third/pay/{}/wechat/webhooks", realm_id))
                .header("Content-Type", "application/json")
                .header("Wechatpay-Timestamp", timestamp)
                .header("Wechatpay-Nonce", nonce_str)
                .header("Wechatpay-Signature", signature)
                .header("Wechatpay-Serial", "1A2B3C4D5E6F")
                .body(Body::from(body_str))
                .unwrap(),
        )
        .await
        .unwrap()
}

/// Send WeChat webhook with wrong signature
pub async fn send_wechat_webhook_with_wrong_signature(
    app: &axum::Router,
    realm_id: &str,
    body_str: &str,
) -> axum::response::Response {
    let timestamp = chrono::Utc::now().timestamp().to_string();
    let nonce = Uuid::now_v7().to_string();

    // Use obviously wrong signature
    let signature = "invalid_signature_XYZ123";

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/third/pay/{}/wechat/webhooks", realm_id))
                .header("Content-Type", "application/json")
                .header("Wechatpay-Timestamp", timestamp)
                .header("Wechatpay-Nonce", nonce)
                .header("Wechatpay-Signature", signature)
                .header("Wechatpay-Serial", "1A2B3C4D5E6F")
                .body(Body::from(body_str.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

/// Send WeChat webhook without signature headers
pub async fn send_wechat_webhook_raw(
    app: &axum::Router,
    realm_id: &str,
    payload: serde_json::Value,
) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/third/pay/{}/wechat/webhooks", realm_id))
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap()
}

/// ============================================================================
/// Signature Generation Helper
/// ============================================================================
/// Generate valid WeChat Pay webhook signature for testing
fn generate_wechat_signature(
    timestamp: &str,
    nonce: &str,
    body: &str,
    private_key_pem: &str,
) -> String {
    use base64::Engine;
    use rsa::RsaPrivateKey;
    use rsa::pkcs1::DecodeRsaPrivateKey;
    use rsa::pkcs1v15::Pkcs1v15Sign;
    use rsa::pkcs8::DecodePrivateKey;
    use sha2::{Digest, Sha256};

    // Create message: timestamp + "\n" + nonce + "\n" + body
    let message = format!("{}\n{}\n{}", timestamp, nonce, body);
    let digest = Sha256::digest(message.as_bytes());
    let private_key = RsaPrivateKey::from_pkcs8_pem(private_key_pem)
        .or_else(|_| RsaPrivateKey::from_pkcs1_pem(private_key_pem))
        .expect("valid RSA private key");
    let signature_bytes = private_key
        .sign(Pkcs1v15Sign::new::<Sha256>(), &digest)
        .expect("RSA signature");

    // Base64 encode
    base64::engine::general_purpose::STANDARD.encode(signature_bytes)
}

/// ============================================================================
/// Order Test Data Helpers
/// ============================================================================
/// Create a test WeChat payment order (uses admin user or specified user_id)
pub async fn create_test_wechat_order(
    ctx: &mut TestContext,
    realm_id: &str,
    plan_id: Uuid,
    status: &str,
) -> Uuid {
    create_test_wechat_order_with_user(ctx, realm_id, plan_id, status, None).await
}

/// Create a test WeChat payment order with optional user_id
pub async fn create_test_wechat_order_with_user(
    ctx: &mut TestContext,
    realm_id: &str,
    plan_id: Uuid,
    status: &str,
    user_id: Option<Uuid>,
) -> Uuid {
    let actual_user_id = if let Some(uid) = user_id {
        uid
    } else {
        // Get a test user - create one if needed
        sqlx::query_scalar("SELECT id FROM account WHERE realm_id = $1 LIMIT 1")
            .bind(realm_id)
            .fetch_optional(&ctx._app_state.pool)
            .await
            .expect("Failed to query user")
            .unwrap_or_else(|| {
                // Create a test user if none exists
                Uuid::now_v7()
            })
    };

    create_test_wechat_order_for_user(ctx, realm_id, plan_id, actual_user_id, status).await
}

/// Create a test WeChat payment order for a specific user
pub async fn create_test_wechat_order_for_user(
    ctx: &mut TestContext,
    realm_id: &str,
    plan_id: Uuid,
    user_id: Uuid,
    status: &str,
) -> Uuid {
    let order_id = Uuid::now_v7();

    // Generate out_trade_no using the same format as production (max 32 chars for WeChat Pay)
    // Format: CAS_{4_char_realm}_{20_char_uuid}
    let prefix = &realm_id[..4.min(realm_id.len())];
    let uuid_str = Uuid::now_v7().to_string().replace('-', "");
    let compact_uuid = &uuid_str[..20.min(uuid_str.len())];
    let out_trade_no = format!("CAS_{}_{}", prefix, compact_uuid);

    sqlx::query(
        "INSERT INTO wechat_payment_order (id, realm_id, user_id, plan_id, out_trade_no, status, amount, code_url, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW() + INTERVAL '2 hours')"
    )
    .bind(order_id)
    .bind(realm_id)
    .bind(user_id)
    .bind(plan_id)
    .bind(&out_trade_no)
    .bind(status)
    .bind(2500) // Default amount
    .bind("weixin://wxpay/bizpayurl?pr=test_code_url") // Mock code URL for testing
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create test WeChat order");

    order_id
}

/// Get out_trade_no from order ID
pub async fn get_order_out_trade_no(ctx: &mut TestContext, order_id: Uuid) -> String {
    sqlx::query_scalar("SELECT out_trade_no FROM wechat_payment_order WHERE id = $1")
        .bind(order_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap()
}

/// ============================================================================
/// Assertion Helpers
/// ============================================================================
/// Assert that WeChat config sensitive fields are masked
pub fn assert_wechat_config_masked(body: &serde_json::Value) {
    assert!(body["privateKey"].is_string());
    let private_key = body["privateKey"].as_str().unwrap();
    assert!(
        private_key.contains("configured") || private_key.contains("***"),
        "Private key should be masked, got: {}",
        private_key
    );

    assert!(body["v3Key"].is_string());
    let v3_key = body["v3Key"].as_str().unwrap();
    assert!(
        v3_key.contains("***") || v3_key.len() <= 6,
        "V3 key should be masked, got: {}",
        v3_key
    );
}
