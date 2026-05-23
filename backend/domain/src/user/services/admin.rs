// User Admin Service Implementations
//
// These services contain the business logic for user admin operations

use std::sync::Arc;
use uuid::Uuid;

use crate::audit::{
    ActorType, AuditAction, AuditCategory, AuditEventRepository, AuditResult, AuditTargetType,
    NewAuditEvent,
};
use crate::{
    authentication::Identity,
    authorization::permission_service::PermissionService,
    common::{entities::app_errors::CoreError, policies::ensure_policy},
};

use super::super::{
    AdminUser, AdminUserRepository, AdminUserService, CreateUserWithRolesRequest, PermissionDetail,
    PermissionListData, PermissionManagementService, PermissionSource, RoleAssignmentService,
    RoleDetail, RoleEntity, RolePolicyRepository, UpdateUserAdminRequest, UserAdminError,
    UserAdminResult, UserPermissionService, UserRoleRepository,
};

// ============================================================================
// Shared permission check helper
// ============================================================================

async fn require_permission<P: PermissionService>(
    permission_checker: &P,
    identity: &Identity,
    realm_id: &str,
    resource: &str,
    action: &str,
    policy_msg: &str,
) -> UserAdminResult<()> {
    let principal = identity.principal_ref();
    let allowed = permission_checker
        .check_principal_permission(
            realm_id,
            principal.principal_type,
            &principal.principal_id,
            resource,
            action,
        )
        .await
        .map_err(|e| UserAdminError::InternalError(format!("Permission check failed: {}", e)))?;

    ensure_policy(allowed, policy_msg).map_err(|e| match e {
        CoreError::Forbidden(msg) => UserAdminError::PermissionDenied(msg),
        _ => UserAdminError::InternalError(format!("Policy check error: {:?}", e)),
    })
}

// ============================================================================
// Admin User Service Implementation
// ============================================================================

pub struct AdminUserServiceImpl<R, P, AE>
where
    R: AdminUserRepository,
    P: PermissionService,
    AE: AuditEventRepository + 'static,
{
    user_repository: Arc<R>,
    permission_checker: Arc<P>,
    pub(crate) audit_event_repository: Arc<AE>,
}

impl<R, P, AE> AdminUserServiceImpl<R, P, AE>
where
    R: AdminUserRepository,
    P: PermissionService,
    AE: AuditEventRepository + 'static,
{
    pub fn new(
        user_repository: Arc<R>,
        permission_checker: Arc<P>,
        audit_event_repository: Arc<AE>,
    ) -> Self {
        Self {
            user_repository,
            permission_checker,
            audit_event_repository,
        }
    }

    async fn hash_password(&self, password: &str) -> UserAdminResult<String> {
        bcrypt::hash(password, bcrypt::DEFAULT_COST)
            .map_err(|e| UserAdminError::InternalError(format!("Password hashing failed: {}", e)))
    }
}

impl<R, P, AE> AdminUserService for AdminUserServiceImpl<R, P, AE>
where
    R: AdminUserRepository,
    P: PermissionService,
    AE: AuditEventRepository + 'static,
{
    async fn create_user_with_roles(
        &self,
        identity: Identity,
        realm_id: &str,
        request: CreateUserWithRolesRequest,
    ) -> UserAdminResult<AdminUser> {
        require_permission(
            &*self.permission_checker,
            &identity,
            realm_id,
            "users",
            "create",
            "Insufficient permissions to create users",
        )
        .await?;

        // Realm boundary check
        if identity.realm_id() != realm_id {
            return Err(UserAdminError::PermissionDenied(
                "Cannot create users in a different realm".to_string(),
            ));
        }

        // Check if email already exists
        if self
            .user_repository
            .email_exists(realm_id, &request.email)
            .await?
        {
            return Err(UserAdminError::DuplicateEmail(request.email));
        }

        // Hash password
        let password_hash = self.hash_password(&request.password).await?;

        // Create user with profile
        let user_id = self
            .user_repository
            .create_user_with_profile(
                realm_id,
                &request.email,
                &password_hash,
                request.nickname.as_deref(),
                request.status.unwrap_or(1),
            )
            .await?;

        // Fetch created user for response
        let user_entity = self
            .user_repository
            .get_user_with_profile(user_id)
            .await?
            .ok_or_else(|| {
                UserAdminError::InternalError("Failed to fetch created user".to_string())
            })?;

        let admin_user = AdminUser {
            id: user_entity.id,
            realm_id: user_entity.realm_id.clone(),
            email: user_entity.email.clone(),
            nickname: user_entity.nickname,
            status: user_entity.status,
            created_at: user_entity.created_at.to_rfc3339(),
        };

        // Record audit event (failure does not fail the operation)
        if let Err(e) = self
            .audit_event_repository
            .create(NewAuditEvent {
                realm_id: realm_id.to_string(),
                category: AuditCategory::UserManagement,
                action: AuditAction::UserCreate,
                actor_id: identity.user_id().to_string(),
                actor_type: Some(ActorType::Admin),
                actor_name: identity.as_user().map(|u| u.email.clone()),
                target_type: AuditTargetType::User,
                target_id: admin_user.id.to_string(),
                target_name: Some(admin_user.email.clone()),
                result: AuditResult::Success,
                details: Some(serde_json::json!({"email": admin_user.email})),
                ip_address: None,
                user_agent: None,
                trace_id: None,
            })
            .await
        {
            tracing::warn!(error = %e, "Failed to record audit event");
        }

        Ok(admin_user)
    }

    async fn update_user_admin(
        &self,
        identity: Identity,
        realm_id: &str,
        user_id: Uuid,
        request: UpdateUserAdminRequest,
    ) -> UserAdminResult<AdminUser> {
        require_permission(
            &*self.permission_checker,
            &identity,
            realm_id,
            "users",
            "manage",
            "Insufficient permissions to update users",
        )
        .await?;

        // Realm boundary check
        if identity.realm_id() != realm_id {
            return Err(UserAdminError::PermissionDenied(
                "Cannot update users in a different realm".to_string(),
            ));
        }

        // Update user fields
        self.user_repository
            .update_user_fields(
                user_id,
                request.email.as_deref(),
                request.nickname.as_deref(),
                request.status,
            )
            .await?;

        // Fetch updated user
        let user_entity = self
            .user_repository
            .get_user_with_profile(user_id)
            .await?
            .ok_or_else(|| UserAdminError::UserNotFound(user_id.to_string()))?;

        let admin_user = AdminUser {
            id: user_entity.id,
            realm_id: user_entity.realm_id.clone(),
            email: user_entity.email.clone(),
            nickname: user_entity.nickname,
            status: user_entity.status,
            created_at: user_entity.created_at.to_rfc3339(),
        };

        // Record audit event (failure does not fail the operation)
        if let Err(e) = self
            .audit_event_repository
            .create(NewAuditEvent {
                realm_id: realm_id.to_string(),
                category: AuditCategory::UserManagement,
                action: AuditAction::UserUpdate,
                actor_id: identity.user_id().to_string(),
                actor_type: Some(ActorType::Admin),
                actor_name: identity.as_user().map(|u| u.email.clone()),
                target_type: AuditTargetType::User,
                target_id: admin_user.id.to_string(),
                target_name: Some(admin_user.email.clone()),
                result: AuditResult::Success,
                details: Some(serde_json::json!({"email": admin_user.email})),
                ip_address: None,
                user_agent: None,
                trace_id: None,
            })
            .await
        {
            tracing::warn!(error = %e, "Failed to record audit event");
        }

        Ok(admin_user)
    }

    async fn get_user_admin(
        &self,
        identity: Identity,
        realm_id: &str,
        user_id: Uuid,
    ) -> UserAdminResult<AdminUser> {
        require_permission(
            &*self.permission_checker,
            &identity,
            realm_id,
            "users",
            "view",
            "Insufficient permissions to read users",
        )
        .await?;

        // Realm boundary check
        if identity.realm_id() != realm_id {
            return Err(UserAdminError::PermissionDenied(
                "Cannot read users in a different realm".to_string(),
            ));
        }

        let user_entity = self
            .user_repository
            .get_user_with_profile(user_id)
            .await?
            .ok_or_else(|| UserAdminError::UserNotFound(user_id.to_string()))?;

        Ok(AdminUser {
            id: user_entity.id,
            realm_id: user_entity.realm_id,
            email: user_entity.email,
            nickname: user_entity.nickname,
            status: user_entity.status,
            created_at: user_entity.created_at.to_rfc3339(),
        })
    }

    async fn delete_user(
        &self,
        identity: Identity,
        realm_id: &str,
        user_id: Uuid,
    ) -> UserAdminResult<()> {
        require_permission(
            &*self.permission_checker,
            &identity,
            realm_id,
            "users",
            "manage",
            "Insufficient permissions to delete users",
        )
        .await?;

        // Realm boundary check
        if identity.realm_id() != realm_id {
            return Err(UserAdminError::PermissionDenied(
                "Cannot delete users in a different realm".to_string(),
            ));
        }

        // Delete user (transactional - profile and account)
        let deleted = self.user_repository.delete_user(user_id).await?;

        if !deleted {
            return Err(UserAdminError::UserNotFound(user_id.to_string()));
        }

        // Record audit event (failure does not fail the operation)
        if let Err(e) = self
            .audit_event_repository
            .create(NewAuditEvent {
                realm_id: realm_id.to_string(),
                category: AuditCategory::UserManagement,
                action: AuditAction::UserDelete,
                actor_id: identity.user_id().to_string(),
                actor_type: Some(ActorType::Admin),
                actor_name: identity.as_user().map(|u| u.email.clone()),
                target_type: AuditTargetType::User,
                target_id: user_id.to_string(),
                target_name: None,
                result: AuditResult::Success,
                details: None,
                ip_address: None,
                user_agent: None,
                trace_id: None,
            })
            .await
        {
            tracing::warn!(error = %e, "Failed to record audit event");
        }

        Ok(())
    }

    async fn reset_user_password(
        &self,
        identity: Identity,
        realm_id: &str,
        user_id: Uuid,
    ) -> UserAdminResult<String> {
        require_permission(
            &*self.permission_checker,
            &identity,
            realm_id,
            "users",
            "manage",
            "Insufficient permissions to manage users",
        )
        .await?;

        // Realm boundary check
        if identity.realm_id() != realm_id {
            return Err(UserAdminError::PermissionDenied(
                "Cannot reset passwords for users in a different realm".to_string(),
            ));
        }

        // Generate 16-character random password
        let new_password = generate_random_password();

        // Hash the new password
        let password_hash = self.hash_password(&new_password).await?;

        // Update password in database
        let updated = self
            .user_repository
            .update_user_password(user_id, &password_hash)
            .await?;

        if !updated {
            return Err(UserAdminError::UserNotFound(user_id.to_string()));
        }

        Ok(new_password)
    }
}

/// Generate random password (alphanumeric, 16 characters)
fn generate_random_password() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();
    (0..16)
        .map(|_| CHARSET[rng.gen_range(0..CHARSET.len())] as char)
        .collect()
}

// ============================================================================
// Role Assignment Service Implementation
// ============================================================================

pub struct RoleAssignmentServiceImpl<R, RP, P>
where
    R: UserRoleRepository,
    RP: RolePolicyRepository,
    P: PermissionService,
{
    user_role_repository: Arc<R>,
    role_policy_repository: Arc<RP>,
    permission_checker: Arc<P>,
}

impl<R, RP, P> RoleAssignmentServiceImpl<R, RP, P>
where
    R: UserRoleRepository,
    RP: RolePolicyRepository,
    P: PermissionService,
{
    pub fn new(
        user_role_repository: Arc<R>,
        role_policy_repository: Arc<RP>,
        permission_checker: Arc<P>,
    ) -> Self {
        Self {
            user_role_repository,
            role_policy_repository,
            permission_checker,
        }
    }
}

impl<R, RP, P> RoleAssignmentService for RoleAssignmentServiceImpl<R, RP, P>
where
    R: UserRoleRepository,
    RP: RolePolicyRepository,
    P: PermissionService,
{
    async fn assign_user_roles(
        &self,
        identity: Identity,
        realm_id: &str,
        user_id: Uuid,
        role_ids: Vec<Uuid>,
    ) -> UserAdminResult<()> {
        // Check if can assign roles
        if !self
            .can_assign_roles(&identity, realm_id, &role_ids)
            .await?
        {
            return Err(UserAdminError::PermissionDenied(
                "Cannot assign these roles without roles.manage permission".to_string(),
            ));
        }

        // Get client ID (hardcoded for now, should come from state)
        let client_id = "admin-web-console";

        // Replace user roles
        self.user_role_repository
            .replace_user_roles(user_id, realm_id, client_id, &role_ids)
            .await?;

        // Invalidate cache
        let _ = self
            .permission_checker
            .invalidate_user_role_cache(realm_id, &user_id.to_string())
            .await;

        Ok(())
    }

    async fn get_user_roles(
        &self,
        identity: Identity,
        realm_id: &str,
        user_id: Uuid,
    ) -> UserAdminResult<Vec<RoleDetail>> {
        require_permission(
            &*self.permission_checker,
            &identity,
            realm_id,
            "users",
            "view",
            "Insufficient permissions to read user roles",
        )
        .await?;

        // Realm boundary check
        if identity.realm_id() != realm_id {
            return Err(UserAdminError::PermissionDenied(
                "Cannot read roles in a different realm".to_string(),
            ));
        }

        let roles = self.user_role_repository.get_user_roles(user_id).await?;

        Ok(roles
            .into_iter()
            .map(|r| RoleDetail {
                id: r.id,
                name: r.name,
                description: r.description,
            })
            .collect())
    }

    async fn can_assign_roles(
        &self,
        identity: &Identity,
        realm_id: &str,
        role_ids: &[Uuid],
    ) -> UserAdminResult<bool> {
        // Check if principal has roles.manage permission
        let principal = identity.principal_ref();
        let can_manage_roles = self
            .permission_checker
            .check_principal_permission(
                realm_id,
                principal.principal_type,
                &principal.principal_id,
                "roles",
                "manage",
            )
            .await
            .unwrap_or(false);

        if can_manage_roles {
            return Ok(true);
        }

        // If no roles.manage permission, can only assign "user" role
        if role_ids.len() != 1 {
            return Ok(false);
        }

        let roles = self
            .role_policy_repository
            .get_roles_by_ids(role_ids)
            .await?;

        if roles.len() != 1 {
            return Ok(false);
        }

        Ok(roles[0].name == "user")
    }

    async fn assign_api_key_roles(
        &self,
        identity: Identity,
        realm_id: &str,
        api_key_id: &str,
        role_ids: Vec<Uuid>,
    ) -> UserAdminResult<()> {
        require_permission(
            &*self.permission_checker,
            &identity,
            realm_id,
            "roles",
            "manage",
            "Insufficient permissions to manage roles",
        )
        .await?;

        // Realm boundary check
        if identity.realm_id() != realm_id {
            return Err(UserAdminError::PermissionDenied(
                "Cannot manage roles in a different realm".to_string(),
            ));
        }

        // Validate roles: all must exist in realm, none can be builtin
        if !role_ids.is_empty() {
            let roles = self
                .role_policy_repository
                .get_roles_by_ids(&role_ids)
                .await?;

            // Check all roles were found
            if roles.len() != role_ids.len() {
                let found_ids: std::collections::HashSet<Uuid> =
                    roles.iter().map(|r| r.id).collect();
                let missing: Vec<String> = role_ids
                    .iter()
                    .filter(|id| !found_ids.contains(id))
                    .map(|id| id.to_string())
                    .collect();
                return Err(UserAdminError::RoleNotFound(missing.join(", ")));
            }

            // Check none are builtin
            if let Some(builtin) = roles.iter().find(|r| r.is_builtin) {
                return Err(UserAdminError::InvalidRoleAssignment(format!(
                    "Cannot assign builtin role '{}' to API Key",
                    builtin.name
                )));
            }

            // Check all belong to realm
            if let Some(wrong_realm) = roles.iter().find(|r| r.realm_id != realm_id) {
                return Err(UserAdminError::RoleNotFound(wrong_realm.id.to_string()));
            }
        }

        // Use the built-in API Key client ID
        let client_id = crate::client_api_keys::constants::ADMIN_API_CLIENT_ID;

        self.user_role_repository
            .replace_api_key_roles(api_key_id, realm_id, client_id, &role_ids)
            .await?;

        // Invalidate principal cache
        let _ = self
            .permission_checker
            .invalidate_principal_role_cache(realm_id, "api_key", api_key_id)
            .await;

        Ok(())
    }

    async fn get_api_key_roles(
        &self,
        identity: Identity,
        realm_id: &str,
        api_key_id: &str,
    ) -> UserAdminResult<Vec<RoleEntity>> {
        require_permission(
            &*self.permission_checker,
            &identity,
            realm_id,
            "api_keys",
            "view",
            "Insufficient permissions to view API keys",
        )
        .await?;

        // Realm boundary check
        if identity.realm_id() != realm_id {
            return Err(UserAdminError::PermissionDenied(
                "Cannot view API key roles in a different realm".to_string(),
            ));
        }

        self.user_role_repository
            .get_api_key_roles(api_key_id)
            .await
    }
}

// ============================================================================
// User Permission Service Implementation
// ============================================================================

pub struct UserPermissionServiceImpl<R, RP, P>
where
    R: UserRoleRepository,
    RP: RolePolicyRepository,
    P: PermissionService,
{
    user_role_repository: Arc<R>,
    role_policy_repository: Arc<RP>,
    permission_checker: Arc<P>,
}

impl<R, RP, P> UserPermissionServiceImpl<R, RP, P>
where
    R: UserRoleRepository,
    RP: RolePolicyRepository,
    P: PermissionService,
{
    pub fn new(
        user_role_repository: Arc<R>,
        role_policy_repository: Arc<RP>,
        permission_checker: Arc<P>,
    ) -> Self {
        Self {
            user_role_repository,
            role_policy_repository,
            permission_checker,
        }
    }
}

impl<R, RP, P> UserPermissionService for UserPermissionServiceImpl<R, RP, P>
where
    R: UserRoleRepository,
    RP: RolePolicyRepository,
    P: PermissionService,
{
    async fn get_effective_permissions(
        &self,
        identity: Identity,
        realm_id: &str,
        user_id: Uuid,
    ) -> UserAdminResult<Vec<PermissionDetail>> {
        require_permission(
            &*self.permission_checker,
            &identity,
            realm_id,
            "users",
            "view",
            "Insufficient permissions to read user permissions",
        )
        .await?;

        // Realm boundary check
        if identity.realm_id() != realm_id {
            return Err(UserAdminError::PermissionDenied(
                "Cannot read permissions in a different realm".to_string(),
            ));
        }

        // 1. Get user's role IDs
        let role_ids = self.user_role_repository.get_user_role_ids(user_id).await?;

        // 2. Get role policies (from assigned roles)
        let mut permissions = Vec::new();

        if !role_ids.is_empty() {
            // Get role policies
            let role_policies = self
                .role_policy_repository
                .get_role_policies_for_user(realm_id, &role_ids)
                .await?;

            // Get role details for mapping
            let roles = self
                .role_policy_repository
                .get_roles_by_ids(&role_ids)
                .await?;

            // Create a role ID -> role name map
            let role_map: std::collections::HashMap<Uuid, String> =
                roles.into_iter().map(|r| (r.id, r.name)).collect();

            // Add role-based permissions
            // Note: Currently get_role_policies_for_user doesn't return which role they came from
            // We'll mark all as coming from the first role for now
            // TODO: Enhance get_role_policies_for_user to return role information
            for policy in role_policies {
                // For now, assign to first role (this is a limitation of current implementation)
                let first_role_id = role_ids.first().copied().unwrap_or_default();
                let role_name = role_map
                    .get(&first_role_id)
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());

                permissions.push(PermissionDetail {
                    resource: policy.resource,
                    action: policy.action,
                    source: PermissionSource::Role {
                        role_id: first_role_id,
                        role_name,
                    },
                });
            }
        }

        // 3. Get direct user policies (not via roles)
        let direct_policies = self
            .role_policy_repository
            .get_direct_user_policies(user_id)
            .await?;

        // Add direct permissions
        for policy in direct_policies {
            permissions.push(PermissionDetail {
                resource: policy.resource,
                action: policy.action,
                source: PermissionSource::Direct,
            });
        }

        // 4. Deduplicate (direct permissions take precedence over role permissions)
        let mut seen = std::collections::HashSet::new();
        let mut unique_permissions = Vec::new();

        for perm in permissions {
            let key = (perm.resource.clone(), perm.action.clone());
            if !seen.contains(&key) {
                seen.insert(key);
                unique_permissions.push(perm);
            }
        }

        Ok(unique_permissions)
    }

    async fn assign_direct_permission(
        &self,
        identity: Identity,
        realm_id: &str,
        user_id: Uuid,
        policy_id: Uuid,
    ) -> UserAdminResult<()> {
        require_permission(
            &*self.permission_checker,
            &identity,
            realm_id,
            "permissions",
            "manage",
            "Insufficient permissions to assign direct permissions",
        )
        .await?;

        // Realm boundary check
        if identity.realm_id() != realm_id {
            return Err(UserAdminError::PermissionDenied(
                "Cannot assign permissions in a different realm".to_string(),
            ));
        }

        self.role_policy_repository
            .assign_direct_permission(user_id, realm_id, policy_id)
            .await?;

        // Invalidate cache
        let _ = self
            .permission_checker
            .invalidate_user_role_cache(realm_id, &user_id.to_string())
            .await;

        Ok(())
    }

    async fn remove_direct_permission(
        &self,
        identity: Identity,
        realm_id: &str,
        user_id: Uuid,
        policy_id: Uuid,
    ) -> UserAdminResult<()> {
        require_permission(
            &*self.permission_checker,
            &identity,
            realm_id,
            "permissions",
            "manage",
            "Insufficient permissions to remove direct permissions",
        )
        .await?;

        // Realm boundary check
        if identity.realm_id() != realm_id {
            return Err(UserAdminError::PermissionDenied(
                "Cannot remove permissions in a different realm".to_string(),
            ));
        }

        self.role_policy_repository
            .remove_direct_permission(user_id, policy_id)
            .await?;

        // Invalidate cache
        let _ = self
            .permission_checker
            .invalidate_user_role_cache(realm_id, &user_id.to_string())
            .await;

        Ok(())
    }
}

// ============================================================================
// Permission Management Service Implementation
// ============================================================================

pub struct PermissionManagementServiceImpl<UR, RP, P>
where
    UR: UserRoleRepository,
    RP: RolePolicyRepository,
    P: PermissionService,
{
    user_role_repository: Arc<UR>,
    role_policy_repository: Arc<RP>,
    permission_checker: Arc<P>,
}

impl<UR, RP, P> PermissionManagementServiceImpl<UR, RP, P>
where
    UR: UserRoleRepository,
    RP: RolePolicyRepository,
    P: PermissionService,
{
    pub fn new(
        user_role_repository: Arc<UR>,
        role_policy_repository: Arc<RP>,
        permission_checker: Arc<P>,
    ) -> Self {
        Self {
            user_role_repository,
            role_policy_repository,
            permission_checker,
        }
    }
}

impl<UR, RP, P> PermissionManagementService for PermissionManagementServiceImpl<UR, RP, P>
where
    UR: UserRoleRepository + 'static,
    RP: RolePolicyRepository + 'static,
    P: PermissionService + 'static,
{
    async fn create_permission(
        &self,
        identity: Identity,
        realm_id: &str,
        client_id: &str,
        role_id: Option<Uuid>,
        user_id: Option<Uuid>,
        role: Option<Uuid>,
        resource: Option<String>,
        action: Option<String>,
    ) -> UserAdminResult<()> {
        // Principal-based permission check: policies:manage
        require_permission(
            &*self.permission_checker,
            &identity,
            realm_id,
            "policies",
            "manage",
            "Insufficient permissions to manage policies",
        )
        .await?;

        // Realm boundary check
        if identity.realm_id() != realm_id {
            return Err(UserAdminError::PermissionDenied(
                "Cannot manage policies in a different realm".to_string(),
            ));
        }

        // Create role policy
        if let (Some(rid), Some(res), Some(act)) = (role_id, resource, action) {
            self.role_policy_repository
                .create_role_policy(rid, realm_id, &res, &act)
                .await?;

            // Invalidate realm cache
            let _ = self
                .permission_checker
                .invalidate_realm_cache(realm_id)
                .await;
        }

        // Add user role
        if let (Some(uid), Some(r)) = (user_id, role) {
            self.user_role_repository
                .add_user_role(uid, r, realm_id, client_id)
                .await?;

            // Invalidate user cache
            let _ = self
                .permission_checker
                .invalidate_user_role_cache(realm_id, &uid.to_string())
                .await;
        }

        Ok(())
    }

    async fn delete_permission(
        &self,
        identity: Identity,
        realm_id: &str,
        client_id: &str,
        role_id: Option<Uuid>,
        user_id: Option<Uuid>,
        role: Option<Uuid>,
        resource: Option<String>,
        action: Option<String>,
    ) -> UserAdminResult<()> {
        require_permission(
            &*self.permission_checker,
            &identity,
            realm_id,
            "policies",
            "manage",
            "Insufficient permissions to manage policies",
        )
        .await?;

        if identity.realm_id() != realm_id {
            return Err(UserAdminError::PermissionDenied(
                "Cannot manage policies in a different realm".to_string(),
            ));
        }

        // Delete role policy
        if let (Some(rid), Some(res), Some(act)) = (role_id, resource, action) {
            self.role_policy_repository
                .delete_role_policy(rid, &res, &act)
                .await?;

            // Invalidate realm cache
            let _ = self
                .permission_checker
                .invalidate_realm_cache(realm_id)
                .await;
        }

        // Remove user role
        if let (Some(uid), Some(r)) = (user_id, role) {
            self.user_role_repository
                .remove_user_role(uid, r, client_id)
                .await?;

            // Invalidate user cache
            let _ = self
                .permission_checker
                .invalidate_user_role_cache(realm_id, &uid.to_string())
                .await;
        }

        Ok(())
    }

    async fn list_permissions(
        &self,
        realm_id: &str,
        client_id: &str,
    ) -> UserAdminResult<PermissionListData> {
        let role_policies = self
            .role_policy_repository
            .list_role_policies_by_realm(realm_id)
            .await?;

        let user_roles = self
            .user_role_repository
            .list_user_roles_by_realm_client(realm_id, client_id)
            .await?;

        Ok(PermissionListData {
            role_policies,
            user_roles,
        })
    }
}
