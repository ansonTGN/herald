// Herald API Ext Module
// External API handlers (API Key authentication)

pub mod api_key_auth;
pub mod billing;
pub mod client_helper;
pub mod permission;
pub mod points;
pub mod subscription;

#[cfg(test)]
mod api_key_auth_test;

use axum::Router;
use herald_api_base::application::http::state::AppState;

/// OpenAPI specification for external API module
#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
        crate::permission::check_permission,
        crate::subscription::get_subscription,
        crate::billing::get_subscription,
        crate::billing::list_plans,
        crate::billing::list_plan_assignments,
        crate::points::get_balance_ext,
        crate::points::consume_points_ext,
    ),
    components(schemas(
        crate::permission::PermissionCheckRequest,
        crate::permission::PermissionCheckResponse,
        crate::subscription::SubscriptionResponse,
        crate::billing::SubscriptionDetail,
        crate::billing::Plan,
        crate::billing::PlanAssignment,
        crate::billing::PlansListResponse,
        crate::billing::AssignmentsListResponse,
        crate::points::ExtPointsBalanceResponse,
        crate::points::ExtConsumePointsRequest,
        crate::points::ExtConsumePointsResponse,
    ))
)]
pub struct ApiDoc;

/// Create third-party API router
///
/// All routes in this router require API Key authentication.
pub fn create_router(state: AppState) -> Router<AppState> {
    let api_key_middleware =
        axum::middleware::from_fn_with_state(state.clone(), api_key_auth::api_key_auth_middleware);

    Router::new()
        .route(
            "/permission/check",
            axum::routing::post(permission::check_permission),
        )
        .route(
            "/subscription/{clientAppId}",
            axum::routing::get(subscription::get_subscription),
        )
        .route(
            "/bill/{realmId}/client/{clientAppId}/subscription",
            axum::routing::get(billing::get_subscription),
        )
        .route(
            "/bill/{realmId}/plans",
            axum::routing::get(billing::list_plans),
        )
        .route(
            "/bill/{realmId}/client/{clientAppId}/plans",
            axum::routing::get(billing::list_plan_assignments),
        )
        .route(
            "/points/{realmId}/balance",
            axum::routing::get(points::get_balance_ext),
        )
        .route(
            "/points/{realmId}/consume",
            axum::routing::post(points::consume_points_ext),
        )
        .route(
            "/points/{realmId}/transactions/{transactionId}",
            axum::routing::get(points::get_transaction_ext),
        )
        .layer(api_key_middleware)
}
