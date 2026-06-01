// Input DTOs for Points API

use uuid::Uuid;
use validator::Validate;

use crate::common::entities::app_errors::CoreError;
use crate::points::grant_schedule::GrantPeriodType;

/// Input for consuming points
#[derive(Debug, Clone, Validate)]
pub struct ConsumePointsInput {
    #[validate(length(min = 1))]
    pub user_id: String,

    #[validate(length(min = 1))]
    pub client_app_id: String,

    #[validate(range(min = 1))]
    pub amount: i64,

    #[validate(length(max = 500))]
    pub description: Option<String>,
}

impl TryFrom<ConsumePointsInput> for (Uuid, Uuid, i64, Option<String>) {
    type Error = CoreError;

    fn try_from(input: ConsumePointsInput) -> Result<Self, Self::Error> {
        input
            .validate()
            .map_err(|e| CoreError::BadRequest(format!("Invalid consume points input: {}", e)))?;

        let user_id = input
            .user_id
            .parse::<Uuid>()
            .map_err(|_| CoreError::bad_request("user_id", "Invalid UUID format"))?;

        let client_app_id = input
            .client_app_id
            .parse::<Uuid>()
            .map_err(|_| CoreError::bad_request("client_app_id", "Invalid UUID format"))?;

        Ok((user_id, client_app_id, input.amount, input.description))
    }
}

/// Input for creating a plan config
#[derive(Debug, Clone, Validate)]
pub struct CreatePlanConfigInput {
    #[validate(length(min = 1))]
    pub plan_id: String,

    pub grant_period_type: String,

    #[validate(range(min = 1))]
    pub points_per_period: i64,

    #[validate(range(min = 0))]
    pub validity_days: i64,

    pub grant_on_subscribe: bool,

    pub max_periods: Option<i64>,
}

impl TryFrom<CreatePlanConfigInput> for (Uuid, String, i64, i64, bool, Option<i64>) {
    type Error = CoreError;

    fn try_from(input: CreatePlanConfigInput) -> Result<Self, Self::Error> {
        input.validate().map_err(|e| {
            CoreError::BadRequest(format!("Invalid create plan config input: {}", e))
        })?;

        let plan_id = input
            .plan_id
            .parse::<Uuid>()
            .map_err(|_| CoreError::bad_request("plan_id", "Invalid UUID format"))?;

        // Validate grant_period_type using the enum
        let _grant_period_type =
            input
                .grant_period_type
                .parse::<GrantPeriodType>()
                .map_err(|_| {
                    CoreError::bad_request(
                        "grant_period_type",
                        "Must be one of: once, daily, weekly, monthly",
                    )
                })?;

        if let Some(max) = input.max_periods
            && max < 0
        {
            return Err(CoreError::bad_request(
                "max_periods",
                "Must be non-negative",
            ));
        }

        Ok((
            plan_id,
            input.grant_period_type,
            input.points_per_period,
            input.validity_days,
            input.grant_on_subscribe,
            input.max_periods,
        ))
    }
}

/// Input for updating a plan config
#[derive(Debug, Clone, Validate)]
pub struct UpdatePlanConfigInput {
    pub grant_period_type: Option<String>,
    pub points_per_period: Option<i64>,
    pub validity_days: Option<i64>,
    pub grant_on_subscribe: Option<bool>,
    pub max_periods: Option<Option<i64>>,
}

impl TryFrom<UpdatePlanConfigInput>
    for (
        Option<String>,
        Option<i64>,
        Option<i64>,
        Option<bool>,
        Option<i64>,
    )
{
    type Error = CoreError;

    fn try_from(input: UpdatePlanConfigInput) -> Result<Self, Self::Error> {
        input.validate().map_err(|e| {
            CoreError::BadRequest(format!("Invalid update plan config input: {}", e))
        })?;

        if let Some(points) = input.points_per_period
            && points <= 0
        {
            return Err(CoreError::bad_request(
                "points_per_period",
                "Must be greater than 0",
            ));
        }

        if let Some(days) = input.validity_days
            && days < 0
        {
            return Err(CoreError::bad_request(
                "validity_days",
                "Must be non-negative",
            ));
        }

        // Validate grant_period_type if provided, using the enum
        if let Some(ref period_type) = input.grant_period_type {
            period_type.parse::<GrantPeriodType>().map_err(|_| {
                CoreError::bad_request(
                    "grant_period_type",
                    "Must be one of: once, daily, weekly, monthly",
                )
            })?;
        }

        let max_periods = match input.max_periods {
            Some(Some(max)) => {
                if max < 0 {
                    return Err(CoreError::bad_request(
                        "max_periods",
                        "Must be non-negative",
                    ));
                }
                Some(max)
            }
            Some(None) | None => None,
        };

        Ok((
            input.grant_period_type,
            input.points_per_period,
            input.validity_days,
            input.grant_on_subscribe,
            max_periods,
        ))
    }
}

/// Input for granting points (admin or SDK)
#[derive(Debug, Clone)]
pub struct GrantPointsInput {
    pub user_id: Uuid,
    pub amount: i64,
    pub reason: String,
    pub validity_days: Option<i64>,
    pub source_type: crate::points::entities::CreditSourceType,
    pub source_id: String,
}

impl GrantPointsInput {
    /// Validate grant points input
    pub fn validate(&self) -> Result<(), CoreError> {
        if self.amount <= 0 {
            return Err(CoreError::bad_request(
                "amount",
                "Amount must be greater than 0",
            ));
        }
        if self.reason.trim().is_empty() {
            return Err(CoreError::bad_request("reason", "Reason must not be empty"));
        }
        if let Some(days) = self.validity_days
            && days <= 0
        {
            return Err(CoreError::bad_request(
                "validityDays",
                "Validity days must be greater than 0",
            ));
        }
        Ok(())
    }
}

/// Output for granting points
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GrantPointsOutput {
    pub transaction_id: Uuid,
    pub user_id: Uuid,
    pub amount: i64,
    pub granted_balance: i64,
    pub total_balance: i64,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Input for revoking points (internal API)
#[derive(Debug, Clone, Validate)]
pub struct RevokePointsInput {
    #[validate(length(min = 1))]
    pub realm_id: String,

    #[validate(length(min = 1))]
    pub user_id: String,

    pub credit_type: crate::points::entities::CreditType,

    #[validate(range(min = 1))]
    pub revoked_amount: i64,

    pub revocation_type: crate::points::entities::RevocationType,

    #[validate(length(min = 1, max = 100))]
    pub reason: String,

    #[validate(length(min = 1, max = 200))]
    pub idempotency_key: String,

    pub reference_id: Option<String>,
}

/// Output for revoking points
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RevokePointsOutput {
    pub revocation_id: Uuid,
    pub ledger_ids: Vec<Uuid>,
    pub total_revoked: i64,
    pub revoked_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_consume_points_amount() {
        let input = ConsumePointsInput {
            user_id: uuid::Uuid::now_v7().to_string(),
            client_app_id: uuid::Uuid::now_v7().to_string(),
            amount: 0,
            description: None,
        };

        assert!(input.validate().is_err());
    }

    #[test]
    fn test_valid_create_plan_config_input() {
        let input = CreatePlanConfigInput {
            plan_id: uuid::Uuid::now_v7().to_string(),
            grant_period_type: "monthly".to_string(),
            points_per_period: 1000,
            validity_days: 30,
            grant_on_subscribe: true,
            max_periods: Some(12),
        };

        assert!(input.validate().is_ok());
        assert!(
            <(Uuid, String, i64, i64, bool, Option<i64>) as std::convert::TryFrom<
                CreatePlanConfigInput,
            >>::try_from(input.clone())
            .is_ok()
        );
    }

    #[test]
    fn test_invalid_grant_period_type() {
        let input = CreatePlanConfigInput {
            plan_id: uuid::Uuid::now_v7().to_string(),
            grant_period_type: "invalid".to_string(),
            points_per_period: 1000,
            validity_days: 30,
            grant_on_subscribe: true,
            max_periods: None,
        };

        assert!(input.validate().is_ok());
        assert!(
            <(Uuid, String, i64, i64, bool, Option<i64>) as std::convert::TryFrom<
                CreatePlanConfigInput,
            >>::try_from(input.clone())
            .is_err()
        );
    }

    #[test]
    fn test_invalid_update_plan_config_negative_points() {
        let input = UpdatePlanConfigInput {
            grant_period_type: None,
            points_per_period: Some(-100),
            validity_days: None,
            grant_on_subscribe: None,
            max_periods: None,
        };

        assert!(input.validate().is_ok());
        assert!(
            <(
                Option<String>,
                Option<i64>,
                Option<i64>,
                Option<bool>,
                Option<i64>
            ) as std::convert::TryFrom<UpdatePlanConfigInput>>::try_from(input.clone())
            .is_err()
        );
    }

    #[test]
    fn test_invalid_update_plan_config_zero_points() {
        let input = UpdatePlanConfigInput {
            grant_period_type: None,
            points_per_period: Some(0),
            validity_days: None,
            grant_on_subscribe: None,
            max_periods: None,
        };

        assert!(input.validate().is_ok());
        assert!(
            <(
                Option<String>,
                Option<i64>,
                Option<i64>,
                Option<bool>,
                Option<i64>
            ) as std::convert::TryFrom<UpdatePlanConfigInput>>::try_from(input.clone())
            .is_err()
        );
    }

    fn valid_grant_input() -> GrantPointsInput {
        GrantPointsInput {
            user_id: Uuid::now_v7(),
            amount: 100,
            reason: "Admin grant".to_string(),
            validity_days: Some(30),
            source_type: crate::points::entities::CreditSourceType::AdminGrant,
            source_id: "admin-user-id".to_string(),
        }
    }

    #[test]
    fn test_grant_input_valid() {
        assert!(valid_grant_input().validate().is_ok());
    }

    #[test]
    fn test_grant_input_valid_without_validity() {
        let mut input = valid_grant_input();
        input.validity_days = None;
        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_grant_input_rejects_zero_amount() {
        let mut input = valid_grant_input();
        input.amount = 0;
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_grant_input_rejects_negative_amount() {
        let mut input = valid_grant_input();
        input.amount = -50;
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_grant_input_rejects_empty_reason() {
        let mut input = valid_grant_input();
        input.reason = "   ".to_string();
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_grant_input_rejects_zero_validity_days() {
        let mut input = valid_grant_input();
        input.validity_days = Some(0);
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_grant_input_rejects_negative_validity_days() {
        let mut input = valid_grant_input();
        input.validity_days = Some(-10);
        assert!(input.validate().is_err());
    }
}
