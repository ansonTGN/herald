// Herald API Points Module
// Points, wallets, transactions, realm configs

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
        crate::realm_configs::get_realm_default_config,
        crate::realm_configs::create_realm_default_config,
        crate::realm_configs::update_realm_default_config,
        crate::user_configs::get_user_points_config,
        crate::grant::grant_points,
    ),
    components(schemas(
        crate::types::ConsumePointsRequest,
        crate::types::ConsumePointsResponse,
        crate::types::ListTransactionsQuery,
        crate::types::ListWalletsQuery,
        crate::types::PointsWalletResponse,
        crate::types::PointsBalanceResponse,
        crate::types::PointsTransactionResponse,
        crate::types::BalancesByType,
        crate::types::WalletByBucketResponse,
        crate::types::ListWalletsByBucketResponse,
        crate::types::RealmDefaultConfigResponse,
        crate::types::CreateRealmConfigRequest,
        crate::types::UpdateRealmConfigRequest,
        crate::types::UserPointsConfigResponse,
        crate::types::GrantPointsRequest,
        crate::types::GrantPointsResponse,
    ))
)]
pub struct ApiDoc;

// Re-export routes for use by the main api crate
pub use routes::points_router;
// Re-export auth_middleware for server routing
pub use auth_middleware::flexible_auth_middleware;
