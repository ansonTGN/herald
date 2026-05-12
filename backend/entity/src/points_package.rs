use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "points_packages")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub realm_id: String,
    pub name: String,
    pub title: String,
    #[sea_orm(nullable)]
    pub description: Option<String>,
    pub points: i64,
    pub price: i64,
    pub currency: String,
    #[sea_orm(default_value = "0")]
    pub sort_order: i32,
    #[sea_orm(default_value = "true")]
    pub enabled: bool,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::points_package_payment_provider::Entity")]
    PaymentProviders,
    #[sea_orm(has_many = "super::points_package_purchase::Entity")]
    Purchases,
}

impl Related<super::points_package_payment_provider::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PaymentProviders.def()
    }
}

impl Related<super::points_package_purchase::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Purchases.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
