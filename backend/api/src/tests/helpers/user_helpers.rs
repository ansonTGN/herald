// =============================================================================
// 通用用户管理辅助函数
// =============================================================================

#![allow(dead_code)]

use crate::tests::schema_test_context::SchemaTestContext as TestContext;

/// ============================================================================
/// 用户创建和管理
/// ============================================================================
///
/// 创建测试用户（直接在数据库中创建，使用假密码哈希）
///
/// **返回**: user_id (Uuid)
///
pub async fn create_simple_test_user(ctx: &TestContext, email: &str) -> uuid::Uuid {
    let user_uuid = uuid::Uuid::now_v7();
    sqlx::query(
        "INSERT INTO account (id, realm_id, email, password, status) VALUES ($1, $2, $3, $4, 1)",
    )
    .bind(user_uuid)
    .bind(&ctx._realm_id)
    .bind(email)
    .bind("$2a$12$dummy_password_hash") // 假密码哈希
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create test user");
    user_uuid
}
