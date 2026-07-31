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
    pub entitlement_key: String,
    #[sea_orm(nullable)]
    pub billing_type: Option<String>,
    #[sea_orm(nullable)]
    pub billing_period: Option<String>,
    /// Fixed service-period length (days) for non-renewing mappings
    /// must be `>= 1` (enforced by `chk_pem_service_duration_days`).
    #[sea_orm(nullable)]
    pub service_duration_days: Option<i32>,
    #[sea_orm(default_value = false)]
    pub enabled: bool,
    #[sea_orm(nullable)]
    pub provider_product_info: Option<Json>,
    /// Empty array = no role grant. Follows the `account.provider_ids` UUID[]
    /// precedent (account.rs:13-14).
    #[sea_orm(default_value = "ARRAY[]::UUID[]")]
    pub granted_role_ids: Vec<Uuid>,
    #[sea_orm(nullable)]
    pub synced_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
