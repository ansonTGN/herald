use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "client_app_subscription_plan")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub client_app_id: Uuid,
    pub plan_id: Uuid,
    #[sea_orm(default_value = "true")]
    pub enabled: bool,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::client_app::Entity",
        from = "Column::ClientAppId",
        to = "super::client_app::Column::Id"
    )]
    ClientApp,
    #[sea_orm(
        belongs_to = "super::subscription_plan::Entity",
        from = "Column::PlanId",
        to = "super::subscription_plan::Column::Id"
    )]
    SubscriptionPlan,
}

impl Related<super::client_app::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ClientApp.def()
    }
}

impl Related<super::subscription_plan::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SubscriptionPlan.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
