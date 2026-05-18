use axum::{
    Json,
    extract::{Extension, Path, State},
    http::HeaderMap,
};
use axum_valid::Valid;
use herald_core::domain::authentication::Identity;
use uuid::Uuid;

use crate::client_apps::types::{ClientAppItem, ClientAppUpdateRequest};
use herald_api_base::application::http::server::api_entities::{ApiError, ApiResult};
use herald_api_base::application::http::state::AppState;
use herald_core::domain::client::ports::ClientService;
use herald_core::domain::client::value_objects::UpdateClientAppRequest;

/// Update a client app configuration
///
/// Updates the configuration of an existing OAuth client application.
/// Optionally regenerate the client secret.
#[utoipa::path(
    put,
    path = "/api/client/{realmId}/{clientAppId}",
    tag = "client",
    request_body = ClientAppUpdateRequest,
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("clientAppId" = Uuid, Path, description = "client app UUID"),
    ),
    responses(
        (status = 200, description = "ClientApp updated", body = ClientAppItem),
        (status = 404, description = "ClientApp not found", body = herald_api_base::application::http::server::api_entities::ErrorResponse),
        (status = 500, description = "Internal server error", body = herald_api_base::application::http::server::api_entities::ErrorResponse)
    )
)]
pub async fn update_client_app(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((_realm_id, id)): Path<(String, Uuid)>,
    _headers: HeaderMap,
    Valid(Json(payload)): Valid<Json<ClientAppUpdateRequest>>,
) -> Result<ApiResult<ClientAppItem>, ApiError> {
    // Extract identity from request extension (injected by inject_identity middleware)
    let identity_realm_id = identity.realm_id();
    let current_user_id = identity.user_id();

    tracing::debug!(
        realm_id = %identity_realm_id,
        user_id = %current_user_id,
        "Updating client app"
    );

    // Create service request
    let service_request = UpdateClientAppRequest {
        name: payload.name.clone(),
        description: payload.description.clone(),
        redirect_uris: payload.redirect_uris.clone(),
        enabled: payload.enabled,
        icon_url: payload.icon_url.clone(),
        session_ttl_seconds: payload.session_ttl_seconds,
        session_renewal_ttl_seconds: payload.session_renewal_ttl_seconds,
        regenerate_secret: payload.regenerate_secret,
        device_code_grant_enabled: payload.device_code_grant_enabled,
    };

    // Call service layer
    let client_service = state.service.client_service();
    let client_app = client_service
        .update_client_app(identity, id, service_request)
        .await
        .map_err(|e| match e {
            herald_core::domain::common::entities::app_errors::CoreError::NotFound => {
                ApiError::not_found("client_app not found")
            }
            herald_core::domain::common::entities::app_errors::CoreError::BadRequest(msg) => {
                tracing::error!("Bad request: {}", msg);
                ApiError::bad_request(msg)
            }
            e => {
                tracing::error!("Failed to update client app: {}", e);
                ApiError::internal(format!("Failed to update client app: {e}"))
            }
        })?;

    // Convert domain model to API response model
    let response: ClientAppItem = ClientAppItem {
        id: client_app.id,
        realm_id: client_app.realm_id,
        client_id: client_app.client_id,
        name: client_app.name,
        description: client_app.description,
        redirect_uris: client_app.redirect_uris,
        enabled: client_app.enabled,
        icon_url: client_app.icon_url,
        session_ttl_seconds: client_app.session_ttl_seconds,
        session_renewal_ttl_seconds: client_app.session_renewal_ttl_seconds,
        client_secret: payload
            .regenerate_secret
            .filter(|regenerate| *regenerate)
            .and(client_app.client_secret),
        device_code_grant_enabled: client_app.device_code_grant_enabled,
    };

    Ok(ApiResult::ok(response))
}
