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
    fn check_permission(
        &self,
        realm_id: &str,
        user_id: &str,
        resource: &str,
        action: &str,
    ) -> impl std::future::Future<Output = Result<bool, CoreError>> + Send;

    fn get_user_roles(
        &self,
        realm_id: &str,
        user_id: &str,
    ) -> impl std::future::Future<Output = Result<Vec<String>, CoreError>> + Send;

    fn get_role_policies(
        &self,
        realm_id: &str,
        role_id: &str,
    ) -> impl std::future::Future<Output = Result<Vec<Policy>, CoreError>> + Send;

    fn invalidate_user_role_cache(
        &self,
        realm_id: &str,
        user_id: &str,
    ) -> impl std::future::Future<Output = Result<(), CoreError>> + Send;

    fn invalidate_role_policy_cache(
        &self,
        realm_id: &str,
        role_id: &str,
    ) -> impl std::future::Future<Output = Result<(), CoreError>> + Send;

    fn invalidate_realm_cache(
        &self,
        realm_id: &str,
    ) -> impl std::future::Future<Output = Result<(), CoreError>> + Send;

    /// Returns permission strings in format "resource:action"
    fn get_user_permissions(
        &self,
        realm_id: &str,
        user_id: &str,
    ) -> impl std::future::Future<Output = Result<Vec<String>, CoreError>> + Send;

    /// Generalized permission check for any principal type (user, api_key, client).
    fn check_principal_permission(
        &self,
        realm_id: &str,
        principal_type: &str,
        principal_id: &str,
        resource: &str,
        action: &str,
    ) -> impl std::future::Future<Output = Result<bool, CoreError>> + Send;

    /// Invalidate cached roles and permissions for any principal type.
    fn invalidate_principal_role_cache(
        &self,
        realm_id: &str,
        principal_type: &str,
        principal_id: &str,
    ) -> impl std::future::Future<Output = Result<(), CoreError>> + Send;
}
