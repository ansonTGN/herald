use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "user_totp_backup_codes")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    pub user_totp_config_id: Uuid,
    pub code_hash: String,
    pub used: bool,
    pub used_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user_totp_config::Entity",
        from = "Column::UserTotpConfigId",
        to = "super::user_totp_config::Column::Id"
    )]
    UserTotpConfig,
}

impl Related<super::user_totp_config::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserTotpConfig.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
