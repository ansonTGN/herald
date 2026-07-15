// User Points Config Handlers

use axum::{
    Json,
    extract::{Extension, Path, State},
};
use uuid::Uuid;

use crate::types::UserPointsConfigResponse;
use herald_api_base::application::http::common::auth_utils::AdminIdentity;
use herald_api_base::application::http::server::api_entities::{ApiError, ErrorResponse};
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;

/// Convert domain UserPointsConfig to API response
fn user_config_to_response(
    config: herald_core::domain::points::UserPointsConfig,
) -> UserPointsConfigResponse {
    UserPointsConfigResponse {
        user_id: config.user_id,
        realm_id: config.realm_id,
        registration_bonus_points: config.registration_bonus_points,
        free_periodic_points_amount: config.free_periodic_points_amount,
        free_periodic_grant_period_type: config
            .free_periodic_grant_period_type
            .map(|pt| pt.to_string()),
        free_periodic_validity_days: config.free_periodic_validity_days,
        next_grant_time: config.next_grant_time.map(|dt| dt.to_rfc3339()),
        granted_periods: config.granted_periods,
        grant_schedule_id: config.grant_schedule_id,
        created_at: config.created_at.to_rfc3339(),
        updated_at: config.updated_at.to_rfc3339(),
    }
}

/// Get user points config
#[utoipa::path(
    get,
    path = "/api/points/{realmId}/user-configs/{userId}",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("userId" = String, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User points config retrieved successfully", body = UserPointsConfigResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "points"
)]
pub async fn get_user_points_config(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, user_id)): Path<(String, String)>,
) -> Result<Json<UserPointsConfigResponse>, ApiError> {
    let admin = AdminIdentity::require(identity, &realm_id, "user points config")?;
    admin.require_permission(&state, "points", "view").await?;

    let user_uuid = user_id
        .parse::<Uuid>()
        .map_err(|_| ApiError::bad_request("Invalid user ID"))?;

    match state
        .realm_config_service
        .get_user_points_config(admin.identity().clone(), &realm_id, user_uuid)
        .await
    {
        Ok(config) => Ok(Json(user_config_to_response(config))),
        Err(e) => Err(ApiError::from(e)),
    }
}
