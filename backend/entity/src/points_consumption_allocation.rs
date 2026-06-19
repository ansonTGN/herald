//! `SeaORM` Entity for points_consumption_allocations table
//!
//! This table tracks how each consumption transaction is allocated across multiple ledger entries.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "points_consumption_allocations")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub transaction_id: Uuid,
    pub ledger_id: Uuid,
    pub wallet_id: Uuid,
    pub user_id: Uuid,
    pub realm_id: String,
    pub bucket_id: Uuid,

    // 分摊详情
    pub allocated_amount: i64,
    pub ledger_remaining_after: i64,

    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::points_wallet::Entity",
        from = "Column::WalletId",
        to = "super::points_wallet::Column::Id"
    )]
    PointsWallet,
    #[sea_orm(
        belongs_to = "super::credit_bucket::Entity",
        from = "Column::BucketId",
        to = "super::credit_bucket::Column::Id"
    )]
    CreditBucket,
    #[sea_orm(
        belongs_to = "super::points_transaction::Entity",
        from = "Column::TransactionId",
        to = "super::points_transaction::Column::Id"
    )]
    PointsTransaction,

    #[sea_orm(
        belongs_to = "super::points_credit_ledger::Entity",
        from = "Column::LedgerId",
        to = "super::points_credit_ledger::Column::Id"
    )]
    PointsCreditLedger,
}

impl Related<super::points_wallet::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PointsWallet.def()
    }
}

impl Related<super::credit_bucket::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CreditBucket.def()
    }
}

impl Related<super::points_transaction::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PointsTransaction.def()
    }
}

impl Related<super::points_credit_ledger::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PointsCreditLedger.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
