// Billing Policy - Permission control for billing operations
// PermissionBasedBillingPolicy moved to infrastructure/authorization/policies.rs

use std::future::Future;

use crate::authentication::Identity;

/// Billing Policy - Permission control for billing operations
#[allow(clippy::manual_async_fn)]
pub trait BillingPolicy: Send + Sync {
    /// Check if user can view billing
    fn can_view_billing(&self, identity: Identity) -> impl Future<Output = bool> + Send;

    /// Check if user can manage billing
    fn can_manage_billing(&self, identity: Identity) -> impl Future<Output = bool> + Send;
}

/// Allow-all policy (for development/testing)
#[derive(Debug, Clone)]
pub struct AllowAllBillingPolicy;

#[allow(clippy::manual_async_fn)]
impl BillingPolicy for AllowAllBillingPolicy {
    fn can_view_billing(&self, _identity: Identity) -> impl Future<Output = bool> + Send {
        async move { true }
    }

    fn can_manage_billing(&self, _identity: Identity) -> impl Future<Output = bool> + Send {
        async move { true }
    }
}
