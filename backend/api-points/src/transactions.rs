// Points Transaction Handlers

use axum::extract::{Extension, Path, Query, State};
use uuid::Uuid;

use crate::types::{ListTransactionsQuery, PointsTransactionResponse};
use herald_api_base::application::http::server::api_entities::{
    ApiError, ApiResult, ErrorResponse, PageResponse,
};
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::points::ports::TransactionFilters;

/// List points transactions with filters
#[utoipa::path(
    get,
    path = "/api/points/{realmId}/transactions",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("userId" = Option<String>, Query, description = "Filter by user ID"),
        ("transactionType" = Option<String>, Query, description = "Filter by transaction type"),
        ("clientAppId" = Option<String>, Query, description = "Filter by client app ID"),
        ("subscriptionId" = Option<String>, Query, description = "Filter by subscription ID"),
        ("startTime" = Option<String>, Query, description = "Filter by start time (ISO 8601)"),
        ("endTime" = Option<String>, Query, description = "Filter by end time (ISO 8601)"),
        ("page" = Option<u64>, Query, description = "Page number (0-based, default: 0)"),
        ("pageSize" = Option<u64>, Query, description = "Page size (default: 20, max: 100)")
    ),
    responses(
        (status = 200, description = "Transactions retrieved successfully", body = PageResponse<PointsTransactionResponse>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Points"
)]
pub async fn list_transactions(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Query(query): Query<ListTransactionsQuery>,
) -> Result<ApiResult<PageResponse<PointsTransactionResponse>>, ApiError> {
    // Parse filters
    let user_id = query.user_id.and_then(|s| s.parse::<Uuid>().ok());

    let client_app_id = query.client_app_id.and_then(|s| s.parse::<Uuid>().ok());

    let subscription_id = query.subscription_id.and_then(|s| s.parse::<Uuid>().ok());

    let transaction_type = query.transaction_type.and_then(|s| {
        s.parse::<herald_core::domain::points::TransactionType>()
            .ok()
    });

    let start_time = query.start_time.and_then(|s| {
        chrono::DateTime::parse_from_rfc3339(&s)
            .ok()
            .map(|dt| dt.with_timezone(&chrono::Utc))
    });

    let end_time = query.end_time.and_then(|s| {
        chrono::DateTime::parse_from_rfc3339(&s)
            .ok()
            .map(|dt| dt.with_timezone(&chrono::Utc))
    });

    let filters = TransactionFilters {
        user_id,
        transaction_type,
        client_app_id,
        subscription_id,
        external_ref_id: query.external_ref_id.unwrap_or_default(),
        start_time,
        end_time,
        page: query.page,
        page_size: query.page_size,
    };

    match state
        .points_service
        .list_transactions(identity, &realm_id, filters)
        .await
    {
        Ok(paginated) => {
            let data = paginated
                .data
                .into_iter()
                .map(|transaction| PointsTransactionResponse {
                    id: transaction.id,
                    wallet_id: transaction.wallet_id,
                    user_id: transaction.user_id,
                    realm_id: transaction.realm_id,
                    transaction_type: transaction.transaction_type.as_str().to_string(),
                    amount: transaction.amount,
                    balance_after: transaction.balance_after,
                    description: transaction.description,
                    client_app_id: transaction.client_app_id,
                    subscription_id: transaction.subscription_id,
                    external_ref_id: transaction.external_ref_id,
                    created_at: transaction.created_at.to_rfc3339(),
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
            _ => ApiError::internal("Internal server error"),
        }),
    }
}
