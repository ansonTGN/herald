// Realm Default Config Handlers

use axum::{
    Json,
    extract::{Extension, Path, State},
};
use validator::Validate;

use crate::types::{
    CreateRealmConfigRequest, RealmDefaultConfigResponse, UpdateRealmConfigRequest,
};
use herald_api_base::application::http::auth::util::require_permission;
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
    let user_id =
        require_authenticated_user_in_realm(&identity, &realm_id, "realm default config")?;
    require_permission(
        &state,
        &realm_id,
        &user_id.to_string(),
        "settings",
        "view",
        "settings.view",
    )
    .await?;

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
    let user_id =
        require_authenticated_user_in_realm(&identity, &realm_id, "realm default config")?;
    require_permission(
        &state,
        &realm_id,
        &user_id.to_string(),
        "settings",
        "manage",
        "settings.manage",
    )
    .await?;

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
    let user_id =
        require_authenticated_user_in_realm(&identity, &realm_id, "realm default config")?;
    require_permission(
        &state,
        &realm_id,
        &user_id.to_string(),
        "settings",
        "manage",
        "settings.manage",
    )
    .await?;

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
