/// 场景测试：统一权限层级功能验证
///
/// 测试目标：验证权限层级系统正常工作
/// - manage 权限包含 view 权限
/// - view 权限不包含 manage 权限
/// - 自定义权限（admin, create）必须精确匹配
/// - 多权限层级验证
///
/// 来源：`.ai/design/fix-permission.md` Section 5.5.2
/// 用户故事：US-RA-009 (`.ai/future/police-migrate.md`)
///
/// **测试方法**：
/// - API 测试：通过 HTTP API 验证权限
/// - Direct 测试：直接调用 PermissionChecker 验证权限
#[cfg(test)]
mod tests {
    use crate::tests::helpers::*;
    use crate::tests::schema_test_context::SchemaTestContext;
    use herald_core::domain::authorization::PermissionService;
    use test_context::test_context;

    use SchemaTestContext as PermissionHierarchyTestContext;

    // =============================================================================
    // Test Case Definitions
    // =============================================================================

    /// 权限测试用例枚举
    #[derive(Clone, Copy, Debug)]
    pub enum PermissionTestCase {
        ManageIncludesView,      // manage 包含 view
        ViewNotIncludesManage,   // view 不包含 manage
        CustomRequireExactMatch, // 自定义权限需要精确匹配
        MultiplePermissions,     // 多权限层级验证
    }

    /// 测试方法枚举
    #[derive(Clone, Copy, Debug)]
    pub enum TestMethod {
        Api,    // 通过 HTTP API 测试
        Direct, // 直接调用 PermissionChecker
    }

    // =============================================================================
    // Common Setup Helpers
    // =============================================================================

    /// 为角色分配权限
    async fn assign_permission_to_role(
        ctx: &mut PermissionHierarchyTestContext,
        role_id: uuid::Uuid,
        resource: &str,
        action: &str,
    ) {
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

    /// 清除权限缓存
    async fn clear_permission_cache(ctx: &mut PermissionHierarchyTestContext, user_id: &str) {
        let _ = ctx
            ._app_state
            .permission_checker
            .invalidate_user_role_cache(&ctx._realm_id, user_id)
            .await;
    }

    /// 设置测试环境：创建用户、角色和权限
    async fn setup_test_environment(
        ctx: &mut PermissionHierarchyTestContext,
        user_email: &str,
        role_name: &str,
        role_desc: &str,
        permissions: Vec<(&str, &str)>, // (resource, action) tuples
    ) -> (String, String, uuid::Uuid) {
        // 创建用户
        let (admin_token, user_id_str) =
            create_admin_session_with_user(ctx, user_email, 1800).await;

        // 临时授予 realm-admin 角色来创建测试角色
        grant_realm_admin_role(ctx, &user_id_str).await;

        // 创建角色
        let role_id = create_role(ctx, &ctx._realm_id, &admin_token, role_name, role_desc).await;

        // 撤销 realm-admin 角色
        sqlx::query("DELETE FROM user_roles WHERE user_id = $1::uuid")
            .bind(&user_id_str)
            .execute(&ctx._app_state.pool)
            .await
            .expect("Failed to revoke realm-admin role");

        // 清除缓存
        clear_permission_cache(ctx, &user_id_str).await;

        // 分配权限
        for (resource, action) in permissions {
            assign_permission_to_role(ctx, role_id, resource, action).await;
        }

        // 分配角色给用户
        assign_role_to_user(
            ctx,
            &ctx._realm_id,
            &admin_token,
            uuid::Uuid::parse_str(&user_id_str).unwrap(),
            role_id,
        )
        .await;

        // 清除权限缓存
        clear_permission_cache(ctx, &user_id_str).await;

        (admin_token, user_id_str, role_id)
    }

    // =============================================================================
    // Test Execution Helpers
    // =============================================================================

    /// 通过 API 检查权限
    async fn check_permission_via_api(
        ctx: &mut PermissionHierarchyTestContext,
        admin_token: &str,
        resource: &str,
        action: &str,
    ) -> bool {
        let result = check_permission(ctx, admin_token, resource, action).await;
        result["allowed"].as_bool().unwrap_or(false)
    }

    /// 直接使用 PermissionChecker 检查权限
    async fn check_permission_direct(
        ctx: &mut PermissionHierarchyTestContext,
        user_id: &str,
        resource: &str,
        action: &str,
    ) -> bool {
        let permission_checker = &ctx._app_state.permission_checker;

        let result: Result<bool, _> = permission_checker
            .check_permission(&ctx._realm_id, user_id, resource, action)
            .await;

        assert!(
            result.is_ok(),
            "Permission check should succeed: {:?}",
            result
        );
        result.unwrap()
    }

    // =============================================================================
    // Test Cases
    // =============================================================================

    /// 执行权限层级测试
    async fn execute_permission_hierarchy_test(
        ctx: &mut PermissionHierarchyTestContext,
        test_case: PermissionTestCase,
        method: TestMethod,
    ) {
        match test_case {
            PermissionTestCase::ManageIncludesView => {
                let (admin_token, user_id_str, _) = setup_test_environment(
                    ctx,
                    "test-manage-user@test.com",
                    "test-manage-role",
                    "Role with users:manage permission",
                    vec![("users", "manage")],
                )
                .await;

                let allowed = match method {
                    TestMethod::Api => {
                        check_permission_via_api(ctx, &admin_token, "users", "view").await
                    }
                    TestMethod::Direct => {
                        check_permission_direct(ctx, &user_id_str, "users", "view").await
                    }
                };

                assert!(allowed, "users:manage should grant access to users:view");
                tracing::info!(
                    "✓ Manage permission correctly includes view permission (via {:?})",
                    method
                );
            }

            PermissionTestCase::ViewNotIncludesManage => {
                let (admin_token, user_id_str, _) = setup_test_environment(
                    ctx,
                    "test-view-user@test.com",
                    "test-view-role",
                    "Role with users:view permission",
                    vec![("users", "view")],
                )
                .await;

                let allowed = match method {
                    TestMethod::Api => {
                        check_permission_via_api(ctx, &admin_token, "users", "manage").await
                    }
                    TestMethod::Direct => {
                        check_permission_direct(ctx, &user_id_str, "users", "manage").await
                    }
                };

                assert!(
                    !allowed,
                    "users:view should NOT grant access to users:manage"
                );
                tracing::info!(
                    "✓ View permission correctly does not include manage permission (via {:?})",
                    method
                );
            }

            PermissionTestCase::CustomRequireExactMatch => {
                let (admin_token, user_id_str, _) = setup_test_environment(
                    ctx,
                    "test-admin-user@test.com",
                    "test-admin-role",
                    "Role with users:admin permission",
                    vec![("users", "admin")],
                )
                .await;

                let allowed = match method {
                    TestMethod::Api => {
                        check_permission_via_api(ctx, &admin_token, "users", "view").await
                    }
                    TestMethod::Direct => {
                        check_permission_direct(ctx, &user_id_str, "users", "view").await
                    }
                };

                assert!(
                    !allowed,
                    "users:admin should NOT grant access to users:view (custom actions require exact match)"
                );
                tracing::info!(
                    "✓ Custom permission correctly requires exact match (via {:?})",
                    method
                );
            }

            PermissionTestCase::MultiplePermissions => {
                let (admin_token, user_id_str, _) = setup_test_environment(
                    ctx,
                    "test-multi-user@test.com",
                    "test-multi-role",
                    "Role with multiple permissions",
                    vec![("users", "manage"), ("realm", "view")],
                )
                .await;

                // 检查 users:view（通过 users:manage 覆盖）
                let allowed_1 = match method {
                    TestMethod::Api => {
                        check_permission_via_api(ctx, &admin_token, "users", "view").await
                    }
                    TestMethod::Direct => {
                        check_permission_direct(ctx, &user_id_str, "users", "view").await
                    }
                };
                assert!(allowed_1, "users:manage should grant access to users:view");

                // 检查 realm:view（精确匹配）
                let allowed_2 = match method {
                    TestMethod::Api => {
                        check_permission_via_api(ctx, &admin_token, "realm", "view").await
                    }
                    TestMethod::Direct => {
                        check_permission_direct(ctx, &user_id_str, "realm", "view").await
                    }
                };
                assert!(allowed_2, "realm:view should grant access to realm:view");

                tracing::info!(
                    "✓ Multiple permissions with hierarchy work correctly (via {:?})",
                    method
                );
            }
        }
    }

    // =============================================================================
    // API Tests
    // =============================================================================

    /// 场景测试：Manage 权限包含 View 权限（API）
    ///
    /// **Given**: 用户拥有 `users:manage` 权限
    /// **When**: 用户请求访问需要 `users:view` 权限的资源
    /// **Then**: 权限检查返回 true（允许访问）
    #[test_context(PermissionHierarchyTestContext)]
    #[tokio::test]
    async fn test_scenario_manage_permission_includes_view_api(
        ctx: &mut PermissionHierarchyTestContext,
    ) {
        execute_permission_hierarchy_test(
            ctx,
            PermissionTestCase::ManageIncludesView,
            TestMethod::Api,
        )
        .await;
    }

    /// 场景测试：View 权限不包含 Manage 权限（API）
    ///
    /// **Given**: 用户拥有 `users:view` 权限
    /// **When**: 用户请求访问需要 `users:manage` 权限的资源
    /// **Then**: 权限检查返回 false（拒绝访问）
    #[test_context(PermissionHierarchyTestContext)]
    #[tokio::test]
    async fn test_scenario_view_permission_does_not_include_manage_api(
        ctx: &mut PermissionHierarchyTestContext,
    ) {
        execute_permission_hierarchy_test(
            ctx,
            PermissionTestCase::ViewNotIncludesManage,
            TestMethod::Api,
        )
        .await;
    }

    /// 场景测试：自定义权限必须精确匹配（API）
    ///
    /// **Given**: 用户拥有 `users:admin` 权限（自定义权限）
    /// **When**: 用户请求访问需要 `users:view` 权限的资源
    /// **Then**: 权限检查返回 false（拒绝访问）
    #[test_context(PermissionHierarchyTestContext)]
    #[tokio::test]
    async fn test_scenario_custom_permissions_require_exact_match_api(
        ctx: &mut PermissionHierarchyTestContext,
    ) {
        execute_permission_hierarchy_test(
            ctx,
            PermissionTestCase::CustomRequireExactMatch,
            TestMethod::Api,
        )
        .await;
    }

    /// 场景测试：多权限层级验证（API）
    ///
    /// **Given**: 用户拥有 `users:manage` 和 `realm:view` 权限
    /// **When**: 用户请求访问需要 `users:view` 和 `realm:view` 权限的资源
    /// **Then**: 两个权限检查都返回 true（允许访问）
    #[test_context(PermissionHierarchyTestContext)]
    #[tokio::test]
    async fn test_scenario_multiple_permissions_with_hierarchy_api(
        ctx: &mut PermissionHierarchyTestContext,
    ) {
        execute_permission_hierarchy_test(
            ctx,
            PermissionTestCase::MultiplePermissions,
            TestMethod::Api,
        )
        .await;
    }
}
