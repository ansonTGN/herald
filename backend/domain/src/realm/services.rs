use std::sync::Arc;

use crate::audit::{
    ActorType, AuditAction, AuditCategory, AuditEventRepository, AuditResult, AuditTargetType,
    NewAuditEvent,
};
use crate::authentication::Identity;
use crate::authorization::{AssignRoleToUserRequest, RoleRepository, UserRoleRepository};
use crate::client::ports::ClientRepository;
use crate::client_api_keys::constants::ADMIN_API_CLIENT_ID;
use crate::common::entities::app_errors::CoreError;
use crate::common::policies::{RealmPolicy, ensure_policy};
use crate::rbac_init::RealmInitializationService;
use crate::realm::{
    CreateRealmRequest, ListRealmsFilters, PaginatedRealmsResponse, Realm,
    RealmPointsConfigInitializer, RealmRepository, RealmService, RealmSummary, UpdateRealmRequest,
};
use crate::realm_config::{ConfigType, RealmConfigRepository, UpsertRealmConfigRequest};
use crate::user::ports::{UserRepository, UserService};
use crate::user::value_objects::CreateUserRequest;

pub struct RealmServiceImpl<R, P, RB, C, UR, RR, U, UR2, RCR, RPCI, AE>
where
    R: RealmRepository,
    P: RealmPolicy,
    RB: RealmInitializationService,
    C: ClientRepository,
    UR: UserRoleRepository,
    RR: RoleRepository,
    U: UserRepository,
    UR2: UserService,
    RCR: RealmConfigRepository,
    RPCI: RealmPointsConfigInitializer,
    AE: AuditEventRepository + 'static,
{
    pub(crate) realm_repository: Arc<R>,
    pub(crate) policy: Arc<P>,
    pub(crate) rbac_init_service: Arc<RB>,
    pub(crate) client_repository: Arc<C>,
    pub(crate) user_role_repository: Arc<UR>,
    pub(crate) role_repository: Arc<RR>,
    pub(crate) user_repository: Arc<U>,
    pub(crate) user_service: Arc<UR2>,
    pub(crate) realm_config_repository: Arc<RCR>,
    pub(crate) realm_points_config_initializer: Arc<RPCI>,
    pub(crate) audit_event_repository: Arc<AE>,
}

impl<R, P, RB, C, UR, RR, U, UR2, RCR, RPCI, AE>
    RealmServiceImpl<R, P, RB, C, UR, RR, U, UR2, RCR, RPCI, AE>
where
    R: RealmRepository,
    P: RealmPolicy,
    RB: RealmInitializationService,
    C: ClientRepository,
    UR: UserRoleRepository,
    RR: RoleRepository,
    U: UserRepository,
    UR2: UserService,
    RCR: RealmConfigRepository,
    RPCI: RealmPointsConfigInitializer,
    AE: AuditEventRepository + 'static,
{
    pub fn new(
        realm_repository: Arc<R>,
        policy: Arc<P>,
        rbac_init_service: Arc<RB>,
        client_repository: Arc<C>,
        user_role_repository: Arc<UR>,
        role_repository: Arc<RR>,
        user_repository: Arc<U>,
        user_service: Arc<UR2>,
        realm_config_repository: Arc<RCR>,
        realm_points_config_initializer: Arc<RPCI>,
        audit_event_repository: Arc<AE>,
    ) -> Self {
        Self {
            realm_repository,
            policy,
            rbac_init_service,
            client_repository,
            user_role_repository,
            role_repository,
            user_repository,
            user_service,
            realm_config_repository,
            realm_points_config_initializer,
            audit_event_repository,
        }
    }

    /// Get the RBAC initialization service
    /// This is used during application startup to initialize admin realm RBAC
    pub fn get_rbac_init_service(&self) -> Arc<RB> {
        self.rbac_init_service.clone()
    }
}

impl<R, P, RB, C, UR, RR, U, UR2, RCR, RPCI, AE> RealmService
    for RealmServiceImpl<R, P, RB, C, UR, RR, U, UR2, RCR, RPCI, AE>
where
    R: RealmRepository,
    P: RealmPolicy,
    RB: RealmInitializationService,
    C: ClientRepository,
    UR: UserRoleRepository,
    RR: RoleRepository,
    U: UserRepository,
    UR2: UserService,
    RCR: RealmConfigRepository,
    RPCI: RealmPointsConfigInitializer,
    AE: AuditEventRepository + 'static,
{
    async fn create_realm(
        &self,
        identity: Identity,
        request: CreateRealmRequest,
    ) -> Result<Realm, CoreError> {
        // Policy check
        ensure_policy(
            self.policy.can_create_realm(identity.clone()).await,
            "Insufficient permissions to create realm",
        )?;

        // Extract admin_user before moving request
        let admin_user = request.admin_user.clone();

        // Call repository to create realm
        let realm = self.realm_repository.create_realm(request).await?;

        // Initialize default realm config (registration settings)
        match self
            .realm_config_repository
            .upsert(
                &realm.id,
                UpsertRealmConfigRequest {
                    config_type: ConfigType::Registration,
                    config_key: "enabled".to_string(),
                    config_value: "false".to_string(),
                    is_secret: Some(false),
                    enabled: Some(true),
                    metadata: None,
                },
            )
            .await
        {
            Ok(_) => {
                tracing::info!(
                    realm_id = %realm.id,
                    "Default registration config initialized successfully"
                );
            }
            Err(e) => {
                tracing::error!(
                    realm_id = %realm.id,
                    error = %e,
                    "Failed to initialize default registration config, rolling back realm"
                );
                // Rollback: delete the realm that was created
                // TODO: Realm deletion is not supported - partial realm data may remain
                return Err(CoreError::InternalServerError(format!(
                    "Failed to initialize default registration config for realm {}: {}",
                    realm.id, e
                )));
            }
        }

        match self
            .realm_points_config_initializer
            .create_default_realm_points_config(&realm.id)
            .await
        {
            Ok(_) => {
                tracing::info!(
                    realm_id = %realm.id,
                    "Default points config initialized successfully"
                );
            }
            Err(e) => {
                tracing::error!(
                    realm_id = %realm.id,
                    error = %e,
                    "Failed to initialize default points config, rolling back realm"
                );
                // TODO: Realm deletion is not supported - partial realm data may remain
                return Err(CoreError::InternalServerError(format!(
                    "Failed to initialize default points config for realm {}: {}",
                    realm.id, e
                )));
            }
        }

        // Initialize default RBAC for the realm
        // 1. Query admin-web-console client, create if doesn't exist
        let web_console = self
            .client_repository
            .get_client_app_by_client_id(&realm.id, "admin-web-console")
            .await;

        let client = match web_console {
            Ok(client) => {
                tracing::info!(
                    realm_id = %realm.id,
                    "Found existing admin-web-console client"
                );
                client
            }
            Err(_) => {
                // Create admin-web-console client if it doesn't exist
                tracing::info!(
                    realm_id = %realm.id,
                    "admin-web-console client not found, creating it"
                );
                let result = self
                    .client_repository
                    .create_client_app(crate::client::value_objects::CreateClientAppRequest {
                        realm_id: realm.id.clone(),
                        client_id: "admin-web-console".to_string(),
                        name: "Admin Web Console".to_string(),
                        description: Some("Admin web console client application".to_string()),
                        redirect_uris: None,
                        enabled: Some(true),
                        icon_url: None,
                        session_ttl_seconds: Some(1800),
                        session_renewal_ttl_seconds: Some(1800),
                        device_code_grant_enabled: None,
                    })
                    .await;

                match result {
                    Ok(client) => client,
                    Err(e) => {
                        tracing::error!(
                            realm_id = %realm.id,
                            error = %e,
                            "Failed to create admin-web-console client"
                        );
                        // Rollback: delete the realm that was created
                        // TODO: Realm deletion is not supported - partial realm data may remain
                        return Err(CoreError::InternalServerError(format!(
                            "Failed to create admin-web-console client for realm {}: {}",
                            realm.id, e
                        )));
                    }
                }
            }
        };

        // 2. Call RBAC initialization (failure should rollback realm creation)
        match self
            .rbac_init_service
            .init_default_rbac(
                identity.clone(),
                crate::rbac_init::RealmRBACInitRequest {
                    realm_id: realm.id.clone(),
                    admin_web_console_client_id: client.client_id.clone(), // Use client.client_id (e.g., "admin-web-console"), not UUID
                },
            )
            .await
        {
            Ok(_) => {
                tracing::info!(
                    realm_id = %realm.id,
                    "RBAC initialized successfully for realm"
                );
            }
            Err(e) => {
                tracing::error!(
                    realm_id = %realm.id,
                    error = %e,
                    "RBAC initialization failed, rolling back realm creation"
                );
                // Rollback: delete the realm that was created
                // TODO: Realm deletion is not supported - partial realm data may remain
                // Return a generic error message
                return Err(CoreError::InternalServerError(format!(
                    "RBAC initialization failed for realm {}: {}",
                    realm.id, e
                )));
            }
        }

        // 2.5 Create built-in API Key Client App (client_id='admin-api-client')
        {
            let existing_api_client = self
                .client_repository
                .get_client_app_by_client_id(&realm.id, ADMIN_API_CLIENT_ID)
                .await;

            if existing_api_client.is_err() {
                tracing::info!(
                    realm_id = %realm.id,
                    "Creating built-in API Key Client App"
                );
                match self
                    .client_repository
                    .create_client_app(crate::client::value_objects::CreateClientAppRequest {
                        realm_id: realm.id.clone(),
                        client_id: ADMIN_API_CLIENT_ID.to_string(),
                        name: "API Key Client".to_string(),
                        description: Some("Built-in client for API key authentication".to_string()),
                        redirect_uris: None,
                        enabled: Some(true),
                        icon_url: None,
                        session_ttl_seconds: None,
                        session_renewal_ttl_seconds: None,
                        device_code_grant_enabled: None,
                    })
                    .await
                {
                    Ok(_) => {
                        tracing::info!(
                            realm_id = %realm.id,
                            "Built-in API Key Client App created successfully"
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            realm_id = %realm.id,
                            error = %e,
                            "Failed to create built-in API Key Client App, rolling back realm creation"
                        );
                        // TODO: Realm deletion is not supported - partial realm data may remain
                        return Err(CoreError::InternalServerError(format!(
                            "Failed to create API Key Client App for realm {}: {}",
                            realm.id, e
                        )));
                    }
                }
            }
        }

        // 3. Create admin user (now required)
        {
            // 3.1 Create user
            let user = match self
                .user_service
                .create_user_without_identity_check(CreateUserRequest {
                    realm_id: realm.id.clone(),
                    email: admin_user.email.clone(),
                    password: Some(admin_user.password),
                    provider_ids: None,
                })
                .await
            {
                Ok(user) => user,
                Err(e) => {
                    tracing::error!(
                        realm_id = %realm.id,
                        error = %e,
                        "Failed to create admin user, rolling back realm"
                    );
                    // TODO: Realm deletion is not supported - partial realm data may remain
                    // Preserve the original error type (BadRequest, Conflict, etc.)
                    return Err(e);
                }
            };

            // 3.2 Find realm-admin role (use client.client_id string, not UUID)
            let role = match self
                .role_repository
                .find_by_name("realm-admin", &realm.id, &client.client_id)
                .await
            {
                Ok(role) => role,
                Err(e) => {
                    tracing::error!(
                        realm_id = %realm.id,
                        error = %e,
                        "Failed to find realm-admin role, rolling back realm"
                    );
                    // TODO: Realm deletion is not supported - partial realm data may remain
                    return Err(CoreError::InternalServerError(format!(
                        "Failed to find realm-admin role: {}",
                        e
                    )));
                }
            };

            // 3.3 Verify user (set status to Normal) so they can login immediately
            use crate::user::value_objects::UpdateUserRequest;
            match self
                .user_repository
                .update_user(
                    user.id,
                    UpdateUserRequest {
                        status: Some(crate::user::UserStatus::Normal.into()),
                        nickname: None,
                    },
                )
                .await
            {
                Ok(_) => {
                    tracing::info!(
                        realm_id = %realm.id,
                        user_id = %user.id,
                        "Admin user verified successfully"
                    );
                }
                Err(e) => {
                    tracing::error!(
                        realm_id = %realm.id,
                        user_id = %user.id,
                        error = %e,
                        "Failed to verify admin user, rolling back realm"
                    );
                    // TODO: Realm deletion is not supported - partial realm data may remain
                    return Err(CoreError::InternalServerError(format!(
                        "Failed to verify admin user: {}",
                        e
                    )));
                }
            }

            // 3.4 Assign role to user
            match self
                .user_role_repository
                .assign_role_to_user(AssignRoleToUserRequest {
                    realm_id: realm.id.clone(),
                    user_id: user.id,
                    role_id: role.id,
                    client_id: "admin-web-console".to_string(),
                })
                .await
            {
                Ok(_) => {
                    tracing::info!(
                        realm_id = %realm.id,
                        user_id = %user.id,
                        role_id = %role.id,
                        "Admin user created and role assigned successfully"
                    );
                }
                Err(e) => {
                    tracing::error!(
                        realm_id = %realm.id,
                        user_id = %user.id,
                        error = %e,
                        "Failed to assign role to admin user, rolling back realm"
                    );
                    // TODO: Realm deletion is not supported - partial realm data may remain
                    return Err(CoreError::InternalServerError(format!(
                        "Failed to assign role to admin user: {}",
                        e
                    )));
                }
            }
        }

        // Log audit event for RBAC initialization
        if let Err(e) = self
            .audit_event_repository
            .create(NewAuditEvent {
                realm_id: realm.id.clone(),
                category: AuditCategory::RealmManagement,
                action: AuditAction::RealmCreate,
                actor_id: identity.user_id().to_string(),
                actor_type: Some(ActorType::Admin),
                actor_name: identity.as_user().map(|u| u.email.clone()),
                target_type: AuditTargetType::Realm,
                target_id: realm.id.clone(),
                target_name: Some(realm.name.clone()),
                result: AuditResult::Success,
                details: Some(serde_json::json!({"status": "created"})),
                ip_address: None,
                user_agent: None,
                trace_id: None,
            })
            .await
        {
            tracing::warn!(error = %e, "Failed to record audit event");
        }

        if let Err(e) = self
            .audit_event_repository
            .create(NewAuditEvent {
                realm_id: realm.id.clone(),
                category: AuditCategory::RealmManagement,
                action: AuditAction::RealmRbacInit,
                actor_id: identity.user_id().to_string(),
                actor_type: Some(ActorType::System),
                actor_name: None,
                target_type: AuditTargetType::Realm,
                target_id: realm.id.clone(),
                target_name: Some(realm.name.clone()),
                result: AuditResult::Success,
                details: Some(serde_json::json!({"status": "initialized"})),
                ip_address: None,
                user_agent: None,
                trace_id: None,
            })
            .await
        {
            tracing::warn!(error = %e, "Failed to record audit event");
        }

        Ok(realm)
    }

    async fn get_realm(&self, identity: Identity, id: String) -> Result<Realm, CoreError> {
        // Self-realm: any authenticated user can view own realm basic info
        // Cross-realm: requires realm.manage in admin realm
        if id != identity.realm_id() {
            ensure_policy(
                self.policy.can_create_realm(identity.clone()).await,
                "Insufficient permissions to view other realm",
            )?;
        }

        self.realm_repository.get_realm_by_id(&id).await
    }

    async fn list_realms(&self, identity: Identity) -> Result<Vec<Realm>, CoreError> {
        // Policy check
        ensure_policy(
            self.policy.can_list_realms(identity.clone()).await,
            "Insufficient permissions to list realms",
        )?;

        let can_list_all_realms = identity.realm_id() == "admin";

        Ok(self
            .realm_repository
            .list_realms()
            .await?
            .into_iter()
            .filter(|realm| can_list_all_realms || identity.has_access_to_realm(&realm.id))
            .collect())
    }

    async fn list_realms_paginated(
        &self,
        identity: Identity,
        filters: ListRealmsFilters,
    ) -> Result<PaginatedRealmsResponse, CoreError> {
        // Policy check
        ensure_policy(
            self.policy.can_list_realms(identity.clone()).await,
            "Insufficient permissions to list realms",
        )?;

        let mut filters = filters;
        // Admin realm lists all realms; other realms are scoped to their own.
        // Mirrors the non-paginated `list_realms` access semantics.
        if identity.realm_id() != "admin" {
            filters.accessible_realm_id = Some(identity.realm_id());
        }

        self.realm_repository.list_realms_paginated(filters).await
    }

    async fn update_realm(
        &self,
        identity: Identity,
        id: String,
        request: UpdateRealmRequest,
    ) -> Result<Realm, CoreError> {
        // Only self-realm editing is allowed, requires settings.manage
        // Cross-realm editing is NOT allowed even for Super Admin (realm.manage is for create/delete only)
        ensure_policy(
            id == identity.realm_id(),
            "Insufficient permissions to update realm",
        )?;

        ensure_policy(
            self.policy
                .can_update_own_realm_settings(identity.clone())
                .await,
            "Insufficient permissions to update realm",
        )?;

        let name = request
            .name
            .ok_or(CoreError::BadRequest("Name is required".to_string()))?;
        self.realm_repository
            .update_realm(&id, name, request.description)
            .await
    }

    async fn get_public_realm_info(&self, id: String) -> Result<RealmSummary, CoreError> {
        let realm = self.realm_repository.get_realm_by_id(&id).await?;
        Ok(RealmSummary {
            id: realm.id,
            name: realm.name,
            description: realm.description,
        })
    }
}

impl<R, P, RB, C, UR, RR, U, UR2, RCR, RPCI, AE> std::fmt::Debug
    for RealmServiceImpl<R, P, RB, C, UR, RR, U, UR2, RCR, RPCI, AE>
where
    R: RealmRepository,
    P: RealmPolicy,
    RB: RealmInitializationService,
    C: ClientRepository,
    UR: UserRoleRepository,
    RR: RoleRepository,
    U: UserRepository,
    UR2: UserService,
    RCR: RealmConfigRepository,
    RPCI: RealmPointsConfigInitializer,
    AE: AuditEventRepository + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RealmServiceImpl").finish()
    }
}
