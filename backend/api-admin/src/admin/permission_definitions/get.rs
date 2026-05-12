use axum::{
    Extension,
    extract::{Path, State},
};
use herald_core::domain::authentication::Identity;
use herald_core::domain::authorization::PermissionService;
use uuid::Uuid;

use crate::admin::permission_definitions::types::{ErrorResponse, PermissionResponse};
use herald_api_base::application::http::server::api_entities::{ApiError, ApiResult};
use herald_api_base::application::http::state::AppState;

/// Get permission by ID
#[utoipa::path(
    get,
    path = "/api/permission/{realmId}/define/{permissionDefinitionId}",
    tag = "permission-definitions",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("permissionDefinitionId" = Uuid, Path, description = "Permission ID")
    ),
    responses(
        (status = 200, description = "Permission found", body = PermissionResponse),
        (status = 404, description = "Permission not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_permission(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, id)): Path<(String, Uuid)>,
) -> Result<ApiResult<PermissionResponse>, ApiError> {
    let current_user_id = identity.user_id();

    if identity.realm_id() != realm_id {
        return Err(ApiError::forbidden(
            "Access denied: cannot view permissions in a different realm",
        ));
    }

    let has_permission = state
        .permission_checker
        .check_permission(&realm_id, &current_user_id, "permissions", "view")
        .await
        .map_err(|e| {
            tracing::error!(
                current_user_id = %current_user_id,
                realm_id = %realm_id,
                error = %e,
                "Failed to check permissions.view permission"
            );
            ApiError::internal("Failed to check permission")
        })?;

    if !has_permission {
        return Err(ApiError::forbidden(
            "Insufficient permissions: requires permissions.view",
        ));
    }

    let row = sqlx::query_as::<_, PermissionResponse>(
        r#"
        SELECT id, name, resource, action, description, realm_id, is_builtin
        FROM permissions
        WHERE id = $1 AND realm_id = $2
        "#,
    )
    .bind(id)
    .bind(&realm_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to get permission: {e}");
        ApiError::internal("Failed to get permission")
    })?;

    let row = row.ok_or_else(|| ApiError::not_found("Permission not found"))?;

    Ok(ApiResult::ok(row))
}
