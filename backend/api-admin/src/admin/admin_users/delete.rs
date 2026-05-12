use crate::admin::admin_users::types::ErrorResponse;
use axum::{
    Extension,
    extract::{Path, State},
    http::HeaderMap,
};
use herald_api_base::application::http::server::api_entities::{ApiError, ApiResult};
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::user::AdminUserService;
use herald_core::domain::user::admin_errors::UserAdminError;
use uuid::Uuid;

/// Delete user
#[utoipa::path(
    delete,
    path = "/api/users/{realmId}/{userId}",
    tag = "users",
    summary = "Delete a user",
    description = "Delete a user from the realm. Requires `users.manage` permission.",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("userId" = Uuid, Path, description = "User ID")
    ),
    responses(
        (status = 204, description = "User deleted"),
        (status = 403, description = "Forbidden - Insufficient permissions (requires users.manage) or realm boundary violation", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_user(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, target_user_id)): Path<(String, Uuid)>,
    _headers: HeaderMap,
) -> Result<ApiResult<()>, ApiError> {
    tracing::info!(
        realm_id = %realm_id,
        user_id = %target_user_id,
        "Deleting user"
    );

    // Get admin_user_service
    let admin_user_service = &state.admin_user_service;

    // Call service layer
    admin_user_service
        .delete_user(identity, &realm_id, target_user_id)
        .await
        .map_err(|e| match e {
            UserAdminError::PermissionDenied(msg) => {
                tracing::warn!(
                    realm_id = %realm_id,
                    user_id = %target_user_id,
                    error = %msg,
                    "Permission denied for user deletion"
                );
                ApiError::forbidden(msg)
            }
            UserAdminError::UserNotFound(id) => {
                tracing::debug!(
                    realm_id = %realm_id,
                    user_id = %target_user_id,
                    "User deletion failed: user not found"
                );
                ApiError::not_found(format!("User not found: {}", id))
            }
            UserAdminError::DatabaseError(msg) => {
                tracing::error!(
                    realm_id = %realm_id,
                    user_id = %target_user_id,
                    error = %msg,
                    "Failed to delete user"
                );
                ApiError::internal(format!("Database error: {}", msg))
            }
            UserAdminError::InternalError(msg) => {
                tracing::error!(
                    realm_id = %realm_id,
                    user_id = %target_user_id,
                    error = %msg,
                    "Failed to delete user"
                );
                ApiError::internal(msg)
            }
            _ => {
                tracing::error!(
                    realm_id = %realm_id,
                    user_id = %target_user_id,
                    "Unexpected error during user deletion"
                );
                ApiError::internal("Unexpected error")
            }
        })?;

    tracing::info!(
        user_id = %target_user_id,
        realm_id = %realm_id,
        "User deleted successfully"
    );

    Ok(ApiResult::no_content())
}
