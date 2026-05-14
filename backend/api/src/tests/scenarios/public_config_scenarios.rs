// =============================================================================
// Public Configuration API Endpoint Scenarios
// =============================================================================
//
// 测试 GET /api/{realmId}/public-config 端点的各种场景
//
// **测试目标**：
// 1. 验证端点可以在不同配置下正确返回公共配置
// 2. 验证端点无需认证即可访问
// 3. 验证响应格式符合 OpenAPI 规范
// 4. 验证边界情况（不存在的 realm）
//
// **运行方式**：
// ```bash
// cargo nextest run --workspace public_config_scenarios
// ```
//
// =============================================================================

use crate::application::http::public_config::PublicConfigResponse;
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use test_context::test_context;
use tower::ServiceExt;

/// ============================================================================
/// User Story: Public config with registration enabled, no OAuth providers
///
/// **场景描述**：
/// 当 realm 配置了允许注册，但未配置 OAuth 提供商时，端点应返回正确的注册配置和空的 OAuth 提供商列表。
///
/// **验收标准**：
/// - registration.allowed = true
/// - registration.requireEmailVerification = false (默认值)
/// - oauth_providers 为空数组
/// - 响应状态码为 200
/// - 无需认证即可访问
///
/// **运行方式**：
/// ```bash
/// cargo nextest run test_scenario_public_config_registration_enabled_no_oauth
/// ```
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_public_config_registration_enabled_no_oauth(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();

    // 配置 realm: 允许注册
    sqlx::query(
        "INSERT INTO realm_config (realm_id, config_type, config_key, config_value, enabled)
         VALUES ($1, 'registration', 'allowed', 'true', true)
         ON CONFLICT (realm_id, config_type, config_key) DO UPDATE
         SET config_value = EXCLUDED.config_value, enabled = EXCLUDED.enabled",
    )
    .bind(&realm_id)
    .execute(&ctx._app_state.pool)
    .await
    .unwrap();

    // 创建路由（无需认证）
    let app = ctx.create_unified_test_router();

    // 发送请求（无需认证）
    let request = Request::builder()
        .uri(format!("/api/public-config/{}", realm_id))
        .method(Method::GET)
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 解析响应
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let response_value: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let response_json: PublicConfigResponse =
        serde_json::from_value(response_value.clone()).unwrap();

    // 验证响应
    assert!(response_json.registration.allowed);
    assert!(!response_json.registration.require_email_verification);
    assert!(response_json.oauth_providers.is_empty());

    println!("✅ Test passed: registration enabled, no OAuth providers");
}

/// ============================================================================
/// User Story: Public config with registration disabled, OAuth providers present
///
/// **场景描述**：
/// 当 realm 禁止注册，但配置了 OAuth 提供商时，端点应返回正确的配置。
///
/// **验收标准**：
/// - registration.allowed = false
/// - registration.requireEmailVerification = false (默认值)
/// - oauth_providers 包含配置的提供商
/// - 响应状态码为 200
/// - 无需认证即可访问
///
/// **运行方式**：
/// ```bash
/// cargo nextest run test_scenario_public_config_registration_disabled_with_oauth
/// ```
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_public_config_registration_disabled_with_oauth(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();

    // 配置 realm: 禁止注册
    sqlx::query(
        "INSERT INTO realm_config (realm_id, config_type, config_key, config_value, enabled)
         VALUES ($1, 'registration', 'allowed', 'false', true)
         ON CONFLICT (realm_id, config_type, config_key) DO UPDATE
         SET config_value = EXCLUDED.config_value, enabled = EXCLUDED.enabled",
    )
    .bind(&realm_id)
    .execute(&ctx._app_state.pool)
    .await
    .unwrap();

    // 添加 OAuth 提供商
    let google_provider_id = uuid::Uuid::now_v7();
    sqlx::query(
        "INSERT INTO oauth_provider_config (id, realm_id, provider_type, client_id, client_secret, scopes, enabled)
         VALUES ($1, $2, 'google', 'test-google-client-id', 'test-google-secret', ARRAY['openid', 'email'], true)",
    )
    .bind(google_provider_id)
    .bind(&realm_id)
    .execute(&ctx._app_state.pool)
    .await
    .unwrap();

    // 创建路由
    let app = ctx.create_unified_test_router();

    // 发送请求（无需认证）
    let request = Request::builder()
        .uri(format!("/api/public-config/{}", realm_id))
        .method(Method::GET)
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 解析响应
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let response_value: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let response_json: PublicConfigResponse =
        serde_json::from_value(response_value.clone()).unwrap();

    // 验证响应
    assert!(!response_json.registration.allowed);
    assert!(!response_json.registration.require_email_verification);
    assert_eq!(response_json.oauth_providers.len(), 1);
    assert_eq!(response_json.oauth_providers[0].name, "google");
    assert_eq!(response_json.oauth_providers[0].display_name, "Google");
    assert!(response_json.oauth_providers[0].enabled);

    println!("✅ Test passed: registration disabled, OAuth providers present");
}
/// ============================================================================
/// User Story: Public config with non-existent realm
///
/// **场景描述**：
/// 当请求的 realm 不存在时，端点应返回空配置或 404 错误。
///
/// **验收标准**：
/// - 响应状态码为 200（端点默认返回空配置）或 404（可选）
/// - registration.allowed = false (默认值)
/// - registration.requireEmailVerification = false (默认值)
/// - oauth_providers 为空数组
///
/// **运行方式**：
/// ```bash
/// cargo nextest run test_scenario_public_config_non_existent_realm
/// ```
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_public_config_non_existent_realm(ctx: &mut TestContext) {
    // 使用不存在的 realm ID
    let non_existent_realm_id = "non-existent-realm-12345";

    // 创建路由
    let app = ctx.create_unified_test_router();

    // 发送请求
    let request = Request::builder()
        .uri(format!("/api/public-config/{}", non_existent_realm_id))
        .method(Method::GET)
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();

    // 当前实现返回 200，即使 realm 不存在（因为查询返回空结果，使用默认值）
    assert_eq!(response.status(), StatusCode::OK);

    // 解析响应
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let response_value: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let response_json: PublicConfigResponse =
        serde_json::from_value(response_value.clone()).unwrap();

    // 验证响应（默认值）
    assert!(!response_json.registration.allowed);
    assert!(!response_json.registration.require_email_verification);
    assert!(response_json.oauth_providers.is_empty());

    println!("✅ Test passed: non-existent realm returns default config");
}

/// ============================================================================
/// User Story: Public config response format validation
///
/// **场景描述**：
/// 验证端点返回的响应格式符合 OpenAPI 规范和 camelCase 约定。
///
/// **验收标准**：
/// - 响应是有效的 JSON
/// - 字段名称使用 camelCase (registration, allowed, requireEmailVerification, oauthProviders)
/// - 字段类型正确（allowed/requireEmailVerification/enabled 为 boolean，oauth_providers 为数组）
/// - 响应可以反序列化为 PublicConfigResponse 结构体
///
/// **运行方式**：
/// ```bash
/// cargo nextest run test_scenario_public_config_response_format
/// ```
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_public_config_response_format(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();

    // 配置 realm
    sqlx::query(
        "INSERT INTO realm_config (realm_id, config_type, config_key, config_value, enabled)
         VALUES ($1, 'registration', 'allowed', 'true', true)",
    )
    .bind(&realm_id)
    .execute(&ctx._app_state.pool)
    .await
    .unwrap();

    // 添加 OAuth 提供商
    let google_provider_id = uuid::Uuid::now_v7();
    sqlx::query(
        "INSERT INTO oauth_provider_config (id, realm_id, provider_type, client_id, client_secret, scopes, enabled)
         VALUES ($1, $2, 'google', 'test-google-client-id', 'test-google-secret', ARRAY['openid', 'email'], true)",
    )
    .bind(google_provider_id)
    .bind(&realm_id)
    .execute(&ctx._app_state.pool)
    .await
    .unwrap();

    // 创建路由
    let app = ctx.create_unified_test_router();

    // 发送请求
    let request = Request::builder()
        .uri(format!("/api/public-config/{}", realm_id))
        .method(Method::GET)
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 获取响应字符串
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = std::str::from_utf8(&body).unwrap();

    // 验证 JSON 格式
    assert!(serde_json::from_str::<serde_json::Value>(body_str).is_ok());

    // 验证 camelCase 字段名称
    assert!(body_str.contains("\"registration\""));
    assert!(body_str.contains("\"allowed\""));
    assert!(body_str.contains("\"requireEmailVerification\""));
    assert!(body_str.contains("\"oauthProviders\""));

    // 验证可以反序列化为强类型结构体
    let response_value: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let response_json: PublicConfigResponse =
        serde_json::from_value(response_value.clone()).unwrap();

    // 验证类型
    assert!(response_json.registration.allowed);
    assert!(!response_json.registration.require_email_verification);
    assert_eq!(response_json.oauth_providers.len(), 1);

    let provider = &response_json.oauth_providers[0];
    assert_eq!(provider.name, "google");
    assert_eq!(provider.display_name, "Google");
    assert!(provider.enabled);

    println!("✅ Test passed: response format validation");
}
