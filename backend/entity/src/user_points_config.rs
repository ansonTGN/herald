//! `SeaORM` Entity for user_points_configs table

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "user_points_configs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub user_id: Uuid,
    pub realm_id: String,
    pub registration_bonus_points: i64,
    pub free_periodic_points_amount: i64,
    pub free_periodic_grant_period_type: Option<String>,
    pub free_periodic_validity_days: i64,
    pub next_grant_time: Option<DateTimeWithTimeZone>,
    pub granted_periods: i64,
    pub grant_schedule_id: Option<Uuid>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_creation() {
        let now = DateTimeWithTimeZone::from(chrono::Utc::now());
        let model = Model {
            user_id: Uuid::now_v7(),
            realm_id: "test-realm".to_string(),
            registration_bonus_points: 0,
            free_periodic_points_amount: 0,
            free_periodic_grant_period_type: None,
            free_periodic_validity_days: 1,
            next_grant_time: Some(now),
            granted_periods: 0,
            grant_schedule_id: None,
            created_at: now,
            updated_at: now,
        };
        assert_eq!(model.registration_bonus_points, 0);
        assert_eq!(model.free_periodic_points_amount, 0);
        assert!(model.free_periodic_grant_period_type.is_none());
    }

    #[test]
    fn test_model_creation_with_period_type() {
        let now = DateTimeWithTimeZone::from(chrono::Utc::now());
        let model = Model {
            user_id: Uuid::now_v7(),
            realm_id: "test-realm".to_string(),
            registration_bonus_points: 1000,
            free_periodic_points_amount: 50,
            free_periodic_grant_period_type: Some("weekly".to_string()),
            free_periodic_validity_days: 7,
            next_grant_time: Some(now),
            granted_periods: 0,
            grant_schedule_id: None,
            created_at: now,
            updated_at: now,
        };
        assert_eq!(
            model.free_periodic_grant_period_type,
            Some("weekly".to_string())
        );
    }
}
