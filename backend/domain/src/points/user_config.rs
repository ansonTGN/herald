use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::points::grant_schedule::GrantPeriodType;

/// User points configuration domain model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPointsConfig {
    pub user_id: Uuid,
    pub realm_id: String,
    pub registration_bonus_points: i64,
    pub free_periodic_points_amount: i64,
    pub free_periodic_grant_period_type: Option<GrantPeriodType>,
    pub free_periodic_validity_days: i64,
    pub next_grant_time: Option<DateTime<Utc>>,
    pub granted_periods: i64,
    pub grant_schedule_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl UserPointsConfig {
    /// Check if periodic grant is enabled and due
    pub fn is_periodic_grant_due(&self, now: DateTime<Utc>) -> bool {
        self.free_periodic_points_amount > 0 && self.next_grant_time.is_some_and(|next| next <= now)
    }

    /// Check if this is a free user config (no subscription)
    pub fn is_free_user(&self) -> bool {
        self.grant_schedule_id.is_none()
    }

    /// Check if this is a one-time grant that has already been granted
    pub fn is_once_grant_completed(&self) -> bool {
        matches!(
            self.free_periodic_grant_period_type,
            Some(GrantPeriodType::Once)
        ) && self.granted_periods > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_points_config_is_periodic_grant_due() {
        let now = Utc::now();
        let past = now - chrono::Duration::hours(1);
        let future = now + chrono::Duration::hours(1);

        let mut config = UserPointsConfig {
            user_id: Uuid::now_v7(),
            realm_id: "test".to_string(),
            registration_bonus_points: 0,
            free_periodic_points_amount: 50,
            free_periodic_grant_period_type: Some(GrantPeriodType::Daily),
            free_periodic_validity_days: 1,
            next_grant_time: Some(past),
            granted_periods: 0,
            grant_schedule_id: None,
            created_at: now,
            updated_at: now,
        };

        // Should be due when next_grant_time is in the past and amount > 0
        assert!(config.is_periodic_grant_due(now));

        // Should not be due when next_grant_time is in the future
        config.next_grant_time = Some(future);
        assert!(!config.is_periodic_grant_due(now));

        // Should not be due when free_periodic_points_amount is 0
        config.next_grant_time = Some(past);
        config.free_periodic_points_amount = 0;
        assert!(!config.is_periodic_grant_due(now));
    }

    #[test]
    fn test_user_points_config_is_free_user() {
        let config = UserPointsConfig {
            user_id: Uuid::now_v7(),
            realm_id: "test".to_string(),
            registration_bonus_points: 0,
            free_periodic_points_amount: 0,
            free_periodic_grant_period_type: None,
            free_periodic_validity_days: 1,
            next_grant_time: None,
            granted_periods: 0,
            grant_schedule_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(config.is_free_user());

        let mut paid_config = config;
        paid_config.grant_schedule_id = Some(Uuid::now_v7());
        assert!(!paid_config.is_free_user());
    }

    #[test]
    fn test_user_points_config_is_once_grant_completed() {
        let now = Utc::now();

        // Once grant with 0 periods granted - not completed
        let config = UserPointsConfig {
            user_id: Uuid::now_v7(),
            realm_id: "test".to_string(),
            registration_bonus_points: 0,
            free_periodic_points_amount: 100,
            free_periodic_grant_period_type: Some(GrantPeriodType::Once),
            free_periodic_validity_days: 0,
            next_grant_time: Some(now),
            granted_periods: 0,
            grant_schedule_id: None,
            created_at: now,
            updated_at: now,
        };
        assert!(!config.is_once_grant_completed());

        // Once grant with 1 period granted - completed
        let mut config = config;
        config.granted_periods = 1;
        assert!(config.is_once_grant_completed());

        // Daily grant - never considered "once completed"
        config.free_periodic_grant_period_type = Some(GrantPeriodType::Daily);
        assert!(!config.is_once_grant_completed());
    }
}
