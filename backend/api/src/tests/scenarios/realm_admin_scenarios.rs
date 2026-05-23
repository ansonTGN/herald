// =============================================================================
// Realm Admin RBAC 场景测试
// =============================================================================
//
// 测试 Realm Admin 用户的 RBAC 权限是否正确设置
//
// 问题：当 Realm Admin 用户登录并尝试访问 /api/roles/{realmId}/users 时，
// 得到 403 "Realm admin privileges required" 错误
//
// 修复：确保 realm-admin 角色自动获得 realm.manage 等权限
//
// 运行方式：
// cd backend
// cargo nextest run test_scenario_realm_admin_can_access_users
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
use uuid::Uuid;
use axum::body::to_bytes;

/// ============================================================================
/// User Story: Realm Admin 可以访问用户管理 API
///
/// **场景描述**：
/// Realm Admin 用户登录后，可以访问 /api/roles/{realmId}/users 端点
/// 来管理本 realm 的用户。
///
/// **测试步骤**：
/// 1. Super Admin 创建新 Realm
/// 2. 在新 Realm 中创建用户，分配 realm-admin 角色
/// 3. 用户登录系统
/// 4. 用户尝试访问 /api/roles/{realmId}/users
/// 5. 验证用户可以成功访问（返回 200 OK）
///
/// **验收标准**：
/// - Realm Admin 用户可以访问用户列表 API
/// - 返回 200 OK（不是 403 Forbidden）
/// - realm-admin 角色拥有 dashboard.view, audit.view, api_keys.view, realm.manage 等权限
///
/// **运行方式**：
/// ```bash
/// cargo nextest run test_scenario_realm_admin_can_access_users
/// ```
///
/// **相关用户故事**: docs/user-stories/02-realm-admin-user-stories.md
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_realm_admin_can_access_users(ctx: &mut TestContext) {
    // 使用统一的测试路由器，包含所有路由
    let app = ctx.create_unified_test_router();

    // ============================================================================
    // Setup: 创建 Super Admin 会话
    // ============================================================================
    println!("[Setup] 创建 Super Admin 会话");
    let (super_admin_token, super_admin_user_id) = create_admin_session_with_user(
        ctx,
        "super-admin-test@test.com",
        1800
    ).await;

    // 授予 Realm Admin 角色
    grant_realm_admin_role(ctx, &super_admin_user_id).await;
    println!("[Setup] ✓ Super Admin 会话创建成功");

    // ============================================================================
    // Step 1: 创建新 Realm
    // ============================================================================
    println!("[Step 1] 创建新 Realm");
    let timestamp = chrono::Utc::now().timestamp_millis();
    let new_realm_id = format!("testrealm{}", timestamp % 1000000000);
    let new_realm_name = "Test Realm for Realm Admin";

    let create_payload = json!({
        "id": new_realm_id,
        "name": new_realm_name,
        "adminUser": {
            "email": "admin@testrealm.com",
            "password": "Password123!"
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
    assert_eq!(resp.status(), StatusCode::OK, "创建 Realm 应返回 200");

    let body: serde_json::Value = crate::tests::response_json(resp).await;
    assert_eq!(body["id"], new_realm_id, "Realm ID 应匹配");
    println!("[Step 1] ✓ Realm 创建成功: {} ({})", new_realm_name, new_realm_id);

    // ============================================================================
    // Step 2: 获取新 Realm 的 client_app 和 realm-admin 角色
    // ============================================================================
    println!("[Step 2] 获取新 Realm 的配置");

    // 获取新 realm 的 client_app
    let client_app: (Uuid, String) = sqlx::query_as(
        "SELECT id, client_id FROM client_app WHERE realm_id = $1 AND client_id = 'admin-web-console'",
    )
    .bind(&new_realm_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to get admin-web-console client app");

    let new_client_uuid = client_app.0;
    let new_client_id = client_app.1;
    println!(
        "[Step 2] ✓ 获取 client_app: {} (UUID: {})",
        new_client_id, new_client_uuid
    );

    // 获取 realm-admin 角色
    let realm_admin_role: (Uuid, String, String) = sqlx::query_as(
        "SELECT id, name, description FROM roles WHERE realm_id = $1 AND name = 'realm-admin'",
    )
    .bind(&new_realm_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to get realm-admin role");

    let realm_admin_role_id = realm_admin_role.0;
    println!(
        "[Step 2] ✓ 获取 realm-admin 角色: {} (UUID: {})",
        realm_admin_role.1, realm_admin_role_id
    );

    // ============================================================================
    // Step 3: 在新 Realm 中创建用户，分配 realm-admin 角色
    // ============================================================================
    println!("[Step 3] 创建 Realm Admin 用户");

    let realm_admin_email = "realm-admin@example.com";
    let realm_admin_password = "SecurePassword123!";

    // 启用注册
    sqlx::query(
        "INSERT INTO realm_config (realm_id, config_type, config_key, config_value, enabled)
         VALUES ($1, 'registration', 'allowed', 'true', true)
         ON CONFLICT (realm_id, config_type, config_key) DO UPDATE SET enabled = true",
    )
    .bind(&new_realm_id)
    .execute(&ctx._app_state.pool)
    .await
    .unwrap();

    // 注册用户
    let payload = json!({
        "email": realm_admin_email,
        "password": realm_admin_password,
        "turnstileToken": "dummy"
    });
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/auth/{}/register", new_realm_id))
        .header("content-type", "application/json")
        .header("x-forwarded-for", "3.3.3.3")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "注册失败");

    // 获取验证码并验证邮箱
    let code: String = sqlx::query_scalar(
        "select verification_code from email_verification_code
         where email = $1 order by id desc limit 1",
    )
    .bind(realm_admin_email)
    .fetch_one(&ctx._app_state.pool)
    .await
    .unwrap();

    let req = Request::builder()
        .method("GET")
        .uri(format!(
            "/api/auth/{}/verify_email/confirm/{}",
            new_realm_id, code
        ))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "邮箱验证失败");

    // Verify email verification response
    let body = hyper::body::to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["message"], "ok", "Email verification response should be ok");

    // 获取 user_id
    let realm_admin_user_id: Uuid = sqlx::query_scalar("select id from account where email = $1")
        .bind(realm_admin_email)
        .fetch_one(&ctx._app_state.pool)
        .await
        .unwrap();

    // 分配 realm-admin 角色给用户（直接操作 user_roles 表）
    // 注意：不使用 API，因为 API 有安全限制（需要 roles.manage 权限）
    // 测试代码可以直接操作数据库来设置测试场景
    {
        let user_role_id = uuid::Uuid::now_v7();
        sqlx::query(
            "INSERT INTO user_roles (id, user_id, role_id, realm_id, client_id, principal_type, principal_id)
             VALUES ($1, $2, $3, $4, $5, $6, $2::text)"
        )
        .bind(user_role_id)
        .bind(realm_admin_user_id)
        .bind(realm_admin_role_id)
        .bind(&new_realm_id)
        .bind(&new_client_id)
        .bind(herald_core::domain::authorization::principal_types::USER)
        .execute(&ctx._app_state.pool)
        .await
        .expect("Failed to assign realm-admin role to user");

        // Invalidate cache
        let _ = ctx
            ._app_state
            .permission_checker
            .invalidate_user_role_cache(&new_realm_id, &realm_admin_user_id.to_string())
            .await;

        println!("[Step 3] ✓ 直接通过 user_roles 表分配 realm-admin 角色");
    }

    println!(
        "[Step 3] ✓ Realm Admin 用户创建成功: {} (ID: {})",
        realm_admin_email, realm_admin_user_id
    );

    // ============================================================================
    // Step 4: 用户登录
    // ============================================================================
    println!("[Step 4] Realm Admin 用户登录");

    let payload = json!({
        "clientId": new_client_id,
        "email": realm_admin_email,
        "password": realm_admin_password,
        "turnstileToken": "dummy"
    });
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/auth/{}/login", new_realm_id))
        .header("content-type", "application/json")
        .header("x-forwarded-for", "3.3.3.3")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "登录失败");

    // 提取 token
    let set_cookie = resp
        .headers()
        .get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .unwrap()
        .to_string();

    let realm_admin_token = crate::tests::extract_set_cookie_token(&set_cookie, "X-Auth").unwrap();
    println!("[Step 4] ✓ Realm Admin 用户登录成功");

    // ============================================================================
    // Step 5: 验证 realm-admin 角色的权限策略
    // ============================================================================
    println!("[Step 5] 验证 realm-admin 角色的权限策略");

    // 检查是否存在 realm.manage 权限策略（admin realm 下的 realm-admin）
    let has_realm_manage_policy: bool = sqlx::query(
        "SELECT EXISTS(SELECT 1 FROM role_policies
         WHERE role_id = $1 AND realm_id = $2 AND resource = 'realm' AND action = 'manage')"
    )
    .bind(realm_admin_role_id)
    .bind(&new_realm_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .unwrap()
    .get("exists");

    // 检查是否存在 dashboard.view 权限策略
    let has_dashboard_view_policy: bool = sqlx::query(
        "SELECT EXISTS(SELECT 1 FROM role_policies
         WHERE role_id = $1 AND realm_id = $2 AND resource = 'dashboard' AND action = 'view')"
    )
    .bind(realm_admin_role_id)
    .bind(&new_realm_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .unwrap()
    .get("exists");

    if !has_realm_manage_policy && !has_dashboard_view_policy {
        println!(
            "[Step 5] Warning: realm-admin 角色缺少标准权限策略"
        );
    }

    // ============================================================================
    // Step 6: Realm Admin 尝试访问用户列表 API
    // ============================================================================
    println!("[Step 6] Realm Admin 访问用户列表 API");

    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/roles/{}/users", new_realm_id))
        .header("cookie", format!("X-Auth={}", realm_admin_token))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();

    // 验证响应状态码
    let status = resp.status();
    println!("[Step 6] 响应状态码: {}", status);

    if status == StatusCode::OK {
        println!("[Step 6] OK 测试通过：Realm Admin 可以访问用户列表 (200 OK)");

        // 验证响应内容
        let body: serde_json::Value = crate::tests::response_json(resp).await;
        assert!(body["items"].is_array(), "响应应包含 items 数组");
        println!(
            "[Step 6] OK 用户列表返回成功，共 {} 个用户",
            body["items"].as_array().unwrap().len()
        );

        // 最终断言 - 验证标准权限策略存在
        assert!(
            has_realm_manage_policy || has_dashboard_view_policy,
            "realm-admin 角色应该有 realm.manage 或 dashboard.view 权限"
        );
    } else {
        // 打印调试信息
        println!("[Step 6] FAIL 测试失败：Realm Admin 无法访问用户列表 ({} )", status);
        println!("[Step 6] 原因：realm-admin 角色缺少必要权限");

        println!("\n=== 调试信息 ===");
        println!("Realm ID: {}", new_realm_id);
        println!("Client UUID: {}", new_client_uuid);
        println!("Client ID: {}", new_client_id);
        println!("Realm Admin Role ID: {}", realm_admin_role_id);
        println!("Realm Admin User ID: {}", realm_admin_user_id);

        // 检查用户是否有 realm-admin 角色（在 user_roles 表中）
        let user_role_exists: bool = sqlx::query(
            "SELECT EXISTS(SELECT 1 FROM user_roles WHERE user_id = $1::uuid AND role_id = $2)"
        )
        .bind(realm_admin_user_id)
        .bind(realm_admin_role_id)
        .fetch_one(&ctx._app_state.pool)
        .await
        .unwrap()
        .get("exists");

        println!("User has realm-admin role: {}", user_role_exists);

        // 列出 realm-admin 角色的所有权限
        let role_policies: Vec<(String, String)> = sqlx::query_as(
            "SELECT resource, action FROM role_policies WHERE role_id = $1 AND realm_id = $2"
        )
        .bind(realm_admin_role_id)
        .bind(&new_realm_id)
        .fetch_all(&ctx._app_state.pool)
        .await
        .unwrap();

        println!("\nRealm Admin Role Policies (权限策略):");
        for (resource, action) in &role_policies {
            println!("  {}: {}", resource, action);
        }

        assert_eq!(
            status,
            StatusCode::OK,
            "Realm Admin 应该能访问用户列表"
        );
    }

    println!("\n✅ Realm Admin RBAC 场景测试完成！");
}

/// ============================================================================
/// User Story: 验证 Realm 创建时自动设置 realm-admin 权限
///
/// **场景描述**：
/// 验证当创建新 Realm 时，系统自动授予 realm-admin 角色
/// dashboard.view, audit.view, api_keys.view, realm.manage 等权限。
///
/// **测试步骤**：
/// 1. Super Admin 创建新 Realm
/// 2. 检查 realm-admin 角色是否存在
/// 3. 检查 realm-admin 角色是否有 dashboard.view 等权限
///
/// **验收标准**：
/// - realm-admin 角色存在
/// - realm-admin 角色有 dashboard.view 等权限
///
/// **运行方式**：
/// ```bash
/// cargo nextest run test_scenario_realm_creation_sets_realm_admin_permissions
/// ```
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_realm_creation_sets_realm_admin_permissions(ctx: &mut TestContext) {
    // 使用统一的测试路由器，包含所有路由
    let app = ctx.create_unified_test_router();

    // ============================================================================
    // Setup: 创建 Super Admin 会话
    // ============================================================================
    println!("[Setup] 创建 Super Admin 会话");
    let (super_admin_token, _user_id) = create_admin_session_with_user(
        ctx,
        "super-admin-perm-test@test.com",
        1800
    ).await;

    // 授予 Realm Admin 角色
    grant_realm_admin_role(ctx, &_user_id).await;
    println!("[Setup] ✓ Super Admin 会话创建成功");

    // ============================================================================
    // Step 1: 创建新 Realm
    // ============================================================================
    println!("[Step 1] 创建新 Realm");
    let timestamp = chrono::Utc::now().timestamp_millis();
    let new_realm_id = format!("permtest{}", timestamp % 1000000000);
    let new_realm_name = "Permission Test Realm";

    let create_payload = json!({
        "id": new_realm_id,
        "name": new_realm_name,
        "adminUser": {
            "email": "admin@testrealm.com",
            "password": "Password123!"
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
    assert_eq!(resp.status(), StatusCode::OK, "创建 Realm 应返回 200");

    let body: serde_json::Value = crate::tests::response_json(resp).await;
    assert_eq!(body["id"], new_realm_id, "Realm ID 应匹配");
    println!("[Step 1] ✓ Realm 创建成功: {}", new_realm_id);

    // ============================================================================
    // Step 2: 获取新 Realm 的配置
    // ============================================================================
    println!("[Step 2] 获取新 Realm 的配置");

    // 获取新 realm 的 client_app
    let client_app: (Uuid, String) = sqlx::query_as(
        "SELECT id, client_id FROM client_app WHERE realm_id = $1 AND client_id = 'admin-web-console'",
    )
    .bind(&new_realm_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to get admin-web-console client app");

    let new_client_uuid = client_app.0;

    // 获取 realm-admin 角色
    let realm_admin_role: (Uuid,) = sqlx::query_as(
        "SELECT id FROM roles WHERE realm_id = $1 AND name = 'realm-admin'",
    )
    .bind(&new_realm_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to get realm-admin role");

    let realm_admin_role_id = realm_admin_role.0;
    println!("[Step 2] ✓ 获取 realm-admin 角色: {}", realm_admin_role_id);

    // ============================================================================
    // Step 3: 验证 realm-admin 角色有 dashboard.view 等权限
    // ============================================================================
    println!("[Step 3] 验证 realm-admin 角色的权限");

    // 检查是否存在 dashboard.view 权限策略（替代原来的 realm.admin）
    let has_dashboard_view_policy: bool = sqlx::query(
        "SELECT EXISTS(SELECT 1 FROM role_policies
         WHERE role_id = $1 AND realm_id = $2 AND resource = 'dashboard' AND action = 'view')"
    )
    .bind(realm_admin_role_id)
    .bind(&new_realm_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .unwrap()
    .get("exists");

    if !has_dashboard_view_policy {
        println!("[Step 3] FAIL 权限策略缺失");
        println!("[Step 3] 预期策略: dashboard.view for role {}", realm_admin_role_id);

        // 列出实际存在的权限策略
        let role_policies: Vec<(String, String)> = sqlx::query_as(
            "SELECT resource, action FROM role_policies WHERE role_id = $1 AND realm_id = $2"
        )
        .bind(realm_admin_role_id)
        .bind(&new_realm_id)
        .fetch_all(&ctx._app_state.pool)
        .await
        .unwrap();

        println!("[Step 3] 当前所有相关权限策略:");
        for (resource, action) in &role_policies {
            println!("  {}: {}", resource, action);
        }

        assert!(
            has_dashboard_view_policy,
            "realm-admin 角色应该有 dashboard.view 权限"
        );
    } else {
        println!("[Step 3] OK realm-admin 角色有 dashboard.view 权限");
    }

    println!("\n✅ Realm 创建时自动设置权限测试完成！");
}
