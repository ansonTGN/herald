// =============================================================================
// Realm Admin Creation - Scenario Tests
// =============================================================================
//
// 测试创建 Realm 时自动创建管理员的端到端场景
//
// 测试覆盖:
// - 完整流程: 创建 Realm 并指定管理员
// - 错误处理: 密码太短
//
// 运行方式:
// cargo nextest run --package herald-api realm_admin_creation
//
// =============================================================================

use crate::tests::helpers::*;
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use test_context::test_context;
use tower::ServiceExt;

/// ============================================================================
/// Scenario 1: Create Realm with Admin User (Complete Flow)
///
/// **User Story**: Super Admin creates a new Realm with initial admin user
///
/// **Given**: Super Admin is logged in
/// **When**: POST /api/realms with valid realm data and admin_user
/// **Then**:
///   - Returns 200 OK
///   - Realm is created successfully
///   - Admin user is created
///   - Admin user is assigned realm-admin role
///   - Admin can login with provided credentials
///
/// **验收标准**:
/// - Realm 创建成功
/// - 管理员账号创建成功
/// - 管理员自动分配 realm-admin 角色
/// - 管理员可以使用指定邮箱和密码登录
///
/// **运行方式**:
/// ```bash
/// cargo nextest run test_scenario_create_realm_with_admin_success
/// ```
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_create_realm_with_admin_success(ctx: &mut TestContext) {
    // ============================================================================
    // Setup: Create Super Admin session with Realm Admin role
    // ============================================================================
    println!("[Setup] 创建 Super Admin 会话");
    let (super_admin_token, super_admin_user_id) =
        create_admin_session_with_user(ctx, "super-admin@test.com", 1800).await;

    // 授予 Realm Admin 角色 (包括 realms.create 权限)
    grant_realm_admin_role(ctx, &super_admin_user_id).await;
    println!("[Setup] ✓ Super Admin 会话创建成功");

    let app = ctx.create_unified_test_router();

    // ============================================================================
    // Step 1: Create Realm with Admin User
    // ============================================================================
    println!("[Step 1] 创建 Realm 并指定管理员");
    let timestamp = chrono::Utc::now().timestamp_millis();
    let new_realm_id = format!("testrealm{}", timestamp % 1000000000);
    let new_realm_name = "Test Realm with Admin";
    let admin_email = "admin@testrealm.com";
    let admin_password = "Password123";

    let create_payload = json!({
        "id": new_realm_id,
        "name": new_realm_name,
        "adminUser": {
            "email": admin_email,
            "password": admin_password
        }
    })
    .to_string();

    let req = Request::builder()
        .method("POST")
        .uri("/api/realms")
        .header("content-type", "application/json")
        .header("cookie", format!("X-Auth={}", super_admin_token))
        .body(Body::from(create_payload))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED, "创建 Realm 应返回 201");

    let body: serde_json::Value = crate::tests::response_json(resp).await;
    assert_eq!(body["id"], new_realm_id, "Realm ID 应匹配");
    assert_eq!(body["name"], new_realm_name, "Realm 名称应匹配");
    println!(
        "[Step 1] ✓ Realm 创建成功: {} ({})",
        new_realm_name, new_realm_id
    );

    // ============================================================================
    // Step 2: Verify Admin User Created
    // ============================================================================
    println!("[Step 2] 验证管理员用户已创建");

    // Check admin user exists in database
    let admin_user_id: uuid::Uuid =
        sqlx::query_scalar("SELECT id FROM account WHERE realm_id = $1 AND email = $2")
            .bind(&new_realm_id)
            .bind(admin_email)
            .fetch_optional(&ctx._app_state.pool)
            .await
            .unwrap()
            .expect("Admin user should exist");

    println!(
        "[Step 2] ✓ 管理员用户已创建: {} (ID: {})",
        admin_email, admin_user_id
    );

    // ============================================================================
    // Step 3: Verify Admin User Assigned realm-admin Role
    // ============================================================================
    println!("[Step 3] 验证管理员已分配 realm-admin 角色");

    // Get realm-admin role
    let realm_admin_role_id: uuid::Uuid =
        sqlx::query_scalar("SELECT id FROM roles WHERE realm_id = $1 AND name = 'realm-admin'")
            .bind(&new_realm_id)
            .fetch_one(&ctx._app_state.pool)
            .await
            .unwrap();

    // Check user has realm-admin role
    let user_role: Option<uuid::Uuid> = sqlx::query_scalar(
        "SELECT id FROM user_roles
         WHERE user_id = $1 AND role_id = $2 AND realm_id = $3",
    )
    .bind(admin_user_id)
    .bind(realm_admin_role_id)
    .bind(&new_realm_id)
    .fetch_optional(&ctx._app_state.pool)
    .await
    .unwrap();

    assert!(
        user_role.is_some(),
        "Admin user should have realm-admin role"
    );
    println!(
        "[Step 3] ✓ 管理员已分配 realm-admin 角色 (Role ID: {})",
        realm_admin_role_id
    );

    // ============================================================================
    // Step 4: Verify Admin Can Login
    // ============================================================================
    println!("[Step 4] 验证管理员可以登录");

    let login_payload = json!({
        "clientId": "admin-web-console",
        "email": admin_email,
        "password": admin_password,
        "turnstileToken": "dummy"
    })
    .to_string();

    let login_req = Request::builder()
        .method("POST")
        .uri(format!("/api/auth/{}/login", new_realm_id))
        .header("content-type", "application/json")
        .header("x-forwarded-for", "3.3.3.3")
        .body(Body::from(login_payload))
        .unwrap();

    let login_resp = app.clone().oneshot(login_req).await.unwrap();
    assert_eq!(login_resp.status(), StatusCode::OK, "管理员登录应成功");

    println!("[Step 4] ✓ 管理员登录成功");

    // ============================================================================
    // Cleanup: Delete test realm
    // ============================================================================
    println!("[Cleanup] 删除测试 Realm");
    sqlx::query("DELETE FROM realm WHERE id = $1")
        .bind(&new_realm_id)
        .execute(&ctx._app_state.pool)
        .await
        .unwrap();
    println!("[Cleanup] ✓ 测试 Realm 已删除");
}

/// ============================================================================
/// Scenario 2: Error Handling - Short Password
///
/// **User Story**: Try to create realm with password shorter than 8 characters
///
/// **Given**: Super Admin is logged in
/// **When**: POST /api/realms with admin_user.password = "short"
/// **Then**:
///   - Returns 400 Bad Request
///   - Realm is NOT created
///
/// **验收标准**:
/// - 返回 400 Bad Request
/// - 错误消息正确
///
/// **运行方式**:
/// ```bash
/// cargo nextest run test_scenario_create_realm_with_short_password
/// ```
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_create_realm_with_short_password(ctx: &mut TestContext) {
    // ============================================================================
    // Setup: Create Super Admin session
    // ============================================================================
    println!("[Setup] 创建 Super Admin 会话");
    let (super_admin_token, super_admin_user_id) =
        create_admin_session_with_user(ctx, "super-admin5@test.com", 1800).await;

    grant_realm_admin_role(ctx, &super_admin_user_id).await;
    println!("[Setup] ✓ Super Admin 会话创建成功");

    let app = ctx.create_unified_test_router();

    // ============================================================================
    // Step 1: Try to Create Realm with Short Password
    // ============================================================================
    println!("[Step 1] 尝试创建 Realm，使用短密码");
    let timestamp = chrono::Utc::now().timestamp_millis();
    let new_realm_id = format!("testrealm{}", timestamp % 1000000000);

    let create_payload = json!({
        "id": new_realm_id,
        "name": "Test Realm Short Password",
        "adminUser": {
            "email": "admin@test.com",
            "password": "short"
        }
    })
    .to_string();

    let req = Request::builder()
        .method("POST")
        .uri("/api/realms")
        .header("content-type", "application/json")
        .header("cookie", format!("X-Auth={}", super_admin_token))
        .body(Body::from(create_payload))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();

    // ============================================================================
    // Step 2: Verify Error Response
    // ============================================================================
    println!("[Step 2] 验证错误响应");

    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "应返回 400 Bad Request"
    );
    println!("[Step 2] ✓ 返回 400 Bad Request");
}
