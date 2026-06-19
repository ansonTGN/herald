//! `SeaORM` Entity for credit_bucket_client_apps table

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "credit_bucket_client_apps")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub bucket_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub client_app_id: Uuid,
    pub realm_id: String,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::credit_bucket::Entity",
        from = "Column::BucketId",
        to = "super::credit_bucket::Column::Id"
    )]
    CreditBucket,
    #[sea_orm(
        belongs_to = "super::client_app::Entity",
        from = "Column::ClientAppId",
        to = "super::client_app::Column::Id"
    )]
    ClientApp,
}

impl Related<super::credit_bucket::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CreditBucket.def()
    }
}

impl Related<super::client_app::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ClientApp.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
