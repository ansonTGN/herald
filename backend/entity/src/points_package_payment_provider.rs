use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "points_package_payment_providers")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub points_package_id: Uuid,
    pub payment_provider: String,
    #[sea_orm(default_value = "true")]
    pub enabled: bool,
    #[sea_orm(nullable)]
    pub external_product_id: Option<String>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::points_package::Entity",
        from = "Column::PointsPackageId",
        to = "super::points_package::Column::Id"
    )]
    PointsPackage,
}

impl Related<super::points_package::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PointsPackage.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
