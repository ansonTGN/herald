//! `SeaORM` Entity for payment_attempt_point_rules table

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "payment_attempt_point_rules")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub payment_attempt_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub rule_id: Uuid,
    pub bucket_id: Uuid,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::payment_attempt::Entity",
        from = "Column::PaymentAttemptId",
        to = "super::payment_attempt::Column::Id"
    )]
    PaymentAttempt,
    #[sea_orm(
        belongs_to = "super::points_distribution_rule::Entity",
        from = "Column::RuleId",
        to = "super::points_distribution_rule::Column::Id"
    )]
    PointsDistributionRule,
    #[sea_orm(
        belongs_to = "super::credit_bucket::Entity",
        from = "Column::BucketId",
        to = "super::credit_bucket::Column::Id"
    )]
    CreditBucket,
}

impl Related<super::payment_attempt::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PaymentAttempt.def()
    }
}

impl Related<super::points_distribution_rule::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PointsDistributionRule.def()
    }
}

impl Related<super::credit_bucket::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CreditBucket.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
