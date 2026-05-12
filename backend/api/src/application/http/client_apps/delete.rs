use axum::{
    extract::{Extension, Path, State},
    http::HeaderMap,
};
use herald_core::domain::authentication::Identity;
use uuid::Uuid;

use crate::application::http::server::api_entities::{ApiError, ApiResult};
use crate::application::http::state::AppState;
use herald_core::domain::client::ports::ClientService;

/// Delete a client app
///
/// Deletes an OAuth client application and its associated roles.
/// The built-in admin console client cannot be deleted.
#[utoipa::path(
    delete,
    path = "/api/client/{realmId}/{clientAppId}",
    tag = "client",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("clientAppId" = Uuid, Path, description = "Client App UUID"),
    ),
    responses(
        (status = 204, description = "Client App deleted"),
        (status = 403, description = "Cannot delete built-in admin console", body = crate::application::http::server::api_entities::ErrorResponse),
        (status = 404, description = "Client App not found", body = crate::application::http::server::api_entities::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::application::http::server::api_entities::ErrorResponse)
    )
)]
pub async fn delete_client_app(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((_realm_id, id)): Path<(String, Uuid)>,
    _headers: HeaderMap,
) -> Result<ApiResult<()>, ApiError> {
    // Extract identity from request extension (injected by inject_identity middleware)
    let identity_realm_id = identity.realm_id();
    let current_user_id = identity.user_id();

    tracing::debug!(
        realm_id = %identity_realm_id,
        user_id = %current_user_id,
        "Deleting client app"
    );

    // Call service layer
    let client_service = state.service.client_service();
    client_service
        .delete_client_app(identity, id)
        .await
        .map_err(|e| match e {
            herald_core::domain::common::entities::app_errors::CoreError::NotFound => {
                ApiError::not_found("client_app not found")
            }
            herald_core::domain::common::entities::app_errors::CoreError::Forbidden(msg) => {
                ApiError::forbidden(msg)
            }
            e => {
                tracing::error!("Failed to delete client app: {}", e);
                ApiError::internal(format!("Failed to delete client app: {e}"))
            }
        })?;

    Ok(ApiResult::no_content())
}
