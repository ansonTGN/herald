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
    /// FK bridge to the single `points_credit_ledger` row this grant record
    /// deduplicates (design §4.3.2 / §5.2 A4). NOT NULL at the SQL layer
    /// (BE-D01); pre-launch, no backfill (A6). Reclaim resolves
    /// `(schedule_id, period_number) -> ledger_id -> ledger row` through this
    /// column without needing `schedule_id` on `points_credit_ledger`.
    pub ledger_id: Uuid,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
