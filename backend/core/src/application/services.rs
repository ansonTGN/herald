// Application Service - Type aliases and service aggregation

use crate::domain::{
    authentication::services::AuthenticationServiceImpl,
    authorization::services::{AuthorizationServiceImpl, PermissionServiceImpl, RoleServiceImpl},
    client::services::ClientServiceImpl,
    oauth::config_service::OAuthConfigService,
    rbac_init::RealmInitializationServiceImpl,
    realm::services::RealmServiceImpl,
    realm_config::RealmConfigServiceImpl,
    user::services::UserServiceImpl,
};
use crate::infrastructure::{
    authentication::RedisSessionRepository,
    authorization::{
        PermissionCheckerAuthorizationRepository, PostgresPermissionRepository,
        PostgresRolePermissionRepository, PostgresRoleRepository, PostgresUserRoleRepository,
        RedisPermissionChecker,
    },
    client::PostgresClientRepository,
    oauth::{PostgresOAuthConfigRepository, PostgresOAuthRepository},
    realm::PostgresRealmRepository,
    realm_config::PostgresRealmConfigRepository,
    user::repositories::{PostgresUserRepository, PostgresVerificationRepository},
};
use std::sync::Arc;

// ============================================================================
// Type Aliases - Simplify complex types
// ============================================================================

// Repository type aliases
type UserRepo = PostgresUserRepository;
type VerificationRepo = PostgresVerificationRepository;
type SessionRepo = RedisSessionRepository;
type RoleRepo = PostgresRoleRepository;
type PermissionRepo = PostgresPermissionRepository;
type RolePermissionRepo = PostgresRolePermissionRepository;
type AuthorizationRepo = PermissionCheckerAuthorizationRepository;
type ClientRepo = PostgresClientRepository;
type RealmRepo = PostgresRealmRepository;
type OAuthConfigRepo = PostgresOAuthConfigRepository;
type OAuthProviderRepo = PostgresOAuthRepository;
type RealmConfigRepo = PostgresRealmConfigRepository;
type RolePolicyRepo = crate::infrastructure::authorization::PostgresRolePolicyRepository;
type UserRoleRepo = PostgresUserRoleRepository;
type RealmInitServiceType =
    RealmInitializationServiceImpl<RoleRepo, PermissionRepo, RolePermissionRepo, RolePolicyRepo>;

// Service type aliases（使用模块特定的 Policy traits）
pub type UserServiceType = UserServiceImpl<
    UserRepo,
    VerificationRepo,
    crate::domain::common::policies::AllowAllUserPolicy,
>;
pub type AuthenticationServiceType = AuthenticationServiceImpl<UserServiceType, SessionRepo>;
pub type RoleServiceType = RoleServiceImpl<RoleRepo>;
pub type PermissionCrudServiceType = PermissionServiceImpl<PermissionRepo>;
pub type AuthorizationServiceType = AuthorizationServiceImpl<RolePermissionRepo, AuthorizationRepo>;
pub type ClientServiceType = ClientServiceImpl<
    ClientRepo,
    crate::infrastructure::authorization::policies::PermissionBasedClientPolicy,
>;
pub type RealmServiceType = RealmServiceImpl<
    RealmRepo,
    crate::infrastructure::authorization::policies::PermissionBasedRealmPolicy,
    RealmInitServiceType,
    ClientRepo,
    UserRoleRepo,
    RoleRepo,
    UserRepo,
    UserServiceType,
    RealmConfigRepo,
>;
pub type RealmConfigServiceType = RealmConfigServiceImpl<
    RealmConfigRepo,
    crate::infrastructure::authorization::policies::PermissionBasedRealmConfigPolicy,
>;
pub type OAuthConfigServiceType = OAuthConfigService<
    OAuthConfigRepo,
    crate::infrastructure::authorization::policies::PermissionBasedOAuthConfigPolicy,
>;

// Permission checker type alias
pub type PermissionCheckerType = RedisPermissionChecker;

// ============================================================================
// Application Service
// ============================================================================

#[derive(Clone, Debug)]
pub struct ApplicationService {
    // All services use pub(crate) visibility
    pub(crate) user_service: Arc<UserServiceType>,
    pub(crate) authentication_service: Arc<AuthenticationServiceType>,
    pub(crate) role_service: Arc<RoleServiceType>,
    pub(crate) permission_crud_service: Arc<PermissionCrudServiceType>,
    pub(crate) authorization_service: Arc<AuthorizationServiceType>,
    pub(crate) client_service: Arc<ClientServiceType>,
    pub(crate) realm_service: Arc<RealmServiceType>,
    pub(crate) oauth_config_repository: Arc<OAuthConfigRepo>,
    pub(crate) oauth_provider_repository: Arc<OAuthProviderRepo>,
    pub(crate) oauth_config_service: Arc<OAuthConfigServiceType>,
    pub(crate) realm_config_service: Arc<RealmConfigServiceType>,
}

impl ApplicationService {
    pub fn new(
        user_service: Arc<UserServiceType>,
        authentication_service: Arc<AuthenticationServiceType>,
        role_service: Arc<RoleServiceType>,
        permission_crud_service: Arc<PermissionCrudServiceType>,
        authorization_service: Arc<AuthorizationServiceType>,
        client_service: Arc<ClientServiceType>,
        realm_service: Arc<RealmServiceType>,
        oauth_config_repository: Arc<OAuthConfigRepo>,
        oauth_provider_repository: Arc<OAuthProviderRepo>,
        oauth_config_service: Arc<OAuthConfigServiceType>,
        realm_config_service: Arc<RealmConfigServiceType>,
    ) -> Self {
        Self {
            user_service,
            authentication_service,
            role_service,
            permission_crud_service,
            authorization_service,
            client_service,
            realm_service,
            oauth_config_repository,
            oauth_provider_repository,
            oauth_config_service,
            realm_config_service,
        }
    }

    /// Get user service
    pub fn user_service(&self) -> Arc<UserServiceType> {
        self.user_service.clone()
    }

    /// Get authentication service
    pub fn authentication_service(&self) -> Arc<AuthenticationServiceType> {
        self.authentication_service.clone()
    }

    /// Get role service
    pub fn role_service(&self) -> Arc<RoleServiceType> {
        self.role_service.clone()
    }

    /// Get permission CRUD service
    pub fn permission_crud_service(&self) -> Arc<PermissionCrudServiceType> {
        self.permission_crud_service.clone()
    }

    /// Get authorization service
    pub fn authorization_service(&self) -> Arc<AuthorizationServiceType> {
        self.authorization_service.clone()
    }

    /// Get client service
    pub fn client_service(&self) -> Arc<ClientServiceType> {
        self.client_service.clone()
    }

    /// Get realm service
    pub fn realm_service(&self) -> Arc<RealmServiceType> {
        self.realm_service.clone()
    }

    /// Get OAuth config repository
    pub fn oauth_config_repository(&self) -> Arc<OAuthConfigRepo> {
        self.oauth_config_repository.clone()
    }

    /// Get OAuth config service
    pub fn oauth_config_service(&self) -> Arc<OAuthConfigServiceType> {
        self.oauth_config_service.clone()
    }

    /// Get OAuth provider repository
    pub fn oauth_provider_repository(&self) -> Arc<OAuthProviderRepo> {
        self.oauth_provider_repository.clone()
    }

    /// Get realm config service
    pub fn realm_config_service(&self) -> Arc<RealmConfigServiceType> {
        self.realm_config_service.clone()
    }

    /// Initialize application (create default data, etc.)
    pub async fn initialize_application(&self) -> Result<(), String> {
        tracing::info!("Application service initialized");
        Ok(())
    }
}
