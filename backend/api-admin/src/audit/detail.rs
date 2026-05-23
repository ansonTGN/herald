use axum::extract::{Extension, Path, State};
use herald_core::domain::audit::AuditEventRepository;
use herald_core::domain::authentication::Identity;
use herald_core::domain::authorization::PermissionService;

use super::types::AuditEventDetailResponse;
use herald_api_base::application::http::server::api_entities::{ApiError, ApiResult};
use herald_api_base::application::http::state::AppState;

/// Get a single audit event by ID
#[utoipa::path(
    get,
    path = "/api/audit/{realmId}/{eventId}",
    tag = "audit",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("eventId" = String, Path, description = "Audit event ID"),
    ),
    responses(
        (status = 200, description = "Audit event detail", body = AuditEventDetailResponse),
        (status = 401, description = "Unauthorized", body = herald_api_base::application::http::server::api_entities::ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = herald_api_base::application::http::server::api_entities::ErrorResponse),
        (status = 404, description = "Audit event not found", body = herald_api_base::application::http::server::api_entities::ErrorResponse),
        (status = 500, description = "Internal server error", body = herald_api_base::application::http::server::api_entities::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_audit_event(
    Path((_realm_id, event_id_str)): Path<(String, String)>,
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
) -> Result<ApiResult<AuditEventDetailResponse>, ApiError> {
    let realm_id = identity.realm_id();

    let allowed = state
        .permission_checker
        .check_permission(&realm_id, &identity.user_id(), "audit", "view")
        .await
        .map_err(|e| {
            tracing::error!("Failed to check audit access permission: {e}");
            ApiError::internal("Failed to check permission")
        })?;

    if !allowed {
        return Err(ApiError::forbidden(
            "Insufficient permissions to view audit logs",
        ));
    }

    let event_id = uuid::Uuid::parse_str(&event_id_str)
        .map_err(|_| ApiError::bad_request("Invalid event ID format"))?;

    let event = state
        .audit_event_repository
        .find_by_id(&realm_id, event_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get audit event: {e}");
            ApiError::internal("Failed to get audit event")
        })?
        .ok_or_else(|| ApiError::not_found("Audit event not found"))?;

    Ok(ApiResult::ok(AuditEventDetailResponse::from_event(event)))
}
