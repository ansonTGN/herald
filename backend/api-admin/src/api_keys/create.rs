use axum::{
    Json,
    extract::{Extension, Path, State},
};
use axum_valid::Valid;
use chrono::Utc;
use herald_api_base::application::http::auth::util::require_permission;
use herald_api_base::application::http::server::api_entities::{ApiError, ApiResult};
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::client_api_keys::services::ClientApiKeyService;
use uuid::Uuid;

use crate::api_keys::client_app_info::resolve_client_app_for_create;
use crate::api_keys::types::{CreateApiKeyRequest, CreateApiKeyResponse};

/// Create a new API Key
///
/// Generates a new API key for the specified realm. The plaintext key is returned
/// only in this response and cannot be retrieved later.
#[utoipa::path(
    post,
    path = "/api/api-keys/{realmId}",
    tag = "api-keys",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    request_body = CreateApiKeyRequest,
    responses(
        (status = 201, description = "API Key created", body = CreateApiKeyResponse),
        (status = 400, description = "Bad request", body = herald_api_base::application::http::server::api_entities::ErrorResponse),
        (status = 403, description = "Forbidden", body = herald_api_base::application::http::server::api_entities::ErrorResponse),
        (status = 500, description = "Internal server error", body = herald_api_base::application::http::server::api_entities::ErrorResponse)
    )
)]
pub async fn create_api_key(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Valid(Json(payload)): Valid<Json<CreateApiKeyRequest>>,
) -> Result<ApiResult<CreateApiKeyResponse>, ApiError> {
    let user_id = identity.user_id();
    require_permission(
        &state,
        &realm_id,
        &user_id,
        "api_keys",
        "manage",
        "api_keys.manage",
    )
    .await?;

    // Generate plaintext key and hash
    let plaintext_key = ClientApiKeyService::generate_api_key();
    let api_key_hash = ClientApiKeyService::hash_api_key(&plaintext_key);

    // Parse optional expiration
    let expires_at = payload
        .expires_at
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| ApiError::bad_request(format!("Invalid expiresAt format: {e}")))
        })
        .transpose()?;

    let now = Utc::now();
    let id = Uuid::now_v7().to_string();

    let client_app =
        resolve_client_app_for_create(&state, &realm_id, payload.client_app_id).await?;

    let api_key = herald_core::domain::client_api_keys::entities::ClientApiKey {
        id: id.clone(),
        name: payload.name.clone(),
        api_key_hash,
        realm_id: realm_id.clone(),
        client_app_id: Some(client_app.id),
        enabled: true,
        expires_at,
        created_at: now,
        last_used_at: None,
        usage_count: 0,
    };

    let saved = state.api_key_repo.create(&api_key).await.map_err(|e| {
        tracing::error!("Failed to create API key: {e}");
        ApiError::internal("Failed to create API key")
    })?;

    let response = CreateApiKeyResponse {
        id: saved.id,
        name: saved.name,
        key: plaintext_key,
        realm_id: saved.realm_id,
        client_app_id: saved.client_app_id,
        client_app_name: Some(client_app.name),
        enabled: saved.enabled,
        expires_at: saved.expires_at.map(|dt| dt.to_rfc3339()),
        created_at: saved.created_at.to_rfc3339(),
    };

    Ok(ApiResult::created(response))
}
