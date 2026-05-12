use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use herald_api_base::application::http::common::public_helper::{parse_bool, query_config_value};
pub use herald_api_base::application::http::server::api_entities::ErrorResponse;
use herald_api_base::application::http::server::api_entities::{ApiError, ApiResult};
use herald_api_base::application::http::state::AppState;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct RegistrationStatusResponse {
    pub enabled: bool,
}

/// Get registration configuration status for a realm
///
/// Returns whether user registration is enabled for the specified realm.
#[utoipa::path(
    post,
    path = "/api/auth/{realmId}/registration/status",
    tag = "auth",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    responses(
        (status = 200, description = "Registration status retrieved successfully", body = RegistrationStatusResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_registration_status(
    Path(realm_id): Path<String>,
    State(state): State<AppState>,
) -> Result<ApiResult<RegistrationStatusResponse>, ApiError> {
    let value = query_config_value(&state.pool, &realm_id, "registration", "allowed").await?;

    // Parse the config value as boolean
    let enabled = value.and_then(|v| parse_bool(&v)).unwrap_or(false); // Default to disabled if no config found

    Ok(ApiResult::ok(RegistrationStatusResponse { enabled }))
}
