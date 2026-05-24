use axum::{http::StatusCode, response::Response};
use herald_api_base::application::http::common::error_codes::ErrorCode;
use herald_api_base::application::http::common::error_helpers::json_error;
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::client_api_keys::constants::ADMIN_API_CLIENT_ID;
use herald_core::entity::client_app;
use sea_orm::EntityTrait;
use uuid::Uuid;

pub async fn ensure_client_app_scope(
    state: &AppState,
    identity: &Identity,
    target_client_app_id: Uuid,
) -> Result<(), Response> {
    let Some(api_key) = identity.as_third_party() else {
        return Ok(());
    };

    let Some(bound_client_app_id) = api_key.client_app_id else {
        return Ok(());
    };

    if bound_client_app_id == target_client_app_id {
        return Ok(());
    }

    let bound_app = client_app::Entity::find_by_id(bound_client_app_id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| {
            tracing::error!("Failed to load API key bound Client App: {e}");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, ErrorCode::InternalError)
        })?;

    if bound_app.is_some_and(|app| app.client_id == ADMIN_API_CLIENT_ID) {
        return Ok(());
    }

    tracing::warn!(
        api_key_id = %api_key.id,
        bound_client_app_id = %bound_client_app_id,
        target_client_app_id = %target_client_app_id,
        "API key attempted to access a different Client App"
    );
    Err(json_error(StatusCode::FORBIDDEN, ErrorCode::Forbidden))
}

pub async fn is_admin_api_key(state: &AppState, identity: &Identity) -> Result<bool, Response> {
    let Some(api_key) = identity.as_third_party() else {
        return Ok(false);
    };
    let Some(bound_client_app_id) = api_key.client_app_id else {
        return Ok(true);
    };

    let bound_app = client_app::Entity::find_by_id(bound_client_app_id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| {
            tracing::error!("Failed to load API key bound Client App: {e}");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, ErrorCode::InternalError)
        })?;

    Ok(bound_app.is_some_and(|app| app.client_id == ADMIN_API_CLIENT_ID))
}

pub async fn bound_client_identifier(
    state: &AppState,
    identity: &Identity,
) -> Result<Option<String>, Response> {
    let Some(api_key) = identity.as_third_party() else {
        return Ok(None);
    };
    let Some(bound_client_app_id) = api_key.client_app_id else {
        return Ok(None);
    };

    let bound_app = client_app::Entity::find_by_id(bound_client_app_id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| {
            tracing::error!("Failed to load API key bound Client App: {e}");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, ErrorCode::InternalError)
        })?;

    Ok(bound_app.map(|app| app.client_id))
}
