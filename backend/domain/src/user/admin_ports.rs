// User Admin Domain Ports (Traits)
//
// These traits define the interfaces for repositories and services
// following the Dependency Inversion Principle.

use std::future::Future;
use uuid::Uuid;

use super::{
    admin_dtos::{
        AdminUser, CreateUserWithRolesRequest, PermissionDetail, RoleDetail, UpdateUserAdminRequest,
    },
    admin_entities::{AdminUserEntity, PolicyEntity, RoleEntity},
    admin_errors::UserAdminResult,
};

type ApiKeyRoleSummaries = Vec<(String, Vec<(Uuid, String)>)>;
use crate::authentication::Identity;

// ============================================================================
// Repository Ports
// ============================================================================

/// Repository for admin user operations
pub trait AdminUserRepository: Send + Sync {
    /// Create a user with profile in a single transaction
    fn create_user_with_profile(
        &self,
        realm_id: &str,
        email: &str,
        password_hash: &str,
        nickname: Option<&str>,
        status: i32,
    ) -> impl Future<Output = UserAdminResult<Uuid>> + Send;

    /// Update user fields
    fn update_user_fields(
        &self,
        user_id: Uuid,
        email: Option<&str>,
        nickname: Option<&str>,
        status: Option<i32>,
    ) -> impl Future<Output = UserAdminResult<()>> + Send;

    /// Get user with profile
    fn get_user_with_profile(
        &self,
        user_id: Uuid,
    ) -> impl Future<Output = UserAdminResult<Option<AdminUserEntity>>> + Send;

    /// Check if email exists in realm
    fn email_exists(
        &self,
        realm_id: &str,
        email: &str,
    ) -> impl Future<Output = UserAdminResult<bool>> + Send;

    /// Get user by email
    fn get_user_by_email(
        &self,
        realm_id: &str,
        email: &str,
    ) -> impl Future<Output = UserAdminResult<Option<AdminUserEntity>>> + Send;

    /// Delete user (including profile and account) in a transaction
    fn delete_user(&self, user_id: Uuid) -> impl Future<Output = UserAdminResult<bool>> + Send;

    /// Update user password
    fn update_user_password(
        &self,
        user_id: Uuid,
        password_hash: &str,
    ) -> impl Future<Output = UserAdminResult<bool>> + Send;
}

/// Repository for user role operations
pub trait UserRoleRepository: Send + Sync {
    /// Replace all roles for a user (transactional)
    fn replace_user_roles(
        &self,
        user_id: Uuid,
        realm_id: &str,
        client_id: &str,
        role_ids: &[Uuid],
    ) -> impl Future<Output = UserAdminResult<()>> + Send;

    /// Get role IDs assigned to a user
    fn get_user_role_ids(
        &self,
        user_id: Uuid,
    ) -> impl Future<Output = UserAdminResult<Vec<Uuid>>> + Send;

    /// Get roles with details for a user
    fn get_user_roles(
        &self,
        user_id: Uuid,
    ) -> impl Future<Output = UserAdminResult<Vec<RoleEntity>>> + Send;

    /// Add a single role to a user
    fn add_user_role(
        &self,
        user_id: Uuid,
        role_id: Uuid,
        realm_id: &str,
        client_id: &str,
    ) -> impl Future<Output = UserAdminResult<()>> + Send;

    /// Remove a single role from a user
    fn remove_user_role(
        &self,
        user_id: Uuid,
        role_id: Uuid,
        client_id: &str,
    ) -> impl Future<Output = UserAdminResult<bool>> + Send;

    /// List all user roles in a realm/client
    fn list_user_roles_by_realm_client(
        &self,
        realm_id: &str,
        client_id: &str,
    ) -> impl Future<Output = UserAdminResult<Vec<(Uuid, Uuid)>>> + Send;

    /// Replace all roles for an API key principal (transactional)
    fn replace_api_key_roles(
        &self,
        api_key_id: &str,
        realm_id: &str,
        client_id: &str,
        role_ids: &[Uuid],
    ) -> impl Future<Output = UserAdminResult<()>> + Send;

    /// Get roles with details for an API key principal
    fn get_api_key_roles(
        &self,
        api_key_id: &str,
    ) -> impl Future<Output = UserAdminResult<Vec<RoleEntity>>> + Send;

    /// Batch get role summaries for multiple API key principals
    /// Returns (principal_id, Vec<(role_id, role_name)>)
    fn get_api_key_role_summaries_batch(
        &self,
        api_key_ids: &[String],
    ) -> impl Future<Output = UserAdminResult<ApiKeyRoleSummaries>> + Send;
}

/// Repository for role policy operations
pub trait RolePolicyRepository: Send + Sync {
    /// Get all role policies for a user (via their assigned roles)
    fn get_role_policies_for_user(
        &self,
        realm_id: &str,
        role_ids: &[Uuid],
    ) -> impl Future<Output = UserAdminResult<Vec<PolicyEntity>>> + Send;

    /// Get direct user policies (not via roles)
    fn get_direct_user_policies(
        &self,
        user_id: Uuid,
    ) -> impl Future<Output = UserAdminResult<Vec<PolicyEntity>>> + Send;

    /// Get roles by IDs
    fn get_roles_by_ids(
        &self,
        role_ids: &[Uuid],
    ) -> impl Future<Output = UserAdminResult<Vec<RoleEntity>>> + Send;

    /// Assign direct permission to user
    fn assign_direct_permission(
        &self,
        user_id: Uuid,
        realm_id: &str,
        policy_id: Uuid,
    ) -> impl Future<Output = UserAdminResult<()>> + Send;

    /// Remove direct permission from user
    fn remove_direct_permission(
        &self,
        user_id: Uuid,
        policy_id: Uuid,
    ) -> impl Future<Output = UserAdminResult<()>> + Send;

    /// Create a role policy
    fn create_role_policy(
        &self,
        role_id: Uuid,
        realm_id: &str,
        resource: &str,
        action: &str,
    ) -> impl Future<Output = UserAdminResult<()>> + Send;

    /// Delete a role policy
    fn delete_role_policy(
        &self,
        role_id: Uuid,
        resource: &str,
        action: &str,
    ) -> impl Future<Output = UserAdminResult<bool>> + Send;

    /// List all role policies in a realm
    fn list_role_policies_by_realm(
        &self,
        realm_id: &str,
    ) -> impl Future<Output = UserAdminResult<Vec<(Uuid, String, String)>>> + Send;
}

// ============================================================================
// Service Ports
// ============================================================================

/// Service for admin user management
pub trait AdminUserService: Send + Sync {
    /// Create a new user with roles
    fn create_user_with_roles(
        &self,
        identity: Identity,
        realm_id: &str,
        request: CreateUserWithRolesRequest,
    ) -> impl Future<Output = UserAdminResult<AdminUser>> + Send;

    /// Update user admin fields
    fn update_user_admin(
        &self,
        identity: Identity,
        realm_id: &str,
        user_id: Uuid,
        request: UpdateUserAdminRequest,
    ) -> impl Future<Output = UserAdminResult<AdminUser>> + Send;

    /// Get user admin details
    fn get_user_admin(
        &self,
        identity: Identity,
        realm_id: &str,
        user_id: Uuid,
    ) -> impl Future<Output = UserAdminResult<AdminUser>> + Send;

    /// Delete a user (including profile and account)
    fn delete_user(
        &self,
        identity: Identity,
        realm_id: &str,
        user_id: Uuid,
    ) -> impl Future<Output = UserAdminResult<()>> + Send;

    /// Reset user password (generate random password)
    fn reset_user_password(
        &self,
        identity: Identity,
        realm_id: &str,
        user_id: Uuid,
    ) -> impl Future<Output = UserAdminResult<String>> + Send;
}

/// Service for role assignment
pub trait RoleAssignmentService: Send + Sync {
    /// Assign roles to a user
    fn assign_user_roles(
        &self,
        identity: Identity,
        realm_id: &str,
        user_id: Uuid,
        role_ids: Vec<Uuid>,
    ) -> impl Future<Output = UserAdminResult<()>> + Send;

    /// Get user roles
    fn get_user_roles(
        &self,
        identity: Identity,
        realm_id: &str,
        user_id: Uuid,
    ) -> impl Future<Output = UserAdminResult<Vec<RoleDetail>>> + Send;

    /// Check if roles can be assigned
    fn can_assign_roles(
        &self,
        identity: &Identity,
        realm_id: &str,
        role_ids: &[Uuid],
    ) -> impl Future<Output = UserAdminResult<bool>> + Send;

    /// Assign roles to an API Key principal
    fn assign_api_key_roles(
        &self,
        identity: Identity,
        realm_id: &str,
        api_key_id: &str,
        role_ids: Vec<Uuid>,
    ) -> impl Future<Output = UserAdminResult<()>> + Send;

    /// Get roles assigned to an API Key principal
    fn get_api_key_roles(
        &self,
        identity: Identity,
        realm_id: &str,
        api_key_id: &str,
    ) -> impl Future<Output = UserAdminResult<Vec<RoleEntity>>> + Send;
}

/// Service for user permission management
pub trait UserPermissionService: Send + Sync {
    /// Get effective permissions for a user (roles + direct)
    fn get_effective_permissions(
        &self,
        identity: Identity,
        realm_id: &str,
        user_id: Uuid,
    ) -> impl Future<Output = UserAdminResult<Vec<PermissionDetail>>> + Send;

    /// Assign direct permission to user
    fn assign_direct_permission(
        &self,
        identity: Identity,
        realm_id: &str,
        user_id: Uuid,
        policy_id: Uuid,
    ) -> impl Future<Output = UserAdminResult<()>> + Send;

    /// Remove direct permission from user
    fn remove_direct_permission(
        &self,
        identity: Identity,
        realm_id: &str,
        user_id: Uuid,
        policy_id: Uuid,
    ) -> impl Future<Output = UserAdminResult<()>> + Send;
}

/// Service for permission and role policy management
///
/// This service handles low-level permission operations including:
/// - Creating/deleting role policies
/// - Adding/removing user role assignments
/// - Listing permissions and roles
pub trait PermissionManagementService: Send + Sync {
    /// Create a permission (role policy or user role)
    fn create_permission(
        &self,
        identity: Identity,
        realm_id: &str,
        client_id: &str,
        role_id: Option<Uuid>,    // For role policies
        user_id: Option<Uuid>,    // For user roles
        role: Option<Uuid>,       // For user roles
        resource: Option<String>, // For role policies
        action: Option<String>,   // For role policies
    ) -> impl Future<Output = UserAdminResult<()>> + Send;

    /// Delete a permission (role policy or user role)
    fn delete_permission(
        &self,
        identity: Identity,
        realm_id: &str,
        client_id: &str,
        role_id: Option<Uuid>,    // For role policies
        user_id: Option<Uuid>,    // For user roles
        role: Option<Uuid>,       // For user roles
        resource: Option<String>, // For role policies
        action: Option<String>,   // For role policies
    ) -> impl Future<Output = UserAdminResult<()>> + Send;

    /// List all permissions in a realm/client
    fn list_permissions(
        &self,
        realm_id: &str,
        client_id: &str,
    ) -> impl Future<Output = UserAdminResult<PermissionListData>> + Send;
}

/// Data structure for permission list response
#[derive(Debug, Clone)]
pub struct PermissionListData {
    /// Role policies: (role_id, resource, action)
    pub role_policies: Vec<(Uuid, String, String)>,
    /// User roles: (user_id, role_id)
    pub user_roles: Vec<(Uuid, Uuid)>,
}
