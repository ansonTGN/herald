use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::common::entities::app_errors::CoreError;
use crate::points::entities::QuotaWindow;
use crate::points::grant_schedule::GrantPeriodType;

/// Upper bound on the number of free-periodic quota windows a realm default
/// config may carry (design §4.2.2 / §4.4.3: window count ≤ 8). Keeps the
/// dashboard editor bounded and the grant-time snapshot cheap.
pub const FREE_PERIODIC_QUOTA_WINDOWS_MAX: usize = 8;

/// Validate a free-periodic quota window list (design §4.2.2 / §4.4.3):
/// `window_seconds > 0`, `limit >= 0`, window count ≤
/// [`FREE_PERIODIC_QUOTA_WINDOWS_MAX`]. Returns `Ok(())` for `None`/empty
/// (no window-model grant). Shared by create + update so the rule cannot
/// drift between the two paths.
pub fn validate_free_periodic_quota_windows(
    windows: Option<&[QuotaWindow]>,
) -> Result<(), CoreError> {
    let Some(windows) = windows else {
        return Ok(());
    };
    if windows.is_empty() {
        return Ok(());
    }
    if windows.len() > FREE_PERIODIC_QUOTA_WINDOWS_MAX {
        return Err(CoreError::BadRequest(format!(
            "free_periodic_quota_windows may have at most {} windows, got {}",
            FREE_PERIODIC_QUOTA_WINDOWS_MAX,
            windows.len()
        )));
    }
    for w in windows {
        if w.window_seconds <= 0 {
            return Err(CoreError::BadRequest(
                "free_periodic_quota_windows.window_seconds must be > 0".to_string(),
            ));
        }
        if w.limit < 0 {
            return Err(CoreError::BadRequest(
                "free_periodic_quota_windows.limit must be >= 0".to_string(),
            ));
        }
    }
    Ok(())
}

/// Shared validation trait for realm configuration
trait RealmConfigValidator {
    fn registration_bonus_points(&self) -> i64;
    fn free_periodic_points_amount(&self) -> i64;
    fn free_periodic_validity_days(&self) -> i64;
    fn free_periodic_grant_period_type(&self) -> Result<GrantPeriodType, CoreError>;
    fn free_periodic_quota_windows(&self) -> Option<&[QuotaWindow]>;

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

        validate_free_periodic_quota_windows(self.free_periodic_quota_windows())?;

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
    /// Free-periodic quota window definition (design §4.3.2:
    /// `realm_default_configs.free_periodic_quota_windows` JSONB column,
    /// added by the BE-D01 migration). Non-empty ⟹ free-periodic grant
    /// routes to a window-quota entitlement (design §5.4); empty ⟹ the
    /// registration path skips the free-periodic grant (fail-safe, mirrors
    /// the pre-redesign zero-amount branch). Hydrated by the infra
    /// repository from the raw JSONB value.
    pub free_periodic_quota_windows: Vec<QuotaWindow>,
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

    fn free_periodic_quota_windows(&self) -> Option<&[QuotaWindow]> {
        Some(&self.free_periodic_quota_windows)
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
    /// Free-periodic quota window definition (design §4.2.2 / §4.3.2:
    /// `realm_default_configs.free_periodic_quota_windows` JSONB column).
    /// `None` ⟺ leave the stored value untouched; `Some([])` ⟺ clear;
    /// `Some([...])` ⟺ replace. Keys are derived by the caller (api layer)
    /// via `derive_window_key` so the stored snapshot carries stable display
    /// keys. Validated by `validate_free_periodic_quota_windows`.
    pub free_periodic_quota_windows: Option<Vec<QuotaWindow>>,
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

    fn free_periodic_quota_windows(&self) -> Option<&[QuotaWindow]> {
        self.free_periodic_quota_windows.as_deref()
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
    /// Free-periodic quota window definition (design §4.2.2 / §4.3.2). On
    /// create, `None` / empty ⟺ no window-model grant (fail-safe, matches the
    /// pre-redesign zero-amount branch). Keys derived by the caller via
    /// `derive_window_key`. Validated by `validate_free_periodic_quota_windows`.
    pub free_periodic_quota_windows: Option<Vec<QuotaWindow>>,
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

    fn free_periodic_quota_windows(&self) -> Option<&[QuotaWindow]> {
        self.free_periodic_quota_windows.as_deref()
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
            free_periodic_quota_windows: Vec::new(),
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
            free_periodic_quota_windows: Vec::new(),
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
            free_periodic_quota_windows: None,
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
    fn test_free_periodic_quota_windows_validation() {
        use crate::points::service::derive_window_key;

        // None / empty → ok (no window-model grant).
        assert!(validate_free_periodic_quota_windows(None).is_ok());
        assert!(validate_free_periodic_quota_windows(Some(&[])).is_ok());

        // Well-formed window list → ok.
        let ok = vec![
            QuotaWindow {
                window_seconds: 5 * 3_600,
                limit: 100,
                key: derive_window_key(5 * 3_600),
            },
            QuotaWindow {
                window_seconds: 7 * 86_400,
                limit: 1_000,
                key: derive_window_key(7 * 86_400),
            },
        ];
        assert!(validate_free_periodic_quota_windows(Some(&ok)).is_ok());

        // window_seconds <= 0 → BadRequest.
        let bad_seconds = vec![QuotaWindow {
            window_seconds: 0,
            limit: 100,
            key: "0s".to_string(),
        }];
        assert!(validate_free_periodic_quota_windows(Some(&bad_seconds)).is_err());

        // limit < 0 → BadRequest.
        let bad_limit = vec![QuotaWindow {
            window_seconds: 3_600,
            limit: -1,
            key: "1h".to_string(),
        }];
        assert!(validate_free_periodic_quota_windows(Some(&bad_limit)).is_err());

        // count > 8 → BadRequest.
        let too_many = (0..9)
            .map(|i| QuotaWindow {
                window_seconds: 3_600 + i,
                limit: 1,
                key: derive_window_key(3_600 + i),
            })
            .collect::<Vec<_>>();
        assert!(validate_free_periodic_quota_windows(Some(&too_many)).is_err());

        // limit == 0 is allowed (window grants nothing but is a valid config).
        let zero_limit = vec![QuotaWindow {
            window_seconds: 3_600,
            limit: 0,
            key: "1h".to_string(),
        }];
        assert!(validate_free_periodic_quota_windows(Some(&zero_limit)).is_ok());
    }
}
