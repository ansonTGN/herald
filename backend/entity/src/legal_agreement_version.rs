use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "legal_agreement_version")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub realm_id: Option<String>,
    pub agreement_type: String,
    pub version_no: i32,
    pub version_label: Option<String>,
    pub content: Json,
    pub source: String,
    pub mode: String,
    pub external_url: Option<String>,
    pub published_at: DateTimeWithTimeZone,
    pub published_by: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
