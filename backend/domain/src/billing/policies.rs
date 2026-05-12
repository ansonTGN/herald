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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::authentication::Identity;
    use crate::common::entities::generate_uuid_v7;
    use crate::user::entities::{User, UserStatus};
    use chrono::Utc;

    fn create_test_identity(user_id: &str, realm_id: &str) -> Identity {
        let user = User {
            id: generate_uuid_v7(),
            realm_id: realm_id.to_string(),
            email: format!("{}@test.com", user_id),
            nickname: None,
            password_hash: None,
            provider_ids: vec![],
            status: UserStatus::Normal,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        Identity::User(user)
    }

    #[tokio::test]
    async fn test_allow_all_billing_policy_allows_view() {
        let policy = AllowAllBillingPolicy;
        let identity = create_test_identity("user123", "test-realm");

        let can_view = policy.can_view_plans(identity).await;
        assert!(can_view);
    }

    #[tokio::test]
    async fn test_allow_all_billing_policy_allows_manage() {
        let policy = AllowAllBillingPolicy;
        let identity = create_test_identity("user456", "test-realm");

        let can_manage = policy.can_manage_plans(identity).await;
        assert!(can_manage);
    }
}
