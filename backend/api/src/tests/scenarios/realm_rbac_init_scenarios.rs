/// 场景测试：Realm 创建时 RBAC 初始化
///
/// 测试目标：验证当创建新 Realm 时，系统自动初始化默认角色（realm-admin 和 user）
///
/// 对应问题：Demo E2E 测试中，Super Admin 创建新 Realm 后尝试创建 Realm Admin 用户时，
/// 用户表单显示 "No roles available"，角色列表 API 返回空数组。
///
/// 测试流程：
/// 1. Super Admin 创建新 Realm
/// 2. 验证 Realm 创建成功
/// 3. 查询角色列表 API
/// 4. 验证返回至少 2 个角色：realm-admin 和 user
/// 5. 验证角色的 client_id = 'admin-web-console'
/// 6. 验证 realm-admin 角色的描述
/// 7. 验证 user 角色的描述
/// 8. 清理测试数据
#[cfg(test)]
mod tests {
    use crate::application::http::role_definitions::types::{RoleListApiResponse, RoleResponse};
    use crate::tests::helpers::*;
    use crate::tests::schema_test_context::SchemaTestContext;
    use axum::{
        body::Body,
        http::{header, Method, Request, StatusCode},
    };
    use serde_json::json;
    use sqlx::Row;
    use test_context::test_context;
    use tower::ServiceExt;

    use SchemaTestContext as RealmRbacInitTestContext;

    /// 场景测试：Realm 创建后默认角色初始化
    ///
    /// Given: Super Admin 已登录
    /// When: Super Admin 创建新 Realm
    /// Then: 系统自动初始化 realm-admin 和 user 角色
    #[test_context(RealmRbacInitTestContext)]
    #[tokio::test]
    async fn test_scenario_realm_creation_initializes_default_roles(ctx: &mut RealmRbacInitTestContext) {
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
        // When: 创建新 Realm
        // ========================================================================
        // Realm ID must be between 3 and 36 alphanumeric characters
        let uuid_str = uuid::Uuid::now_v7().to_string().replace("-", "");
        let new_realm_id_full = format!("test{}", uuid_str);
        // Truncate to 36 characters if needed
        let new_realm_id = if new_realm_id_full.len() > 36 {
            &new_realm_id_full[..36]
        } else {
            &new_realm_id_full
        };
        let new_realm_name = format!("Test Realm {}", new_realm_id);

        let app = ctx.create_unified_test_router();

        let create_realm_req = Request::builder()
            .method(Method::POST)
            .uri("/api/realms")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::COOKIE, format!("X-Auth={}", admin_token))
            .body(Body::from(
                json!({
                    "id": new_realm_id,
                    "name": new_realm_name,
                })
                .to_string(),
            ))
            .unwrap();

        let create_realm_response = app.clone().oneshot(create_realm_req).await.unwrap();

        // ========================================================================
        // Then: 验证 Realm 创建成功
        // ========================================================================
        assert_eq!(
            create_realm_response.status(),
            StatusCode::OK,
            "Realm creation should succeed"
        );

        // 验证 Realm 在数据库中存在
        let realm_exists: bool = sqlx::query(
            "SELECT EXISTS(SELECT 1 FROM realm WHERE id::text = $1)",
        )
        .bind(&new_realm_id)
        .fetch_one(&ctx._app_state.pool)
        .await
        .unwrap()
        .get("exists");

        assert!(
            realm_exists,
            "Realm should exist in database after creation"
        );

        tracing::info!("✓ Realm created successfully: {}", new_realm_id);

        // ========================================================================
        // Then: 验证 admin-web-console 客户端应用已创建
        // ========================================================================
        let client_app_exists: bool = sqlx::query(
            "SELECT EXISTS(SELECT 1 FROM client_app WHERE realm_id::text = $1 AND client_id = 'admin-web-console')"
        )
        .bind(&new_realm_id)
        .fetch_one(&ctx._app_state.pool)
        .await
        .unwrap()
        .get("exists");

        assert!(
            client_app_exists,
            "admin-web-console client app should be created for new realm"
        );

        // 获取 admin-web-console 的 ID（UUID）
        let client_app_id: String = sqlx::query_scalar(
            "SELECT id::text FROM client_app WHERE realm_id::text = $1 AND client_id = 'admin-web-console'"
        )
        .bind(&new_realm_id)
        .fetch_one(&ctx._app_state.pool)
        .await
        .unwrap();

        tracing::info!("✓ admin-web-console client app created: {}", client_app_id);

        // ========================================================================
        // Then: 查询角色列表 API
        // ========================================================================
        let list_roles_req = Request::builder()
            .method(Method::GET)
            .uri(format!("/api/roles/{}/define", new_realm_id))
            .header(header::COOKIE, format!("X-Auth={}", admin_token))
            .body(Body::empty())
            .unwrap();

        let list_roles_response = app.clone().oneshot(list_roles_req).await.unwrap();

        assert_eq!(
            list_roles_response.status(),
            StatusCode::OK,
            "Role listing should succeed"
        );

        let roles_response: RoleListApiResponse =
            crate::tests::response_json(list_roles_response).await;
        let roles: Vec<RoleResponse> = roles_response.data;

        // ========================================================================
        // Then: 验证返回至少 2 个角色
        // ========================================================================
        assert!(
            roles.len() >= 2,
            "Expected at least 2 roles (realm-admin and user), got {}: {:?}",
            roles.len(),
            roles
        );

        tracing::info!(
            "✓ Role list returned {} roles for realm {}",
            roles.len(),
            new_realm_id
        );

        // ========================================================================
        // Then: 验证 realm-admin 角色存在且属性正确
        // ========================================================================
        let realm_admin = roles
            .iter()
            .find(|r| r.name == "realm-admin")
            .expect("realm-admin role should exist");

        assert_eq!(
            realm_admin.client_id, "admin-web-console",
            "realm-admin role should have client_id = 'admin-web-console'"
        );

        assert_eq!(
            realm_admin.realm_id, new_realm_id,
            "realm-admin role should belong to the correct realm"
        );

        assert_eq!(
            realm_admin.description,
            Some("Realm Administrator with full permissions".to_string()),
            "realm-admin role should have correct description"
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
            user_role.client_id,
            "admin-web-console",
            "user role should have client_id = 'admin-web-console'"
        );

        assert_eq!(
            user_role.realm_id, new_realm_id,
            "user role should belong to the correct realm"
        );

        assert_eq!(
            user_role.description,
            Some("Regular user".to_string()),
            "user role should have correct description"
        );

        tracing::info!("✓ user role verified: {:?}", user_role);

        // ========================================================================
        // Then: 验证数据库中的角色记录
        // ========================================================================
        let role_count: i64 = sqlx::query(
            "SELECT count(*) FROM roles WHERE realm_id::text = $1 AND client_id = 'admin-web-console'"
        )
        .bind(&new_realm_id)
        .fetch_one(&ctx._app_state.pool)
        .await
        .unwrap()
        .get("count");

        assert_eq!(
            role_count, 2,
            "Database should have exactly 2 roles for the new realm"
        );

        tracing::info!("✓ Database verification: 2 roles found in database");

        // ========================================================================
        // Then: 验证 realm-admin 角色的权限（11项权限）
        // ========================================================================
        let permission_count: i64 = sqlx::query(
            r#"
            SELECT count(*)
            FROM role_permissions rp
            JOIN permissions p ON rp.permission_id = p.id
            JOIN roles r ON rp.role_id = r.id
            WHERE r.realm_id::text = $1
              AND r.name = 'realm-admin'
              AND r.client_id = 'admin-web-console'
            "#
        )
        .bind(&new_realm_id)
        .fetch_one(&ctx._app_state.pool)
        .await
        .unwrap()
        .get("count");

        assert_eq!(
            permission_count, 11,
            "realm-admin role should have 11 permissions, got {}",
            permission_count
        );

        tracing::info!(
            "✓ realm-admin role has {} permissions",
            permission_count
        );

        // ========================================================================
        // Then: 验证权限策略已创建（在 role_policies 表中）
        // ========================================================================
        let policy_count: i64 = sqlx::query(
            "SELECT count(*) FROM role_policies
             WHERE realm_id::text = $1
               AND role_id = (SELECT id FROM roles WHERE name = 'realm-admin' AND realm_id::text = $1)"
        )
        .bind(&new_realm_id)
        .fetch_one(&ctx._app_state.pool)
        .await
        .unwrap()
        .get("count");

        assert!(
            policy_count > 0,
            "realm-admin role should have permission policies"
        );

        tracing::info!(
            "✓ realm-admin role has {} permission policies",
            policy_count
        );

        // ========================================================================
        // Cleanup: 删除测试 Realm（级联删除相关数据）
        // ========================================================================
        sqlx::query("DELETE FROM realm WHERE id::text = $1")
            .bind(&new_realm_id)
            .execute(&ctx._app_state.pool)
            .await
            .expect("Failed to delete test realm");

        tracing::info!("✓ Cleanup completed: test realm deleted");
    }
}
