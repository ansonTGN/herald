use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use herald_api_base::application::http::common::public_helper::query_config_value;
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
/// and the site key if enabled. Turnstile is considered enabled when a
/// `secret_key` config row exists for the realm; the `site_key` is read from
/// its own config row.
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
    // Turnstile is enabled when a secret_key row exists (and is enabled).
    let secret = query_config_value(&state.pool, &realm_id, "turnstile", "secret_key").await?;

    if secret.is_none() {
        return Ok(ApiResult::ok(TurnstileStatusResponse {
            enabled: false,
            site_key: None,
        }));
    }

    let site_key = query_config_value(&state.pool, &realm_id, "turnstile", "site_key").await?;

    Ok(ApiResult::ok(TurnstileStatusResponse {
        enabled: true,
        site_key,
    }))
}
