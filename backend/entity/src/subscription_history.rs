use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "subscription_history")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: String,
    pub subscription_id: Uuid,
    pub event_type: String,
    pub timestamp: DateTimeWithTimeZone,
    #[sea_orm(nullable)]
    pub actor: Option<String>,
    #[sea_orm(nullable)]
    pub changes: Option<Json>,
    #[sea_orm(nullable)]
    pub previous_state: Option<Json>,
    #[sea_orm(nullable)]
    pub new_state: Option<Json>,
    pub realm_id: String,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::subscription::Entity",
        from = "Column::SubscriptionId",
        to = "super::subscription::Column::Id"
    )]
    Subscription,
}

impl Related<super::subscription::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Subscription.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
