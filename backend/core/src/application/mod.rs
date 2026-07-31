// Application module - Dependency injection and service composition

mod services;
mod webhook;

pub use services::{
    ApplicationService, AuthorizationServiceType, ClientServiceType, PermissionCheckerType,
    PermissionCrudServiceType, RealmServiceType, UserServiceType,
};
pub use webhook::{WebhookContext, WebhookProcessResult, WebhookService};

use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::domain::authorization::services::PermissionServiceImpl;
use crate::domain::common::policies::AllowAllUserPolicy;
use crate::domain::{
    authorization::services::{AuthorizationServiceImpl, RoleServiceImpl},
    client::services::ClientServiceImpl,
    oauth::config_service::OAuthConfigService,
    realm::services::RealmServiceImpl,
    realm_config::RealmConfigServiceImpl,
    user::services::UserServiceImpl,
};
use crate::infrastructure::{
    authorization::{
        PermissionCheckerAuthorizationRepository, PostgresPermissionRepository,
        PostgresRolePermissionRepository, PostgresRolePolicyRepository, PostgresRoleRepository,
        PostgresUserRoleRepository, RedisPermissionChecker,
    },
    client::PostgresClientRepository,
    oauth::PostgresOAuthConfigRepository,
    realm::PostgresRealmRepository,
    realm_config::PostgresRealmConfigRepository,
    user::repositories::{PostgresUserRepository, PostgresVerificationRepository},
};

// ============================================================================
// Application Service Builder
// ============================================================================

pub struct ApplicationServiceBuilder {
    db: Option<Arc<DatabaseConnection>>,
    redis_client: Option<crate::infrastructure::redis::RedisConnectionManager>,
    permission_checker: Option<Arc<RedisPermissionChecker>>,
}

impl ApplicationServiceBuilder {
    pub fn new() -> Self {
        Self {
            db: None,
            redis_client: None,
            permission_checker: None,
        }
    }

    pub fn with_database(mut self, db: Arc<DatabaseConnection>) -> Self {
        self.db = Some(db);
        self
    }

    pub fn with_redis(
        mut self,
        redis_client: crate::infrastructure::redis::RedisConnectionManager,
    ) -> Self {
        self.redis_client = Some(redis_client);
        self
    }

    pub fn with_permission_checker(mut self, checker: Arc<RedisPermissionChecker>) -> Self {
        self.permission_checker = Some(checker);
        self
    }

    pub fn build(self) -> Result<ApplicationService, String> {
        let db = self.db.ok_or("Database connection required")?;
        let redis_client = self.redis_client.ok_or("Redis client required")?;
        let permission_checker = self
            .permission_checker
            .ok_or("Permission checker required")?;

        // Step 1: Create all Repository instances (wrapped in Arc)
        let user_repository = Arc::new(PostgresUserRepository::new(db.clone()));
        let verification_repository = Arc::new(PostgresVerificationRepository::new(db.clone()));
        let _redis_client = redis_client;
        let role_repository = Arc::new(PostgresRoleRepository::new(db.clone()));
        let permission_repository = Arc::new(PostgresPermissionRepository::new(db.clone()));
        let role_permission_repository = Arc::new(PostgresRolePermissionRepository::new(
            db.clone(),
            permission_checker.clone(),
        ));
        let role_policy_repository = Arc::new(PostgresRolePolicyRepository::new(
            db.clone(),
            permission_checker.clone(),
        ));
        let authorization_repository = Arc::new(PermissionCheckerAuthorizationRepository::new(
            permission_checker.clone(),
            db.clone(),
        ));
        let client_repository = Arc::new(PostgresClientRepository::new(db.clone()));
        let realm_repository = Arc::new(PostgresRealmRepository::new(db.clone()));
        let oauth_config_repository = Arc::new(PostgresOAuthConfigRepository::new(db.clone()));
        let oauth_provider_repository = Arc::new(
            crate::infrastructure::oauth::repository::PostgresOAuthRepository::new(db.clone()),
        );
        let realm_config_repository = Arc::new(PostgresRealmConfigRepository::new(db.clone()));
        let user_role_repository = Arc::new(PostgresUserRoleRepository::new(
            db.clone(),
            permission_checker.clone(),
        ));

        // Step 2: Create Module-specific Policies（开发/测试环境使用 AllowAllPolicy）
        use crate::infrastructure::authorization::policies::{
            PermissionBasedClientPolicy, PermissionBasedOAuthConfigPolicy,
            PermissionBasedRealmConfigPolicy, PermissionBasedRealmPolicy,
        };

        // Create RealmService with PermissionBasedRealmPolicy
        let realm_policy = PermissionBasedRealmPolicy::new(permission_checker.clone());

        let client_policy = Arc::new(PermissionBasedClientPolicy::new(permission_checker.clone()));
        let user_policy = Arc::new(AllowAllUserPolicy);
        let realm_config_policy = Arc::new(PermissionBasedRealmConfigPolicy::new(
            permission_checker.clone(),
        ));
        let oauth_config_policy = Arc::new(PermissionBasedOAuthConfigPolicy::new(
            permission_checker.clone(),
        ));

        // Note: Using PermissionBasedRealmPolicy for security, AllowAll policies for other modules for development/testing
        // For production, implement domain-specific policies that use PermissionChecker

        // Step 2.5: Create RealmInitializationService
        let rbac_init_service = Arc::new(
            crate::domain::rbac_init::RealmInitializationServiceImpl::new(
                role_repository.clone(),
                permission_repository.clone(),
                role_permission_repository.clone(),
                role_policy_repository,
            ),
        );

        // Step 3: Create Domain Services (using .clone() to share dependencies)
        let user_service = Arc::new(UserServiceImpl::new(
            user_repository.clone(),
            verification_repository,
            user_policy,
        ));

        let role_service = Arc::new(RoleServiceImpl::new(role_repository.clone()));

        let permission_crud_service = Arc::new(PermissionServiceImpl::new(permission_repository));

        let authorization_service = Arc::new(AuthorizationServiceImpl::new(
            role_permission_repository,
            authorization_repository,
        ));

        let client_service = Arc::new(ClientServiceImpl::new(
            client_repository.clone(),
            client_policy,
        ));

        let realm_service = Arc::new(RealmServiceImpl::new(
            realm_repository,
            Arc::new(realm_policy),
            rbac_init_service,
            client_repository,
            user_role_repository,
            role_repository.clone(),
            user_repository.clone(),
            user_service.clone(),
            realm_config_repository.clone(),
            Arc::new(
                crate::infrastructure::audit::PostgresAuditEventRepository::new((*db).clone()),
            ),
        ));

        let realm_config_service = Arc::new(RealmConfigServiceImpl::new(
            realm_config_repository,
            realm_config_policy,
        ));

        let oauth_config_service = Arc::new(OAuthConfigService::new(
            oauth_config_repository.clone(),
            oauth_config_policy,
            Arc::new(
                crate::infrastructure::audit::PostgresAuditEventRepository::new((*db).clone()),
            ),
        ));

        // Step 4: Assemble ApplicationService
        Ok(ApplicationService::new(
            user_service,
            role_service,
            permission_crud_service,
            authorization_service,
            client_service,
            realm_service,
            oauth_config_repository,
            oauth_provider_repository,
            oauth_config_service,
            realm_config_service,
        ))
    }
}

impl Default for ApplicationServiceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Create Service Function (Dependency Injection)
// ============================================================================

/// Create application service with all dependencies injected
pub async fn create_service(
    db: Arc<DatabaseConnection>,
    redis_client: crate::infrastructure::redis::RedisConnectionManager,
    permission_checker: Arc<RedisPermissionChecker>,
) -> Result<ApplicationService, String> {
    ApplicationServiceBuilder::new()
        .with_database(db)
        .with_redis(redis_client)
        .with_permission_checker(permission_checker)
        .build()
}
