/// 场景测试：权限安全性验证
///
/// 测试目标：验证权限系统的安全功能
/// - 删除权限定义需要 `permissions.manage` 权限
/// - 删除角色定义需要 `roles.manage` 权限
/// - 内置权限不能被删除
/// - 内置角色不能被删除
///
/// 来源：`.ai/design/fix-permission.md` Section 5.5.2
/// 用户故事：US-RA-002, US-RA-010
#[cfg(test)]
mod tests {
    use crate::application::http::admin::permission_definitions::types::PermissionCreateRequest;
    use crate::tests::helpers::*;
    use crate::tests::schema_test_context::SchemaTestContext;
    use axum::{
        body::Body,
        http::{Request, StatusCode, header},
    };
    use herald_core::domain::authorization::PermissionService;
    use serde_json::json;
    use test_context::test_context;
    use tower::ServiceExt;

    use SchemaTestContext as PermissionSecurityTestContext;

    /// 场景测试：删除权限定义需要 manage 权限
    ///
    /// **Given**: 用户拥有 `permissions:view` 权限但没有 `permissions.manage` 权限
    /// **When**: 用户尝试删除权限定义
    /// **Then**: API 返回 403 Forbidden
    /// **And**: 错误消息包含 "Missing permissions.manage permission"
    #[test_context(PermissionSecurityTestContext)]
    #[tokio::test]
    async fn test_scenario_delete_permission_requires_manage_permission(
        ctx: &mut PermissionSecurityTestContext,
    ) {
        // ========================================================================
        // Given: 创建测试用户并授予 permissions:view 权限
        // ========================================================================
        let (admin_token, user_id_str) =
            create_admin_session_with_user(ctx, "test-view-perm@test.com", 1800).await;
        let _user_id = uuid::Uuid::parse_str(&user_id_str).expect("Invalid user_id UUID");

        // 临时授予 realm-admin 角色来创建测试角色
        grant_realm_admin_role(ctx, &user_id_str).await;

        // 创建自定义角色
        let role_name = "test-perm-view-role";
        let role_id = create_role(
            ctx,
            &ctx._realm_id,
            &admin_token,
            role_name,
            "Role with permissions:view only",
        )
        .await;

        // 撤销 realm-admin 角色
        sqlx::query("DELETE FROM user_roles WHERE user_id = $1::uuid")
            .bind(&user_id_str)
            .execute(&ctx._app_state.pool)
            .await
            .expect("Failed to revoke realm-admin role");

        // 清除缓存
        let _ = ctx
            ._app_state
            .permission_checker
            .invalidate_user_role_cache(&ctx._realm_id, &user_id_str)
            .await;

        // 为角色分配 permissions:view 权限（没有 permissions.manage）
        sqlx::query(
            "INSERT INTO role_policies (id, realm_id, role_id, resource, action, created_at)
             VALUES ($1, $2, $3, $4, $5, NOW())",
        )
        .bind(uuid::Uuid::now_v7())
        .bind(&ctx._realm_id)
        .bind(role_id)
        .bind("permissions")
        .bind("view")
        .execute(&ctx._app_state.pool)
        .await
        .expect("Failed to assign permission to role");

        // 为用户分配角色
        assign_role_to_user(
            ctx,
            &ctx._realm_id,
            &admin_token,
            uuid::Uuid::parse_str(&user_id_str).unwrap(),
            role_id,
        )
        .await;

        // 清除权限缓存
        let _ = ctx
            ._app_state
            .permission_checker
            .invalidate_user_role_cache(&ctx._realm_id, &user_id_str)
            .await;

        // 创建测试权限定义
        let app = ctx.create_unified_test_router();
        let create_req = Request::builder()
            .method("POST")
            .uri(format!("/api/permission/{}/define", ctx._realm_id))
            .header("content-type", "application/json")
            .header(header::COOKIE, format!("X-Auth={}", admin_token))
            .body(Body::from(
                json!(PermissionCreateRequest {
                    name: "test.delete".to_string(),
                    description: Some("Test permission to delete".to_string()),
                })
                .to_string(),
            ))
            .unwrap();

        // 注意：这里需要临时授予 permissions.manage 权限来创建测试数据
        // 然后撤销该权限再测试删除
        let _ = sqlx::query(
            "INSERT INTO role_policies (id, realm_id, role_id, resource, action, created_at)
             VALUES ($1, $2, $3, $4, $5, NOW())",
        )
        .bind(uuid::Uuid::now_v7())
        .bind(&ctx._realm_id)
        .bind(role_id)
        .bind("permissions")
        .bind("manage")
        .execute(&ctx._app_state.pool)
        .await;

        let create_response = app.clone().oneshot(create_req).await.unwrap();
        assert_eq!(create_response.status(), StatusCode::CREATED);

        let created_permission: serde_json::Value =
            crate::tests::response_json(create_response).await;
        let permission_id = created_permission["id"].as_str().unwrap();

        // 撤销 permissions.manage 权限
        sqlx::query("DELETE FROM role_policies WHERE role_id = $1 AND resource = 'permissions' AND action = 'manage'")
            .bind(role_id)
            .execute(&ctx._app_state.pool)
            .await
            .expect("Failed to revoke permission");

        // 清除角色策略缓存
        let _ = ctx
            ._app_state
            .permission_checker
            .invalidate_role_policy_cache(&ctx._realm_id, &role_id.to_string())
            .await;

        // 清除用户权限缓存
        let _ = ctx
            ._app_state
            .permission_checker
            .invalidate_user_role_cache(&ctx._realm_id, &user_id_str)
            .await;

        // ========================================================================
        // When: 尝试删除权限定义
        // ========================================================================
        let app = ctx.create_unified_test_router();
        let delete_response = Request::builder()
            .method("DELETE")
            .uri(format!(
                "/api/permission/{}/define/{}",
                ctx._realm_id, permission_id
            ))
            .header("cookie", format!("X-Auth={}", admin_token))
            .body(Body::empty())
            .unwrap();

        let delete_response = app.oneshot(delete_response).await.unwrap();

        // ========================================================================
        // Then: 验证返回 403 Forbidden
        // ========================================================================
        assert_eq!(
            delete_response.status(),
            StatusCode::FORBIDDEN,
            "Delete should be forbidden without permissions.manage"
        );

        let error_body: serde_json::Value = crate::tests::response_json(delete_response).await;
        let error_message = error_body["message"].as_str().unwrap();

        assert!(
            error_message.contains("permissions") && error_message.contains("manage"),
            "Error message should mention missing permissions.manage permission, got: {}",
            error_message
        );

        tracing::info!("✓ Delete permission correctly requires permissions.manage");
    }

    /// 场景测试：删除角色定义需要 manage 权限
    ///
    /// **Given**: 用户拥有 `roles:view` 权限但没有 `roles.manage` 权限
    /// **When**: 用户尝试删除角色定义
    /// **Then**: API 返回 403 Forbidden
    /// **And**: 错误消息包含 "Missing roles.manage permission"
    #[test_context(PermissionSecurityTestContext)]
    #[tokio::test]
    async fn test_scenario_delete_role_requires_manage_permission(
        ctx: &mut PermissionSecurityTestContext,
    ) {
        // ========================================================================
        // Given: 创建测试用户并授予 roles:view 权限
        // ========================================================================
        let (admin_token, user_id_str) =
            create_admin_session_with_user(ctx, "test-view-role@test.com", 1800).await;
        let _user_id = uuid::Uuid::parse_str(&user_id_str).expect("Invalid user_id UUID");

        // 临时授予 realm-admin 角色来创建测试角色
        grant_realm_admin_role(ctx, &user_id_str).await;

        // 创建自定义角色
        let role_name = "test-role-view-role";
        let role_id = create_role(
            ctx,
            &ctx._realm_id,
            &admin_token,
            role_name,
            "Role with roles:view only",
        )
        .await;

        // 撤销 realm-admin 角色
        sqlx::query("DELETE FROM user_roles WHERE user_id = $1::uuid")
            .bind(&user_id_str)
            .execute(&ctx._app_state.pool)
            .await
            .expect("Failed to revoke realm-admin role");

        // 清除缓存
        let _ = ctx
            ._app_state
            .permission_checker
            .invalidate_user_role_cache(&ctx._realm_id, &user_id_str)
            .await;

        // 为角色分配 roles:view 权限（没有 roles.manage）
        sqlx::query(
            "INSERT INTO role_policies (id, realm_id, role_id, resource, action, created_at)
             VALUES ($1, $2, $3, $4, $5, NOW())",
        )
        .bind(uuid::Uuid::now_v7())
        .bind(&ctx._realm_id)
        .bind(role_id)
        .bind("roles")
        .bind("view")
        .execute(&ctx._app_state.pool)
        .await
        .expect("Failed to assign permission to role");

        // 为用户分配角色
        assign_role_to_user(
            ctx,
            &ctx._realm_id,
            &admin_token,
            uuid::Uuid::parse_str(&user_id_str).unwrap(),
            role_id,
        )
        .await;

        // 清除权限缓存
        let _ = ctx
            ._app_state
            .permission_checker
            .invalidate_user_role_cache(&ctx._realm_id, &user_id_str)
            .await;

        // 创建测试角色定义
        let _app = ctx.create_unified_test_router();

        // 临时授予 roles.manage 权限来创建测试数据
        let _ = sqlx::query(
            "INSERT INTO role_policies (id, realm_id, role_id, resource, action, created_at)
             VALUES ($1, $2, $3, $4, $5, NOW())",
        )
        .bind(uuid::Uuid::now_v7())
        .bind(&ctx._realm_id)
        .bind(role_id)
        .bind("roles")
        .bind("manage")
        .execute(&ctx._app_state.pool)
        .await;

        let test_role_id = create_role(
            ctx,
            &ctx._realm_id,
            &admin_token,
            "test-role-to-delete",
            "Test role to delete",
        )
        .await;

        // 撤销 roles.manage 权限
        sqlx::query("DELETE FROM role_policies WHERE role_id = $1 AND resource = 'roles' AND action = 'manage'")
            .bind(role_id)
            .execute(&ctx._app_state.pool)
            .await
            .expect("Failed to revoke permission");

        // 清除角色策略缓存
        let _ = ctx
            ._app_state
            .permission_checker
            .invalidate_role_policy_cache(&ctx._realm_id, &role_id.to_string())
            .await;

        // 清除用户权限缓存
        let _ = ctx
            ._app_state
            .permission_checker
            .invalidate_user_role_cache(&ctx._realm_id, &user_id_str)
            .await;

        // ========================================================================
        // When: 尝试删除角色定义
        // ========================================================================
        let app = ctx.create_unified_test_router();
        let delete_response = Request::builder()
            .method("DELETE")
            .uri(format!(
                "/api/roles/{}/define/{}",
                ctx._realm_id, test_role_id
            ))
            .header("cookie", format!("X-Auth={}", admin_token))
            .body(Body::empty())
            .unwrap();

        let delete_response = app.oneshot(delete_response).await.unwrap();

        // ========================================================================
        // Then: 验证返回 403 Forbidden
        // ========================================================================
        assert_eq!(
            delete_response.status(),
            StatusCode::FORBIDDEN,
            "Delete should be forbidden without roles.manage"
        );

        let error_body: serde_json::Value = crate::tests::response_json(delete_response).await;
        let error_message = error_body["message"].as_str().unwrap();

        assert!(
            error_message.contains("roles") && error_message.contains("manage"),
            "Error message should mention missing roles.manage permission, got: {}",
            error_message
        );

        tracing::info!("✓ Delete role correctly requires roles.manage");
    }
}
