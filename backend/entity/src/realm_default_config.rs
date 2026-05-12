//! `SeaORM` Entity for realm_default_configs table

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "realm_default_configs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub realm_id: String,
    pub registration_bonus_points: i64,
    pub free_periodic_points_amount: i64,
    pub free_periodic_grant_period_type: String,
    pub free_periodic_validity_days: i64,
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
            realm_id: "test-realm".to_string(),
            registration_bonus_points: 1000,
            free_periodic_points_amount: 50,
            free_periodic_grant_period_type: "daily".to_string(),
            free_periodic_validity_days: 1,
            created_at: now,
            updated_at: now,
        };
        assert_eq!(model.registration_bonus_points, 1000);
        assert_eq!(model.free_periodic_points_amount, 50);
        assert_eq!(model.free_periodic_validity_days, 1);
        assert_eq!(model.free_periodic_grant_period_type, "daily");
    }
}
