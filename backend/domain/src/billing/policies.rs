// Billing Policy - Permission control for billing operations
// PermissionBasedBillingPolicy moved to infrastructure/authorization/policies.rs

use crate::authentication::Identity;

/// Billing Policy - 计费管理权限
#[allow(clippy::manual_async_fn)]
pub trait BillingPolicy: Send + Sync {
    /// 检查用户是否可以查看计费计划
    fn can_view_plans(&self, identity: Identity) -> impl Future<Output = bool> + Send;

    /// 检查用户是否可以管理计费计划
    fn can_manage_plans(&self, identity: Identity) -> impl Future<Output = bool> + Send;
}

/// 允许所有策略（开发/测试用）
#[derive(Debug, Clone)]
pub struct AllowAllBillingPolicy;

#[allow(clippy::manual_async_fn)]
impl BillingPolicy for AllowAllBillingPolicy {
    fn can_view_plans(&self, _identity: Identity) -> impl Future<Output = bool> + Send {
        async move { true }
    }

    fn can_manage_plans(&self, _identity: Identity) -> impl Future<Output = bool> + Send {
        async move { true }
    }
}
