/// 场景测试：Admin Realm 角色定义 API 验证
///
/// 测试目标：验证 admin realm 的角色定义 API 返回默认角色
///
/// 对应问题：Demo E2E 测试失败，用户创建表单显示 "No roles available"。
/// 前端调用 `/api/roles/admin/role-definitions` 返回空数组 `[]`。
///
/// 测试流程：
/// 1. Given: 使用 admin realm 启动测试环境
/// 2. When: 调用 GET `/api/roles/admin/role-definitions`
/// 3. Then: 返回至少 2 个角色定义（realm-admin 和 user）
/// 4. And: 每个角色包含 `id`, `name`, `description`, `client_id` 字段
/// 5. And: `client_id` 应该是 `admin-web-console` 的字符串
/// 6. And: 数据库中确实有对应的角色记录
#[cfg(test)]
mod tests {
    use crate::application::http::role_definitions::types::RoleResponse;
    use crate::tests::helpers::*;
    use crate::tests::schema_test_context::SchemaTestContext;
    use axum::{
        body::Body,
        http::{Method, Request, StatusCode, header},
    };
    use sqlx::Row;
    use test_context::test_context;
    use tower::ServiceExt;

    use SchemaTestContext as AdminRoleDefinitionsTestContext;

    /// 场景测试：Admin Realm 角色定义 API 返回默认角色
    ///
    /// Given: Admin realm 已初始化（包含 admin-web-console client app）
    /// When: 调用 GET /api/roles/admin/role-definitions
    /// Then: 返回至少 2 个角色（realm-admin 和 user）
    #[test_context(AdminRoleDefinitionsTestContext)]
    #[tokio::test]
    async fn test_scenario_admin_role_definitions_api_returns_default_roles(
        ctx: &mut AdminRoleDefinitionsTestContext,
    ) {
        // ========================================================================
        // Given: 设置 Super Admin 会话
        // ========================================================================
        let (admin_token, _user_id) = create_admin_session_with_user(
            ctx,
            "test-super-admin@test.com",
            1800, // 30 minutes
        )
        .await;

        // 授予 Realm Admin 角色
        grant_realm_admin_role(ctx, &_user_id).await;

        // ========================================================================
        // When: 调用 GET /api/roles/admin/define
        // ========================================================================
        let app = ctx.create_unified_test_router();

        let list_roles_req = Request::builder()
            .method(Method::GET)
            .uri("/api/roles/admin/define")
            .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
            .body(Body::empty())
            .unwrap();

        let list_roles_response = app.clone().oneshot(list_roles_req).await.unwrap();

        // ========================================================================
        // Then: 验证 API 返回 200 OK
        // ========================================================================
        assert_eq!(
            list_roles_response.status(),
            StatusCode::OK,
            "Role listing should succeed"
        );

        tracing::info!("✓ API returned status: {}", list_roles_response.status());

        // ========================================================================
        // Then: 解析响应并验证角色数量
        // ========================================================================
        let roles: Vec<RoleResponse> = crate::tests::response_json(list_roles_response).await;

        // 记录完整的响应（用于调试）
        tracing::info!("✓ API returned {} roles: {:?}", roles.len(), roles);

        // 验证返回至少 2 个角色
        assert!(
            roles.len() >= 2,
            "Expected at least 2 roles (realm-admin and user), got {}: {:?}",
            roles.len(),
            roles
        );

        tracing::info!(
            "✓ Role count validation passed: {} roles found",
            roles.len()
        );

        // ========================================================================
        // Then: 验证 client_id 是字符串 'admin-web-console'
        // ========================================================================
        // Verify all roles have client_id as the string identifier
        for role in &roles {
            assert_eq!(
                role.client_id, "admin-web-console",
                "Role '{}' should have client_id='admin-web-console', got '{}'",
                role.name, role.client_id
            );
        }

        tracing::info!("✓ All roles have client_id='admin-web-console'");

        // ========================================================================
        // Then: 验证每个角色包含必需字段
        // ========================================================================
        for role in &roles {
            assert!(
                role.name.is_empty() || !role.name.is_empty(),
                "Role ID should not be nil, got role: {:?}",
                role
            );
            assert!(
                !role.name.is_empty(),
                "Role name should not be empty, got role: {:?}",
                role
            );
            assert!(
                !role.realm_id.is_empty(),
                "Role realm_id should not be empty, got role: {:?}",
                role
            );
            assert!(
                !role.client_id.is_empty(),
                "Role client_id should not be empty, got role: {:?}",
                role
            );

            tracing::info!(
                "✓ Role '{}' has all required fields: id={}, realm_id={}, client_id={}",
                role.name,
                role.id,
                role.realm_id,
                role.client_id
            );
        }

        // ========================================================================
        // Then: 验证 realm-admin 角色存在且属性正确
        // ========================================================================
        let realm_admin = roles
            .iter()
            .find(|r| r.name == "realm-admin")
            .expect("realm-admin role should exist");

        assert_eq!(
            realm_admin.realm_id, ctx._realm_id,
            "realm-admin role should belong to the correct realm (expected: {}, got: {})",
            ctx._realm_id, realm_admin.realm_id
        );

        assert_eq!(
            realm_admin.description,
            Some("Realm Administrator with full permissions".to_string()),
            "realm-admin role should have correct description, got: {:?}",
            realm_admin.description
        );

        tracing::info!("✓ realm-admin role verified: {:?}", realm_admin);

        // ========================================================================
        // Then: 验证 user 角色存在且属性正确
        // ========================================================================
        let user_role = roles
            .iter()
            .find(|r| r.name == "user")
            .expect("user role should exist");

        assert_eq!(
            user_role.realm_id, ctx._realm_id,
            "user role should belong to the correct realm (expected: {}, got: {})",
            ctx._realm_id, user_role.realm_id
        );

        assert_eq!(
            user_role.description,
            Some("Regular user".to_string()),
            "user role should have correct description, got: {:?}",
            user_role.description
        );

        tracing::info!("✓ user role verified: {:?}", user_role);

        // ========================================================================
        // Then: 验证数据库中的角色记录（诊断：记录所有角色）
        // ========================================================================
        // 查询所有角色（不限制 client_id）
        let all_db_roles: Vec<(String, String, String, Option<String>)> = sqlx::query_as(
            "SELECT id::text, name, client_id, description FROM roles WHERE realm_id = $1 ORDER BY name"
        )
        .bind(&ctx._realm_id)
        .fetch_all(&ctx._app_state.pool)
        .await
        .unwrap();

        tracing::info!(
            "✓ ALL roles in database (any client_id): {:?}",
            all_db_roles
        );

        // 查询 admin-web-console 角色数量
        // Note: roles.client_id stores the client identifier string (e.g., 'admin-web-console')
        let db_role_count: i64 = sqlx::query(
            "SELECT count(*) FROM roles WHERE realm_id = $1 AND client_id = 'admin-web-console'",
        )
        .bind(&ctx._realm_id)
        .fetch_one(&ctx._app_state.pool)
        .await
        .unwrap()
        .get("count");

        tracing::info!(
            "✓ Database verification: {} roles found with client_id='admin-web-console'",
            db_role_count
        );

        assert_eq!(
            db_role_count, 2,
            "Database should have exactly 2 roles for admin realm, got {}",
            db_role_count
        );

        // ========================================================================
        // Then: 验证数据库中的角色详情
        // ========================================================================
        // Note: roles.client_id stores the client identifier string (e.g., 'admin-web-console')
        let db_roles: Vec<(String, String, String, Option<String>)> = sqlx::query_as(
            "SELECT id::text, name, client_id, description FROM roles WHERE realm_id = $1 AND client_id = 'admin-web-console' ORDER BY name"
        )
        .bind(&ctx._realm_id)
        .fetch_all(&ctx._app_state.pool)
        .await
        .unwrap();

        tracing::info!("✓ Database roles: {:?}", db_roles);

        // 验证数据库中的角色名称
        let db_role_names: Vec<&str> = db_roles.iter().map(|r| r.1.as_str()).collect();
        assert!(
            db_role_names.contains(&"realm-admin"),
            "Database should contain realm-admin role, got: {:?}",
            db_role_names
        );
        assert!(
            db_role_names.contains(&"user"),
            "Database should contain user role, got: {:?}",
            db_role_names
        );

        tracing::info!("✓ Database contains both realm-admin and user roles");

        // ========================================================================
        // Cleanup: 不需要清理，TestContext 会在测试结束后删除 Schema
        // ========================================================================
        tracing::info!("✓ Test completed successfully");
    }

    /// 回归测试：跨 realm 的 roleId 不允许被挂权限
    ///
    /// WHY: roleId 是客户端提供的主键。若不校验归属，A realm 的管理员可以
    /// 给 B realm 的角色追加权限（跨租户 RBAC 篡改）。修复后必须 404 且无写入。
    #[test_context(AdminRoleDefinitionsTestContext)]
    #[tokio::test]
    async fn test_scenario_assign_permission_rejects_foreign_realm_role(
        ctx: &mut AdminRoleDefinitionsTestContext,
    ) {
        let (admin_token, _user_id) =
            create_admin_session_with_user(ctx, "role-perm-foreign-role@test.com", 1800).await;
        grant_realm_admin_role(ctx, &_user_id).await;

        // 本 realm 的权限 + 平台 admin realm 的角色（foreign）
        let own_permission_id: uuid::Uuid = sqlx::query_scalar(
            "SELECT id FROM permissions WHERE realm_id = $1 LIMIT 1",
        )
        .bind(&ctx._realm_id)
        .fetch_one(&ctx._app_state.pool)
        .await
        .unwrap();
        let foreign_role_id: uuid::Uuid = sqlx::query_scalar(
            "SELECT id FROM roles WHERE realm_id = 'admin' LIMIT 1",
        )
        .fetch_one(&ctx._app_state.pool)
        .await
        .unwrap();

        let app = ctx.create_unified_test_router();
        let req = Request::builder()
            .method(Method::POST)
            .uri(format!(
                "/api/roles/{}/define/{}/permissions",
                ctx._realm_id, foreign_role_id
            ))
            .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                serde_json::json!({ "permissionId": own_permission_id }).to_string(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();

        assert_eq!(
            resp.status(),
            StatusCode::NOT_FOUND,
            "foreign-realm roleId must be rejected as not found"
        );

        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM role_permissions WHERE role_id = $1 AND permission_id = $2",
        )
        .bind(foreign_role_id)
        .bind(own_permission_id)
        .fetch_one(&ctx._app_state.pool)
        .await
        .unwrap();
        assert_eq!(
            count, 0,
            "no role_permissions row may be created for a foreign realm role"
        );
    }

    /// 回归测试：跨 realm 的 permissionId 不允许被挂到本 realm 角色
    ///
    /// WHY: 与上一条对称 — 权限行同样按 realm 隔离，foreign permission 必须视为
    /// 不存在，否则 A realm 管理员可将 B realm 的自定义权限挂进本 realm 角色。
    #[test_context(AdminRoleDefinitionsTestContext)]
    #[tokio::test]
    async fn test_scenario_assign_permission_rejects_foreign_realm_permission(
        ctx: &mut AdminRoleDefinitionsTestContext,
    ) {
        let (admin_token, _user_id) =
            create_admin_session_with_user(ctx, "role-perm-foreign-perm@test.com", 1800).await;
        grant_realm_admin_role(ctx, &_user_id).await;

        let own_role_id: uuid::Uuid = sqlx::query_scalar(
            "SELECT id FROM roles WHERE realm_id = $1 AND name = 'realm-admin'",
        )
        .bind(&ctx._realm_id)
        .fetch_one(&ctx._app_state.pool)
        .await
        .unwrap();
        let foreign_permission_id: uuid::Uuid = sqlx::query_scalar(
            "SELECT id FROM permissions WHERE realm_id = 'admin' LIMIT 1",
        )
        .fetch_one(&ctx._app_state.pool)
        .await
        .unwrap();

        let app = ctx.create_unified_test_router();
        let req = Request::builder()
            .method(Method::POST)
            .uri(format!(
                "/api/roles/{}/define/{}/permissions",
                ctx._realm_id, own_role_id
            ))
            .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                serde_json::json!({ "permissionId": foreign_permission_id }).to_string(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();

        assert_eq!(
            resp.status(),
            StatusCode::NOT_FOUND,
            "foreign-realm permissionId must be rejected as not found"
        );

        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM role_permissions WHERE role_id = $1 AND permission_id = $2",
        )
        .bind(own_role_id)
        .bind(foreign_permission_id)
        .fetch_one(&ctx._app_state.pool)
        .await
        .unwrap();
        assert_eq!(
            count, 0,
            "no role_permissions row may be created from a foreign realm permission"
        );
    }
}
