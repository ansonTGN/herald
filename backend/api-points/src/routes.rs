// Points API routes

use axum::{Router, routing};

use herald_api_base::application::http::state::AppState;

use super::{
    accounts::{get_account, list_accounts},
    plan_configs::{create_plan_config, delete_plan_config, list_plan_configs, update_plan_config},
    realm_configs::{
        create_realm_default_config, get_free_user_statistics, get_realm_default_config,
        update_realm_default_config,
    },
    transactions::list_transactions,
    user_configs::get_user_points_config,
};

/// Points router with flexible authentication (session or API key)
///
/// This router is meant to be nested under `/api/points/{realmId}` in server/mod.rs.
///
/// Routes (when nested under /api/points/{realmId}):
/// - GET /api/points/{realmId}/accounts - List accounts (admin, session or API key)
/// - GET /api/points/{realmId}/accounts/{userId} - Get account (session or API key)
/// - GET /api/points/{realmId}/transactions - List transactions (session or API key)
/// - GET /api/points/{realmId}/plan-configs - List plan configs (admin session only)
/// - POST /api/points/{realmId}/plan-configs - Create plan config (admin session only)
/// - PUT /api/points/{realmId}/plan-configs/{configId} - Update plan config (admin session only)
/// - DELETE /api/points/{realmId}/plan-configs/{configId} - Delete plan config (admin session only)
/// - GET /api/points/{realmId}/default-config - Get realm default config (admin session only)
/// - POST /api/points/{realmId}/default-config - Create realm default config (admin session only)
/// - PUT /api/points/{realmId}/default-config - Update realm default config (admin session only)
/// - GET /api/points/{realmId}/statistics/free-users - Get free user statistics (admin session only)
/// - GET /api/points/{realmId}/user-configs/{userId} - Get user points config (admin session only)
///
/// Note: Balance and consume endpoints have been moved to /api/ext/points/ for SDK compatibility.
pub fn points_router() -> Router<AppState> {
    Router::new()
        .route("/accounts", routing::get(list_accounts))
        .route("/accounts/{userId}", routing::get(get_account))
        .route("/transactions", routing::get(list_transactions))
        .route(
            "/plan-configs",
            routing::get(list_plan_configs).post(create_plan_config),
        )
        .route(
            "/plan-configs/{configId}",
            routing::put(update_plan_config).delete(delete_plan_config),
        )
        .route(
            "/default-config",
            routing::get(get_realm_default_config)
                .post(create_realm_default_config)
                .put(update_realm_default_config),
        )
        .route(
            "/statistics/free-users",
            routing::get(get_free_user_statistics),
        )
        .route(
            "/user-configs/{userId}",
            routing::get(get_user_points_config),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_creation() {
        let _router = points_router();

        // Routers are created successfully
        // The actual routing will be tested in integration tests
    }
}
