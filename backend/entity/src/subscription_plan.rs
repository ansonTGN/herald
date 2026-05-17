use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "subscription_plan")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub realm_id: String,
    pub name: String,
    pub title: String,
    #[sea_orm(nullable)]
    pub description: Option<String>,
    #[sea_orm(column_name = "type")]
    pub r#type: String, // "monthly" | "yearly"
    pub price: i32,
    pub currency: String,
    #[sea_orm(nullable)]
    pub checkout_url: Option<String>,
    #[sea_orm(default_value = "true")]
    pub active: bool,
    #[sea_orm(default_value = "0")]
    pub trial_days: i32,
    #[sea_orm(default_value = "0")]
    pub sort_order: i32,
    pub product_id: Uuid,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::client_app_subscription_plan::Entity")]
    ClientAppSubscriptionPlan,
    #[sea_orm(
        belongs_to = "super::product::Entity",
        from = "Column::ProductId",
        to = "super::product::Column::Id"
    )]
    Product,
    #[sea_orm(
        belongs_to = "super::realm::Entity",
        from = "Column::RealmId",
        to = "super::realm::Column::Id"
    )]
    Realm,
}

impl Related<super::client_app_subscription_plan::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ClientAppSubscriptionPlan.def()
    }
}

impl Related<super::product::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Product.def()
    }
}

impl Related<super::realm::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Realm.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
