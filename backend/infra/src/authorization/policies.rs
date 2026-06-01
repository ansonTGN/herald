// Permission-based policy implementations
// These are infrastructure-layer implementations that use RedisPermissionChecker directly.
// Moved from domain layer to eliminate domain -> infrastructure dependency.

#![allow(clippy::manual_async_fn)]

use std::sync::Arc;

use crate::authorization::redis_permission_checker::RedisPermissionChecker;
use herald_domain::authentication::Identity;
use herald_domain::authorization::permission_service::PermissionService;
use herald_domain::billing::policies::BillingPolicy;
use herald_domain::common::policies::{
    ClientPolicy, OAuthConfigPolicy, RealmConfigPolicy, RealmPolicy,
};
use herald_domain::points::policies::PointsPolicy;

use uuid::Uuid;

// ============================================================================
// Realm Policy
// ============================================================================

/// Permission-based realm policy
/// Only allows users with "realm.manage" permission to manage realms
///
/// NOTE: Uses concrete RedisPermissionChecker type instead of trait object
/// because PermissionService trait has `impl Future` return types which are
/// not dyn-compatible (cannot be used as `dyn PermissionService`).
#[derive(Debug, Clone)]
pub struct PermissionBasedRealmPolicy {
    permission_checker: Arc<RedisPermissionChecker>,
}

impl PermissionBasedRealmPolicy {
    pub fn new(permission_checker: Arc<RedisPermissionChecker>) -> Self {
        Self { permission_checker }
    }
}

impl RealmPolicy for PermissionBasedRealmPolicy {
    fn can_create_realm(&self, identity: Identity) -> impl Future<Output = bool> + Send {
        let checker = self.permission_checker.clone();
        let principal = identity.principal_ref();
        async move {
            match checker
                .check_principal_permission(
                    "admin",
                    principal.principal_type,
                    &principal.principal_id,
                    "realm",
                    "manage",
                )
                .await
            {
                Ok(has_permission) => {
                    tracing::debug!(
                        principal_type = %principal.principal_type,
                        principal_id = %principal.principal_id,
                        has_permission,
                        "Checked realm.manage permission"
                    );
                    has_permission
                }
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        principal_type = %principal.principal_type,
                        principal_id = %principal.principal_id,
                        "Failed to check realm.manage permission"
                    );
                    false
                }
            }
        }
    }

    fn can_read_realm(&self, identity: Identity) -> impl Future<Output = bool> + Send {
        let checker = self.permission_checker.clone();
        let principal = identity.principal_ref();
        async move {
            checker
                .check_principal_permission(
                    &principal.realm_id,
                    principal.principal_type,
                    &principal.principal_id,
                    "realm",
                    "view",
                )
                .await
                .unwrap_or_default()
        }
    }

    fn can_update_realm(&self, identity: Identity) -> impl Future<Output = bool> + Send {
        let checker = self.permission_checker.clone();
        let principal = identity.principal_ref();
        async move {
            checker
                .check_principal_permission(
                    "admin",
                    principal.principal_type,
                    &principal.principal_id,
                    "realm",
                    "manage",
                )
                .await
                .unwrap_or_default()
        }
    }

    fn can_list_realms(&self, identity: Identity) -> impl Future<Output = bool> + Send {
        let checker = self.permission_checker.clone();
        let principal = identity.principal_ref();
        async move {
            checker
                .check_principal_permission(
                    &principal.realm_id,
                    principal.principal_type,
                    &principal.principal_id,
                    "realm",
                    "view",
                )
                .await
                .unwrap_or_default()
        }
    }
}

// ============================================================================
// Client Policy
// ============================================================================

/// Permission-based client policy
/// Uses clients.view and clients.manage permissions
///
/// NOTE: Uses concrete RedisPermissionChecker type instead of trait object
/// because PermissionService trait has `impl Future` return types which are
/// not dyn-compatible (cannot be used as `dyn PermissionService`).
#[derive(Debug, Clone)]
pub struct PermissionBasedClientPolicy {
    permission_checker: Arc<RedisPermissionChecker>,
}

impl PermissionBasedClientPolicy {
    pub fn new(permission_checker: Arc<RedisPermissionChecker>) -> Self {
        Self { permission_checker }
    }
}

impl ClientPolicy for PermissionBasedClientPolicy {
    fn can_create_client(&self, identity: Identity) -> impl Future<Output = bool> + Send {
        let checker = self.permission_checker.clone();
        let principal = identity.principal_ref();
        async move {
            checker
                .check_principal_permission(
                    &principal.realm_id,
                    principal.principal_type,
                    &principal.principal_id,
                    "clients",
                    "manage",
                )
                .await
                .unwrap_or_default()
        }
    }

    fn can_read_client(&self, identity: Identity) -> impl Future<Output = bool> + Send {
        let checker = self.permission_checker.clone();
        let principal = identity.principal_ref();
        async move {
            checker
                .check_principal_permission(
                    &principal.realm_id,
                    principal.principal_type,
                    &principal.principal_id,
                    "clients",
                    "view",
                )
                .await
                .unwrap_or_default()
        }
    }

    fn can_update_client(&self, identity: Identity) -> impl Future<Output = bool> + Send {
        let checker = self.permission_checker.clone();
        let principal = identity.principal_ref();
        async move {
            checker
                .check_principal_permission(
                    &principal.realm_id,
                    principal.principal_type,
                    &principal.principal_id,
                    "clients",
                    "manage",
                )
                .await
                .unwrap_or_default()
        }
    }

    fn can_delete_client(&self, identity: Identity) -> impl Future<Output = bool> + Send {
        let checker = self.permission_checker.clone();
        let principal = identity.principal_ref();
        async move {
            checker
                .check_principal_permission(
                    &principal.realm_id,
                    principal.principal_type,
                    &principal.principal_id,
                    "clients",
                    "manage",
                )
                .await
                .unwrap_or_default()
        }
    }

    fn can_list_clients(&self, identity: Identity) -> impl Future<Output = bool> + Send {
        let checker = self.permission_checker.clone();
        let principal = identity.principal_ref();
        async move {
            checker
                .check_principal_permission(
                    &principal.realm_id,
                    principal.principal_type,
                    &principal.principal_id,
                    "clients",
                    "view",
                )
                .await
                .unwrap_or_default()
        }
    }
}

// ============================================================================
// RealmConfig Policy
// ============================================================================

/// Permission-based realm config policy
/// Uses settings.view and settings.manage permissions
#[derive(Debug, Clone)]
pub struct PermissionBasedRealmConfigPolicy {
    permission_checker: Arc<RedisPermissionChecker>,
}

impl PermissionBasedRealmConfigPolicy {
    pub fn new(permission_checker: Arc<RedisPermissionChecker>) -> Self {
        Self { permission_checker }
    }
}

impl RealmConfigPolicy for PermissionBasedRealmConfigPolicy {
    fn can_create_config(&self, identity: Identity) -> impl Future<Output = bool> + Send {
        let checker = self.permission_checker.clone();
        let user_id = identity.user_id();
        let realm_id = identity.realm_id();
        async move {
            checker
                .check_permission(&realm_id, &user_id, "settings", "manage")
                .await
                .unwrap_or_default()
        }
    }

    fn can_read_config(&self, identity: Identity) -> impl Future<Output = bool> + Send {
        let checker = self.permission_checker.clone();
        let user_id = identity.user_id();
        let realm_id = identity.realm_id();
        async move {
            checker
                .check_permission(&realm_id, &user_id, "settings", "view")
                .await
                .unwrap_or_default()
        }
    }

    fn can_update_config(&self, identity: Identity) -> impl Future<Output = bool> + Send {
        let checker = self.permission_checker.clone();
        let user_id = identity.user_id();
        let realm_id = identity.realm_id();
        async move {
            checker
                .check_permission(&realm_id, &user_id, "settings", "manage")
                .await
                .unwrap_or_default()
        }
    }

    fn can_delete_config(&self, identity: Identity) -> impl Future<Output = bool> + Send {
        let checker = self.permission_checker.clone();
        let user_id = identity.user_id();
        let realm_id = identity.realm_id();
        async move {
            checker
                .check_permission(&realm_id, &user_id, "settings", "manage")
                .await
                .unwrap_or_default()
        }
    }
}

// ============================================================================
// OAuthConfig Policy
// ============================================================================

/// Permission-based OAuth config policy
/// Uses settings.view and settings.manage permissions
#[derive(Debug, Clone)]
pub struct PermissionBasedOAuthConfigPolicy {
    permission_checker: Arc<RedisPermissionChecker>,
}

impl PermissionBasedOAuthConfigPolicy {
    pub fn new(permission_checker: Arc<RedisPermissionChecker>) -> Self {
        Self { permission_checker }
    }
}

impl OAuthConfigPolicy for PermissionBasedOAuthConfigPolicy {
    fn can_create_config(&self, identity: Identity) -> impl Future<Output = bool> + Send {
        let checker = self.permission_checker.clone();
        let user_id = identity.user_id();
        let realm_id = identity.realm_id();
        async move {
            checker
                .check_permission(&realm_id, &user_id, "settings", "manage")
                .await
                .unwrap_or_default()
        }
    }

    fn can_read_config(&self, identity: Identity) -> impl Future<Output = bool> + Send {
        let checker = self.permission_checker.clone();
        let user_id = identity.user_id();
        let realm_id = identity.realm_id();
        async move {
            checker
                .check_permission(&realm_id, &user_id, "settings", "view")
                .await
                .unwrap_or_default()
        }
    }

    fn can_update_config(&self, identity: Identity) -> impl Future<Output = bool> + Send {
        let checker = self.permission_checker.clone();
        let user_id = identity.user_id();
        let realm_id = identity.realm_id();
        async move {
            checker
                .check_permission(&realm_id, &user_id, "settings", "manage")
                .await
                .unwrap_or_default()
        }
    }

    fn can_delete_config(&self, identity: Identity) -> impl Future<Output = bool> + Send {
        let checker = self.permission_checker.clone();
        let user_id = identity.user_id();
        let realm_id = identity.realm_id();
        async move {
            checker
                .check_permission(&realm_id, &user_id, "settings", "manage")
                .await
                .unwrap_or_default()
        }
    }

    fn can_list_configs(&self, identity: Identity) -> impl Future<Output = bool> + Send {
        let checker = self.permission_checker.clone();
        let user_id = identity.user_id();
        let realm_id = identity.realm_id();
        async move {
            checker
                .check_permission(&realm_id, &user_id, "settings", "view")
                .await
                .unwrap_or_default()
        }
    }
}

// ============================================================================
// Billing Policy
// ============================================================================

/// 基于权限的策略（生产环境）
///
/// NOTE: Uses concrete RedisPermissionChecker type instead of trait object
/// because PermissionService trait has `impl Future` return types which are
/// not dyn-compatible (cannot be used as `dyn PermissionService`).
#[derive(Debug, Clone)]
pub struct PermissionBasedBillingPolicy {
    permission_checker: Arc<RedisPermissionChecker>,
}

impl PermissionBasedBillingPolicy {
    pub fn new(permission_checker: Arc<RedisPermissionChecker>) -> Self {
        Self { permission_checker }
    }
}

#[allow(clippy::manual_async_fn)]
impl BillingPolicy for PermissionBasedBillingPolicy {
    fn can_view_subscription_plans(&self, identity: Identity) -> impl Future<Output = bool> + Send {
        let checker = self.permission_checker.clone();
        let user_id = identity.user_id();
        async move {
            let realm_id = identity.realm_id();
            match checker
                .check_permission(&realm_id, &user_id, "billing", "view")
                .await
            {
                Ok(has_permission) => {
                    tracing::debug!(user_id = %user_id, realm_id = %realm_id, has_permission,
                        "Checked billing.view permission");
                    has_permission
                }
                Err(e) => {
                    tracing::error!(error = %e, user_id = %user_id, realm_id = %realm_id,
                        "Failed to check billing.view permission");
                    false
                }
            }
        }
    }

    fn can_manage_subscription_plans(
        &self,
        identity: Identity,
    ) -> impl Future<Output = bool> + Send {
        let checker = self.permission_checker.clone();
        let user_id = identity.user_id();
        async move {
            let realm_id = identity.realm_id();
            match checker
                .check_permission(&realm_id, &user_id, "billing", "manage")
                .await
            {
                Ok(has_permission) => {
                    tracing::debug!(user_id = %user_id, realm_id = %realm_id, has_permission,
                        "Checked billing.manage permission");
                    has_permission
                }
                Err(e) => {
                    tracing::error!(error = %e, user_id = %user_id, realm_id = %realm_id,
                        "Failed to check billing.manage permission");
                    false
                }
            }
        }
    }
}

// ============================================================================
// Points Policy
// ============================================================================

/// 基于权限的策略（生产环境）
///
/// NOTE: Uses concrete RedisPermissionChecker type instead of trait object
/// because PermissionService trait has `impl Future` return types which are
/// not dyn-compatible (cannot be used as `dyn PermissionService`).
#[derive(Debug, Clone)]
pub struct PermissionBasedPointsPolicy {
    permission_checker: Arc<RedisPermissionChecker>,
}

impl PermissionBasedPointsPolicy {
    pub fn new(permission_checker: Arc<RedisPermissionChecker>) -> Self {
        Self { permission_checker }
    }

    pub fn blocks_third_party_points_configs(identity: &Identity, action: &str) -> bool {
        let blocked = matches!(identity, Identity::ThirdParty(_));
        if blocked {
            tracing::debug!(
                identity = %identity,
                action,
                "ThirdParty identity cannot access points config APIs"
            );
        }
        blocked
    }
}

#[allow(clippy::manual_async_fn)]
impl PointsPolicy for PermissionBasedPointsPolicy {
    fn can_view_points(
        &self,
        identity: Identity,
        target_user_id: Option<Uuid>,
    ) -> impl Future<Output = bool> + Send {
        let checker = self.permission_checker.clone();
        let user_id = identity.user_id();
        async move {
            // ThirdParty identity (API key) can view points
            if matches!(identity, Identity::ThirdParty(_)) {
                tracing::debug!(
                    identity = %identity,
                    "ThirdParty identity viewing points"
                );
                return true;
            }

            let realm_id = identity.realm_id();

            // Users can always view their own data
            if let (Some(target), Some(current)) = (target_user_id, user_id.parse::<Uuid>().ok())
                && target == current
            {
                tracing::debug!(
                    user_id = %user_id,
                    "User viewing own points wallet"
                );
                return true;
            }

            // Check if user has points.view permission
            let has_permission_result = checker
                .check_permission(&realm_id, &user_id, "points", "view")
                .await;

            let has_permission = match has_permission_result {
                Ok(perm) => perm,
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        user_id = %user_id,
                        realm_id = %realm_id,
                        "Failed to check points.view permission"
                    );
                    return false;
                }
            };

            if !has_permission {
                return false;
            }

            // Check if user has points.manage permission (admin)
            let has_manage_permission_result = checker
                .check_permission(&realm_id, &user_id, "points", "manage")
                .await;

            let has_manage_permission = match has_manage_permission_result {
                Ok(perm) => perm,
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        user_id = %user_id,
                        realm_id = %realm_id,
                        "Failed to check points.manage permission"
                    );
                    false
                }
            };

            if has_manage_permission {
                return true;
            }

            // User with points.view but not manage can only view their own
            match (target_user_id, user_id.parse::<Uuid>().ok()) {
                (None, Some(_current)) => {
                    tracing::debug!(
                        user_id = %user_id,
                        "User viewing own transaction history"
                    );
                    true
                }
                _ => false,
            }
        }
    }

    fn can_manage_points(&self, identity: Identity) -> impl Future<Output = bool> + Send {
        let checker = self.permission_checker.clone();
        let user_id = identity.user_id();
        async move {
            let realm_id = identity.realm_id();
            match checker
                .check_permission(&realm_id, &user_id, "points", "manage")
                .await
            {
                Ok(has_permission) => {
                    tracing::debug!(
                        user_id = %user_id,
                        realm_id = %realm_id,
                        has_permission,
                        "Checked points.manage permission"
                    );
                    has_permission
                }
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        user_id = %user_id,
                        realm_id = %realm_id,
                        "Failed to check points.manage permission"
                    );
                    false
                }
            }
        }
    }

    fn can_consume_points(&self, identity: Identity) -> impl Future<Output = bool> + Send {
        async move {
            let has_access = matches!(identity, Identity::ThirdParty(_));
            tracing::debug!(
                identity = %identity,
                has_access,
                "Checked points.consume permission"
            );
            has_access
        }
    }

    fn can_view_points_configs(&self, identity: Identity) -> impl Future<Output = bool> + Send {
        let checker = self.permission_checker.clone();
        let user_id = identity.user_id();
        async move {
            if Self::blocks_third_party_points_configs(&identity, "view") {
                return false;
            }

            let realm_id = identity.realm_id();
            match checker
                .check_permission(&realm_id, &user_id, "points", "view")
                .await
            {
                Ok(has_permission) => {
                    tracing::debug!(
                        user_id = %user_id,
                        realm_id = %realm_id,
                        has_permission,
                        "Checked points.view permission (for configs)"
                    );
                    has_permission
                }
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        user_id = %user_id,
                        realm_id = %realm_id,
                        "Failed to check points.view permission"
                    );
                    false
                }
            }
        }
    }

    fn can_manage_points_configs(&self, identity: Identity) -> impl Future<Output = bool> + Send {
        let checker = self.permission_checker.clone();
        let user_id = identity.user_id();
        async move {
            if Self::blocks_third_party_points_configs(&identity, "manage") {
                return false;
            }

            let realm_id = identity.realm_id();
            match checker
                .check_permission(&realm_id, &user_id, "points", "manage")
                .await
            {
                Ok(has_permission) => {
                    tracing::debug!(
                        user_id = %user_id,
                        realm_id = %realm_id,
                        has_permission,
                        "Checked points.manage permission (for configs)"
                    );
                    has_permission
                }
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        user_id = %user_id,
                        realm_id = %realm_id,
                        "Failed to check points.manage permission"
                    );
                    false
                }
            }
        }
    }
}
