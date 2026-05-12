use axum::{
    Extension,
    extract::{Path, State},
};
use herald_core::domain::authentication::Identity;
use herald_core::domain::authorization::PermissionService;

use crate::admin::permission_definitions::types::{ErrorResponse, PermissionResponse};
use herald_api_base::application::http::server::api_entities::{ApiError, ApiResult};
use herald_api_base::application::http::state::AppState;

/// List permissions by realm_id
#[utoipa::path(
    get,
    path = "/api/permission/{realmId}/define",
    tag = "permission-definitions",
    summary = "List permissions in the realm",
    description = "List all permission definitions in the realm. Requires `permissions.view` permission.",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    responses(
        (status = 200, description = "List of permissions", body = Vec<PermissionResponse>),
        (status = 403, description = "Forbidden - Insufficient permissions (requires permissions.view)", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_permissions(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
) -> Result<ApiResult<Vec<PermissionResponse>>, ApiError> {
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

    let rows = sqlx::query_as::<_, PermissionResponse>(
        r#"
        SELECT id, name, resource, action, description, realm_id, is_builtin
        FROM permissions
        WHERE realm_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(&realm_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to list permissions: {e}");
        ApiError::internal("Failed to list permissions")
    })?;

    Ok(ApiResult::ok(rows))
}
