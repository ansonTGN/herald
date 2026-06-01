// Points Wallet Handlers

use axum::{
    Json,
    extract::{Extension, Path, Query, State},
};
use uuid::Uuid;

use crate::types::{ListWalletsQuery, PointsWalletResponse};
use herald_api_base::application::http::auth::util::require_permission;
use herald_api_base::application::http::common::auth_utils::require_authenticated_user_in_realm;
use herald_api_base::application::http::common::error_codes::POINTS_UNIT;
use herald_api_base::application::http::server::api_entities::{
    ApiError, ApiResult, ErrorResponse, PageResponse,
};
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::points::ports::WalletFilters;

fn wallet_to_response(
    account: herald_core::domain::points::entities::PointsWallet,
) -> PointsWalletResponse {
    PointsWalletResponse {
        id: account.id,
        user_id: account.user_id,
        realm_id: account.realm_id,
        balance: account.total_balance,
        total_recharged: account.total_recharged,
        total_consumed: account.total_consumed,
        status: account.status.as_str().to_string(),
        created_at: account.created_at.to_rfc3339(),
        updated_at: account.updated_at.to_rfc3339(),
        unit: POINTS_UNIT.to_string(),
        currency: POINTS_UNIT.to_string(),
    }
}

/// List all points wallets in a realm (admin only)
#[utoipa::path(
    get,
    path = "/api/points/{realmId}/wallets",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("status" = Option<String>, Query, description = "Filter by wallet status"),
        ("search" = Option<String>, Query, description = "Search by user ID"),
        ("page" = Option<u64>, Query, description = "Page number (0-based, default: 0)"),
        ("pageSize" = Option<u64>, Query, description = "Page size (default: 20, max: 100)")
    ),
    responses(
        (status = 200, description = "Accounts retrieved successfully", body = PageResponse<PointsWalletResponse>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Points"
)]
pub async fn list_wallets(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Query(query): Query<ListWalletsQuery>,
) -> Result<ApiResult<PageResponse<PointsWalletResponse>>, ApiError> {
    let user_id = require_authenticated_user_in_realm(&identity, &realm_id, "points wallets")?;
    require_permission(
        &state,
        &realm_id,
        &user_id.to_string(),
        "points",
        "view",
        "points.view",
    )
    .await?;

    let filters = WalletFilters {
        status: query.status,
        search: query.search,
        page: query.page,
        page_size: query.page_size,
    };

    match state
        .points_service
        .list_wallets(identity, &realm_id, filters)
        .await
    {
        Ok(paginated) => {
            let data = paginated.data.into_iter().map(wallet_to_response).collect();

            Ok(ApiResult::ok(PageResponse {
                items: data,
                total: paginated.total as i64,
                page: paginated.page as i64,
                page_size: paginated.page_size as i64,
            }))
        }
        Err(e) => Err(ApiError::from(e)),
    }
}

/// Get points wallet for a specific user
#[utoipa::path(
    get,
    path = "/api/points/{realmId}/wallets/{userId}",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("userId" = String, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "Account retrieved successfully"),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Points"
)]
pub async fn get_wallet(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, user_id)): Path<(String, String)>,
) -> Result<Json<PointsWalletResponse>, ApiError> {
    let _user_id = require_authenticated_user_in_realm(&identity, &realm_id, "points wallet")?;
    require_permission(
        &state,
        &realm_id,
        &_user_id.to_string(),
        "points",
        "view",
        "points.view",
    )
    .await?;

    let user_uuid = user_id
        .parse::<Uuid>()
        .map_err(|_| ApiError::bad_request("Invalid user ID format"))?;
    match state
        .points_service
        .get_wallet(identity, &realm_id, user_uuid)
        .await
    {
        Ok(account) => Ok(Json(wallet_to_response(account))),
        Err(e) => Err(ApiError::from(e)),
    }
}
