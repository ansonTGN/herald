use crate::role_definitions::types::{AssignPermissionRequest, ErrorResponse, PermissionResponse};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use herald_api_base::application::http::server::api_entities::{ApiError, ApiResult};
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::authorization::permission_service::PermissionService;
use uuid::Uuid;

/// Assign permission to role
#[utoipa::path(
    post,
    path = "/api/roles/{realmId}/define/{roleId}/permissions",
    tag = "role-definitions",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("roleId" = Uuid, Path, description = "Role ID")
    ),
    request_body = AssignPermissionRequest,
    responses(
        (status = 200, description = "Permission assigned to role"),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Role or permission not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn assign_permission_to_role(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, role_id)): Path<(String, Uuid)>,
    Json(payload): Json<AssignPermissionRequest>,
) -> Result<ApiResult<()>, ApiError> {
    // Extract user_id from identity
    let current_user_id = identity.user_id();
    let identity_realm_id = identity.realm_id();

    // Check realm boundary
    if identity_realm_id != realm_id {
        return Err(ApiError::forbidden(
            "Access denied: cannot manage roles from a different realm",
        ));
    }

    // Check roles.manage permission
    let allowed = state
        .permission_checker
        .check_permission(&realm_id, &current_user_id, "roles", "manage")
        .await
        .map_err(|e| {
            tracing::error!("Permission check error: {e}");
            ApiError::internal("Failed to check permission")
        })?;

    if !allowed {
        return Err(ApiError::forbidden(
            "Insufficient permissions to assign permissions to role",
        ));
    }
    sqlx::query(
        r#"
        INSERT INTO role_permissions (role_id, permission_id)
        VALUES ($1, $2)
        ON CONFLICT (role_id, permission_id) DO NOTHING
        "#,
    )
    .bind(role_id)
    .bind(payload.permission_id)
    .execute(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to assign permission to role: {e}");
        if e.to_string().contains("foreign key constraint") {
            ApiError::not_found("Role or permission not found")
        } else {
            ApiError::internal("Failed to assign permission")
        }
    })?;

    Ok(ApiResult::ok(()))
}

/// Remove permission from role
#[utoipa::path(
    delete,
    path = "/api/roles/{realmId}/define/{roleId}/permissions/{permissionId}",
    tag = "role-definitions",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("roleId" = Uuid, Path, description = "Role ID"),
        ("permissionId" = Uuid, Path, description = "Permission ID")
    ),
    responses(
        (status = 204, description = "Permission removed from role"),
        (status = 403, description = "Forbidden - attempting to remove built-in permission from built-in role", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn remove_permission_from_role(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, role_id, permission_id)): Path<(String, Uuid, Uuid)>,
) -> Result<ApiResult<()>, ApiError> {
    // Extract user_id from identity
    let current_user_id = identity.user_id();
    let identity_realm_id = identity.realm_id();

    // Check realm boundary
    if identity_realm_id != realm_id {
        return Err(ApiError::forbidden(
            "Access denied: cannot manage roles from a different realm",
        ));
    }

    // Check roles.manage permission
    let allowed = state
        .permission_checker
        .check_permission(&realm_id, &current_user_id, "roles", "manage")
        .await
        .map_err(|e| {
            tracing::error!("Permission check error: {e}");
            ApiError::internal("Failed to check permission")
        })?;

    if !allowed {
        return Err(ApiError::forbidden(
            "Insufficient permissions to remove permissions from role",
        ));
    }
    let role: Option<(String, bool)> =
        sqlx::query_as("SELECT name, is_builtin FROM roles WHERE id = $1")
            .bind(role_id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to check role: {e}");
                ApiError::internal("Failed to check role")
            })?;

    if let Some((role_name, is_builtin)) = role
        && is_builtin
        && role_name == "realm-admin"
    {
        let permission: Option<(bool,)> =
            sqlx::query_as("SELECT is_builtin FROM permissions WHERE id = $1")
                .bind(permission_id)
                .fetch_optional(&state.pool)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to check permission: {e}");
                    ApiError::internal("Failed to check permission")
                })?;

        if let Some((perm_is_builtin,)) = permission
            && perm_is_builtin
        {
            return Err(ApiError::forbidden(
                "Cannot remove built-in permission from built-in role",
            ));
        }
    }

    sqlx::query("DELETE FROM role_permissions WHERE role_id = $1 AND permission_id = $2")
        .bind(role_id)
        .bind(permission_id)
        .execute(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to remove permission from role: {e}");
            ApiError::internal("Failed to remove permission")
        })?;

    Ok(ApiResult::no_content())
}

/// Get permissions for a role
#[utoipa::path(
    get,
    path = "/api/roles/{realmId}/define/{roleId}/permissions",
    tag = "role-definitions",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("roleId" = Uuid, Path, description = "Role ID")
    ),
    responses(
        (status = 200, description = "List of permissions", body = Vec<PermissionResponse>),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_role_permissions(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, role_id)): Path<(String, Uuid)>,
) -> Result<ApiResult<Vec<PermissionResponse>>, ApiError> {
    // Extract user_id from identity
    let current_user_id = identity.user_id();
    let identity_realm_id = identity.realm_id();

    // Check realm boundary
    if identity_realm_id != realm_id {
        return Err(ApiError::forbidden(
            "Access denied: cannot view roles from a different realm",
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
        return Err(ApiError::forbidden(
            "Insufficient permissions to view role permissions",
        ));
    }
    let rows = sqlx::query_as::<_, PermissionResponse>(
        r#"
        SELECT p.id, p.name, p.resource, p.action, p.description, p.realm_id, p.is_builtin
        FROM permissions p
        INNER JOIN role_permissions rp ON p.id = rp.permission_id
        WHERE rp.role_id = $1
        ORDER BY p.created_at DESC
        "#,
    )
    .bind(role_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to get role permissions: {e}");
        ApiError::internal("Failed to get role permissions")
    })?;

    Ok(ApiResult::ok(rows))
}
