// =============================================================================
// 测试新的 RBAC 角色辅助函数
// =============================================================================

use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use test_context::test_context;

/// 验证 grant_realm_admin_role 辅助函数
///
/// 这个测试验证：
/// 1. realm-admin 角色存在于数据库中
/// 2. grant_realm_admin_role 成功将用户加入角色
/// 3. 用户通过 user_roles 表与角色关联
#[test_context(TestContext)]
#[tokio::test]
async fn test_grant_realm_admin_role_helper(ctx: &mut TestContext) {
    use crate::tests::helpers::auth_helpers::*;

    // 1. 创建测试用户
    let user_id = uuid::Uuid::now_v7();
    let email = format!("test-admin-{}@example.com", user_id);
    sqlx::query(
        "INSERT INTO account (id, realm_id, email, password, status) VALUES ($1, $2, $3, $4, 1)",
    )
    .bind(user_id)
    .bind(&ctx._realm_id)
    .bind(&email)
    .bind("$2a$12$dummy_password_hash")
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create test user");

    // 2. 使用新辅助函数授予角色（此函数会创建 realm-admin 角色如果不存在）
    grant_realm_admin_role(ctx, &user_id.to_string()).await;

    // 3. 验证 realm-admin 角色现在存在
    let realm_admin_role: Option<String> = sqlx::query_scalar(
        "SELECT id::text FROM roles WHERE realm_id = $1 AND name = 'realm-admin'",
    )
    .bind(&ctx._realm_id)
    .fetch_optional(&ctx._app_state.pool)
    .await
    .expect("Failed to query realm-admin role");

    assert!(
        realm_admin_role.is_some(),
        "realm-admin role should exist after grant_realm_admin_role"
    );

    // 4. 验证用户已被加入角色（通过 user_roles 表）
    let user_role: Option<String> = sqlx::query_scalar(
        "SELECT role_id::text FROM user_roles WHERE user_id = $1::uuid AND realm_id = $2",
    )
    .bind(user_id)
    .bind(&ctx._realm_id)
    .fetch_optional(&ctx._app_state.pool)
    .await
    .expect("Failed to query user_roles");

    assert!(
        user_role.is_some(),
        "User should be added to realm-admin role via user_roles table"
    );
    assert_eq!(
        user_role, realm_admin_role,
        "User role should match realm-admin role"
    );

    // 清理测试数据
    sqlx::query("DELETE FROM user_roles WHERE user_id = $1::uuid")
        .bind(user_id)
        .execute(&ctx._app_state.pool)
        .await
        .expect("Failed to cleanup user_roles");

    sqlx::query("DELETE FROM account WHERE id = $1")
        .bind(user_id)
        .execute(&ctx._app_state.pool)
        .await
        .expect("Failed to cleanup test user");
}

// 注意: 以下测试已被删除，因为 grant_super_admin_role 函数已被删除
// 根据新架构设计（.ai/design/cas-permission-architecture-refactor.md）：
// - 没有跨 Realm 访问
// - Realm 严格隔离
// - 所有用户只能访问自己的 Realm
// - 所有测试应使用 grant_realm_admin_role
//
// 原测试 test_grant_super_admin_role_helper 已被移除，因为它测试的函数已不存在。
