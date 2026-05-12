use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "client_app")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub realm_id: String,
    pub client_id: String,
    pub name: String,
    pub description: Option<String>,

    // New fields for Client App settings
    pub redirect_uris: JsonValue,
    pub enabled: bool,
    pub icon_url: Option<String>,
    pub session_ttl_seconds: i32,
    pub session_renewal_ttl_seconds: Option<i32>,
    pub client_secret: Option<String>,

    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

// 辅助方法：UUID 转换
impl Model {
    pub fn id_as_string(&self) -> String {
        self.id.to_string()
    }
}
