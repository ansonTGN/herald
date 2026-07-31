//! `SeaORM` Entity for points_distribution_rules table

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "points_distribution_rules")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub realm_id: String,
    /// `entitlement_mapping` or `realm_registration`
    pub owner_type: String,
    /// Required when `owner_type = entitlement_mapping`, NULL for
    /// `realm_registration`. Owner/mapping pairing is enforced by
    /// `chk_pdr_owner_mapping`.
    #[sea_orm(nullable)]
    pub entitlement_mapping_id: Option<Uuid>,
    pub bucket_id: Uuid,
    /// Non-empty subset of the six automatic triggers. DB CHECK enforces
    /// non-empty / no-null / subset of the allowed set; the domain layer
    /// further constrains the subset by owner and billing type.
    pub trigger_sources: Vec<String>,
    /// `fixed` or `quota`
    pub grant_mode: String,
    #[sea_orm(nullable)]
    pub points_amount: Option<i64>,
    #[sea_orm(nullable)]
    pub validity_days: Option<i64>,
    /// `once` / `daily` / `weekly` / `monthly` for free-periodic fixed rules
    #[sea_orm(nullable)]
    pub grant_period_type: Option<String>,
    /// Snapshot `[{windowSeconds, limit, key}]`; required for quota mode
    #[sea_orm(column_type = "Json", nullable)]
    pub quota_windows: Option<Json>,
    pub enabled: bool,
    pub display_order: i32,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::provider_entitlement_mapping::Entity",
        from = "Column::EntitlementMappingId",
        to = "super::provider_entitlement_mapping::Column::Id"
    )]
    ProviderEntitlementMapping,
    #[sea_orm(
        belongs_to = "super::credit_bucket::Entity",
        from = "Column::BucketId",
        to = "super::credit_bucket::Column::Id"
    )]
    CreditBucket,
}

impl Related<super::provider_entitlement_mapping::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProviderEntitlementMapping.def()
    }
}

impl Related<super::credit_bucket::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CreditBucket.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
