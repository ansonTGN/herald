// =============================================================================
// Realm Isolation Scenarios - Super-Admin Removal Tests
// =============================================================================
//
// **任务**: 测试移除 super-admin 概念后的 Realm 严格隔离
//
// **验收标准**:
// 1. admin 用户不能访问其他 realm 资源
// 2. realm-1 admin 不能访问 realm-2 资源
// 3. admin 用户可以创建 realm（特殊权限）
// 4. "All" 权限策略不再生效
//
// **运行方式**:
// ```bash
// cargo nextest run --workspace realm_isolation_scenarios
// ```
//
// =============================================================================

use crate::tests::helpers::*;
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use herald_core::domain::authorization::PermissionService;
use serde_json::json;
use test_context::test_context;
use tower::ServiceExt;

/// ============================================================================
/// Scenario 1: Admin 用户不能访问其他 Realm
///
/// **Given**: admin 用户属于 admin realm，存在另一个 realm (realm-1)
/// **When**: admin 用户尝试访问 realm-1 的用户列表
/// **Then**: 返回 403 Forbidden
///
/// **验收标准**:
/// - 状态码为 403
/// - 错误消息包含 "cannot access resources from a different realm"
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_admin_cannot_access_other_realms(ctx: &mut TestContext) {
    println!("\n=== Scenario: Admin 用户不能访问其他 Realm ===\n");

    let app = ctx.create_unified_test_router();

    // ============================================================================
    // Setup: 创建 admin realm 的管理员
    // ============================================================================
    println!("[Setup] 创建 admin realm 管理员");

    // 在 admin realm 创建管理员用户
    let (admin_token, admin_user_id) =
        create_admin_session_with_user(ctx, "admin@cas.com", 1800).await;

    // 授予 realm-admin 角色
    grant_realm_admin_role(ctx, &admin_user_id).await;

    println!(
        "[Setup] ✓ admin 用户创建成功，realm_id={}, user_id={}",
        ctx._realm_id, admin_user_id
    );

    // ============================================================================
    // Step 1: 创建另一个 realm (realm-1)
    // ============================================================================
    println!("\n[Step 1] 创建 realm-1");

    let realm1_name = "Test Realm 1";
    let realm1_payload = json!({
        "name": realm1_name,
        "adminUser": {
            "email": "admin@realm1.com",
            "password": "password123"
        }
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/realms")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, format!("X-Auth={}", admin_token))
        .body(Body::from(realm1_payload.to_string()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED, "创建 realm-1 应该成功");

    let body: serde_json::Value = crate::tests::response_json(resp).await;
    let realm1_id = body["id"].as_str().expect("Failed to get realm1_id");

    println!("[Step 1] ✓ realm-1 创建成功: {}", realm1_id);

    // ============================================================================
    // When: admin 用户尝试访问 realm-1 的用户列表
    // ============================================================================
    println!("\n[When] admin 用户尝试访问 realm-1 的用户列表");

    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/users/{}?page=0&pageSize=20", realm1_id))
        .header(header::COOKIE, format!("X-Auth={}", admin_token))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();

    println!("[Then] 响应状态码: {}", status);

    // ============================================================================
    // Then: 返回 403 Forbidden
    // ============================================================================
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "admin 用户访问其他 realm 应该返回 403 Forbidden"
    );

    // 验证错误消息（权限检查在 realm 边界检查之前执行）
    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str =
        String::from_utf8(body_bytes.to_vec()).unwrap_or_else(|_| "<invalid utf8>".to_string());

    // 错误可能是权限不足或 realm 边界违规（取决于检查顺序）
    assert!(
        body_str.contains("Insufficient permissions")
            || body_str.contains("cannot access resources from a different realm")
            || body_str.contains("Access denied")
            || body_str.contains("cannot list users from a different realm"),
        "错误消息应该包含权限或边界违规提示，实际: {}",
        body_str
    );

    println!("\n✅ 测试通过: admin 用户无法访问其他 realm");
}
/// ============================================================================
/// Scenario 3: Realm-1 Admin 不能访问 Realm-2
///
/// **Given**: realm-1-admin 属于 realm-1
/// **When**: realm-1-admin 访问 realm-1 的用户列表（应成功）
///        realm-1-admin 尝试访问 realm-2 的用户列表（应失败）
/// **Then**: 第一个请求返回 200，第二个请求返回 403
///
/// **验收标准**:
/// - realm-1 admin 可以访问 realm-1
/// - realm-1 admin 不能访问 realm-2
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_realm_admin_can_only_manage_own_realm(ctx: &mut TestContext) {
    println!("\n=== Scenario: Realm-1 Admin 只能管理 Realm-1 ===\n");

    let app = ctx.create_unified_test_router();

    // ============================================================================
    // Setup: 在当前测试 realm 创建 realm-admin
    // ============================================================================
    println!("[Setup] 创建 realm-1 管理员");

    let (realm1_admin_token, realm1_admin_user_id) =
        create_admin_session_with_user(ctx, "realm1-admin@test.com", 1800).await;

    grant_realm_admin_role(ctx, &realm1_admin_user_id).await;

    println!(
        "[Setup] ✓ realm-1 管理员创建成功，realm_id={}",
        ctx._realm_id
    );

    // ============================================================================
    // Test 1: realm-1-admin 可以访问 realm-1
    // ============================================================================
    println!("\n[Test 1] realm-1-admin 访问 realm-1 的用户列表");

    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/users/{}?page=0&pageSize=20", ctx._realm_id))
        .header(header::COOKIE, format!("X-Auth={}", realm1_admin_token))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();

    println!("[Test 1] 响应状态码: {}", status);

    assert_eq!(
        status,
        StatusCode::OK,
        "realm-1 admin 访问自己的 realm 应该返回 200 OK"
    );

    println!("[Test 1] ✅ realm-1 admin 可以访问 realm-1");

    // ============================================================================
    // Test 2: realm-1-admin 不能访问 realm-2
    // ============================================================================
    println!("\n[Test 2] realm-1-admin 尝试访问 realm-2 的用户列表");

    let req = Request::builder()
        .method("GET")
        .uri("/api/users/realm-2?page=0&pageSize=20")
        .header(header::COOKIE, format!("X-Auth={}", realm1_admin_token))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();

    println!("[Test 2] 响应状态码: {}", status);

    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "realm-1 admin 访问其他 realm 应该返回 403 Forbidden"
    );

    println!("[Test 2] ✅ realm-1 admin 不能访问 realm-2");

    println!("\n✅ 测试通过: Realm Admin 严格隔离");
}

/// ============================================================================
/// Scenario 4: "All" 权限策略不再生效
///
/// **Given**: 数据库中存在 "All" 权限策略（模拟旧数据）
/// **When**: 检查用户是否有特定权限
/// **Then**: 返回 false（"All" 策略不再匹配）
///
/// **验收标准**:
/// - 权限检查器使用精确匹配
/// - "All" 资源不匹配任何实际资源
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_all_permission_no_longer_matches(ctx: &mut TestContext) {
    println!("\n=== Scenario: \"All\" 权限策略不再生效 ===\n");

    // ============================================================================
    // Setup: 创建测试用户并授予 "All" 权限策略
    // ============================================================================
    println!("[Setup] 创建测试用户并授予 \"All\" 权限策略");

    let (user_token, user_id) =
        create_admin_session_with_user(ctx, "all-permission-test@test.com", 1800).await;

    // 创建一个测试角色
    let role_uuid = uuid::Uuid::now_v7();
    sqlx::query(
        "INSERT INTO roles (id, name, description, realm_id, client_id, is_builtin)
         VALUES ($1, 'test-all-role', 'Test role with All permission', $2, $3, false)",
    )
    .bind(role_uuid)
    .bind(&ctx._realm_id)
    .bind(&ctx._client_id)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create test role");

    println!("[Setup] ✓ 测试角色创建成功");

    // 添加 "All" 权限策略（模拟旧数据）
    let policy_uuid = uuid::Uuid::now_v7();
    sqlx::query(
        "INSERT INTO role_policies (id, role_id, realm_id, resource, action)
         VALUES ($1, $2, $3, 'All', 'allow')",
    )
    .bind(policy_uuid)
    .bind(role_uuid)
    .bind(&ctx._realm_id)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to add All permission policy");

    println!("[Setup] ✓ \"All\" 权限策略已添加");

    // 将用户加入角色
    let user_role_id = uuid::Uuid::now_v7();
    let user_uuid = uuid::Uuid::parse_str(&user_id).expect("Failed to parse user_id");
    sqlx::query(
        "INSERT INTO user_roles (id, user_id, role_id, realm_id, client_id, principal_type, principal_id)
         VALUES ($1, $2, $3, $4, $5, $6, $2::text)",
    )
    .bind(user_role_id)
    .bind(user_uuid)
    .bind(role_uuid)
    .bind(&ctx._realm_id)
    .bind(&ctx._client_id)
    .bind(herald_core::domain::authorization::principal_types::USER)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to add user to role");

    println!("[Setup] ✓ 用户已加入测试角色");

    // Invalidate cache
    let _ = ctx
        ._app_state
        .permission_checker
        .invalidate_user_role_cache(&ctx._realm_id, &user_id)
        .await;

    // ============================================================================
    // When: 检查用户是否有 "users.view" 权限
    // ============================================================================
    println!("\n[When] 检查用户是否有 users.view 权限");

    let app = ctx.create_unified_test_router();

    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/users/{}?page=0&pageSize=20", ctx._realm_id))
        .header(header::COOKIE, format!("X-Auth={}", user_token))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();

    println!("[Then] 响应状态码: {}", status);

    // ============================================================================
    // Then: 返回 403 Forbidden（"All" 策略不再匹配）
    // ============================================================================
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "\"All\" 权限策略不应该匹配 users.view 权限，应返回 403 Forbidden"
    );

    println!("\n✅ 测试通过: \"All\" 权限策略不再生效");

    // ============================================================================
    // Cleanup: 删除测试数据
    // ============================================================================
    sqlx::query("DELETE FROM user_roles WHERE user_id::text = $1")
        .bind(&user_id)
        .execute(&ctx._app_state.pool)
        .await
        .unwrap();

    sqlx::query("DELETE FROM role_policies WHERE role_id::text = $1")
        .bind(role_uuid.to_string())
        .execute(&ctx._app_state.pool)
        .await
        .unwrap();

    sqlx::query("DELETE FROM roles WHERE id::text = $1")
        .bind(role_uuid.to_string())
        .execute(&ctx._app_state.pool)
        .await
        .unwrap();
}
/// ============================================================================
/// Scenario 6: Admin 用户拥有 realm-admin 角色
///
/// **Given**: 初始化 admin 用户
/// **When**: 查询 admin 用户的角色
/// **Then**: admin 用户只有 realm-admin 角色
///
/// **验收标准**:
/// - admin 用户有且仅有 realm-admin 角色
/// - 不包含 super-admin 角色
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_admin_user_has_realm_admin_role(ctx: &mut TestContext) {
    println!("\n=== Scenario: Admin 用户拥有 realm-admin 角色 ===\n");

    // ============================================================================
    // Setup: 创建 admin 用户
    // ============================================================================
    println!("[Setup] 创建 admin 用户");

    let admin_email = "admin-role-test@cas.com";
    let (_admin_token, admin_user_id) =
        create_admin_session_with_user(ctx, admin_email, 1800).await;

    grant_realm_admin_role(ctx, &admin_user_id).await;

    println!("[Setup] ✓ admin 用户创建成功");

    // ============================================================================
    // When: 查询 admin 用户的角色
    // ============================================================================
    println!("\n[When] 查询 admin 用户的角色");

    let user_roles: Vec<(String, String)> = sqlx::query_as(
        "SELECT r.name, r.id::text
         FROM user_roles ur
         JOIN roles r ON ur.role_id = r.id
         WHERE ur.user_id::text = $1 AND ur.realm_id::text = $2",
    )
    .bind(&admin_user_id)
    .bind(&ctx._realm_id)
    .fetch_all(&ctx._app_state.pool)
    .await
    .unwrap();

    println!("[Then] 检查用户的角色数量和名称");

    // ============================================================================
    // Then: admin 用户只有 realm-admin 角色
    // ============================================================================
    assert_eq!(
        user_roles.len(),
        1,
        "admin 用户应该只有 1 个角色，实际: {:?}",
        user_roles
    );

    let (role_name, _role_id) = &user_roles[0];
    assert_eq!(
        role_name, "realm-admin",
        "admin 用户应该有 realm-admin 角色，实际: {}",
        role_name
    );

    println!("[Then] ✅ admin 用户有且仅有 realm-admin 角色");

    // 验证没有 super-admin 角色
    let has_super_admin = user_roles.iter().any(|(name, _)| name == "super-admin");

    assert!(!has_super_admin, "admin 用户不应该有 super-admin 角色");

    println!("[Then] ✅ admin 用户没有 super-admin 角色");

    println!("\n✅ 测试通过: Admin 用户拥有正确的角色");
}
