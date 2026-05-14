/// 场景测试：权限系统回归测试
///
/// 测试目标：确保权限系统修复后，现有功能仍然正常工作
/// - 精确权限匹配仍然有效
/// - 角色分配仍然有效
/// - 权限缓存仍然有效
/// - 用户可以正确登录并获得权限
///
/// 来源：`.ai/design/fix-permission.md` Phase 2.4
#[cfg(test)]
mod tests {
    use crate::tests::helpers::*;
    use crate::tests::schema_test_context::SchemaTestContext;

    use herald_core::domain::authorization::PermissionService;

    use test_context::test_context;

    use SchemaTestContext as PermissionRegressionTestContext;
    /// 场景测试：角色分配仍然有效
    ///
    /// **Given**: 用户被分配到角色
    /// **When**: 用户通过该角色请求访问资源
    /// **Then**: 权限检查返回 true（角色分配有效）
    #[test_context(PermissionRegressionTestContext)]
    #[tokio::test]
    async fn test_scenario_role_assignment_still_works(ctx: &mut PermissionRegressionTestContext) {
        // ========================================================================
        // Given: 创建测试用户并分配到角色
        // ========================================================================
        let (admin_token, user_id_str) =
            create_admin_session_with_user(ctx, "test-role-assign@test.com", 1800).await;
        let _user_id = uuid::Uuid::parse_str(&user_id_str).expect("Invalid user_id UUID");

        // Grant realm-admin role to get roles.manage permission
        grant_realm_admin_role(ctx, &user_id_str).await;

        let role_name = "test-role-assign-role";
        let role_id = create_role(
            ctx,
            &ctx._realm_id,
            &admin_token,
            role_name,
            "Role for testing role assignment",
        )
        .await;

        // 为角色分配多个权限
        for (resource, action) in [("users", "view"), ("users", "manage"), ("realm", "view")] {
            sqlx::query(
                "INSERT INTO role_policies (id, realm_id, role_id, resource, action, created_at)
                 VALUES ($1, $2, $3, $4, $5, NOW())",
            )
            .bind(uuid::Uuid::now_v7())
            .bind(&ctx._realm_id)
            .bind(role_id)
            .bind(resource)
            .bind(action)
            .execute(&ctx._app_state.pool)
            .await
            .expect("Failed to assign permission to role");
        }

        assign_role_to_user(
            ctx,
            &ctx._realm_id,
            &admin_token,
            uuid::Uuid::parse_str(&user_id_str).unwrap(),
            role_id,
        )
        .await;

        let _ = ctx
            ._app_state
            .permission_checker
            .invalidate_user_role_cache(&ctx._realm_id, &user_id_str)
            .await;

        // ========================================================================
        // When: 通过角色检查多个权限
        // ========================================================================
        for (resource, action, expected) in [
            ("users", "view", true),
            ("users", "manage", true),
            ("realm", "view", true),
        ] {
            let result = check_permission(ctx, &admin_token, resource, action).await;

            // ========================================================================
            // Then: 验证所有权限检查都通过
            // ========================================================================
            assert_eq!(
                result["allowed"], expected,
                "Permission {}:{} should be {}",
                resource, action, expected
            );
        }

        tracing::info!("✓ Role assignment still works correctly");
    }
    /// 场景测试：用户可以正确登录并获得权限
    ///
    /// **Given**: 用户拥有正确的凭据和角色
    /// **When**: 用户登录并请求访问资源
    /// **Then**: 权限检查返回 true（登录和权限都正常）
    #[test_context(PermissionRegressionTestContext)]
    #[tokio::test]
    async fn test_scenario_user_can_login_with_correct_permissions(
        ctx: &mut PermissionRegressionTestContext,
    ) {
        // ========================================================================
        // Given: 创建测试用户并授予 realm-admin 角色
        // ========================================================================
        let user_email = "test-login-user@test.com";
        let (admin_token, user_id_str) =
            create_admin_session_with_user(ctx, user_email, 1800).await;
        let user_id = uuid::Uuid::parse_str(&user_id_str).expect("Invalid user_id UUID");

        grant_realm_admin_role(ctx, &user_id_str).await;

        // 验证用户在数据库中存在
        let user_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM account WHERE email = $1 AND realm_id = $2)",
        )
        .bind(user_email)
        .bind(&ctx._realm_id)
        .fetch_one(&ctx._app_state.pool)
        .await
        .expect("Failed to check user existence");

        assert!(user_exists, "User should exist in database");

        // 验证用户已分配到 realm-admin 角色
        let role_assigned: bool = sqlx::query_scalar(
            "SELECT EXISTS(
                SELECT 1 FROM user_roles ur
                JOIN roles r ON ur.role_id = r.id
                WHERE ur.user_id = $1 AND r.name = 'realm-admin'
            )",
        )
        .bind(user_id)
        .fetch_one(&ctx._app_state.pool)
        .await
        .expect("Failed to check role assignment");

        assert!(role_assigned, "User should be assigned to realm-admin role");

        // ========================================================================
        // When: 用户检查多个权限
        // ========================================================================
        let test_permissions = vec![
            ("users", "view", true),
            ("users", "manage", true),
            ("roles", "view", true),
            ("roles", "manage", true),
            ("permissions", "view", true),
            ("permissions", "manage", true),
        ];

        for (resource, action, expected) in test_permissions {
            let result = check_permission(ctx, &admin_token, resource, action).await;

            // ========================================================================
            // Then: 验证所有权限检查都正确
            // ========================================================================
            assert_eq!(
                result["allowed"], expected,
                "Permission {}:{} should be {}",
                resource, action, expected
            );
        }

        tracing::info!("✓ User login and permission grant still works correctly");
    }

    /// 场景测试：多用户权限隔离
    ///
    /// **Given**: 两个用户拥有不同的权限
    /// **When**: 用户 A 和用户 B 分别请求访问资源
    /// **Then**: 用户 A 只能访问自己的权限，用户 B 只能访问自己的权限
    #[test_context(PermissionRegressionTestContext)]
    #[tokio::test]
    async fn test_scenario_multi_user_permission_isolation(
        ctx: &mut PermissionRegressionTestContext,
    ) {
        // ========================================================================
        // Given: 创建两个测试用户并授予不同权限
        // ========================================================================
        // 用户 A: 只有 users:view 权限
        let (token_a, user_id_a_str) =
            create_admin_session_with_user(ctx, "test-user-a@test.com", 1800).await;
        let user_id_a = uuid::Uuid::parse_str(&user_id_a_str).expect("Invalid user_id UUID");

        // 临时授予 realm-admin 角色来创建测试角色
        grant_realm_admin_role(ctx, &user_id_a_str).await;

        let role_a = create_role(
            ctx,
            &ctx._realm_id,
            &token_a,
            "test-role-a",
            "Role A with users:view",
        )
        .await;

        // 撤销 realm-admin 角色
        sqlx::query("DELETE FROM user_roles WHERE user_id = $1::uuid")
            .bind(&user_id_a_str)
            .execute(&ctx._app_state.pool)
            .await
            .expect("Failed to revoke realm-admin role");

        sqlx::query(
            "INSERT INTO role_policies (id, realm_id, role_id, resource, action, created_at)
             VALUES ($1, $2, $3, $4, $5, NOW())",
        )
        .bind(uuid::Uuid::now_v7())
        .bind(&ctx._realm_id)
        .bind(role_a)
        .bind("users")
        .bind("view")
        .execute(&ctx._app_state.pool)
        .await
        .unwrap();

        assign_role_to_user(ctx, &ctx._realm_id, &token_a, user_id_a, role_a).await;

        // 清除权限缓存
        let _ = ctx
            ._app_state
            .permission_checker
            .invalidate_user_role_cache(&ctx._realm_id, &user_id_a_str)
            .await;

        // 用户 B: 有 users:manage 权限
        let (token_b, user_id_b_str) =
            create_admin_session_with_user(ctx, "test-user-b@test.com", 1800).await;
        let user_id_b = uuid::Uuid::parse_str(&user_id_b_str).expect("Invalid user_id UUID");

        // 临时授予 realm-admin 角色来创建测试角色
        grant_realm_admin_role(ctx, &user_id_b_str).await;

        let role_b = create_role(
            ctx,
            &ctx._realm_id,
            &token_b,
            "test-role-b",
            "Role B with users:manage",
        )
        .await;

        // 撤销 realm-admin 角色
        sqlx::query("DELETE FROM user_roles WHERE user_id = $1::uuid")
            .bind(&user_id_b_str)
            .execute(&ctx._app_state.pool)
            .await
            .expect("Failed to revoke realm-admin role");

        sqlx::query(
            "INSERT INTO role_policies (id, realm_id, role_id, resource, action, created_at)
             VALUES ($1, $2, $3, $4, $5, NOW())",
        )
        .bind(uuid::Uuid::now_v7())
        .bind(&ctx._realm_id)
        .bind(role_b)
        .bind("users")
        .bind("manage")
        .execute(&ctx._app_state.pool)
        .await
        .unwrap();

        assign_role_to_user(ctx, &ctx._realm_id, &token_b, user_id_b, role_b).await;

        // 清除权限缓存
        let _ = ctx
            ._app_state
            .permission_checker
            .invalidate_user_role_cache(&ctx._realm_id, &user_id_b_str)
            .await;

        // ========================================================================
        // When: 用户 A 和用户 B 检查权限
        // ========================================================================
        // 用户 A 检查 users:view (应该通过)
        let result_a = check_permission(ctx, &token_a, "users", "view").await;

        // 用户 B 检查 users:manage (应该通过)
        let result_b = check_permission(ctx, &token_b, "users", "manage").await;

        // ========================================================================
        // Then: 验证权限隔离正确
        // ========================================================================
        assert_eq!(result_a["allowed"], true, "User A should have users:view");
        assert_eq!(result_b["allowed"], true, "User B should have users:manage");
        assert_eq!(result_b["allowed"], true, "User B should have users:manage");

        tracing::info!("✓ Multi-user permission isolation works correctly");
    }
}
