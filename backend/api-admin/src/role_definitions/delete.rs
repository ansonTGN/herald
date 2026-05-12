use crate::role_definitions::types::ErrorResponse;
use axum::{
    Extension,
    extract::{Path, State},
};
use herald_api_base::application::http::server::api_entities::{ApiError, ApiResult};
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::authorization::PermissionService;
use uuid::Uuid;

/// Delete role
#[utoipa::path(
    delete,
    path = "/api/roles/{realmId}/define/{roleId}",
    tag = "role-definitions",
    summary = "Delete a role",
    description = "Delete a role definition. Built-in roles cannot be deleted. Requires `roles.manage` permission.",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("roleId" = Uuid, Path, description = "Role ID")
    ),
    responses(
        (status = 204, description = "Role deleted"),
        (status = 403, description = "Forbidden - Insufficient permissions (requires roles.manage) or attempting to delete built-in role", body = ErrorResponse),
        (status = 404, description = "Role not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_role(
    State(state): State<AppState>,
    Path((realm_id, id)): Path<(String, Uuid)>,
    Extension(identity): Extension<Identity>,
) -> Result<ApiResult<()>, ApiError> {
    // Check permission: requires roles.manage
    let current_user_id = identity.user_id();
    let identity_realm_id = identity.realm_id();

    // Realm boundary check
    if identity_realm_id != realm_id {
        return Err(ApiError::forbidden(
            "Access denied: cannot manage roles in a different realm",
        ));
    }

    let has_permission = state
        .permission_checker
        .check_permission(&realm_id, &current_user_id, "roles", "manage")
        .await
        .map_err(|e| {
            tracing::error!(
                current_user_id = %current_user_id,
                realm_id = %realm_id,
                error = %e,
                "Failed to check roles.manage permission"
            );
            ApiError::internal("Failed to check permission")
        })?;

    if !has_permission {
        return Err(ApiError::forbidden(
            "Insufficient permissions: requires roles.manage",
        ));
    }

    // 3. Check if role is built-in
    let role: Option<(bool, String)> =
        sqlx::query_as("SELECT is_builtin, name FROM roles WHERE id = $1 AND realm_id = $2")
            .bind(id)
            .bind(&realm_id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to check role: {e}");
                ApiError::internal("Failed to check role")
            })?;

    match role {
        Some((is_builtin, role_name)) => {
            if is_builtin {
                tracing::warn!(
                    user_id = %identity.user_id(),
                    role_id = %id,
                    role_name = %role_name,
                    "Attempted to delete built-in role"
                );
                return Err(ApiError::forbidden("Cannot delete built-in role"));
            }
        }
        None => {
            return Err(ApiError::not_found("Role not found"));
        }
    }

    let role_in_use: Option<(bool,)> = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM user_roles WHERE role_id = $1 AND realm_id = $2)",
    )
    .bind(id)
    .bind(&realm_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to check role usage: {e}");
        ApiError::internal("Failed to check role usage")
    })?;

    if matches!(role_in_use, Some((true,))) {
        return Err(ApiError::conflict(
            "Cannot delete role that is still assigned to users",
        ));
    }

    // 4. Execute deletion
    let result = sqlx::query("DELETE FROM roles WHERE id = $1 AND realm_id = $2")
        .bind(id)
        .bind(&realm_id)
        .execute(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete role: {e}");
            ApiError::internal("Failed to delete role")
        })?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Role not found"));
    }

    Ok(ApiResult::no_content())
}
