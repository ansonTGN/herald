use chrono::{DateTime, Utc};
use std::sync::Arc;
use uuid::Uuid;

use crate::authentication::Identity;
use crate::common::entities::app_errors::CoreError;
use crate::common::policies::ensure_policy;
use crate::points::{
    CreateRealmConfigInput, PointsPolicy, PointsRepository, RealmDefaultConfig,
    UpdateRealmConfigInput, UserPointsConfig,
};

fn ensure_realm_access(identity: &Identity, realm_id: &str, action: &str) -> Result<(), CoreError> {
    if identity.has_access_to_realm(realm_id) {
        return Ok(());
    }

    Err(CoreError::Forbidden(format!(
        "Access denied: cannot {action} from a different realm"
    )))
}

/// Realm Config Service - Manage realm default configurations
pub struct RealmConfigService<R, P>
where
    R: PointsRepository,
    P: PointsPolicy,
{
    repository: Arc<R>,
    policy: Arc<P>,
}

impl<R, P> RealmConfigService<R, P>
where
    R: PointsRepository + Send + Sync,
    P: PointsPolicy,
{
    pub fn new(repository: Arc<R>, policy: Arc<P>) -> Self {
        Self { repository, policy }
    }

    /// Get realm default config
    pub async fn get_realm_config(
        &self,
        identity: Identity,
        realm_id: &str,
    ) -> Result<RealmDefaultConfig, CoreError> {
        // Check realm boundary
        ensure_realm_access(&identity, realm_id, "access config")?;

        self.repository
            .find_realm_config(realm_id)
            .await?
            .ok_or(CoreError::NotFound)
    }

    /// Create or initialize realm config
    pub async fn create_realm_config(
        &self,
        identity: Identity,
        input: CreateRealmConfigInput,
    ) -> Result<RealmDefaultConfig, CoreError> {
        // Check realm boundary
        ensure_realm_access(&identity, &input.realm_id, "create config")?;

        // Validate input
        input.validate()?;

        // Check if already exists
        if self
            .repository
            .find_realm_config(&input.realm_id)
            .await?
            .is_some()
        {
            return Err(CoreError::BadRequest(format!(
                "Realm config for {} already exists",
                input.realm_id
            )));
        }

        let config = self.repository.create_realm_config(input).await?;

        tracing::info!(
            realm_id = %config.realm_id,
            "Realm config created"
        );

        Ok(config)
    }

    /// Update realm config
    pub async fn update_realm_config(
        &self,
        identity: Identity,
        realm_id: &str,
        input: UpdateRealmConfigInput,
    ) -> Result<RealmDefaultConfig, CoreError> {
        // Check realm boundary
        ensure_realm_access(&identity, realm_id, "update config")?;

        // Validate input
        input.validate()?;

        // Check if exists
        let _existing = self
            .repository
            .find_realm_config(realm_id)
            .await?
            .ok_or(CoreError::NotFound)?;

        let config = self.repository.update_realm_config(realm_id, input).await?;

        tracing::info!(
            realm_id = %realm_id,
            "Realm config updated"
        );

        Ok(config)
    }

    /// Get user points config
    pub async fn get_user_points_config(
        &self,
        identity: Identity,
        realm_id: &str,
        user_id: Uuid,
    ) -> Result<UserPointsConfig, CoreError> {
        // Check manage permissions
        ensure_policy(
            self.policy
                .can_manage_points_configs(identity.clone())
                .await,
            "Insufficient permissions to view user points config",
        )?;

        // Check realm boundary
        ensure_realm_access(&identity, realm_id, "access config")?;

        self.repository
            .find_user_config(user_id)
            .await?
            .ok_or(CoreError::NotFound)
    }

    /// Get free user statistics
    pub async fn get_free_user_statistics(
        &self,
        identity: Identity,
        realm_id: &str,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Result<FreeUserStatistics, CoreError> {
        // Check manage permissions
        ensure_policy(
            self.policy
                .can_manage_points_configs(identity.clone())
                .await,
            "Insufficient permissions to view free user statistics",
        )?;

        // Check realm boundary
        ensure_realm_access(&identity, realm_id, "access statistics")?;

        self.repository
            .get_free_user_statistics(realm_id, start_date, end_date)
            .await
    }
}

/// Free user statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FreeUserStatistics {
    pub total_free_users: i64,
    pub active_free_users: i64,
    pub total_registration_bonus_granted: i64,
    pub total_periodic_points_granted: i64,
    pub average_periodic_points_per_user: f64,
    pub upgrade_rate: f64,
    pub last_updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::entities::generate_uuid_v7;
    use crate::user::entities::{User, UserStatus};

    #[test]
    fn test_ensure_realm_access_accepts_same_realm() {
        let identity = Identity::User(User {
            id: generate_uuid_v7(),
            realm_id: "realm-a".to_string(),
            email: "user@example.com".to_string(),
            nickname: None,
            password_hash: None,
            provider_ids: vec![],
            status: UserStatus::Normal,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });

        assert!(ensure_realm_access(&identity, "realm-a", "access config").is_ok());
    }

    #[test]
    fn test_ensure_realm_access_rejects_other_realm() {
        let identity = Identity::User(User {
            id: generate_uuid_v7(),
            realm_id: "realm-a".to_string(),
            email: "user@example.com".to_string(),
            nickname: None,
            password_hash: None,
            provider_ids: vec![],
            status: UserStatus::Normal,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });

        let error = ensure_realm_access(&identity, "realm-b", "update config").unwrap_err();
        assert!(matches!(error, CoreError::Forbidden(_)));
    }
}
