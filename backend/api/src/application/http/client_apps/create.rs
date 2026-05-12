use axum::{
    Json,
    extract::{Extension, Path, State},
};
use axum_valid::Valid;
use herald_core::domain::authentication::Identity;

use crate::application::http::client_apps::types::{ClientAppCreateRequest, ClientAppItem};
use crate::application::http::server::api_entities::{ApiError, ApiResult};
use crate::application::http::state::AppState;
use herald_core::domain::client::ports::ClientService;
use herald_core::domain::client::value_objects::CreateClientAppRequest;

/// Create a new client app
///
/// Creates a new OAuth client application with the specified configuration.
#[utoipa::path(
    post,
    path = "/api/client/{realmId}",
    tag = "client",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    request_body = ClientAppCreateRequest,
    responses(
        (status = 201, description = "ClientApp created", body = ClientAppItem),
        (status = 400, description = "Bad request", body = crate::application::http::server::api_entities::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::application::http::server::api_entities::ErrorResponse)
    )
)]
pub async fn create_client_app(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Valid(Json(payload)): Valid<Json<ClientAppCreateRequest>>,
) -> Result<ApiResult<ClientAppItem>, ApiError> {
    // Extract identity from request extension (injected by inject_identity middleware)
    let identity_realm_id = identity.realm_id();
    let current_user_id = identity.user_id();

    tracing::debug!(
        realm_id = %identity_realm_id,
        user_id = %current_user_id,
        "Creating client app"
    );

    // Create service request
    let service_request = CreateClientAppRequest {
        realm_id: realm_id.clone(),
        client_id: payload.client_id.clone(),
        name: payload.name.clone(),
        description: payload.description.clone(),
        redirect_uris: payload.redirect_uris.clone(),
        enabled: payload.enabled,
        icon_url: payload.icon_url.clone(),
        session_ttl_seconds: payload.session_ttl_seconds,
        session_renewal_ttl_seconds: payload.session_renewal_ttl_seconds,
    };

    // Call service layer
    let client_service = state.service.client_service();
    let client_app = client_service
        .create_client_app(identity, service_request)
        .await
        .map_err(|e| match e {
            herald_core::domain::common::entities::app_errors::CoreError::Conflict(msg) => {
                tracing::error!("Client app conflict: {}", msg);
                ApiError::bad_request(msg)
            }
            herald_core::domain::common::entities::app_errors::CoreError::BadRequest(msg) => {
                tracing::error!("Bad request: {}", msg);
                ApiError::bad_request(msg)
            }
            e => {
                tracing::error!("Failed to create client app: {}", e);
                ApiError::internal(format!("Failed to create client app: {e}"))
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
        client_secret: client_app.client_secret,
    };

    Ok(ApiResult::created(response))
}
