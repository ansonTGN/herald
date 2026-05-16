// =============================================================================
// 通用认证辅助函数
// =============================================================================
//
// ## 权限授予方式（基于 RBAC 角色）
//
// - `grant_realm_admin_role` - 授予 Realm Admin 角色（获得所有标准权限）
//
// ## ⚠️ 重要：Realm 严格隔离
//
// 根据最新架构设计（`.ai/design/cas-permission-architecture-refactor.md`）：
// - **没有跨 Realm 访问**：所有用户只能访问自己所属的 Realm
// - **admin realm 是普通 realm**：只有 `realm.create` 权限可以创建新 Realm
// - **权限验证在 Service 层**：不在 HTTP middleware
//
// ## 参考
// - 权限标准文档: `docs/permission-standard.md`
// - RBAC 产品文档: `docs/permission.md`
// - 架构设计: `.ai/design/cas-permission-architecture-refactor.md`
// - RBAC 初始化: `core/src/domain/rbac_init/services.rs`

#![allow(dead_code)]

use crate::schema_test_context::SchemaTestContext as TestContext;
use herald_api::application::http::auth::util::{SessionData, store_session};
use herald_core::domain::authorization::permission_service::PermissionService;

/// ============================================================================
/// 会话管理
/// ============================================================================
///
/// 创建管理员会话并在数据库中创建对应的测试用户
///
/// # Arguments
/// * `ctx` - 测试上下文
/// * `email` - 用户邮箱（用作标识）
/// * `ttl_seconds` - 会话过期时间（秒）
///
/// # 返回
/// 返回 (token, user_id) 元组，其中 user_id 是有效的 UUID
///
/// # 注意
/// 此函数会在数据库中创建一个测试用户，适用于需要用户身份验证的路由。
pub async fn create_admin_session_with_user(
    ctx: &TestContext,
    email: &str,
    ttl_seconds: usize,
) -> (String, String) {
    use uuid::Uuid;

    // 检查用户是否已存在
    let existing_user: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM account WHERE email = $1 AND realm_id = $2")
            .bind(email)
            .bind(&ctx._realm_id)
            .fetch_optional(&ctx.app_state.pool)
            .await
            .unwrap();

    let user_uuid = if let Some(uuid) = existing_user {
        uuid
    } else {
        // 创建新用户
        let new_user_uuid = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO account (id, realm_id, email, password, status) VALUES ($1, $2, $3, $4, 1)"
        )
        .bind(new_user_uuid)
        .bind(&ctx._realm_id)
        .bind(email)
        .bind("$2a$12$dummy_password_hash") // 假密码哈希
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();
        new_user_uuid
    };

    let user_id_str = user_uuid.to_string();
    let token = ctx.generate_test_token();
    let session_state = SessionData {
        realm_id: ctx._realm_id.clone(),
        client_id: ctx._client_id.clone(), // TEXT identifier
        user_id: user_id_str.clone(),
        client_ip: "127.0.0.1".to_string(),
        renewal_ttl_seconds: None,
    };
    store_session(&ctx.app_state, &token, &session_state, ttl_seconds)
        .await
        .unwrap();

    (token, user_id_str)
}

/// ============================================================================
/// 基于 RBAC 角色系统的权限授予
/// ============================================================================
///
/// 创建管理员会话（简化版）
///
/// **返回**: token (String)
///
pub async fn create_admin_session(ctx: &TestContext, email: &str, ttl_seconds: usize) -> String {
    let (token, _user_id) = create_admin_session_with_user(ctx, email, ttl_seconds).await;
    token
}

/// 授予 Realm Admin 角色（通过 user_roles 表）
///
/// 将用户加入 realm-admin 角色，自动获得标准权限：
/// - realm.view, realm.admin
/// - users.view, users.manage
/// - clients.view, clients.manage
/// - roles.view, roles.manage
/// - permissions.view, permissions.manage
/// - policies.view, policies.manage
/// - settings.view, settings.manage
/// - billing.view, billing.manage
///
/// **特殊权限（仅 admin realm）**：
/// - realm.create (仅当 realm_id == "admin" 时授予)
///
/// **参数**:
/// - `ctx`: 测试上下文
/// - `user_id`: 用户 ID
///
/// **参考**:
/// - RBAC 初始化: `core/src/domain/rbac_init/services.rs` 第 75-265 行
/// - 权限标准文档: `docs/permission-standard.md` 第 52-66 行
///
/// **示例**:
/// ```no_run
/// # use crate::tests::helpers::auth_helpers::grant_realm_admin_role;
/// # async fn test(ctx: &mut TestContext) {
/// let (token, user_id) = create_admin_session_with_user(ctx, "admin@test.com", 1800).await;
/// grant_realm_admin_role(ctx, &user_id).await;
/// // 用户现在拥有 Realm Admin 的所有权限
/// // （如果是 admin realm，还包括创建 Realm 的权限）
/// # }
/// ```
pub async fn grant_realm_admin_role(ctx: &TestContext, user_id: &str) {
    // 检查 realm-admin 角色是否存在，不存在则创建
    let realm_admin_role_id: String = match sqlx::query_scalar(
        "SELECT id::text FROM roles WHERE realm_id = $1 AND name = 'realm-admin'",
    )
    .bind(&ctx._realm_id)
    .fetch_optional(&ctx.app_state.pool)
    .await
    .unwrap()
    {
        Some(id) => {
            tracing::debug!(realm_id = %ctx._realm_id, "realm-admin role exists: {}", id);
            id
        }
        None => {
            tracing::debug!(realm_id = %ctx._realm_id, "realm-admin role not found, creating it");
            // 创建 realm-admin 角色
            let role_uuid = uuid::Uuid::now_v7();
            sqlx::query(
                "INSERT INTO roles (id, name, description, realm_id, client_id, is_builtin)
                 VALUES ($1, 'realm-admin', 'Realm Administrator with full access to this realm', $2, $3, false)"
            )
            .bind(role_uuid)
            .bind(&ctx._realm_id)
            .bind(&ctx._client_id)
            .execute(&ctx.app_state.pool)
            .await
            .expect("Failed to create realm-admin role");
            let role_id = role_uuid.to_string();

            // 为 realm-admin 角色添加标准权限（统一 action: view, manage, create）
            // 注意：只有 admin realm 才有 realm.create 权限
            let mut permissions = vec![
                ("realm", "view"),
                ("realm", "admin"),
                ("users", "view"),
                ("users", "manage"),
                ("clients", "view"),
                ("clients", "manage"),
                ("roles", "view"),
                ("roles", "manage"),
                ("permissions", "view"),
                ("permissions", "manage"),
                ("policies", "view"),
                ("policies", "manage"),
                ("settings", "view"),
                ("settings", "manage"),
                ("billing", "view"),
                ("billing", "manage"),
            ];

            // 只有 admin realm 才有创建新 Realm 的权限
            if ctx._realm_id == "admin" {
                permissions.push(("realm", "create"));
            }

            // 为每个权限添加策略到 role_policies 表
            for (resource, action) in &permissions {
                let policy_id = uuid::Uuid::now_v7();
                let role_uuid =
                    uuid::Uuid::parse_str(&role_id).expect("Failed to parse role_id as UUID");
                sqlx::query(
                    "INSERT INTO role_policies (id, role_id, realm_id, resource, action)
                     VALUES ($1, $2, $3, $4, $5)",
                )
                .bind(policy_id)
                .bind(role_uuid)
                .bind(&ctx._realm_id)
                .bind(resource)
                .bind(action)
                .execute(&ctx.app_state.pool)
                .await
                .expect("Failed to add policy for realm-admin role");
            }

            let permission_count = if ctx._realm_id == "admin" { 15 } else { 14 };
            tracing::debug!(
                realm_id = %ctx._realm_id,
                role_id = %role_id,
                "Created realm-admin role with {} standard permissions{}",
                permission_count,
                if ctx._realm_id == "admin" { " (including realms.create)" } else { "" }
            );

            role_id
        }
    };

    // 通过 user_roles 表将用户加入角色
    let user_role_id = uuid::Uuid::now_v7();
    let user_uuid = uuid::Uuid::parse_str(user_id).expect("Failed to parse user_id as UUID");
    let role_uuid =
        uuid::Uuid::parse_str(&realm_admin_role_id).expect("Failed to parse role_id as UUID");
    sqlx::query(
        "INSERT INTO user_roles (id, user_id, role_id, realm_id, client_id)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (user_id, role_id, realm_id) DO NOTHING",
    )
    .bind(user_role_id)
    .bind(user_uuid)
    .bind(role_uuid)
    .bind(&ctx._realm_id)
    .bind(&ctx._client_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to add user to realm-admin role");

    // Invalidate cache
    let _ = ctx
        .app_state
        .permission_checker
        .invalidate_user_role_cache(&ctx._realm_id, user_id)
        .await;

    tracing::debug!(
        user_id = %user_id,
        realm_id = %ctx._realm_id,
        role_id = %realm_admin_role_id,
        "Granted realm-admin role to user"
    );
}
