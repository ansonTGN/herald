use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use herald_api_base::application::http::common::public_helper::query_config_with_metadata;
pub use herald_api_base::application::http::server::api_entities::ErrorResponse;
use herald_api_base::application::http::server::api_entities::{ApiError, ApiResult};
use herald_api_base::application::http::state::AppState;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct TurnstileStatusResponse {
    pub enabled: bool,
    pub site_key: Option<String>,
}

/// Get Turnstile configuration status for a realm
///
/// Returns whether Turnstile verification is enabled for the specified realm
/// and the site key if enabled.
#[utoipa::path(
    post,
    path = "/api/auth/{realmId}/turnstile/status",
    tag = "auth",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    responses(
        (status = 200, description = "Turnstile status retrieved successfully", body = TurnstileStatusResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_turnstile_status(
    Path(realm_id): Path<String>,
    State(state): State<AppState>,
) -> Result<ApiResult<TurnstileStatusResponse>, ApiError> {
    let row =
        query_config_with_metadata(&state.pool, &realm_id, "turnstile", "site_secret").await?;

    match row {
        Some((_secret, metadata)) => {
            // Turnstile is configured and enabled
            let site_key = metadata
                .as_ref()
                .and_then(|value| value.get("site_key"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            Ok(ApiResult::ok(TurnstileStatusResponse {
                enabled: true,
                site_key,
            }))
        }
        None => {
            // Turnstile not configured or disabled
            Ok(ApiResult::ok(TurnstileStatusResponse {
                enabled: false,
                site_key: None,
            }))
        }
    }
}
