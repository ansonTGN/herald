// Points Plan Config Handlers

use axum::{
    Json,
    extract::{Extension, Path, State},
    http::StatusCode,
};
use uuid::Uuid;

use crate::types::{
    CreatePlanConfigRequest, ListPlanConfigsResponse, PointsPlanConfigResponse,
    UpdatePlanConfigRequest,
};
use herald_api_base::application::http::auth::util::require_permission;
use herald_api_base::application::http::common::auth_utils::require_authenticated_user_in_realm;
use herald_api_base::application::http::server::api_entities::{ApiError, ErrorResponse};
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::points::dtos::{CreatePlanConfigInput, UpdatePlanConfigInput};

/// List all points plan configs in a realm
#[utoipa::path(
    get,
    path = "/api/points/{realmId}/plan-configs",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    responses(
        (status = 200, description = "Plan configs retrieved successfully", body = ListPlanConfigsResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Points"
)]
pub async fn list_plan_configs(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
) -> Result<Json<ListPlanConfigsResponse>, ApiError> {
    let user_id =
        require_authenticated_user_in_realm(&identity, &realm_id, "points configuration APIs")?;
    require_permission(
        &state,
        &realm_id,
        &user_id.to_string(),
        "points",
        "view",
        "points.view",
    )
    .await?;

    match state
        .points_service
        .list_plan_configs(identity, &realm_id)
        .await
    {
        Ok(configs) => {
            let data: Vec<PointsPlanConfigResponse> = configs
                .into_iter()
                .map(|config| PointsPlanConfigResponse {
                    config_id: config.id,
                    realm_id: config.realm_id,
                    plan_id: config.plan_id,
                    grant_period_type: config.grant_period_type,
                    points_per_period: config.points_per_period,
                    validity_days: config.validity_days,
                    grant_on_subscribe: config.grant_on_subscribe,
                    max_periods: config.max_periods,
                    active: config.active,
                    created_at: config.created_at.to_rfc3339(),
                    updated_at: config.updated_at.to_rfc3339(),
                })
                .collect();

            Ok(Json(ListPlanConfigsResponse { configs: data }))
        }
        Err(e) => Err(ApiError::from(e)),
    }
}

/// Create a new points plan config
#[utoipa::path(
    post,
    path = "/api/points/{realmId}/plan-configs",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    request_body = CreatePlanConfigRequest,
    responses(
        (status = 201, description = "Plan config created successfully", body = PointsPlanConfigResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Points"
)]
pub async fn create_plan_config(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Json(request): Json<CreatePlanConfigRequest>,
) -> Result<(StatusCode, Json<PointsPlanConfigResponse>), ApiError> {
    let user_id =
        require_authenticated_user_in_realm(&identity, &realm_id, "points configuration APIs")?;
    require_permission(
        &state,
        &realm_id,
        &user_id.to_string(),
        "points",
        "manage",
        "points.manage",
    )
    .await?;

    let input = CreatePlanConfigInput {
        plan_id: request.plan_id,
        grant_period_type: request.grant_period_type,
        points_per_period: request.points_per_period,
        validity_days: request.validity_days,
        grant_on_subscribe: request.grant_on_subscribe,
        max_periods: request.max_periods,
    };

    match state
        .points_service
        .create_plan_config(identity, &realm_id, input)
        .await
    {
        Ok(config) => Ok((
            StatusCode::CREATED,
            Json(PointsPlanConfigResponse {
                config_id: config.id,
                realm_id: config.realm_id,
                plan_id: config.plan_id,
                grant_period_type: config.grant_period_type,
                points_per_period: config.points_per_period,
                validity_days: config.validity_days,
                grant_on_subscribe: config.grant_on_subscribe,
                max_periods: config.max_periods,
                active: config.active,
                created_at: config.created_at.to_rfc3339(),
                updated_at: config.updated_at.to_rfc3339(),
            }),
        )),
        Err(e) => Err(ApiError::from(e)),
    }
}

/// Update a points plan config
#[utoipa::path(
    put,
    path = "/api/points/{realmId}/plan-configs/{configId}",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("configId" = String, Path, description = "Config ID")
    ),
    request_body = UpdatePlanConfigRequest,
    responses(
        (status = 200, description = "Plan config updated successfully", body = PointsPlanConfigResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Points"
)]
pub async fn update_plan_config(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, config_id)): Path<(String, String)>,
    Json(request): Json<UpdatePlanConfigRequest>,
) -> Result<Json<PointsPlanConfigResponse>, ApiError> {
    let user_id =
        require_authenticated_user_in_realm(&identity, &realm_id, "points configuration APIs")?;
    require_permission(
        &state,
        &realm_id,
        &user_id.to_string(),
        "points",
        "manage",
        "points.manage",
    )
    .await?;

    let config_uuid = config_id
        .parse::<Uuid>()
        .map_err(|_| ApiError::bad_request("Invalid config ID"))?;

    let input = UpdatePlanConfigInput {
        grant_period_type: request.grant_period_type,
        points_per_period: request.points_per_period,
        validity_days: request.validity_days,
        grant_on_subscribe: request.grant_on_subscribe,
        max_periods: request.max_periods,
    };

    match state
        .points_service
        .update_plan_config(identity, &realm_id, config_uuid, input)
        .await
    {
        Ok(config) => Ok(Json(PointsPlanConfigResponse {
            config_id: config.id,
            realm_id: config.realm_id,
            plan_id: config.plan_id,
            grant_period_type: config.grant_period_type,
            points_per_period: config.points_per_period,
            validity_days: config.validity_days,
            grant_on_subscribe: config.grant_on_subscribe,
            max_periods: config.max_periods,
            active: config.active,
            created_at: config.created_at.to_rfc3339(),
            updated_at: config.updated_at.to_rfc3339(),
        })),
        Err(e) => Err(ApiError::from(e)),
    }
}

/// Delete a points plan config
#[utoipa::path(
    delete,
    path = "/api/points/{realmId}/plan-configs/{configId}",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("configId" = String, Path, description = "Config ID")
    ),
    responses(
        (status = 204, description = "Plan config deleted successfully"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Points"
)]
pub async fn delete_plan_config(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, config_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let user_id =
        require_authenticated_user_in_realm(&identity, &realm_id, "points configuration APIs")?;
    require_permission(
        &state,
        &realm_id,
        &user_id.to_string(),
        "points",
        "manage",
        "points.manage",
    )
    .await?;

    let config_uuid = config_id
        .parse::<Uuid>()
        .map_err(|_| ApiError::bad_request("Invalid config ID"))?;

    match state
        .points_service
        .delete_plan_config(identity, &realm_id, config_uuid)
        .await
    {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
        Err(e) => Err(ApiError::from(e)),
    }
}
