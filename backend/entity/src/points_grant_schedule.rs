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
