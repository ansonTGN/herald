use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "payment_attempts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub realm_id: String,
    pub user_id: Uuid,
    pub payment_provider: String,
    pub target_type: String,
    pub target_id: Uuid,
    pub amount: i64,
    pub currency: String,
    pub status: String,
    #[sea_orm(nullable)]
    pub provider_reference: Option<String>,
    #[sea_orm(nullable)]
    pub provider_status: Option<String>,
    #[sea_orm(nullable)]
    pub metadata: Option<Json>,
    pub expires_at: DateTimeWithTimeZone,
    #[sea_orm(nullable)]
    pub completed_at: Option<DateTimeWithTimeZone>,
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
    #[sea_orm(has_many = "super::points_package_purchase::Entity")]
    Purchases,
}

impl Related<super::account::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::points_package_purchase::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Purchases.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
