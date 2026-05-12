use crate::role_definitions::types::{ErrorResponse, RoleCreateRequest, RoleResponse};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use axum_valid::Valid;
use herald_api_base::application::http::server::api_entities::{ApiError, ApiResult};
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::authorization::PermissionService;

/// Create a new role
#[utoipa::path(
    post,
    path = "/api/roles/{realmId}/define",
    tag = "role-definitions",
    summary = "Create a new role",
    description = "Create a new role definition. Requires `roles.manage` permission.",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    request_body = RoleCreateRequest,
    responses(
        (status = 201, description = "Role created", body = RoleResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions (requires roles.manage)", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_role(
    Path(realm_id): Path<String>,
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Valid(Json(payload)): Valid<Json<RoleCreateRequest>>,
) -> Result<ApiResult<RoleResponse>, ApiError> {
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
    let row = sqlx::query_as::<_, RoleResponse>(
        r#"
        INSERT INTO roles (name, description, realm_id, client_id, is_builtin)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, name, description, realm_id, client_id, is_builtin
        "#,
    )
    .bind(&payload.name)
    .bind(&payload.description)
    .bind(&realm_id)
    .bind(&payload.client_id)
    .bind(false) // is_builtin = false for user-created roles
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create role: {e}");
        if let sqlx::Error::Database(db_err) = &e
            && db_err.code().as_deref() == Some("23505")
        // PostgreSQL unique constraint violation
        {
            ApiError::bad_request("Role name already exists in this realm")
        } else {
            ApiError::internal("Failed to create role")
        }
    })?;

    Ok(ApiResult::created(row))
}
