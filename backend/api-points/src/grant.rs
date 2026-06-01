// Points Grant Handler (Admin)

use axum::{
    Json,
    extract::{Extension, Path, State},
};
use uuid::Uuid;

use crate::types::{GrantPointsRequest, GrantPointsResponse};
use herald_api_base::application::http::auth::util::require_permission;
use herald_api_base::application::http::common::auth_utils::require_authenticated_user_in_realm;
use herald_api_base::application::http::server::api_entities::{ApiError, ErrorResponse};
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::points::dtos::GrantPointsInput;
use herald_core::domain::points::entities::CreditSourceType;

/// Grant points to a user (admin)
#[utoipa::path(
    post,
    path = "/api/points/{realmId}/grant",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    request_body = GrantPointsRequest,
    responses(
        (status = 200, description = "Points granted successfully", body = GrantPointsResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Points"
)]
pub async fn grant_points(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Json(request): Json<GrantPointsRequest>,
) -> Result<Json<GrantPointsResponse>, ApiError> {
    let user_id = require_authenticated_user_in_realm(&identity, &realm_id, "points grant")?;
    require_permission(
        &state,
        &realm_id,
        &user_id.to_string(),
        "points",
        "manage",
        "points.manage",
    )
    .await?;

    // Parse user_id as UUID
    let user_id = request
        .user_id
        .parse::<Uuid>()
        .map_err(|_| ApiError::bad_request("Invalid user ID"))?;

    // Validate amount > 0
    if request.amount <= 0 {
        return Err(ApiError::bad_request("Amount must be greater than 0"));
    }

    // Validate reason non-empty
    if request.reason.trim().is_empty() {
        return Err(ApiError::bad_request("Reason must not be empty"));
    }

    // Validate validity_days is None or > 0
    if let Some(days) = request.validity_days
        && days <= 0
    {
        return Err(ApiError::bad_request(
            "Validity days must be greater than 0",
        ));
    }

    let input = GrantPointsInput {
        user_id,
        amount: request.amount,
        reason: request.reason,
        validity_days: request.validity_days,
        source_type: CreditSourceType::AdminGrant,
        source_id: identity.user_id(),
    };

    match state
        .points_service
        .grant_points(identity, &realm_id, input)
        .await
    {
        Ok(output) => Ok(Json(GrantPointsResponse {
            transaction_id: output.transaction_id,
            user_id: output.user_id,
            amount: output.amount,
            granted_balance: output.granted_balance,
            total_balance: output.total_balance,
            expires_at: output.expires_at.map(|dt| dt.to_rfc3339()),
        })),
        Err(e) => Err(ApiError::from(e)),
    }
}
