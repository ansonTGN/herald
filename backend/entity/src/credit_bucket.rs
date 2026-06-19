//! `SeaORM` Entity for credit_buckets table

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "credit_buckets")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub realm_id: String,
    pub bucket_key: String,
    pub name: String,
    #[sea_orm(nullable)]
    pub description: Option<String>,
    pub display_order: i32,
    pub receives_registration_credits: bool,
    pub enabled: bool,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::credit_bucket_client_app::Entity")]
    CreditBucketClientApps,
    #[sea_orm(has_many = "super::points_wallet::Entity")]
    PointsWallets,
}

impl Related<super::credit_bucket_client_app::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CreditBucketClientApps.def()
    }
}

impl Related<super::points_wallet::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PointsWallets.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
