use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "payment_event")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub realm_id: String,
    /// External event ID from payment provider (unique per provider)
    pub external_event_id: String,
    /// Payment provider type (creem, stripe, etc.)
    #[sea_orm(default_value = "creem")]
    pub payment_provider: String,
    pub event_type: String,
    #[sea_orm(nullable)]
    pub subscription_id: Option<Uuid>,
    pub payload: Json,
    #[sea_orm(default_value = "false")]
    pub processed: bool,
    #[sea_orm(nullable)]
    pub processing_started_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
