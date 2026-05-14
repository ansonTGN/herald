use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "audit_events")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub realm_id: String,
    pub category: String,
    pub action: String,
    pub actor_id: String,
    #[sea_orm(nullable)]
    pub actor_type: Option<String>,
    #[sea_orm(nullable)]
    pub actor_name: Option<String>,
    pub target_type: String,
    pub target_id: String,
    #[sea_orm(nullable)]
    pub target_name: Option<String>,
    pub result: String,
    #[sea_orm(nullable)]
    pub details: Option<Json>,
    #[sea_orm(nullable)]
    pub ip_address: Option<String>,
    #[sea_orm(nullable)]
    pub user_agent: Option<String>,
    #[sea_orm(nullable)]
    pub trace_id: Option<String>,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
