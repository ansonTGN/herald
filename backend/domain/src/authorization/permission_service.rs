// =============================================================================
// Permission Service Interface - Domain Layer
// =============================================================================
//
// Defines the trait for permission checking operations
// Domain layer has ZERO external dependencies (no sea_orm, no redis)
//
// =============================================================================

use crate::common::entities::app_errors::CoreError;

// ============================================================================
// Domain Entities
// ============================================================================

/// Policy structure representing a role's permission policy
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Policy {
    pub resource: String,
    pub action: String,
}

// ============================================================================
// Permission Service Trait
// ============================================================================

/// Permission checking service interface
///
/// This trait defines the contract for permission checking operations.
/// Implementations are provided in the Infrastructure layer.
#[cfg_attr(test, mockall::automock)]
pub trait PermissionService: Send + Sync {
    /// Check if a user has permission
    ///
    /// # Arguments
    /// * `realm_id` - Realm ID for multi-tenant isolation
    /// * `user_id` - User ID to check permissions for
    /// * `resource` - Resource identifier (e.g., "users", "clients")
    /// * `action` - Action identifier (e.g., "view", "manage")
    ///
    /// # Returns
    /// * `Ok(true)` if permission is granted
    /// * `Ok(false)` if permission is denied
    /// * `Err(CoreError)` if an error occurs
    fn check_permission(
        &self,
        realm_id: &str,
        user_id: &str,
        resource: &str,
        action: &str,
    ) -> impl std::future::Future<Output = Result<bool, CoreError>> + Send;

    /// Get user's roles
    ///
    /// # Arguments
    /// * `realm_id` - Realm ID for multi-tenant isolation
    /// * `user_id` - User ID to get roles for
    ///
    /// # Returns
    /// * `Ok(Vec<String>)` - List of role IDs
    /// * `Err(CoreError)` if an error occurs
    fn get_user_roles(
        &self,
        realm_id: &str,
        user_id: &str,
    ) -> impl std::future::Future<Output = Result<Vec<String>, CoreError>> + Send;

    /// Get role's permission policies
    ///
    /// # Arguments
    /// * `realm_id` - Realm ID for multi-tenant isolation
    /// * `role_id` - Role ID to get policies for
    ///
    /// # Returns
    /// * `Ok(Vec<Policy>)` - List of policies
    /// * `Err(CoreError)` if an error occurs
    fn get_role_policies(
        &self,
        realm_id: &str,
        role_id: &str,
    ) -> impl std::future::Future<Output = Result<Vec<Policy>, CoreError>> + Send;

    /// Invalidate user role cache
    ///
    /// Call this when a user's roles are assigned/removed
    ///
    /// # Arguments
    /// * `realm_id` - Realm ID for multi-tenant isolation
    /// * `user_id` - User ID to invalidate cache for
    fn invalidate_user_role_cache(
        &self,
        realm_id: &str,
        user_id: &str,
    ) -> impl std::future::Future<Output = Result<(), CoreError>> + Send;

    /// Invalidate role policy cache
    ///
    /// Call this when a role's policies are added/removed
    ///
    /// # Arguments
    /// * `realm_id` - Realm ID for multi-tenant isolation
    /// * `role_id` - Role ID to invalidate cache for
    fn invalidate_role_policy_cache(
        &self,
        realm_id: &str,
        role_id: &str,
    ) -> impl std::future::Future<Output = Result<(), CoreError>> + Send;

    /// Invalidate all cache for a realm
    ///
    /// Call this when a realm's RBAC configuration is initialized or updated
    ///
    /// # Arguments
    /// * `realm_id` - Realm ID to invalidate cache for
    fn invalidate_realm_cache(
        &self,
        realm_id: &str,
    ) -> impl std::future::Future<Output = Result<(), CoreError>> + Send;

    /// Get user's effective permissions
    ///
    /// Returns all permissions a user has, including:
    /// - Permissions inherited from roles
    /// - Direct permissions assigned to the user
    ///
    /// Returns permission strings in format "resource:action" (e.g., "users.view", "roles.manage")
    ///
    /// # Arguments
    /// * `realm_id` - Realm ID for multi-tenant isolation
    /// * `user_id` - User ID to get permissions for
    ///
    /// # Returns
    /// * `Ok(Vec<String>)` - List of unique permissions
    /// * `Err(CoreError)` if an error occurs
    fn get_user_permissions(
        &self,
        realm_id: &str,
        user_id: &str,
    ) -> impl std::future::Future<Output = Result<Vec<String>, CoreError>> + Send;
}
