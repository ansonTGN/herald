use axum::{
    extract::{Path, State},
    http::HeaderMap,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use herald_api_base::application::http::auth::util::{get_cookie, load_session};
pub use herald_api_base::application::http::server::api_entities::ErrorResponse;
use herald_api_base::application::http::server::api_entities::{ApiError, ApiResult};
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authorization::PermissionService;

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StatusResponse {
    pub authenticated: bool,
    pub realm_id: Option<String>,
    pub user_id: Option<String>,
    pub permissions: Option<Vec<String>>,
}

/// Get authentication status
///
/// Returns the current session status including whether the user is authenticated,
/// their user/realm IDs, and their permissions if valid session exists.
#[utoipa::path(
  get,
  path = "/api/auth/{realmId}/status",
  tag = "auth",
  params(
    ("realmId" = String, Path, description = "Realm ID")
  ),
  responses(
    (status = 200, description = "Session status.", body = StatusResponse),
    (status = 500, description = "Internal server error", body = ErrorResponse)
  )
)]
pub async fn status(
    Path(realm_id): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<ApiResult<StatusResponse>, ApiError> {
    let Some(token) = get_cookie(&headers, "X-Auth") else {
        return Ok(ApiResult::ok(StatusResponse {
            authenticated: false,
            realm_id: None,
            user_id: None,
            permissions: None,
        }));
    };

    let Some(sess) = load_session(&state, &token).await? else {
        return Ok(ApiResult::ok(StatusResponse {
            authenticated: false,
            realm_id: None,
            user_id: None,
            permissions: None,
        }));
    };

    if sess.realm_id != realm_id {
        return Ok(ApiResult::ok(StatusResponse {
            authenticated: false,
            realm_id: None,
            user_id: None,
            permissions: None,
        }));
    }

    // Get user's effective permissions via PermissionService
    let permissions = Some(
        state
            .permission_checker
            .get_user_permissions(&sess.realm_id, &sess.user_id)
            .await
            .map_err(|e| {
                tracing::error!(
                    realm_id = %sess.realm_id,
                    user_id = %sess.user_id,
                    error = %e,
                    "Failed to get user permissions"
                );
                ApiError::internal(format!("Failed to get user permissions: {}", e))
            })?,
    );

    Ok(ApiResult::ok(StatusResponse {
        authenticated: true,
        realm_id: Some(sess.realm_id),
        user_id: Some(sess.user_id),
        permissions,
    }))
}
