use axum::extract::{Request, State};
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
pub struct UserProfileRolesResponse {
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

/// 获取当前用户角色信息和权限
///
/// 返回当前登录用户的所有角色（role names）和权限（permission names）。
/// 同时检查用户是否具有内置角色（realm-admin）。
#[utoipa::path(
    get,
    path = "/api/user/roles",
    tag = "user",
    responses(
        (status = 200, description = "User roles and permissions retrieved", body = UserProfileRolesResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_user_roles(
    State(state): State<AppState>,
    req: Request,
) -> Result<ApiResult<UserProfileRolesResponse>, ApiError> {
    let headers = req.headers().clone();
    let (_token, sess) = require_session(&state, &headers).await?;

    let permission_checker = &state.permission_checker;
    let role_repo = PostgresRoleRepository::new(state.db.clone());
    let permission_repo = PostgresPermissionRepository::new(state.db.clone());

    // Find realm-admin role
    let realm_admin_role_id = match role_repo
        .find_by_name(BUILTIN_ROLE_REALM_ADMIN, &sess.realm_id, &sess.client_id)
        .await
    {
        Ok(role) => Some(role.id.to_string()),
        Err(_) => None,
    };

    // Get user's roles
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

    let has_realm_admin_role = if let Some(ref role_id) = realm_admin_role_id {
        role_ids.contains(role_id)
    } else {
        false
    };

    // Get role names
    let role_names = if !role_ids.is_empty() {
        let role_uuids: Vec<Uuid> = role_ids
            .iter()
            .filter_map(|id| Uuid::parse_str(id).ok())
            .collect();

        if role_uuids.is_empty() {
            Vec::new()
        } else {
            match role_repo.find_by_ids(role_uuids).await {
                Ok(roles) => roles.into_iter().map(|r| r.name).collect(),
                Err(e) => {
                    tracing::error!("Failed to fetch role details: {}", e);
                    Vec::new()
                }
            }
        }
    } else {
        Vec::new()
    };

    // Get permissions
    let permissions = if has_realm_admin_role {
        // realm-admin: return all permissions
        permission_repo
            .list_permissions(&sess.realm_id)
            .await
            .map_err(|e| {
                tracing::error!(
                    error = %e,
                    user_id = %sess.user_id,
                    sess.realm_id = %sess.realm_id,
                    "Failed to fetch all permissions for realm-admin"
                );
                ApiError::internal("Failed to fetch permissions")
            })?
            .into_iter()
            .map(|p| p.name)
            .collect()
    } else {
        // Regular user: use permission_checker
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
                ApiError::internal("Failed to fetch user permissions")
            })?
    };

    Ok(ApiResult::ok(UserProfileRolesResponse {
        roles: role_names,
        permissions,
    }))
}
