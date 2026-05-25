use crate::role_definitions::types::{ErrorResponse, RoleResponse, RoleUpdateRequest};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use axum_valid::Valid;
use herald_api_base::application::http::server::api_entities::{ApiError, ApiResult};
use herald_api_base::application::http::state::AppState;
use herald_core::domain::audit::{
    ActorType, AuditAction, AuditCategory, AuditEventRepository, AuditResult, AuditTargetType,
    NewAuditEvent,
};
use herald_core::domain::authentication::Identity;
use herald_core::domain::authorization::PermissionService;
use uuid::Uuid;

/// Update role
#[utoipa::path(
    put,
    path = "/api/roles/{realmId}/define/{roleId}",
    tag = "role-definitions",
    summary = "Update a role",
    description = "Update role definition name and description. Requires `roles.manage` permission.",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("roleId" = Uuid, Path, description = "Role ID")
    ),
    request_body = RoleUpdateRequest,
    responses(
        (status = 200, description = "Role updated", body = RoleResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions (requires roles.manage)", body = ErrorResponse),
        (status = 404, description = "Role not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_role(
    State(state): State<AppState>,
    Path((realm_id, id)): Path<(String, Uuid)>,
    Extension(identity): Extension<Identity>,
    Valid(Json(payload)): Valid<Json<RoleUpdateRequest>>,
) -> Result<ApiResult<RoleResponse>, ApiError> {
    // Check permission: requires roles.manage
    let current_user_id = identity.user_id();
    let identity_realm_id = identity.realm_id();

    // Realm boundary check
    if identity_realm_id != realm_id {
        if let Err(e) = state
            .audit_event_repository
            .create(NewAuditEvent {
                realm_id: realm_id.clone(),
                category: AuditCategory::Rbac,
                action: AuditAction::RoleUpdate,
                actor_id: identity.user_id().to_string(),
                actor_type: Some(ActorType::Admin),
                actor_name: identity.as_user().map(|u| u.email.clone()),
                target_type: AuditTargetType::Role,
                target_id: id.to_string(),
                target_name: None,
                result: AuditResult::Failure,
                details: Some(serde_json::json!({"reason": "realm_boundary_violation"})),
                ip_address: None,
                user_agent: None,
                trace_id: None,
            })
            .await
        {
            tracing::warn!(error = %e, "Failed to record audit event");
        }
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
        if let Err(e) = state
            .audit_event_repository
            .create(NewAuditEvent {
                realm_id: realm_id.clone(),
                category: AuditCategory::Rbac,
                action: AuditAction::RoleUpdate,
                actor_id: identity.user_id().to_string(),
                actor_type: Some(ActorType::Admin),
                actor_name: identity.as_user().map(|u| u.email.clone()),
                target_type: AuditTargetType::Role,
                target_id: id.to_string(),
                target_name: None,
                result: AuditResult::Failure,
                details: Some(serde_json::json!({"reason": "insufficient_permissions", "required": "roles.manage"})),
                ip_address: None,
                user_agent: None,
                trace_id: None,
            })
            .await
        {
            tracing::warn!(error = %e, "Failed to record audit event");
        }
        return Err(ApiError::forbidden(
            "Insufficient permissions: requires roles.manage",
        ));
    }
    // Check if role exists and get current data
    let current_role: Option<(bool, String)> =
        sqlx::query_as("SELECT is_builtin, name FROM roles WHERE id = $1 AND realm_id = $2")
            .bind(id)
            .bind(&realm_id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to query role: {e}");
                ApiError::internal("Failed to query role")
            })?;

    let (is_builtin, current_name) = match current_role {
        Some(role) => role,
        None => {
            if let Err(e) = state
                .audit_event_repository
                .create(NewAuditEvent {
                    realm_id: realm_id.clone(),
                    category: AuditCategory::Rbac,
                    action: AuditAction::RoleUpdate,
                    actor_id: identity.user_id().to_string(),
                    actor_type: Some(ActorType::Admin),
                    actor_name: identity.as_user().map(|u| u.email.clone()),
                    target_type: AuditTargetType::Role,
                    target_id: id.to_string(),
                    target_name: None,
                    result: AuditResult::Failure,
                    details: Some(serde_json::json!({"reason": "role_not_found"})),
                    ip_address: None,
                    user_agent: None,
                    trace_id: None,
                })
                .await
            {
                tracing::warn!(error = %e, "Failed to record audit event");
            }
            return Err(ApiError::not_found("Role not found"));
        }
    };

    // Protect builtin role name changes
    if is_builtin && payload.name != current_name {
        tracing::warn!(
            "Attempted to change builtin role name from '{}' to '{}'",
            current_name,
            payload.name
        );
        if let Err(e) = state
            .audit_event_repository
            .create(NewAuditEvent {
                realm_id: realm_id.clone(),
                category: AuditCategory::Rbac,
                action: AuditAction::RoleUpdate,
                actor_id: identity.user_id().to_string(),
                actor_type: Some(ActorType::Admin),
                actor_name: identity.as_user().map(|u| u.email.clone()),
                target_type: AuditTargetType::Role,
                target_id: id.to_string(),
                target_name: Some(current_name.clone()),
                result: AuditResult::Failure,
                details: Some(serde_json::json!({"reason": "builtin_role_name_change", "from": current_name, "to": payload.name})),
                ip_address: None,
                user_agent: None,
                trace_id: None,
            })
            .await
        {
            tracing::warn!(error = %e, "Failed to record audit event");
        }
        return Err(ApiError::forbidden("Cannot change built-in role name"));
    }

    let row = sqlx::query_as::<_, RoleResponse>(
        r#"
        UPDATE roles
        SET name = $1, description = $2, updated_at = CURRENT_TIMESTAMP
        WHERE id = $3 AND realm_id = $4
        RETURNING id, name, description, realm_id, client_id, is_builtin
        "#,
    )
    .bind(&payload.name)
    .bind(&payload.description)
    .bind(id)
    .bind(&realm_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to update role: {e}");
        if let sqlx::Error::Database(db_err) = &e
            && db_err.code().as_deref() == Some("23505")
        // PostgreSQL unique constraint violation
        {
            ApiError::bad_request("Role name already exists in this realm")
        } else {
            ApiError::internal("Failed to update role")
        }
    })?;

    let row = match row {
        Some(r) => r,
        None => {
            if let Err(e) = state
                .audit_event_repository
                .create(NewAuditEvent {
                    realm_id: realm_id.clone(),
                    category: AuditCategory::Rbac,
                    action: AuditAction::RoleUpdate,
                    actor_id: identity.user_id().to_string(),
                    actor_type: Some(ActorType::Admin),
                    actor_name: identity.as_user().map(|u| u.email.clone()),
                    target_type: AuditTargetType::Role,
                    target_id: id.to_string(),
                    target_name: None,
                    result: AuditResult::Failure,
                    details: Some(serde_json::json!({"reason": "role_not_found_after_update"})),
                    ip_address: None,
                    user_agent: None,
                    trace_id: None,
                })
                .await
            {
                tracing::warn!(error = %e, "Failed to record audit event");
            }
            return Err(ApiError::not_found("Role not found"));
        }
    };

    // Record audit event (failure does not fail the operation)
    if let Err(e) = state
        .audit_event_repository
        .create(NewAuditEvent {
            realm_id: realm_id.clone(),
            category: AuditCategory::Rbac,
            action: AuditAction::RoleUpdate,
            actor_id: identity.user_id().to_string(),
            actor_type: Some(ActorType::Admin),
            actor_name: identity.as_user().map(|u| u.email.clone()),
            target_type: AuditTargetType::Role,
            target_id: row.id.to_string(),
            target_name: Some(row.name.clone()),
            result: AuditResult::Success,
            details: Some(serde_json::json!({"name": row.name})),
            ip_address: None,
            user_agent: None,
            trace_id: None,
        })
        .await
    {
        tracing::warn!(error = %e, "Failed to record audit event");
    }

    Ok(ApiResult::ok(row))
}
