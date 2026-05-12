use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, header::SET_COOKIE},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use herald_api_base::application::http::auth::util::{
    build_clear_cookie, delete_session, get_cookie,
};
use herald_api_base::application::http::server::api_entities::ApiError;
pub use herald_api_base::application::http::server::api_entities::ErrorResponse;
use herald_api_base::application::http::state::AppState;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LogoutResponse {
    pub message: String,
}

/// Logout user and invalidate session
///
/// Clears the session cookie and invalidates the user's session token.
#[utoipa::path(
  get,
  path = "/api/auth/{realmId}/logout",
  tag = "auth",
  params(
    ("realmId" = String, Path, description = "Realm ID")
  ),
  responses(
    (status = 200, description = "Logged out.", body = LogoutResponse),
    (status = 500, description = "Internal server error", body = ErrorResponse)
  )
)]
pub async fn logout(
    Path(_realm_id): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    if let Some(token) = get_cookie(&headers, "X-Auth") {
        // best effort
        let _ = delete_session(&state, &token).await;
    }

    let is_production = state.app_env == "production";
    let clear_cookie = build_clear_cookie("X-Auth", is_production);
    Ok((
        [(SET_COOKIE, clear_cookie)],
        Json(LogoutResponse {
            message: "ok".to_string(),
        }),
    ))
}
