use axum::extract::State;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::application::http::auth::util::require_session;
pub use crate::application::http::server::api_entities::ErrorResponse;
use crate::application::http::server::api_entities::{ApiError, ApiResult};
use crate::application::http::state::AppState;
use herald_core::admin::user::BUILTIN_ROLE_REALM_ADMIN;
use herald_core::domain::authorization::{
    PermissionRepository, RoleRepository, permission_service::PermissionService,
};
use herald_core::infrastructure::authorization::{
    PostgresPermissionRepository, PostgresRoleRepository,
};

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UserPermissionsResponse {
    pub permissions: Vec<String>, // 权限名称列表，如 ["users.manage", "profile.view"]
    pub has_all: bool,            // 是否拥有所有权限（realm-admin 角色在当前 realm）
}

/// 获取用户权限列表
///
/// 返回当前用户拥有的所有权限名称列表，用于前端菜单权限控制。
///
/// **has_all 字段说明**：
/// - Realm-admin: 在当前 realm 拥有 realm-admin 角色，has_all 为 true
/// - 普通用户: has_all 为 false，只返回其角色的权限
///
/// 当 has_all 为 true 时，返回该 realm 的所有权限名称。
#[utoipa::path(
    get,
    path = "/api/user/permissions",
    tag = "user",
    responses(
        (status = 200, description = "User permissions", body = UserPermissionsResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_user_permissions(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<ApiResult<UserPermissionsResponse>, ApiError> {
    let (_token, sess) = require_session(&state, &headers).await?;

    let permission_checker = &state.permission_checker;

    // Get user roles
    let role_ids = permission_checker
        .get_user_roles(&sess.realm_id, &sess.user_id)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                user_id = %sess.user_id,
                sess.realm_id = %sess.realm_id,
                "Failed to fetch user roles"
            );
            ApiError::internal("Internal server error")
        })?;

    tracing::debug!(
        role_ids = ?role_ids,
        "Found roles for user"
    );

    // Check if user has realm-admin role
    let has_all = if !role_ids.is_empty() {
        let role_repo = PostgresRoleRepository::new(state.db.clone());
        let mut has_realm_admin = false;

        for role_id_str in &role_ids {
            if let Ok(role_id) = Uuid::parse_str(role_id_str)
                && let Ok(role) = role_repo.get_role_by_id(role_id).await
                && role.name == BUILTIN_ROLE_REALM_ADMIN
                && role.realm_id == sess.realm_id
            {
                has_realm_admin = true;
                break;
            }
        }
        has_realm_admin
    } else {
        false
    };

    // Get permissions
    let permissions = if has_all {
        // realm-admin: return all permissions for this realm
        let permission_repo = PostgresPermissionRepository::new(state.db.clone());
        permission_repo
            .list_permissions(&sess.realm_id)
            .await
            .map_err(|e| {
                tracing::error!(
                    error = %e,
                    user_id = %sess.user_id,
                    sess.realm_id = %sess.realm_id,
                    "Failed to fetch all permissions for admin"
                );
                ApiError::internal("Failed to fetch permissions")
            })?
            .into_iter()
            .map(|p| p.name)
            .collect()
    } else {
        // Regular user: use permission_checker to get user's effective permissions
        permission_checker
            .get_user_permissions(&sess.realm_id, &sess.user_id)
            .await
            .map_err(|e| {
                tracing::error!(
                    error = %e,
                    user_id = %sess.user_id,
                    sess.realm_id = %sess.realm_id,
                    "Failed to fetch user permissions"
                );
                ApiError::internal("Failed to fetch permissions")
            })?
    };

    Ok(ApiResult::ok(UserPermissionsResponse {
        permissions,
        has_all,
    }))
}
