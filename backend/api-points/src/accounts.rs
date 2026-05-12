// Points Account Handlers

use axum::{
    Json,
    extract::{Extension, Path, Query, State},
};
use uuid::Uuid;

use crate::types::{ListAccountsQuery, PointsAccountResponse};
use herald_api_base::application::http::server::api_entities::{
    ApiError, ApiResult, ErrorResponse, PageResponse,
};
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::points::ports::AccountFilters;

/// List all points accounts in a realm (admin only)
#[utoipa::path(
    get,
    path = "/api/points/{realmId}/accounts",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("status" = Option<String>, Query, description = "Filter by account status"),
        ("search" = Option<String>, Query, description = "Search by user ID"),
        ("page" = Option<u64>, Query, description = "Page number (0-based, default: 0)"),
        ("pageSize" = Option<u64>, Query, description = "Page size (default: 20, max: 100)")
    ),
    responses(
        (status = 200, description = "Accounts retrieved successfully", body = PageResponse<PointsAccountResponse>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Points"
)]
pub async fn list_accounts(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Query(query): Query<ListAccountsQuery>,
) -> Result<ApiResult<PageResponse<PointsAccountResponse>>, ApiError> {
    let filters = AccountFilters {
        status: query.status,
        search: query.search,
        page: query.page,
        page_size: query.page_size,
    };

    match state
        .points_service
        .list_accounts(identity, &realm_id, filters)
        .await
    {
        Ok(paginated) => {
            let data = paginated
                .data
                .into_iter()
                .map(|account| PointsAccountResponse {
                    id: account.id,
                    user_id: account.user_id,
                    realm_id: account.realm_id,
                    balance: account.total_balance,
                    total_recharged: account.total_recharged,
                    total_consumed: account.total_consumed,
                    status: account.status.as_str().to_string(),
                    created_at: account.created_at.to_rfc3339(),
                    updated_at: account.updated_at.to_rfc3339(),
                    currency:
                        herald_api_base::application::http::common::error_codes::POINTS_CURRENCY
                            .to_string(),
                })
                .collect();

            Ok(ApiResult::ok(PageResponse {
                items: data,
                total: paginated.total as i64,
                page: paginated.page as i64,
                page_size: paginated.page_size as i64,
            }))
        }
        Err(e) => Err(match e {
            herald_core::domain::common::entities::app_errors::CoreError::Unauthorized => {
                ApiError::unauthorized("Unauthorized")
            }
            herald_core::domain::common::entities::app_errors::CoreError::Forbidden(msg) => {
                ApiError::forbidden(msg)
            }
            herald_core::domain::common::entities::app_errors::CoreError::NotFound => {
                ApiError::not_found("Points account not found")
            }
            _ => ApiError::internal("Internal server error"),
        }),
    }
}

/// Get points account for a specific user
#[utoipa::path(
    get,
    path = "/api/points/{realmId}/accounts/{userId}",
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
pub async fn get_account(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, user_id)): Path<(String, String)>,
) -> Result<Json<PointsAccountResponse>, ApiError> {
    let user_uuid = user_id
        .parse::<Uuid>()
        .map_err(|_| ApiError::bad_request("Invalid user ID format"))?;
    match state
        .points_service
        .get_account(identity, &realm_id, user_uuid)
        .await
    {
        Ok(account) => Ok(Json(PointsAccountResponse {
            id: account.id,
            user_id: account.user_id,
            realm_id: account.realm_id,
            balance: account.total_balance,
            total_recharged: account.total_recharged,
            total_consumed: account.total_consumed,
            status: account.status.as_str().to_string(),
            created_at: account.created_at.to_rfc3339(),
            updated_at: account.updated_at.to_rfc3339(),
            currency: herald_api_base::application::http::common::error_codes::POINTS_CURRENCY
                .to_string(),
        })),
        Err(e) => Err(ApiError::from(e)),
    }
}
