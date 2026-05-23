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
use herald_core::domain::client_api_keys::constants::ADMIN_API_CLIENT_ID;
use herald_core::domain::client_api_keys::services::ClientApiKeyService;
use herald_core::entity::client_app;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

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

    // Resolve the realm's built-in API Key Client App
    let builtin_client_app = client_app::Entity::find()
        .filter(client_app::Column::RealmId.eq(&realm_id))
        .filter(client_app::Column::ClientId.eq(ADMIN_API_CLIENT_ID))
        .one(state.db.as_ref())
        .await
        .map_err(|e| {
            tracing::error!("Failed to query built-in API Key Client App: {e}");
            ApiError::internal("Failed to create API key")
        })?;

    let builtin_client_app = match builtin_client_app {
        Some(app) => app,
        None => {
            return Err(ApiError::bad_request(
                "Realm is missing the built-in API Key Client App. Please contact support.",
            ));
        }
    };

    let api_key = herald_core::domain::client_api_keys::entities::ClientApiKey {
        id: id.clone(),
        name: payload.name.clone(),
        api_key_hash,
        realm_id: realm_id.clone(),
        client_app_id: Some(builtin_client_app.id),
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
        enabled: saved.enabled,
        expires_at: saved.expires_at.map(|dt| dt.to_rfc3339()),
        created_at: saved.created_at.to_rfc3339(),
    };

    Ok(ApiResult::created(response))
}
