//! `SeaORM` Entity for points_revocation_records table
//!
//! This table tracks all points revocation operations (refunds, expirations, cancellations).

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "points_revocation_records")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub ledger_id: Uuid,
    pub user_id: Uuid,
    pub realm_id: String,

    // 回收详情
    pub revocation_type: String, // "refund_revoke" | "expire_revoke" | "cancel_revoke"
    pub revoked_amount: i64,
    pub reason: String,
    pub reference_id: Option<String>,

    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::points_credit_ledger::Entity",
        from = "Column::LedgerId",
        to = "super::points_credit_ledger::Column::Id"
    )]
    PointsCreditLedger,
}

impl Related<super::points_credit_ledger::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PointsCreditLedger.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
