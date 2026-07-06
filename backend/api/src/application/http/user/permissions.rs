use axum::extract::{Extension, State};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub use crate::application::http::server::api_entities::ErrorResponse;
use crate::application::http::server::api_entities::{ApiError, ApiResult};
use crate::application::http::state::AppState;
use herald_api_base::application::http::common::auth_utils::SelfIdentity;
use herald_core::domain::authentication::Identity;
use herald_core::domain::authorization::permission_service::PermissionService;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UserPermissionsResponse {
    pub permissions: Vec<String>, // 权限名称列表，如 ["users.manage", "profile.view"]
    pub has_all: bool,            // 兼容旧字段；当前用户接口不表达后台全量权限
}

/// 获取用户权限列表
///
/// 返回当前用户拥有的所有权限名称列表，用于前端菜单权限控制。
///
/// **has_all 字段说明**：
/// 兼容旧响应字段，当前用户接口不再展开 realm 全量权限定义，固定为 false。
#[utoipa::path(
    get,
    path = "/api/user/permissions",
    tag = "user",
    operation_id = "get_current_user_permissions",
    responses(
        (status = 200, description = "User permissions", body = UserPermissionsResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_user_permissions(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
) -> Result<ApiResult<UserPermissionsResponse>, ApiError> {
    let self_identity = SelfIdentity::require(identity)?;
    let realm_id = self_identity.realm_id();
    let user_id = self_identity.user_id_string();

    let permission_checker = &state.permission_checker;

    let permissions = permission_checker
        .get_user_permissions(&realm_id, &user_id)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                user_id = %user_id,
                realm_id = %realm_id,
                "Failed to fetch user permissions"
            );
            ApiError::internal("Failed to fetch permissions")
        })?;

    Ok(ApiResult::ok(UserPermissionsResponse {
        permissions,
        has_all: false,
    }))
}
