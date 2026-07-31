//! `SeaORM` Entity for points_distribution_events table

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "points_distribution_events")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub realm_id: String,
    pub user_id: Uuid,
    /// One of the six automatic distribution triggers
    pub trigger: String,
    /// Stable business event key; unique per (realm, user, trigger)
    pub event_key: String,
    /// Payment / subscription / registration source locator
    pub source_id: String,
    /// `entitlement_mapping` or `realm_registration`
    pub owner_type: String,
    #[sea_orm(nullable)]
    pub entitlement_mapping_id: Option<Uuid>,
    /// `processing` (in-flight, never committed) or `completed`
    pub status: String,
    /// Logical result count at completion; NULL until completed, 0 for zero-rule events
    #[sea_orm(nullable)]
    pub result_count: Option<i32>,
    #[sea_orm(nullable)]
    pub completed_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::account::Entity",
        from = "Column::UserId",
        to = "super::account::Column::Id"
    )]
    Account,
    #[sea_orm(
        belongs_to = "super::provider_entitlement_mapping::Entity",
        from = "Column::EntitlementMappingId",
        to = "super::provider_entitlement_mapping::Column::Id"
    )]
    ProviderEntitlementMapping,
}

impl Related<super::account::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Account.def()
    }
}

impl Related<super::provider_entitlement_mapping::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProviderEntitlementMapping.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
