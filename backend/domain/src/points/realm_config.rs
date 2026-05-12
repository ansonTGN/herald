use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::common::entities::app_errors::CoreError;
use crate::points::grant_schedule::GrantPeriodType;

/// Shared validation trait for realm configuration
trait RealmConfigValidator {
    fn registration_bonus_points(&self) -> i64;
    fn free_periodic_points_amount(&self) -> i64;
    fn free_periodic_validity_days(&self) -> i64;
    fn free_periodic_grant_period_type(&self) -> Result<GrantPeriodType, CoreError>;

    fn validate_fields(&self) -> Result<(), CoreError> {
        if self.registration_bonus_points() < 0 {
            return Err(CoreError::BadRequest(
                "registration_bonus_points cannot be negative".to_string(),
            ));
        }
        if self.free_periodic_points_amount() < 0 {
            return Err(CoreError::BadRequest(
                "free_periodic_points_amount cannot be negative".to_string(),
            ));
        }

        // For 'once' period, allow validity_days = 0 (permanent)
        // For other periods, require validity_days >= 1
        let period_type = self.free_periodic_grant_period_type()?;
        let is_once = matches!(period_type, GrantPeriodType::Once);

        if self.free_periodic_validity_days() < 0 {
            return Err(CoreError::BadRequest(
                "free_periodic_validity_days cannot be negative".to_string(),
            ));
        }

        if !is_once && self.free_periodic_validity_days() < 1 {
            return Err(CoreError::BadRequest(
                "free_periodic_validity_days must be at least 1 for non-once periods".to_string(),
            ));
        }

        Ok(())
    }
}

/// Realm default configuration domain model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealmDefaultConfig {
    pub realm_id: String,
    pub registration_bonus_points: i64,
    pub free_periodic_points_amount: i64,
    pub free_periodic_grant_period_type: GrantPeriodType,
    pub free_periodic_validity_days: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl RealmConfigValidator for RealmDefaultConfig {
    fn registration_bonus_points(&self) -> i64 {
        self.registration_bonus_points
    }

    fn free_periodic_points_amount(&self) -> i64 {
        self.free_periodic_points_amount
    }

    fn free_periodic_validity_days(&self) -> i64 {
        self.free_periodic_validity_days
    }

    fn free_periodic_grant_period_type(&self) -> Result<GrantPeriodType, CoreError> {
        Ok(self.free_periodic_grant_period_type)
    }
}

impl RealmDefaultConfig {
    /// Validate configuration values
    pub fn validate(&self) -> Result<(), CoreError> {
        self.validate_fields()
    }
}

/// Update realm config input DTO
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateRealmConfigInput {
    pub registration_bonus_points: i64,
    pub free_periodic_points_amount: i64,
    pub free_periodic_grant_period_type: String,
    pub free_periodic_validity_days: i64,
}

impl RealmConfigValidator for UpdateRealmConfigInput {
    fn registration_bonus_points(&self) -> i64 {
        self.registration_bonus_points
    }

    fn free_periodic_points_amount(&self) -> i64 {
        self.free_periodic_points_amount
    }

    fn free_periodic_validity_days(&self) -> i64 {
        self.free_periodic_validity_days
    }

    fn free_periodic_grant_period_type(&self) -> Result<GrantPeriodType, CoreError> {
        self.free_periodic_grant_period_type.parse()
    }
}

impl UpdateRealmConfigInput {
    pub fn validate(&self) -> Result<(), CoreError> {
        self.validate_fields()
    }
}

/// Create realm config input DTO (for initialization)
#[derive(Debug, Clone, Deserialize)]
pub struct CreateRealmConfigInput {
    pub realm_id: String,
    pub registration_bonus_points: i64,
    pub free_periodic_points_amount: i64,
    pub free_periodic_grant_period_type: String,
    pub free_periodic_validity_days: i64,
}

impl RealmConfigValidator for CreateRealmConfigInput {
    fn registration_bonus_points(&self) -> i64 {
        self.registration_bonus_points
    }

    fn free_periodic_points_amount(&self) -> i64 {
        self.free_periodic_points_amount
    }

    fn free_periodic_validity_days(&self) -> i64 {
        self.free_periodic_validity_days
    }

    fn free_periodic_grant_period_type(&self) -> Result<GrantPeriodType, CoreError> {
        self.free_periodic_grant_period_type.parse()
    }
}

impl CreateRealmConfigInput {
    pub fn validate(&self) -> Result<(), CoreError> {
        self.validate_fields()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_realm_default_config_validate_daily() {
        let config = RealmDefaultConfig {
            realm_id: "test".to_string(),
            registration_bonus_points: 1000,
            free_periodic_points_amount: 50,
            free_periodic_grant_period_type: GrantPeriodType::Daily,
            free_periodic_validity_days: 1,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert!(config.validate().is_ok());

        let invalid_config = RealmDefaultConfig {
            free_periodic_validity_days: 0,
            ..config
        };
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_realm_default_config_validate_once() {
        let config = RealmDefaultConfig {
            realm_id: "test".to_string(),
            registration_bonus_points: 1000,
            free_periodic_points_amount: 50,
            free_periodic_grant_period_type: GrantPeriodType::Once,
            free_periodic_validity_days: 0, // Once allows permanent (0)
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_update_realm_config_input_validate() {
        let input = UpdateRealmConfigInput {
            registration_bonus_points: 1500,
            free_periodic_points_amount: 100,
            free_periodic_grant_period_type: "weekly".to_string(),
            free_periodic_validity_days: 7,
        };
        assert!(input.validate().is_ok());

        let invalid_input = UpdateRealmConfigInput {
            free_periodic_validity_days: 0,
            ..input.clone()
        };
        assert!(invalid_input.validate().is_err());

        let invalid_period = UpdateRealmConfigInput {
            free_periodic_grant_period_type: "invalid".to_string(),
            ..input
        };
        assert!(invalid_period.validate().is_err());
    }

    #[test]
    fn test_create_realm_config_input_validate() {
        let input = CreateRealmConfigInput {
            realm_id: "test".to_string(),
            registration_bonus_points: 1000,
            free_periodic_points_amount: 50,
            free_periodic_grant_period_type: "monthly".to_string(),
            free_periodic_validity_days: 30,
        };
        assert!(input.validate().is_ok());

        let invalid_input = CreateRealmConfigInput {
            free_periodic_validity_days: 0,
            ..input.clone()
        };
        assert!(invalid_input.validate().is_err());

        let invalid_period = CreateRealmConfigInput {
            free_periodic_grant_period_type: "invalid".to_string(),
            ..input
        };
        assert!(invalid_period.validate().is_err());
    }
}
