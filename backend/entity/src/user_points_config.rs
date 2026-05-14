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
