// Realm Default Config Handlers

use axum::{
    Json,
    extract::{Extension, Path, Query, State},
};
use chrono::{DateTime, Utc};
use validator::Validate;

use crate::types::{
    CreateRealmConfigRequest, FreeUserStatisticsQuery, FreeUserStatisticsResponse,
    RealmDefaultConfigResponse, UpdateRealmConfigRequest,
};
use herald_api_base::application::http::common::auth_utils::require_authenticated_user_in_realm;
use herald_api_base::application::http::server::api_entities::{ApiError, ErrorResponse};
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::points::{CreateRealmConfigInput, UpdateRealmConfigInput};

/// Convert domain RealmDefaultConfig to API response
fn realm_config_to_response(
    config: herald_core::domain::points::RealmDefaultConfig,
) -> RealmDefaultConfigResponse {
    RealmDefaultConfigResponse {
        realm_id: config.realm_id,
        registration_bonus_points: config.registration_bonus_points,
        free_periodic_points_amount: config.free_periodic_points_amount,
        free_periodic_grant_period_type: config.free_periodic_grant_period_type.to_string(),
        free_periodic_validity_days: config.free_periodic_validity_days,
        created_at: config.created_at.to_rfc3339(),
        updated_at: config.updated_at.to_rfc3339(),
    }
}

/// Convert domain FreeUserStatistics to API response
fn statistics_to_response(
    stats: herald_core::domain::points::services::realm_config_service::FreeUserStatistics,
) -> FreeUserStatisticsResponse {
    FreeUserStatisticsResponse {
        total_free_users: stats.total_free_users,
        active_free_users: stats.active_free_users,
        total_registration_bonus_granted: stats.total_registration_bonus_granted,
        total_periodic_points_granted: stats.total_periodic_points_granted,
        average_periodic_points_per_user: stats.average_periodic_points_per_user,
        upgrade_rate: stats.upgrade_rate,
        last_updated_at: stats.last_updated_at.to_rfc3339(),
    }
}

fn parse_optional_rfc3339(
    value: Option<String>,
    field_name: &str,
) -> Result<Option<DateTime<Utc>>, ApiError> {
    value
        .map(|raw| {
            DateTime::parse_from_rfc3339(&raw)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|_| {
                    ApiError::bad_request(format!("Invalid {field_name}: must be RFC3339"))
                })
        })
        .transpose()
}

/// Get realm default config
#[utoipa::path(
    get,
    path = "/api/points/{realmId}/default-config",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    responses(
        (status = 200, description = "Realm default config retrieved successfully", body = RealmDefaultConfigResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Points"
)]
pub async fn get_realm_default_config(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
) -> Result<Json<RealmDefaultConfigResponse>, ApiError> {
    let _ = require_authenticated_user_in_realm(&identity, &realm_id, "points configuration APIs")?;

    match state
        .realm_config_service
        .get_realm_config(identity, &realm_id)
        .await
    {
        Ok(config) => Ok(Json(realm_config_to_response(config))),
        Err(e) => Err(ApiError::from(e)),
    }
}

/// Create or initialize realm default config
#[utoipa::path(
    post,
    path = "/api/points/{realmId}/default-config",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    request_body = CreateRealmConfigRequest,
    responses(
        (status = 200, description = "Realm default config created or retrieved successfully", body = RealmDefaultConfigResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Points"
)]
pub async fn create_realm_default_config(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Json(request): Json<CreateRealmConfigRequest>,
) -> Result<Json<RealmDefaultConfigResponse>, ApiError> {
    let _ = require_authenticated_user_in_realm(&identity, &realm_id, "points configuration APIs")?;

    // Validate request
    request
        .validate()
        .map_err(|e| ApiError::bad_request(format!("Invalid request: {}", e)))?;

    let input = CreateRealmConfigInput {
        realm_id: realm_id.clone(),
        registration_bonus_points: request.registration_bonus_points,
        free_periodic_points_amount: request.free_periodic_points_amount,
        free_periodic_grant_period_type: request.free_periodic_grant_period_type,
        free_periodic_validity_days: request.free_periodic_validity_days,
    };

    match state
        .realm_config_service
        .create_realm_config(identity, input)
        .await
    {
        Ok(config) => Ok(Json(realm_config_to_response(config))),
        Err(e) => Err(ApiError::from(e)),
    }
}

/// Update realm default config
#[utoipa::path(
    put,
    path = "/api/points/{realmId}/default-config",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    request_body = UpdateRealmConfigRequest,
    responses(
        (status = 200, description = "Realm default config updated successfully", body = RealmDefaultConfigResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Points"
)]
pub async fn update_realm_default_config(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Json(request): Json<UpdateRealmConfigRequest>,
) -> Result<Json<RealmDefaultConfigResponse>, ApiError> {
    let _ = require_authenticated_user_in_realm(&identity, &realm_id, "points configuration APIs")?;

    // Validate request
    request
        .validate()
        .map_err(|e| ApiError::bad_request(format!("Invalid request: {}", e)))?;

    let input = UpdateRealmConfigInput {
        registration_bonus_points: request.registration_bonus_points,
        free_periodic_points_amount: request.free_periodic_points_amount,
        free_periodic_grant_period_type: request.free_periodic_grant_period_type,
        free_periodic_validity_days: request.free_periodic_validity_days,
    };

    match state
        .realm_config_service
        .update_realm_config(identity, &realm_id, input)
        .await
    {
        Ok(config) => Ok(Json(realm_config_to_response(config))),
        Err(e) => Err(ApiError::from(e)),
    }
}

/// Get free user statistics for a realm
#[utoipa::path(
    get,
    path = "/api/points/{realmId}/statistics/free-users",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        FreeUserStatisticsQuery
    ),
    responses(
        (status = 200, description = "Free user statistics retrieved successfully", body = FreeUserStatisticsResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Points"
)]
pub async fn get_free_user_statistics(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Query(query): Query<FreeUserStatisticsQuery>,
) -> Result<Json<FreeUserStatisticsResponse>, ApiError> {
    let _ = require_authenticated_user_in_realm(&identity, &realm_id, "points configuration APIs")?;

    let start_date = parse_optional_rfc3339(query.start_date, "startDate")?;
    let end_date = parse_optional_rfc3339(query.end_date, "endDate")?;

    match state
        .realm_config_service
        .get_free_user_statistics(identity, &realm_id, start_date, end_date)
        .await
    {
        Ok(stats) => Ok(Json(statistics_to_response(stats))),
        Err(e) => Err(ApiError::from(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_optional_rfc3339_accepts_valid_timestamp() {
        let parsed =
            parse_optional_rfc3339(Some("2026-03-25T10:00:00Z".to_string()), "startDate").unwrap();

        assert_eq!(
            parsed,
            Some(
                DateTime::parse_from_rfc3339("2026-03-25T10:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc)
            )
        );
    }

    #[test]
    fn test_parse_optional_rfc3339_rejects_invalid_timestamp() {
        assert!(parse_optional_rfc3339(Some("not-a-date".to_string()), "endDate").is_err());
    }
}
