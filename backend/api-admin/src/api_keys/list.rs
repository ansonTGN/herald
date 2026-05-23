use axum::extract::{Extension, Path, Query, State};
use herald_api_base::application::http::auth::util::require_permission;
use herald_api_base::application::http::server::api_entities::{ApiError, ApiResult, PageResponse};
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;

use crate::api_keys::types::{ApiKeyListItem, ListQuery};

/// List all API Keys for a realm
///
/// Returns a paginated list of API keys. Hash and plaintext are never exposed.
#[utoipa::path(
    get,
    path = "/api/api-keys/{realmId}",
    tag = "api-keys",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("page" = Option<i64>, Query, description = "Page number (0-based, default 0)"),
        ("pageSize" = Option<i64>, Query, description = "Page size (default 20)"),
    ),
    responses(
        (status = 200, description = "API Key list", body = PageResponse<ApiKeyListItem>),
        (status = 403, description = "Forbidden", body = herald_api_base::application::http::server::api_entities::ErrorResponse),
        (status = 500, description = "Internal server error", body = herald_api_base::application::http::server::api_entities::ErrorResponse)
    )
)]
pub async fn list_api_keys(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Query(query): Query<ListQuery>,
) -> Result<ApiResult<PageResponse<ApiKeyListItem>>, ApiError> {
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

    let offset = (query.page * query.page_size) as u64;
    let limit = query.page_size as u64;

    let (api_keys, total_count) = {
        let keys = state
            .api_key_repo
            .list(&realm_id, offset, limit)
            .await
            .map_err(|e| {
                tracing::error!("Failed to list API keys: {e}");
                ApiError::internal("Failed to list API keys")
            })?;

        let count = state.api_key_repo.count(&realm_id).await.map_err(|e| {
            tracing::error!("Failed to count API keys: {e}");
            ApiError::internal("Failed to count API keys")
        })?;

        (keys, count)
    };

    let items: Vec<ApiKeyListItem> = api_keys
        .into_iter()
        .map(|k| ApiKeyListItem {
            id: k.id,
            name: k.name,
            realm_id: k.realm_id,
            enabled: k.enabled,
            expires_at: k.expires_at.map(|dt| dt.to_rfc3339()),
            last_used_at: k.last_used_at.map(|dt| dt.to_rfc3339()),
            usage_count: k.usage_count,
            created_at: k.created_at.to_rfc3339(),
        })
        .collect();

    Ok(ApiResult::ok(PageResponse {
        items,
        page: query.page,
        page_size: query.page_size,
        total: total_count as i64,
    }))
}
