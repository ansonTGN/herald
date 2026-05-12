use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "points_package_purchases")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub realm_id: String,
    pub user_id: Uuid,
    pub points_package_id: Uuid,
    pub payment_attempt_id: Uuid,
    pub points: i64,
    pub amount: i64,
    pub currency: String,
    pub payment_provider: String,
    #[sea_orm(nullable)]
    pub points_transaction_id: Option<Uuid>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::account::Entity",
        from = "Column::UserId",
        to = "super::account::Column::Id"
    )]
    User,
    #[sea_orm(
        belongs_to = "super::points_package::Entity",
        from = "Column::PointsPackageId",
        to = "super::points_package::Column::Id"
    )]
    PointsPackage,
    #[sea_orm(
        belongs_to = "super::payment_attempt::Entity",
        from = "Column::PaymentAttemptId",
        to = "super::payment_attempt::Column::Id"
    )]
    PaymentAttempt,
    #[sea_orm(
        belongs_to = "super::points_transaction::Entity",
        from = "Column::PointsTransactionId",
        to = "super::points_transaction::Column::Id"
    )]
    PointsTransaction,
}

impl Related<super::account::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::points_package::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PointsPackage.def()
    }
}

impl Related<super::payment_attempt::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PaymentAttempt.def()
    }
}

impl Related<super::points_transaction::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PointsTransaction.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
