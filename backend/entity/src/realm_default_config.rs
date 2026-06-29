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
    /// Nullable JSONB (BE-D01 migration). Read as a raw JSON value; the
    /// infra repository maps it to `Vec<QuotaWindow>` at the domain boundary.
    #[sea_orm(column_type = "Json", nullable)]
    pub free_periodic_quota_windows: Option<Json>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
