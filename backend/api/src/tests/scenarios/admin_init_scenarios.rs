/// 场景测试：主管理员初始化
///
/// 测试空数据库启动时主管理员的创建流程
#[cfg(test)]
mod tests {
    use crate::tests::schema_test_context::SchemaTestContext;
    use herald_core::admin::user::{BUILTIN_ROLE_REALM_ADMIN, init_admin_user};
    use sqlx::Row;
    use test_context::test_context;
    use uuid::Uuid;

    use SchemaTestContext as AdminInitTestContext;

    /// 测试：空数据库自动创建主管理员
    #[test_context(AdminInitTestContext)]
    #[tokio::test]
    async fn test_empty_database_creates_admin_user(ctx: &mut AdminInitTestContext) {
        // SchemaTestContext 已经调用了 init_admin_user
        // 所以这里验证管理员用户已创建

        // 1. 验证主管理员账户已创建
        let count: i64 = sqlx::query("SELECT count(*) as count FROM account")
            .fetch_one(&ctx.app_state.pool)
            .await
            .unwrap()
            .get("count");
        assert_eq!(count, 1, "Should have exactly 1 admin user");

        // 2. 验证邮箱正确
        let email: String = sqlx::query("SELECT email FROM account LIMIT 1")
            .fetch_one(&ctx.app_state.pool)
            .await
            .unwrap()
            .get("email");
        assert_eq!(email, "admin@cas.com");

        // 3. 验证 profile 已创建
        let nickname: String = sqlx::query("SELECT nickname FROM profile LIMIT 1")
            .fetch_one(&ctx.app_state.pool)
            .await
            .unwrap()
            .get("nickname");
        assert_eq!(nickname, "Admin");

        // 4. 验证 super-admin 角色已创建
        let role_id: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM roles WHERE name = $1 AND realm_id = $2")
                .bind(BUILTIN_ROLE_REALM_ADMIN)
                .bind(&ctx._realm_id)
                .fetch_optional(&ctx.app_state.pool)
                .await
                .unwrap();

        assert!(
            role_id.is_some(),
            "super-admin role should exist after initialization"
        );

        // 5. 验证角色关联到正确的 client_app.client_id
        // 注意：roles.client_id 存储的是 client_app.client_id (字符串标识符)
        let client_app_client_id: String = sqlx::query("SELECT client_id FROM client_app LIMIT 1")
            .fetch_one(&ctx.app_state.pool)
            .await
            .unwrap()
            .get("client_id");

        let role_client_id: String = sqlx::query("SELECT client_id FROM roles WHERE name = $1")
            .bind(BUILTIN_ROLE_REALM_ADMIN)
            .fetch_one(&ctx.app_state.pool)
            .await
            .unwrap()
            .get("client_id");

        assert_eq!(
            role_client_id, client_app_client_id,
            "Role client_id should match client_app.client_id (string identifier)"
        );
    }
    /// 测试：重复运行不重复创建主管理员
    #[test_context(AdminInitTestContext)]
    #[tokio::test]
    async fn test_rerunning_init_does_not_duplicate_admin(ctx: &mut AdminInitTestContext) {
        // SchemaTestContext 已经调用了一次 init_admin_user

        // 获取初始状态
        let initial_count: i64 = sqlx::query("SELECT count(*) as count FROM account")
            .fetch_one(&ctx.app_state.pool)
            .await
            .unwrap()
            .get("count");

        let initial_role_count: i64 = sqlx::query("SELECT count(*) as count FROM roles")
            .fetch_one(&ctx.app_state.pool)
            .await
            .unwrap()
            .get("count");

        let initial_policy_count: i64 = sqlx::query("SELECT count(*) as count FROM role_policies")
            .fetch_one(&ctx.app_state.pool)
            .await
            .unwrap()
            .get("count");
        let initial_grouping_count: i64 = sqlx::query("SELECT count(*) as count FROM user_roles")
            .fetch_one(&ctx.app_state.pool)
            .await
            .unwrap()
            .get("count");

        // 再次运行 init_admin_user
        init_admin_user(&ctx.app_state.pool, "test")
            .await
            .expect("Second init should succeed");

        // 验证没有重复创建
        let final_count: i64 = sqlx::query("SELECT count(*) as count FROM account")
            .fetch_one(&ctx.app_state.pool)
            .await
            .unwrap()
            .get("count");

        assert_eq!(
            initial_count, final_count,
            "Should not create duplicate admin users"
        );

        // 验证角色没有重复
        let final_role_count: i64 = sqlx::query("SELECT count(*) as count FROM roles")
            .fetch_one(&ctx.app_state.pool)
            .await
            .unwrap()
            .get("count");

        assert_eq!(
            initial_role_count, final_role_count,
            "Should not create duplicate roles"
        );

        // 验证策略没有重复
        let final_policy_count: i64 = sqlx::query("SELECT count(*) as count FROM role_policies")
            .fetch_one(&ctx.app_state.pool)
            .await
            .unwrap()
            .get("count");
        let final_grouping_count: i64 = sqlx::query("SELECT count(*) as count FROM user_roles")
            .fetch_one(&ctx.app_state.pool)
            .await
            .unwrap()
            .get("count");

        assert_eq!(
            initial_policy_count, final_policy_count,
            "Should not create duplicate policies"
        );

        assert_eq!(
            initial_grouping_count, final_grouping_count,
            "Should not create duplicate grouping policies"
        );
    }

    /// 测试：init_admin_user 在已有用户时不执行
    #[test_context(AdminInitTestContext)]
    #[tokio::test]
    async fn test_init_with_existing_users_does_nothing(ctx: &mut AdminInitTestContext) {
        // SchemaTestContext 已经创建了管理员用户

        // 创建另一个普通用户
        let realm_id: String = sqlx::query("SELECT id FROM realm LIMIT 1")
            .fetch_one(&ctx.app_state.pool)
            .await
            .unwrap()
            .get("id");

        let hashed_password = bcrypt::hash("testpassword", 10).unwrap();

        sqlx::query(
            "INSERT INTO account (realm_id, email, password, status) VALUES ($1, $2, $3, 1)",
        )
        .bind(&realm_id)
        .bind("test@cas.com")
        .bind(&hashed_password)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // 获取当前状态
        let user_count_before: i64 = sqlx::query("SELECT count(*) as count FROM account")
            .fetch_one(&ctx.app_state.pool)
            .await
            .unwrap()
            .get("count");

        // 再次运行 init_admin_user
        init_admin_user(&ctx.app_state.pool, "test")
            .await
            .expect("Init should succeed when users already exist");

        // 验证用户数量没有变化
        let user_count_after: i64 = sqlx::query("SELECT count(*) as count FROM account")
            .fetch_one(&ctx.app_state.pool)
            .await
            .unwrap()
            .get("count");

        assert_eq!(
            user_count_before, user_count_after,
            "Should not create new admin users when users already exist"
        );
    }

    /// 测试：验证角色 client_id 更新逻辑
    #[test_context(AdminInitTestContext)]
    #[tokio::test]
    async fn test_role_client_id_updates_to_match_client_app(ctx: &mut AdminInitTestContext) {
        // 获取 client_app 的 client_id (字符串标识符)
        // 注意：roles.client_id 存储的是 client_app.client_id (字符串标识符)
        let client_app_client_id: String = sqlx::query("SELECT client_id FROM client_app LIMIT 1")
            .fetch_one(&ctx.app_state.pool)
            .await
            .unwrap()
            .get("client_id");

        // 验证所有内置角色的 client_id 与 client_app.client_id 匹配
        let role_client_ids: Vec<(String, String)> =
            sqlx::query_as("SELECT name, client_id FROM roles WHERE name IN ($1, $2, $3)")
                .bind(BUILTIN_ROLE_REALM_ADMIN)
                .bind(herald_core::admin::user::BUILTIN_ROLE_REALM_ADMIN)
                .bind(herald_core::admin::user::BUILTIN_ROLE_USER)
                .fetch_all(&ctx.app_state.pool)
                .await
                .unwrap();

        for (role_name, role_client_id) in role_client_ids {
            assert_eq!(
                role_client_id, client_app_client_id,
                "Role '{}' should have client_id matching client_app.client_id (string identifier)",
                role_name
            );
        }
    }
    /// 测试：验证管理员用户的完整初始化（包含权限）
    #[test_context(AdminInitTestContext)]
    #[tokio::test]
    async fn test_admin_user_complete_initialization(ctx: &mut AdminInitTestContext) {
        // 1. 验证管理员用户存在
        let user_count: i64 = sqlx::query("SELECT count(*) as count FROM account")
            .fetch_one(&ctx.app_state.pool)
            .await
            .unwrap()
            .get("count");

        assert_eq!(user_count, 1, "Should have exactly 1 admin user");

        // 2. 验证 super-admin 角色存在
        // Note: realm-admin and user roles are created when a new realm is initialized,
        // not during admin user initialization
        let builtin_roles = &[BUILTIN_ROLE_REALM_ADMIN];

        for role_name in builtin_roles {
            let role_exists: bool =
                sqlx::query("SELECT EXISTS(SELECT 1 FROM roles WHERE name = $1 AND realm_id = $2)")
                    .bind(role_name)
                    .bind(&ctx._realm_id)
                    .fetch_one(&ctx.app_state.pool)
                    .await
                    .unwrap()
                    .get("exists");

            assert!(
                role_exists,
                "Built-in role '{}' should exist after initialization",
                role_name
            );
        }

        // 3. 验证管理员用户被分配到 super-admin 角色
        let user_id: Uuid = sqlx::query_scalar("SELECT id FROM account LIMIT 1")
            .fetch_one(&ctx.app_state.pool)
            .await
            .unwrap();

        let role_id: Uuid =
            sqlx::query_scalar("SELECT id FROM roles WHERE name = $1 AND realm_id = $2")
                .bind(BUILTIN_ROLE_REALM_ADMIN)
                .bind(&ctx._realm_id)
                .fetch_one(&ctx.app_state.pool)
                .await
                .unwrap();

        // 3. 验证管理员用户被分配到 super-admin 角色（通过 user_roles 表）
        let user_role_exists: bool = sqlx::query(
            "SELECT EXISTS(SELECT 1 FROM user_roles WHERE user_id = $1::uuid AND role_id = $2)",
        )
        .bind(user_id)
        .bind(role_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap()
        .get("exists");

        assert!(
            user_role_exists,
            "Admin user should be assigned to super-admin role"
        );

        // 4. 验证 super-admin 角色有权限策略
        let policy_count: i64 =
            sqlx::query("SELECT count(*) FROM role_policies WHERE role_id = $1")
                .bind(role_id)
                .fetch_one(&ctx.app_state.pool)
                .await
                .unwrap()
                .get("count");

        assert!(
            policy_count > 0,
            "Super-admin role should have permission policies"
        );
    }
}
