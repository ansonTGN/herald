// Herald API Ext Module
// External API handlers (API Key authentication)

pub mod api_key_auth;
pub mod authz;
pub mod billing;
pub mod client_app;
mod client_app_scope;
pub mod client_helper;
pub mod permission;
pub mod points;
pub mod points_package;
pub mod realm;
pub mod subscription;
pub mod user;

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
        crate::points::get_balance_ext,
        crate::points::consume_points_ext,
        crate::points::grant_points_ext,
        crate::points::get_transaction_ext,
        crate::points::get_transaction_by_external_ref_ext,
        crate::points_package::list_points_packages_ext,
        crate::realm::create_realm,
        crate::realm::list_realms,
        crate::realm::get_realm,
        crate::user::create_user,
        crate::user::list_users,
        crate::user::get_user,
        crate::client_app::create_client_app,
        crate::client_app::list_client_apps,
        crate::client_app::get_client_app,
    ),
    components(schemas(
        crate::permission::PermissionCheckRequest,
        crate::permission::PermissionCheckResponse,
        crate::subscription::SubscriptionResponse,
        crate::billing::SubscriptionDetail,
        crate::points::ExtPointsBalanceResponse,
        crate::points::ExtConsumePointsRequest,
        crate::points::ExtConsumePointsResponse,
        crate::points::ExtGrantPointsRequest,
        crate::points::ExtGrantPointsResponse,
        crate::points::ExtTransactionResponse,
        crate::points_package::ExtPointsPackageItem,
        crate::points_package::ExtPointsPackageListResponse,
        crate::realm::CreateRealmExtRequest,
        crate::realm::AdminUserInput,
        crate::realm::RealmInfoResponse,
        crate::realm::AdminUserOutput,
        crate::realm::RealmListItem,
        crate::realm::RealmListResponse,
        crate::user::CreateUserExtRequest,
        crate::user::UserInfoResponse,
        crate::user::UserListResponse,
        crate::client_app::CreateClientAppExtRequest,
        crate::client_app::ClientAppInfoResponse,
        crate::client_app::ClientAppListItem,
        crate::client_app::ClientAppListResponse,
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
            "/points/{realmId}/balance",
            axum::routing::get(points::get_balance_ext),
        )
        .route(
            "/points/{realmId}/consume",
            axum::routing::post(points::consume_points_ext),
        )
        .route(
            "/points/{realmId}/grant",
            axum::routing::post(points::grant_points_ext),
        )
        .route(
            "/points/{realmId}/transactions/by-external-ref/{externalRefId}",
            axum::routing::get(points::get_transaction_by_external_ref_ext),
        )
        .route(
            "/points/{realmId}/transactions/{transactionId}",
            axum::routing::get(points::get_transaction_ext),
        )
        .route(
            "/{realmId}/points-packages",
            axum::routing::get(points_package::list_points_packages_ext),
        )
        .route(
            "/realms",
            axum::routing::post(realm::create_realm).get(realm::list_realms),
        )
        .route("/realms/{realmId}", axum::routing::get(realm::get_realm))
        .route(
            "/realms/{realmId}/users",
            axum::routing::post(user::create_user).get(user::list_users),
        )
        .route(
            "/realms/{realmId}/users/{userId}",
            axum::routing::get(user::get_user),
        )
        .route(
            "/realms/{realmId}/client-apps",
            axum::routing::post(client_app::create_client_app).get(client_app::list_client_apps),
        )
        .route(
            "/realms/{realmId}/client-apps/{clientAppId}",
            axum::routing::get(client_app::get_client_app),
        )
        .layer(api_key_middleware)
}
