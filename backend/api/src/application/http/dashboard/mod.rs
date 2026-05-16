use axum::Router;
use axum::extract::{Extension, Path, State};
use axum::routing::get;
use herald_core::domain::authentication::Identity;
use herald_core::domain::authorization::PermissionService;
use herald_core::domain::dashboard::DashboardRepository;
use herald_core::infrastructure::dashboard::PostgresDashboardRepository;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::application::http::server::api_entities::{ApiError, ApiResult};
use crate::application::http::state::AppState;

/// User statistics response DTO.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserStatsResponse {
    pub total_users: i64,
    pub new_users: i64,
    pub active_users: i64,
}

/// A single data point in the authentication trend response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuthTrendPointResponse {
    pub date: String,
    pub success_count: i64,
    pub failure_count: i64,
}

/// Aggregated dashboard statistics response DTO.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStatsResponse {
    pub user_stats: UserStatsResponse,
    pub auth_trend: Vec<AuthTrendPointResponse>,
}

/// Get dashboard statistics for a realm
#[utoipa::path(
    get,
    path = "/api/dashboard/{realmId}/stats",
    tag = "dashboard",
    params(("realmId" = String, Path, description = "Realm ID")),
    responses(
        (status = 200, description = "Dashboard statistics", body = DashboardStatsResponse),
        (status = 401, description = "Unauthorized", body = crate::application::http::server::api_entities::ErrorResponse),
        (status = 403, description = "Forbidden", body = crate::application::http::server::api_entities::ErrorResponse),
        (status = 404, description = "Realm not found", body = crate::application::http::server::api_entities::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_dashboard_stats(
    State(state): State<AppState>,
    Path(realm_id): Path<String>,
    Extension(identity): Extension<Identity>,
) -> Result<ApiResult<DashboardStatsResponse>, ApiError> {
    let has_permission = state
        .permission_checker
        .check_permission(&identity.realm_id(), &identity.user_id(), "realm", "admin")
        .await
        .map_err(|e| {
            tracing::error!("Failed to check dashboard access permission: {e}");
            ApiError::internal("Failed to check permission")
        })?;

    if !has_permission {
        return Err(ApiError::forbidden(
            "Insufficient permissions to view dashboard statistics",
        ));
    }

    if !identity.has_access_to_realm(&realm_id) {
        return Err(ApiError::forbidden(
            "Access denied: cannot view dashboard for a different realm",
        ));
    }

    let repo = PostgresDashboardRepository::new(state.db.clone(), state.pool.clone());
    let stats = repo.get_stats(&realm_id).await.map_err(|e| {
        tracing::error!("Failed to fetch dashboard statistics: {e}");
        ApiError::internal("Failed to fetch dashboard statistics")
    })?;

    let response = DashboardStatsResponse {
        user_stats: UserStatsResponse {
            total_users: stats.user_stats.total_users,
            new_users: stats.user_stats.new_users,
            active_users: stats.user_stats.active_users,
        },
        auth_trend: stats
            .auth_trend
            .into_iter()
            .map(|p| AuthTrendPointResponse {
                date: p.date,
                success_count: p.success_count,
                failure_count: p.failure_count,
            })
            .collect(),
    };

    Ok(ApiResult::ok(response))
}

/// Dashboard router with stats endpoint
pub fn dashboard_router() -> Router<AppState> {
    Router::new().route("/stats", get(get_dashboard_stats))
}
