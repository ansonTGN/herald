use crate::role_definitions::types::{ErrorResponse, RoleResponse};
use axum::{
    Extension,
    extract::{Path, State},
};
use herald_api_base::application::http::server::api_entities::{ApiError, ApiResult};
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::authorization::permission_service::PermissionService;
use uuid::Uuid;

/// Get role by ID
#[utoipa::path(
    get,
    path = "/api/roles/{realmId}/define/{roleId}",
    tag = "role-definitions",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("roleId" = Uuid, Path, description = "Role ID")
    ),
    responses(
        (status = 200, description = "Role found", body = RoleResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Role not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_role(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, id)): Path<(String, Uuid)>,
) -> Result<ApiResult<RoleResponse>, ApiError> {
    // Extract user_id from identity
    let current_user_id = identity.user_id();
    let identity_realm_id = identity.realm_id();

    // Check realm boundary
    if identity_realm_id != realm_id {
        return Err(ApiError::forbidden(
            "Access denied: cannot access roles from a different realm",
        ));
    }

    // Check roles.view permission
    let allowed = state
        .permission_checker
        .check_permission(&realm_id, &current_user_id, "roles", "view")
        .await
        .map_err(|e| {
            tracing::error!("Permission check error: {e}");
            ApiError::internal("Failed to check permission")
        })?;

    if !allowed {
        return Err(ApiError::forbidden("Insufficient permissions to get role"));
    }
    let row = sqlx::query_as::<_, RoleResponse>(
        r#"
        SELECT id, name, description, realm_id, client_id, is_builtin
        FROM roles
        WHERE id = $1 AND realm_id = $2
        "#,
    )
    .bind(id)
    .bind(&realm_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to get role: {e}");
        ApiError::internal("Failed to get role")
    })?;

    let row = row.ok_or_else(|| ApiError::not_found("Role not found"))?;

    Ok(ApiResult::ok(row))
}
