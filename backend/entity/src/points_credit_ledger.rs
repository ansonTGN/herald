//! `SeaORM` Entity for points_credit_ledger table
//!
//! This table is the source of truth for all points credits, tracking grants, usage, and revocation.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "points_credit_ledger")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub realm_id: String,
    pub bucket_id: Uuid,

    // 积分类型与来源
    pub credit_type: String, // "topup_credit" | "subscription_credit"
    pub source_type: String, // "subscription_initial" | "subscription_renewal" | "subscription_upgrade" | "topup" | "system_grant"
    pub source_id: String,

    // 积分数量追踪
    pub granted_amount: i64,
    pub used_amount: i64,
    pub revoked_amount: i64,
    pub remaining_amount: i64, // 计算字段: granted_amount - used_amount - revoked_amount

    // 时间与状态
    pub expires_at: Option<DateTimeWithTimeZone>,
    /// Expected effective time (design §4.3.2 / §5.1). NULL ⟺ immediately
    /// available; non-null ⟺ enters the available set only when
    /// `effective_at <= NOW()`. Anchored to the expected period start
    /// (subscription `period_start`, free-periodic `next_grant_time`).
    pub effective_at: Option<DateTimeWithTimeZone>,
    pub status: String, // "active" | "revoked" | "expired" | "fully_used"

    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::credit_bucket::Entity",
        from = "Column::BucketId",
        to = "super::credit_bucket::Column::Id"
    )]
    CreditBucket,

    #[sea_orm(has_many = "super::points_consumption_allocation::Entity")]
    ConsumptionAllocations,

    #[sea_orm(has_many = "super::points_revocation_record::Entity")]
    RevocationRecords,
}

impl Related<super::credit_bucket::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CreditBucket.def()
    }
}

impl Related<super::points_consumption_allocation::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ConsumptionAllocations.def()
    }
}

impl Related<super::points_revocation_record::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RevocationRecords.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
