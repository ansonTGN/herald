//! `SeaORM` Entity for points_grant_schedules table

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "points_grant_schedules")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub realm_id: String,
    pub subscription_id: Option<Uuid>,
    pub plan_config_id: Option<Uuid>,
    pub grant_period_type: String,
    pub base_time: DateTimeWithTimeZone,
    pub next_grant_time: DateTimeWithTimeZone,
    pub points_per_period: i64,
    pub validity_days: i64,
    pub granted_periods: i64,
    pub max_periods: Option<i64>,
    pub active: bool,
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
            id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            realm_id: "test-realm".to_string(),
            subscription_id: None,
            plan_config_id: None,
            grant_period_type: "daily".to_string(),
            base_time: now,
            next_grant_time: now,
            points_per_period: 50,
            validity_days: 1,
            granted_periods: 0,
            max_periods: None,
            active: true,
            created_at: now,
            updated_at: now,
        };
        assert_eq!(model.grant_period_type, "daily");
        assert_eq!(model.points_per_period, 50);
        assert!(model.active);
    }
}
