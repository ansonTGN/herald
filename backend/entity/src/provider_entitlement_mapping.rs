//! `SeaORM` Entity for provider_entitlement_mappings table

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "provider_entitlement_mappings")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub realm_id: String,
    pub payment_provider: String,
    pub external_product_id: String,
    #[sea_orm(nullable)]
    pub external_price_id: Option<String>,
    #[sea_orm(nullable)]
    pub bucket_id: Option<Uuid>,
    pub entitlement_key: String,
    #[sea_orm(nullable)]
    pub billing_type: Option<String>,
    #[sea_orm(nullable)]
    pub billing_period: Option<String>,
    #[sea_orm(nullable)]
    pub points_per_period: Option<i32>,
    #[sea_orm(nullable)]
    pub grant_period_type: Option<String>,
    #[sea_orm(nullable)]
    pub validity_days: Option<i32>,
    #[sea_orm(default_value = false)]
    pub grant_on_subscribe: bool,
    #[sea_orm(nullable)]
    pub max_periods: Option<i32>,
    #[sea_orm(default_value = false)]
    pub enabled: bool,
    #[sea_orm(nullable)]
    pub provider_product_info: Option<Json>,
    #[sea_orm(nullable)]
    pub synced_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
