//! `SeaORM` Entity for points_grant_records table

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "points_grant_records")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub schedule_id: Uuid,
    pub user_id: Uuid,
    pub realm_id: String,
    pub period_number: i64,
    pub granted_amount: i64,
    pub grant_time: DateTimeWithTimeZone,
    pub created_at: DateTimeWithTimeZone,
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
            schedule_id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            realm_id: "test-realm".to_string(),
            period_number: 1,
            granted_amount: 50,
            grant_time: now,
            created_at: now,
        };
        assert_eq!(model.period_number, 1);
        assert_eq!(model.granted_amount, 50);
    }
}
