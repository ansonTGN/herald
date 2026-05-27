// =============================================================================
// Login Flow Scenarios Tests
// =============================================================================
//
// 测试完整的登录流程，包括：
// 1. GET /api/auth/{realmId}/status - 获取当前认证状态
// 2. GET /api/{realmId}/public-config - 获取公共配置
// 3. POST /api/auth/{realmId}/login - 执行登录
//
// **测试目标**：
// 1. 验证完整的登录流程
// 2. 验证 status 端点正确返回认证状态
// 3. 验证 public-config 端点无需认证即可访问
// 4. 验证登录成功后 status 端点返回已认证状态
// 5. 验证用户状态为 Normal（status=1）
//
// **运行方式**：
// ```bash
// cargo nextest run --workspace login_flow
// ```
//
// =============================================================================

use crate::application::http::auth::status::StatusResponse;
use crate::application::http::auth::util::load_session;
use crate::application::http::public_config::PublicConfigResponse;
use crate::tests::helpers::test_setup_helpers::*;
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::{
    body::Body,
    http::{Method, Request, StatusCode, header},
};
use serde_json::json;
use test_context::test_context;
use tower::ServiceExt;

/// ============================================================================
/// User Story: 完整登录流程
///
/// **场景描述**：
/// 模拟真实的前端登录流程，依次调用 status、public-config 和 login 端点，
/// 验证完整的登录流程正确工作。
///
/// **测试步骤**：
/// 1. 在 realm1 中创建测试用户（status=1，已激活状态）
/// 2. 调用 GET /api/auth/{realmId}/status 验证未认证状态
/// 3. 调用 GET /api/{realmId}/public-config 获取公共配置
/// 4. 执行登录 POST /api/auth/{realmId}/login
/// 5. 再次调用 GET /api/auth/{realmId}/status 验证已认证状态
/// 6. 验证返回的用户信息和权限
///
/// **验收标准**：
/// - Step 2: status 返回 authenticated: false，其他字段为 None
/// - Step 3: public-config 返回配置信息（无需认证）
/// - Step 4: 登录成功，返回 200 OK 和 session cookie
/// - Step 5: status 返回 authenticated: true，包含正确的 user_id 和 permissions
///
/// **运行方式**：
/// ```bash
/// cargo nextest run test_scenario_complete_login_flow
/// ```
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_complete_login_flow(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();
    let realm_id = ctx._realm_id.clone();

    // ============================================================================
    // Step 1: 在 realm1 中创建测试用户（status=1，已激活状态）
    // ============================================================================
    println!("[Step 1] 创建测试用户: newuser@cas.com");
    let email = "newuser@cas.com";
    let password = "password123";

    let (user_id, _token) = create_user_and_login(ctx, email, password).await;
    println!("[Step 1] ✓ 用户创建成功: user_id={}", user_id);

    // ============================================================================
    // Step 2: 调用 GET /api/auth/{realmId}/status 验证未认证状态
    // ============================================================================
    println!("[Step 2] 检查未认证状态");
    let status_request = Request::builder()
        .uri(format!("/api/auth/{}/status", realm_id))
        .method(Method::GET)
        .body(Body::empty())
        .unwrap();

    let status_response = app.clone().oneshot(status_request).await.unwrap();
    assert_eq!(
        status_response.status(),
        StatusCode::OK,
        "Status endpoint should return 200 OK"
    );

    let status_body = axum::body::to_bytes(status_response.into_body(), usize::MAX)
        .await
        .expect("Failed to read status response body");
    let status_json: StatusResponse =
        serde_json::from_slice(&status_body).expect("Failed to parse status data");

    assert!(
        !status_json.authenticated,
        "Status should return authenticated: false before login"
    );
    assert!(
        status_json.realm_id.is_none(),
        "realm_id should be None when not authenticated"
    );
    assert!(
        status_json.user_id.is_none(),
        "user_id should be None when not authenticated"
    );
    assert!(
        status_json.permissions.is_none()
            || status_json
                .permissions
                .as_ref()
                .map(|p| p.is_empty())
                .unwrap_or(true),
        "permissions should be None or empty when not authenticated"
    );
    println!("[Step 2] ✓ 未认证状态正确: authenticated=false");

    // ============================================================================
    // Step 3: 调用 GET /api/public-config/{realmId} 获取公共配置
    // ============================================================================
    println!("[Step 3] 获取公共配置（无需认证）");
    let app = ctx.create_unified_test_router();
    let public_config_request = Request::builder()
        .uri(format!("/api/public-config/{}", realm_id))
        .method(Method::GET)
        .body(Body::empty())
        .unwrap();

    let public_config_response = app.clone().oneshot(public_config_request).await.unwrap();

    let public_config_body = axum::body::to_bytes(public_config_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let public_config_json: PublicConfigResponse =
        serde_json::from_slice(&public_config_body).expect("Failed to parse public config data");

    println!(
        "[Step 3] ✓ 公共配置获取成功: registration.enabled={}, oauth_providers.len()={}",
        public_config_json.registration.enabled,
        public_config_json.oauth_providers.len()
    );

    // ============================================================================
    // Step 4: 执行登录 POST /api/auth/{realmId}/login
    // ============================================================================
    println!("[Step 4] 执行登录");
    let login_payload = json!({
        "clientId": ctx._client_id,
        "email": email,
        "password": password,
        "turnstileToken": "dummy"
    });

    let login_request = Request::builder()
        .method("POST")
        .uri(format!("/api/auth/{}/login", realm_id))
        .header("content-type", "application/json")
        .header("x-forwarded-for", "3.3.3.3")
        .body(Body::from(login_payload.to_string()))
        .unwrap();

    let login_response = app.clone().oneshot(login_request).await.unwrap();

    // 获取 session token（必须在 consuming body 之前）
    let set_cookie = login_response
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|v| v.to_str().ok())
        .expect("Should return Set-Cookie header");

    assert_eq!(
        login_response.status(),
        StatusCode::OK,
        "Login should return 200 OK"
    );

    assert!(
        set_cookie.contains("X-Auth="),
        "Set-Cookie should contain X-Auth"
    );

    let token = crate::tests::extract_set_cookie_token(set_cookie, "X-Auth")
        .expect("Should extract X-Auth token");

    // Now consume the body
    let login_body = axum::body::to_bytes(login_response.into_body(), usize::MAX)
        .await
        .expect("Failed to read login response body");
    let login_json: serde_json::Value =
        serde_json::from_slice(&login_body).expect("Failed to parse login JSON");

    assert_eq!(
        login_json["userId"].as_str().unwrap(),
        user_id.to_string().as_str(),
        "user_id should match"
    );

    println!(
        "[Step 4] ✓ 登录成功: user_id={}, token={}",
        login_json["userId"], token
    );

    // ============================================================================
    // Step 5: 再次调用 GET /api/auth/{realmId}/status 验证已认证状态
    // ============================================================================
    println!("[Step 5] 检查已认证状态");
    let authenticated_status_request = Request::builder()
        .uri(format!("/api/auth/{}/status", realm_id))
        .method(Method::GET)
        .header("cookie", format!("X-Auth={}", token))
        .body(Body::empty())
        .unwrap();

    let authenticated_status_response = app
        .clone()
        .oneshot(authenticated_status_request)
        .await
        .unwrap();
    assert_eq!(
        authenticated_status_response.status(),
        StatusCode::OK,
        "Status endpoint should return 200 OK after login"
    );

    let authenticated_status_body =
        axum::body::to_bytes(authenticated_status_response.into_body(), usize::MAX)
            .await
            .expect("Failed to read status response body");
    let authenticated_status_json: StatusResponse =
        serde_json::from_slice(&authenticated_status_body)
            .expect("Failed to parse authenticated status data");

    assert!(
        authenticated_status_json.authenticated,
        "Status should return authenticated: true after login"
    );
    assert_eq!(
        authenticated_status_json.realm_id.as_deref(),
        Some(realm_id.as_str()),
        "realm_id should match"
    );
    assert_eq!(
        authenticated_status_json.user_id.as_deref(),
        Some(user_id.to_string().as_str()),
        "user_id should match"
    );
    println!(
        "[Step 5] ✓ 已认证状态正确: authenticated=true, user_id={}, realm_id={}",
        authenticated_status_json.user_id.unwrap(),
        authenticated_status_json.realm_id.unwrap()
    );

    // ============================================================================
    // Step 6: 验证 session 存储在 Redis 中
    // ============================================================================
    println!("[Step 6] 验证 session 存储在 Redis 中");
    let session = load_session(&ctx._app_state, &token)
        .await
        .expect("Failed to load session")
        .expect("Session should exist");

    assert_eq!(
        session.user_id,
        user_id.to_string(),
        "Session user_id should match"
    );
    assert_eq!(session.realm_id, realm_id, "Session realm_id should match");
    assert_eq!(
        session.client_id, ctx._client_id,
        "Session client_id should match"
    );
    println!(
        "[Step 6] ✓ Session 存储在 Redis 中: user_id={}, realm_id={}, client_id={}",
        session.user_id, session.realm_id, session.client_id
    );

    // 清理测试数据
    sqlx::query("DELETE FROM account WHERE email = $1")
        .bind(email)
        .execute(&ctx._app_state.pool)
        .await
        .unwrap();

    println!("\n✅ User Story 完成：完整登录流程测试成功");
}
/// ============================================================================
/// User Story: 使用错误密码登录失败
///
/// **场景描述**：
/// 用户使用错误的密码尝试登录，系统应该拒绝访问。
///
/// **测试步骤**：
/// 1. 创建测试用户
/// 2. 使用错误的密码登录
/// 3. 验证返回 401 Unauthorized
///
/// **验收标准**：
/// - 返回 401 Unauthorized
/// - 不返回 session cookie
/// - 返回错误信息 "invalid credentials"
///
/// **运行方式**：
/// ```bash
/// cargo nextest run test_scenario_user_login_wrong_password
/// ```
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_user_login_wrong_password(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    // ============================================================================
    // Step 1: 创建测试用户
    // ============================================================================
    println!("[Step 1] 创建测试用户");
    let email = "wrongpassword@cas.com";
    let password = "correctpassword";

    let _user_id = create_test_user(ctx, email, password).await;
    println!("[Step 1] ✓ 用户创建成功");

    // ============================================================================
    // Step 2: 使用错误的密码登录
    // ============================================================================
    println!("[Step 2] 使用错误密码登录");
    let payload = json!({
        "clientId": ctx._client_id,
        "email": email,
        "password": "wrongpassword",
        "turnstileToken": "dummy"
    });

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/auth/{}/login", ctx._realm_id))
        .header("content-type", "application/json")
        .header("x-forwarded-for", "3.3.3.3")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();

    // ============================================================================
    // Step 3: 验证返回 401 Unauthorized
    // ============================================================================
    println!("[Step 3] 验证返回 401 Unauthorized");
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Should return 401 Unauthorized for wrong password"
    );

    // 验证不返回 session cookie
    let set_cookie = response.headers().get(header::SET_COOKIE);
    assert!(
        set_cookie.is_none()
            || !set_cookie
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .contains("X-Auth="),
        "Should not return X-Auth cookie for failed login"
    );

    // 验证错误信息
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");
    let body: serde_json::Value =
        serde_json::from_slice(&body_bytes).expect("Failed to parse JSON");

    assert_response_error(&body, None);

    let error_message = body
        .get("error")
        .or_else(|| body.get("message"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(
        error_message.contains("invalid credentials"),
        "Error message should contain 'invalid credentials'"
    );

    println!("[Step 3] ✓ 返回 401 Unauthorized，错误信息正确");

    // 清理测试数据
    sqlx::query("DELETE FROM account WHERE email = $1")
        .bind(email)
        .execute(&ctx._app_state.pool)
        .await
        .unwrap();

    println!("\n✅ User Story 完成：错误密码登录被正确拒绝");
}

/// ============================================================================
/// User Story: 未激活用户登录失败
///
/// **场景描述**：
/// 用户状态为未激活（status != 1）时，登录应该失败。
///
/// **测试步骤**：
/// 1. 创建测试用户（status=0，未激活）
/// 2. 使用正确的邮箱和密码登录
/// 3. 验证返回 403 Forbidden
///
/// **验收标准**：
/// - 返回 403 Forbidden
/// - 返回错误信息 "email not verified or account not active"
///
/// **运行方式**：
/// ```bash
/// cargo nextest run test_scenario_user_login_inactive_user
/// ```
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_user_login_inactive_user(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    // ============================================================================
    // Step 1: 创建未激活的测试用户（status=0）
    // ============================================================================
    println!("[Step 1] 创建未激活的测试用户");
    let email = "inactive@cas.com";
    let password = "password123";

    let user_uuid = uuid::Uuid::now_v7();
    let password_hash =
        bcrypt::hash(password, bcrypt::DEFAULT_COST).expect("Failed to hash password");

    sqlx::query(
        "INSERT INTO account (id, realm_id, email, password, status)
         VALUES ($1, $2, $3, $4, 0)", // status=0 表示未激活
    )
    .bind(user_uuid)
    .bind(&ctx._realm_id)
    .bind(email)
    .bind(&password_hash)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create test user");

    // 验证用户状态为 0（未激活）
    let status: i16 = sqlx::query_scalar("SELECT status FROM account WHERE email = $1")
        .bind(email)
        .fetch_one(&ctx._app_state.pool)
        .await
        .expect("Failed to fetch user status");

    assert_eq!(status, 0, "User status should be 0 (inactive)");
    println!("[Step 1] ✓ 未激活用户创建成功: status={}", status);

    // ============================================================================
    // Step 2: 使用正确的邮箱和密码登录
    // ============================================================================
    println!("[Step 2] 使用正确的邮箱和密码登录");
    let payload = json!({
        "clientId": ctx._client_id,
        "email": email,
        "password": password,
        "turnstileToken": "dummy"
    });

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/auth/{}/login", ctx._realm_id))
        .header("content-type", "application/json")
        .header("x-forwarded-for", "3.3.3.3")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();

    // ============================================================================
    // Step 3: 验证返回 403 Forbidden
    // ============================================================================
    println!("[Step 3] 验证返回 403 Forbidden");
    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "Should return 403 Forbidden for inactive user"
    );

    // 验证错误信息
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");
    let body: serde_json::Value =
        serde_json::from_slice(&body_bytes).expect("Failed to parse JSON");

    assert_response_error(&body, None);

    let error_message = body
        .get("error")
        .or_else(|| body.get("message"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(
        error_message.contains("email not verified or account not active"),
        "Error message should mention inactive account"
    );

    println!("[Step 3] ✓ 返回 403 Forbidden，错误信息正确");

    // 清理测试数据
    sqlx::query("DELETE FROM account WHERE email = $1")
        .bind(email)
        .execute(&ctx._app_state.pool)
        .await
        .unwrap();

    println!("\n✅ User Story 完成：未激活用户登录被正确拒绝");
}
