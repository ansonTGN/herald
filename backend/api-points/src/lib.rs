// Herald API Points Module
// Points, wallets, transactions, plan configs, realm configs

pub mod auth_middleware;
pub mod grant;
pub mod plan_configs;
pub mod realm_configs;
pub mod routes;
pub mod transactions;
pub mod types;
pub mod user_configs;
pub mod wallets;

/// OpenAPI specification for points module
#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
        crate::wallets::list_wallets,
        crate::wallets::get_wallet,
        crate::transactions::list_transactions,
        crate::plan_configs::list_plan_configs,
        crate::plan_configs::create_plan_config,
        crate::plan_configs::update_plan_config,
        crate::plan_configs::delete_plan_config,
        crate::realm_configs::get_realm_default_config,
        crate::realm_configs::create_realm_default_config,
        crate::realm_configs::update_realm_default_config,
        crate::realm_configs::get_free_user_statistics,
        crate::user_configs::get_user_points_config,
        crate::grant::grant_points,
    ),
    components(schemas(
        crate::types::ConsumePointsRequest,
        crate::types::ConsumePointsResponse,
        crate::types::CreatePlanConfigRequest,
        crate::types::UpdatePlanConfigRequest,
        crate::types::ListTransactionsQuery,
        crate::types::ListWalletsQuery,
        crate::types::PointsWalletResponse,
        crate::types::PointsBalanceResponse,
        crate::types::PointsTransactionResponse,
        crate::types::PointsPlanConfigResponse,
        crate::types::RealmDefaultConfigResponse,
        crate::types::CreateRealmConfigRequest,
        crate::types::UpdateRealmConfigRequest,
        crate::types::UserPointsConfigResponse,
        crate::types::FreeUserStatisticsResponse,
        crate::types::GrantPointsRequest,
        crate::types::GrantPointsResponse,
    ))
)]
pub struct ApiDoc;

// Re-export routes for use by the main api crate
pub use routes::points_router;
// Re-export auth_middleware for server routing
pub use auth_middleware::flexible_auth_middleware;
