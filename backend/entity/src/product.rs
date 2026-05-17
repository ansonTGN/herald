use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "products")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub realm_id: String,
    pub code: String,
    #[sea_orm(default_value = "")]
    pub title: String,
    #[sea_orm(nullable)]
    pub description: Option<String>,
    #[sea_orm(default_value = "true")]
    pub enabled: bool,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::subscription_plan::Entity")]
    SubscriptionPlan,
    #[sea_orm(
        belongs_to = "super::realm::Entity",
        from = "Column::RealmId",
        to = "super::realm::Column::Id"
    )]
    Realm,
}

impl Related<super::subscription_plan::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SubscriptionPlan.def()
    }
}

impl Related<super::realm::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Realm.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
