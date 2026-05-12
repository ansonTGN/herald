use sqlx::PgPool;
use std::sync::Arc;
use std::time::Instant;

use herald_core::application::{ApplicationService, WebhookService};
use herald_core::domain::payment_attempt::PaymentAttemptService;
use herald_core::domain::points_package::PointsPackageService;
use herald_core::domain::user::services::admin::{
    AdminUserServiceImpl, PermissionManagementServiceImpl, RoleAssignmentServiceImpl,
    UserPermissionServiceImpl,
};
use herald_core::infrastructure::authorization::RedisPermissionChecker;
use herald_core::infrastructure::authorization::policies::PermissionBasedPointsPolicy;
use herald_core::infrastructure::billing::{PostgresBillingRepository, PostgresInvoiceRepository};
use herald_core::infrastructure::client_api_keys::{ApiKeyCache, ClientApiKeyRepository};
use herald_core::infrastructure::payment_attempt::PostgresPaymentAttemptRepository;
use herald_core::infrastructure::points::PostgresPointsRepository;
use herald_core::infrastructure::points_package::PostgresPointsPackageRepository;
use herald_core::infrastructure::purchase::PurchaseService;
use herald_core::infrastructure::purchase::{
    PostgresFulfillmentService, PostgresPurchaseRepository,
};
use herald_core::infrastructure::redis::RedisConnectionManager;
use herald_core::infrastructure::user::{
    PostgresAdminUserRepository, PostgresRolePolicyRepository, PostgresUserRoleRepository,
    repositories::PostgresUserRepository,
};
use herald_core::third::email::ResendClient;
use sea_orm::DatabaseConnection;

/// Type alias for the PurchaseService to reduce complexity in AppState
type PurchaseServiceImpl = PurchaseService<
    PostgresBillingRepository,
    PostgresPointsPackageRepository,
    PostgresPaymentAttemptRepository,
    PostgresPurchaseRepository,
    PostgresFulfillmentService<
        PostgresPointsRepository,
        PostgresPointsPackageRepository,
        PostgresPurchaseRepository,
        PostgresBillingRepository,
    >,
>;

/// AppState for API handlers
/// Contains database connections and configuration for HTTP endpoints
#[derive(Clone)]
pub struct AppState {
    /// Core application service (所有领域服务的聚合)
    pub service: ApplicationService,

    /// Database connection pool (sqlx)
    pub pool: PgPool,

    /// Database connection (Sea-ORM) for entity operations
    pub db: Arc<DatabaseConnection>,

    /// Redis connection manager with DB isolation
    /// - Production: uses DB 0 (default_db)
    /// - Test: uses DB 1 (test_db) for automatic isolation
    pub redis_manager: RedisConnectionManager,

    /// Email service (optional)
    pub resend: Option<ResendClient>,

    /// Billing repository
    pub billing_repository: Arc<PostgresBillingRepository>,

    /// Invoice repository
    pub invoice_repository: Arc<PostgresInvoiceRepository>,

    /// Billing service (with policy)
    pub billing_service: Arc<
        herald_core::domain::billing::PlanService<
            PostgresBillingRepository,
            herald_core::infrastructure::authorization::policies::PermissionBasedBillingPolicy,
        >,
    >,

    /// Product service (with policy)
    pub product_service: Arc<
        herald_core::domain::billing::ProductService<
            PostgresBillingRepository,
            herald_core::infrastructure::authorization::policies::PermissionBasedBillingPolicy,
        >,
    >,

    /// Points repository
    pub points_repository: Arc<PostgresPointsRepository>,

    /// Points service (with policy)
    pub points_service: Arc<
        herald_core::domain::points::PointsService<
            PostgresPointsRepository,
            PermissionBasedPointsPolicy,
        >,
    >,

    /// Subscription service (for subscription lifecycle events)
    pub subscription_service: Arc<
        herald_core::domain::points::SubscriptionService<
            PostgresPointsRepository,
            PermissionBasedPointsPolicy,
        >,
    >,

    /// Realm config service (for realm default config management)
    pub realm_config_service: Arc<
        herald_core::domain::points::services::RealmConfigService<
            PostgresPointsRepository,
            PermissionBasedPointsPolicy,
        >,
    >,

    /// Registration service (for free user points on registration)
    pub registration_service: Arc<
        herald_core::domain::points::services::RegistrationService<
            PostgresPointsRepository,
            PermissionBasedPointsPolicy,
        >,
    >,

    /// Public base URL for the API
    pub public_base_url: String,

    /// Permission checker using custom RBAC implementation
    pub permission_checker: Arc<RedisPermissionChecker>,

    /// Application environment (dev/prod/test)
    pub app_env: String,

    /// User repository (used by identity middleware to load user from database)
    /// Note: HTTP handlers should use Extension<Identity> instead of user_repository directly
    pub user_repository: Arc<PostgresUserRepository>,

    /// API Key cache (Redis)
    pub api_key_cache: ApiKeyCache,

    /// API Key repository (PostgreSQL)
    pub api_key_repo: Arc<ClientApiKeyRepository>,

    /// Idempotency service
    pub idempotency_service: Arc<
        herald_core::domain::points::IdempotencyService<
            herald_core::infrastructure::points::RedisIdempotencyStore,
        >,
    >,

    /// Webhook service (for webhook event processing with idempotency)
    pub webhook_service: Arc<WebhookService>,

    /// Server startup time for uptime calculation
    pub startup_time: Instant,

    // ============================================================================
    // Admin User Services
    // ============================================================================
    /// Admin user service
    pub admin_user_service:
        Arc<AdminUserServiceImpl<PostgresAdminUserRepository, RedisPermissionChecker>>,

    /// Role assignment service
    pub role_assignment_service: Arc<
        RoleAssignmentServiceImpl<
            PostgresUserRoleRepository,
            PostgresRolePolicyRepository,
            RedisPermissionChecker,
        >,
    >,

    /// User permission service
    pub user_permission_service: Arc<
        UserPermissionServiceImpl<
            PostgresUserRoleRepository,
            PostgresRolePolicyRepository,
            RedisPermissionChecker,
        >,
    >,

    /// Permission management service
    pub permission_management_service: Arc<
        PermissionManagementServiceImpl<
            PostgresUserRoleRepository,
            PostgresRolePolicyRepository,
            RedisPermissionChecker,
        >,
    >,

    // ============================================================================
    // Points Package & Payment Services
    // ============================================================================
    /// Points package service
    pub points_package_service: Arc<PointsPackageService<PostgresPointsPackageRepository>>,

    /// Payment attempt service
    pub payment_attempt_service: Arc<PaymentAttemptService<PostgresPaymentAttemptRepository>>,

    /// Fulfillment service (for unified purchase handling)
    pub fulfillment_service: Arc<
        PostgresFulfillmentService<
            PostgresPointsRepository,
            PostgresPointsPackageRepository,
            PostgresPurchaseRepository,
            PostgresBillingRepository,
        >,
    >,

    /// Purchase repository (for purchase history and records)
    pub purchase_repository: Arc<PostgresPurchaseRepository>,

    /// Purchase service (routes attempts into fulfillment)
    pub purchase_service: Arc<PurchaseServiceImpl>,
}
