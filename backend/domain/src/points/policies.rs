// Points Policy - Permission control for points operations
// PermissionBasedPointsPolicy moved to infrastructure/authorization/policies.rs

use crate::authentication::Identity;
use uuid::Uuid;

/// Points Policy - 积分管理权限
#[allow(clippy::manual_async_fn)]
pub trait PointsPolicy: Send + Sync {
    /// 检查用户是否可以查看积分
    fn can_view_points(
        &self,
        identity: Identity,
        target_user_id: Option<Uuid>,
    ) -> impl Future<Output = bool> + Send;

    /// 检查用户是否可以管理积分
    fn can_manage_points(&self, identity: Identity) -> impl Future<Output = bool> + Send;

    /// 检查用户是否可以消耗积分（SDK API）
    fn can_consume_points(&self, identity: Identity) -> impl Future<Output = bool> + Send;

    /// 检查用户是否可以查看积分配置
    fn can_view_points_configs(&self, identity: Identity) -> impl Future<Output = bool> + Send;

    /// 检查用户是否可以管理积分配置
    fn can_manage_points_configs(&self, identity: Identity) -> impl Future<Output = bool> + Send;
}

/// 允许所有策略（开发/测试用）
#[derive(Debug, Clone)]
pub struct AllowAllPointsPolicy;

#[allow(clippy::manual_async_fn)]
impl PointsPolicy for AllowAllPointsPolicy {
    fn can_view_points(
        &self,
        _identity: Identity,
        _target_user_id: Option<Uuid>,
    ) -> impl Future<Output = bool> + Send {
        async move { true }
    }

    fn can_manage_points(&self, _identity: Identity) -> impl Future<Output = bool> + Send {
        async move { true }
    }

    fn can_consume_points(&self, _identity: Identity) -> impl Future<Output = bool> + Send {
        async move { true }
    }

    fn can_view_points_configs(&self, _identity: Identity) -> impl Future<Output = bool> + Send {
        async move { true }
    }

    fn can_manage_points_configs(&self, _identity: Identity) -> impl Future<Output = bool> + Send {
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
    async fn test_allow_all_points_policy_allows_view() {
        let policy = AllowAllPointsPolicy;
        let identity = create_test_identity("user123", "test-realm");
        let target_user_id = Some(generate_uuid_v7());

        let can_view = policy.can_view_points(identity, target_user_id).await;
        assert!(can_view);
    }

    #[tokio::test]
    async fn test_allow_all_points_policy_allows_manage() {
        let policy = AllowAllPointsPolicy;
        let identity = create_test_identity("user456", "test-realm");

        let can_manage = policy.can_manage_points(identity).await;
        assert!(can_manage);
    }

    #[tokio::test]
    async fn test_allow_all_points_policy_allows_consume() {
        let policy = AllowAllPointsPolicy;
        let identity = create_test_identity("user789", "test-realm");

        let can_consume = policy.can_consume_points(identity).await;
        assert!(can_consume);
    }

    #[tokio::test]
    async fn test_allow_all_points_policy_allows_view_configs() {
        let policy = AllowAllPointsPolicy;
        let identity = create_test_identity("user001", "test-realm");

        let can_view = policy.can_view_points_configs(identity).await;
        assert!(can_view);
    }

    #[tokio::test]
    async fn test_allow_all_points_policy_allows_manage_configs() {
        let policy = AllowAllPointsPolicy;
        let identity = create_test_identity("user002", "test-realm");

        let can_manage = policy.can_manage_points_configs(identity).await;
        assert!(can_manage);
    }
}
