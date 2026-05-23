use axum::extract::{Extension, Path, State};
use herald_api_base::application::http::auth::util::require_permission;
use herald_api_base::application::http::server::api_entities::{ApiError, ApiResult};
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;

use crate::api_keys::types::ApiKeyListItem;

/// Get a specific API Key by ID
///
/// Retrieves details of an API key. Hash and plaintext are never exposed.
#[utoipa::path(
    get,
    path = "/api/api-keys/{realmId}/{apiKeyId}",
    tag = "api-keys",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("apiKeyId" = String, Path, description = "API Key ID"),
    ),
    responses(
        (status = 200, description = "API Key details", body = ApiKeyListItem),
        (status = 403, description = "Forbidden", body = herald_api_base::application::http::server::api_entities::ErrorResponse),
        (status = 404, description = "API Key not found", body = herald_api_base::application::http::server::api_entities::ErrorResponse),
        (status = 500, description = "Internal server error", body = herald_api_base::application::http::server::api_entities::ErrorResponse)
    )
)]
pub async fn get_api_key(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, api_key_id)): Path<(String, String)>,
) -> Result<ApiResult<ApiKeyListItem>, ApiError> {
    let user_id = identity.user_id();
    require_permission(
        &state,
        &realm_id,
        &user_id,
        "api_keys",
        "view",
        "api_keys.view",
    )
    .await?;

    let api_key = state
        .api_key_repo
        .find_by_id(&api_key_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get API key: {e}");
            ApiError::internal("Failed to get API key")
        })?
        .ok_or_else(|| ApiError::not_found("API key not found"))?;

    if api_key.realm_id != realm_id {
        return Err(ApiError::not_found("API key not found"));
    }

    let response = ApiKeyListItem {
        id: api_key.id,
        name: api_key.name,
        realm_id: api_key.realm_id,
        enabled: api_key.enabled,
        expires_at: api_key.expires_at.map(|dt| dt.to_rfc3339()),
        last_used_at: api_key.last_used_at.map(|dt| dt.to_rfc3339()),
        usage_count: api_key.usage_count,
        created_at: api_key.created_at.to_rfc3339(),
    };

    Ok(ApiResult::ok(response))
}
