use axum::{
    extract::{Extension, Path, Query, State},
    http::HeaderMap,
};
use herald_core::domain::authentication::Identity;

use crate::application::http::client_apps::types::{ClientAppItem, ListQuery};
use crate::application::http::server::api_entities::{ApiError, ApiResult, PageResponse};
use crate::application::http::state::AppState;
use herald_core::domain::client::ports::ClientService;

/// List all client apps for a realm
///
/// Returns a list of OAuth client applications configured for the specified realm.
#[utoipa::path(
    get,
    path = "/api/client/{realmId}",
    tag = "client",
    summary = "List client applications",
    description = "List all OAuth client applications configured for the specified realm with pagination. Requires `clients.view` permission.",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("page" = Option<i64>, Query, description = "Page number (0-based, default 0)"),
        ("pageSize" = Option<i64>, Query, description = "Page size (default 20)"),
    ),
    responses(
        (status = 200, description = "ClientApp list", body = PageResponse<ClientAppItem>),
        (status = 403, description = "Forbidden - Insufficient permissions (requires clients.view)", body = crate::application::http::server::api_entities::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::application::http::server::api_entities::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_client_apps(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Query(query): Query<ListQuery>,
    _headers: HeaderMap,
) -> Result<ApiResult<PageResponse<ClientAppItem>>, ApiError> {
    // Extract identity from request extension (injected by inject_identity middleware)
    let identity_realm_id = identity.realm_id();
    let current_user_id = identity.user_id();

    tracing::debug!(
        realm_id = %identity_realm_id,
        user_id = %current_user_id,
        "Listing client apps"
    );

    // Call service layer with pagination
    let client_service = state.service.client_service();
    let page = query.page as u64;
    let page_size = query.page_size as u64;
    let (client_apps, total_count) = client_service
        .list_client_apps_paginated(identity, realm_id, page, page_size)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list client apps: {}", e);
            ApiError::internal(format!("Failed to list client apps: {e}"))
        })?;

    // Convert domain models to API response models
    let data: Vec<ClientAppItem> = client_apps
        .into_iter()
        .map(|app| ClientAppItem {
            id: app.id,
            realm_id: app.realm_id,
            client_id: app.client_id,
            name: app.name,
            description: app.description,
            redirect_uris: app.redirect_uris,
            enabled: app.enabled,
            icon_url: app.icon_url,
            session_ttl_seconds: app.session_ttl_seconds,
            session_renewal_ttl_seconds: app.session_renewal_ttl_seconds,
            client_secret: None,
        })
        .collect();

    Ok(ApiResult::ok(PageResponse {
        items: data,
        page: query.page,
        page_size: query.page_size,
        total: total_count as i64,
    }))
}
