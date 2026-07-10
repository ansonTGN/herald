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
    pub bucket_id: Uuid,
    pub amount: i64,
    pub currency: String,
    pub status: String,
    pub is_one_time_role: bool,
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
}

impl Related<super::account::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
