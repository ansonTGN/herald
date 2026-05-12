pub mod api_entities;
pub mod app_state;

use axum::routing::{get, post};
use axum::{
    Json, Router,
    extract::State,
    http::{
        HeaderName, HeaderValue, Method,
        header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    },
};
use serde::Serialize;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    services::ServeDir,
    trace::TraceLayer,
};

use super::points::routes;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::application::http::auth::identity_middleware::inject_identity;
use crate::application::http::state::AppState;
use crate::application::http::{
    admin, auth, billing, client_apps, oauth, permission, points, public_config, realm,
    realm_config, user, users,
};

/// Health check response schema
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct HealthCheckResponse {
    /// Overall service status: "healthy" or "unhealthy"
    pub status: String,
    /// PostgreSQL connection status
    pub database: bool,
    /// Redis connection status
    pub redis: bool,
    /// Service version from Cargo.toml
    pub version: String,
    /// Service uptime in seconds since startup
    pub uptime: u64,
    /// Current timestamp in RFC3339 format
    pub timestamp: String,
}

/// Local OpenAPI spec for modules remaining in the api crate
/// Sub-crate specs are merged at runtime via build_openapi_spec()
#[derive(OpenApi)]
#[openapi(
    paths(
        realm_config::list_realm_configs,
        realm_config::list_realm_configs_by_type,
        realm_config::get_realm_config,
        realm_config::upsert_realm_config,
        realm_config::batch_upsert_realm_configs,
        realm_config::delete_realm_config,
        realm::crud::list_realms,
        realm::crud::list_realms_paginated,
        realm::crud::get_realm,
        realm::crud::create_realm,
        realm::crud::update_realm,
        user::roles::get_user_roles,
        user::get_profile,
        user::update_profile,
        user::change_password,
        client_apps::list_client_apps,
        client_apps::create_client_app,
        client_apps::get_client_app,
        client_apps::update_client_app,
        client_apps::delete_client_app,
        users::handle_enable_totp,
        users::handle_verify_totp_setup,
        users::handle_disable_totp,
        users::handle_regenerate_totp,
        users::handle_get_totp_status,
        realm::totp_config::handle_update_realm_totp_config,
        realm::totp_config::handle_get_realm_totp_config,
        public_config::get_public_config,
        health_check,
    ),
    components(
        schemas(
            api_entities::ErrorResponse,
            api_entities::PageResponse<client_apps::types::ClientAppItem>,
            api_entities::PageResponse<points::types::PointsAccountResponse>,
            api_entities::PageResponse<points::types::PointsTransactionResponse>,
            api_entities::PageResponse<realm::RealmResponse>,
            user::profile::ChangePasswordRequest,
            user::profile::UserProfile,
            user::profile::UpdateProfileRequest,
            user::roles::UserProfileRolesResponse,
            realm_config::UpsertRealmConfigValidator,
            realm_config::BatchUpsertRealmConfigValidator,
            realm_config::RealmConfigResponse,
            realm::ListRealmsQuery,
            realm::ListRealmsResponse,
            realm::ListRealmsPaginatedQuery,
            realm::CreateRealmValidator,
            realm::UpdateRealmValidator,
            realm::RealmResponse,
            client_apps::types::ClientAppCreateRequest,
            client_apps::types::ClientAppUpdateRequest,
            client_apps::types::ClientAppItem,
            client_apps::types::PaginationMeta,
            users::user_totp::EnableTotpRequest,
            users::user_totp::EnableTotpResponse,
            users::user_totp::VerifyTotpSetupRequest,
            users::user_totp::VerifyTotpSetupResponse,
            users::user_totp::DisableTotpRequest,
            users::user_totp::DisableTotpResponse,
            users::user_totp::RegenerateTotpRequest,
            users::user_totp::RegenerateTotpResponse,
            users::user_totp::TotpStatusResponse,
            users::user_totp::BackupCodeStatsResponse,
            realm::totp_config::UpdateRealmTotpConfigRequest,
            realm::totp_config::UpdateRealmTotpConfigResponse,
            realm::totp_config::GetRealmTotpConfigResponse,
            realm::totp_config::RealmTotpStatisticsResponse,
            public_config::PublicConfigResponse,
            public_config::RegistrationConfig,
            public_config::OAuthProviderInfo,
            HealthCheckResponse,
        )
    ),
    tags(
        (name = "auth", description = "Authentication & authorization APIs"),
        (name = "user", description = "User personal center APIs"),
        (name = "users", description = "User management APIs (admin)"),
        (name = "oauth", description = "OAuth provider authentication APIs"),
        (name = "realm_config", description = "Realm configuration management APIs"),
        (name = "realms", description = "Realm management APIs"),
        (name = "billing", description = "Billing and subscription management APIs"),
        (name = "billing.payment-providers", description = "Payment provider configuration APIs"),
        (name = "points", description = "Points and virtual currency APIs"),
        (name = "ext", description = "External API (API Key authentication)"),
        (name = "system", description = "System health and monitoring APIs")
    )
)]
pub struct ApiDoc;

/// Build the complete OpenAPI spec by merging local paths with sub-crate specs
pub fn build_openapi_spec() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
        .merge_from(herald_api_auth::ApiDoc::openapi())
        .merge_from(herald_api_admin::ApiDoc::openapi())
        .merge_from(herald_api_billing::ApiDoc::openapi())
        .merge_from(herald_api_oauth::ApiDoc::openapi())
        .merge_from(herald_api_points::ApiDoc::openapi())
        .merge_from(herald_api_ext::ApiDoc::openapi())
}

pub fn create_router(
    state: Arc<AppState>,
    frontend_url: String,
    static_dir: Option<String>,
) -> Router {
    // Build CORS layer
    // Note: frontend_url is validated in main.rs before calling this function
    let frontend_origin = frontend_url
        .parse::<HeaderValue>()
        .expect("Frontend URL validation failed: should have been caught in main.rs");
    let cors = CorsLayer::new()
        .allow_origin(frontend_origin)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            AUTHORIZATION,
            CONTENT_TYPE,
            ACCEPT,
            HeaderName::from_static("x-request-id"),
        ])
        .allow_credentials(true)
        .expose_headers([HeaderName::from_static("x-request-id")]);

    // All API routes (OAuth, Realm Config, Auth, Permission, Client, Roles, User, Users, Realms, Billing)
    let api_routes = create_api_routes(state.clone());

    // Define the request ID header name
    let request_id_header_name = HeaderName::from_static("x-request-id");

    // Merge all stateful routers and convert to stateless by calling with_state
    // api_routes is Router<AppState>, needs with_state
    // health_route is Router<AppState>, needs with_state
    let router = Router::new()
        .merge(api_routes.with_state((*state).clone()))
        .route("/health", get(health_check).with_state(state.clone()))
        .merge(SwaggerUi::new("/swagger").url("/api-docs/openapi.json", build_openapi_spec()))
        .layer(
            ServiceBuilder::new()
                // 1. If request doesn't have X-Request-ID, generate a new UUID
                .layer(SetRequestIdLayer::new(
                    request_id_header_name.clone(),
                    MakeRequestUuid,
                ))
                // 2. Propagate X-Request-ID to downstream services (if any)
                .layer(PropagateRequestIdLayer::new(request_id_header_name))
                // 3. HTTP request tracing (automatically logs request_id)
                .layer(TraceLayer::new_for_http())
                .layer(cors),
        );

    // println!("{router:?}");
    if let Some(dir) = static_dir {
        tracing::info!("Serving static files from: {}", dir);
        router.nest_service(
            "/",
            ServeDir::new(&dir)
                .fallback(ServeDir::new(&dir).append_index_html_on_directories(false)),
        )
    } else {
        router
    }
}

/// Create API routes for both production and testing
///
/// This function extracts the core API routing logic so it can be reused
/// in both production (create_router) and test (create_unified_test_router) contexts.
/// This eliminates code duplication and ensures route consistency.
///
/// # Arguments
///
/// * `state` - Application state shared across all routes
///
/// # Returns
///
/// A configured Router with all API routes (no middleware layers)
pub fn create_api_routes(state: Arc<AppState>) -> Router<AppState> {
    use axum::middleware::from_fn_with_state;

    let auth_routes = auth::auth_router();
    let admin_routes = admin::admin_router_with_middleware((*state).clone());
    let realm_routes = realm::realm_router();
    let billing_routes = billing::billing_routes();

    // Test routes - only included in test builds
    #[cfg(test)]
    let billing_test_routes = billing::billing_test_routes();

    let router = Router::new()
        // Public configuration endpoint (no authentication required) - must come before other nested routes
        .route(
            "/api/public-config/{realmId}",
            get(super::public_config::get_public_config),
        )
        // OAuth routes
        .route(
            "/api/oauth/{realmId}/authorize",
            get(oauth::oauth_authorize),
        )
        .route(
            "/api/oauth/{realmId}/{provider}/login",
            get(oauth::oauth_login),
        )
        .route(
            "/api/oauth/{realmId}/{provider}/callback",
            get(oauth::oauth_callback),
        )
        // WeChat specific routes
        .route(
            "/api/oauth/{realmId}/wechat/login",
            get(oauth::wechat_login),
        )
        .route(
            "/api/oauth/{realmId}/wechat/callback",
            get(oauth::wechat_callback),
        )
        .route(
            "/api/oauth/{realmId}/wechat-miniprogram/login",
            post(oauth::wechat_miniprogram_login),
        )
        .nest(
            "/api/oauth/{realmId}/configs",
            Router::new()
                .route(
                    "/",
                    get(oauth::list_oauth_configs).post(oauth::create_oauth_config),
                )
                .route(
                    "/{providerType}",
                    get(oauth::get_oauth_config)
                        .put(oauth::update_oauth_config)
                        .delete(oauth::delete_oauth_config),
                )
                .layer(from_fn_with_state((*state).clone(), inject_identity)),
        )
        // Realm Config routes
        .nest(
            "/api/configs",
            Router::new()
                .route(
                    "/{realmId}",
                    get(realm_config::list_realm_configs).put(realm_config::upsert_realm_config),
                )
                .route(
                    "/{realmId}/batch",
                    post(realm_config::batch_upsert_realm_configs),
                )
                .route(
                    "/{realmId}/{configType}",
                    get(realm_config::list_realm_configs_by_type),
                )
                .route(
                    "/{realmId}/{configType}/{configKey}",
                    get(realm_config::get_realm_config).delete(realm_config::delete_realm_config),
                )
                .layer(from_fn_with_state((*state).clone(), inject_identity)),
        )
        // Auth routes
        .nest("/api/auth/{realmId}", auth_routes)
        // Permission routes: /check endpoint (NO middleware) + others (WITH middleware)
        .route(
            "/api/permission/check",
            axum::routing::post(crate::application::http::admin::permission::check_permission),
        )
        .nest(
            "/api/permission",
            permission::permission_router()
                .layer(from_fn_with_state((*state).clone(), inject_identity)),
        )
        .nest(
            "/api/client/{realmId}",
            client_apps::router().layer(from_fn_with_state((*state).clone(), inject_identity)),
        )
        .nest("/api/roles", admin_routes)
        // Personal center routes (tag = "user") - no realmId in prefix
        .nest(
            "/api/user",
            user::router()
                .merge(users::router())
                .layer(from_fn_with_state((*state).clone(), inject_identity)),
        )
        // Admin user management (tag = "users") - realm_id required
        .nest(
            "/api/users/{realmId}",
            admin::admin_users::router()
                .layer(from_fn_with_state((*state).clone(), inject_identity)),
        )
        .nest(
            "/api/realms",
            realm_routes.layer(from_fn_with_state((*state).clone(), inject_identity)),
        )
        .merge(billing::billing_public_routes())
        .merge(billing_routes.layer(from_fn_with_state((*state).clone(), inject_identity)))
        // Points endpoints - flexible authentication (session or API key)
        .nest(
            "/api/points/{realmId}",
            routes::points_router().layer(from_fn_with_state(
                (*state).clone(),
                crate::application::http::points::auth_middleware::flexible_auth_middleware,
            )),
        )
        // External API routes
        .nest("/api/ext", super::ext::create_router((*state).clone()));

    #[cfg(test)]
    let router = router.merge(billing_test_routes);

    router
}

/// Health check endpoint for monitoring and orchestration
///
/// Used by:
/// - Kubernetes liveness probes (is the service running?)
/// - Kubernetes readiness probes (can the service handle traffic?)
/// - Monitoring systems (Prometheus, Datadog, etc.)
/// - Load balancers (health checks)
///
/// # Health Criteria
///
/// The service is considered **healthy** when:
/// - PostgreSQL database is reachable (`SELECT 1` succeeds)
/// - Redis cache is reachable (PING succeeds)
///
/// # Response Codes
///
/// - **200 OK**: Service is healthy
/// - **503 Service Unavailable**: Service is unhealthy
#[utoipa::path(
    get,
    path = "/health",
    tag = "system",
    responses(
        (status = 200, description = "Service is healthy", body = HealthCheckResponse),
        (status = 503, description = "Service is unhealthy", body = HealthCheckResponse)
    )
)]
async fn health_check(State(state): State<Arc<AppState>>) -> Json<HealthCheckResponse> {
    // Check database connection
    let db_healthy = sqlx::query("SELECT 1").fetch_one(&state.pool).await.is_ok();

    // Check Redis connection
    let redis_healthy = state.redis_manager.health_check().await.is_ok();

    let status = if db_healthy && redis_healthy {
        "healthy"
    } else {
        "unhealthy"
    };

    // Calculate uptime in seconds
    let uptime = state.startup_time.elapsed().as_secs();

    // Get version from env var (set by Cargo during build)
    let version = env!("CARGO_PKG_VERSION");

    // Get current timestamp
    let timestamp = chrono::Utc::now().to_rfc3339();

    Json(HealthCheckResponse {
        status: status.to_string(),
        database: db_healthy,
        redis: redis_healthy,
        version: version.to_string(),
        uptime,
        timestamp,
    })
}
