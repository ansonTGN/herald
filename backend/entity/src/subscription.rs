use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "subscription")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub realm_id: String,
    #[sea_orm(nullable)]
    pub user_id: Option<Uuid>,
    pub external_subscription_id: String,
    pub external_product_id: String,
    #[sea_orm(default_value = "creem")]
    pub payment_provider: String,
    pub status: String,
    #[sea_orm(default_value = "free")]
    pub tier: String,
    #[sea_orm(nullable)]
    pub current_period_start: Option<DateTimeWithTimeZone>,
    #[sea_orm(nullable)]
    pub current_period_end: Option<DateTimeWithTimeZone>,
    #[sea_orm(default_value = "false")]
    pub cancel_at_period_end: bool,
    #[sea_orm(nullable)]
    pub client_app_id: Option<Uuid>,
    #[sea_orm(nullable)]
    pub plan_id: Option<Uuid>,
    #[sea_orm(default_value = "monthly")]
    pub billing_period: String,
    #[sea_orm(nullable)]
    pub cancel_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::subscription_plan::Entity",
        from = "Column::PlanId",
        to = "super::subscription_plan::Column::Id"
    )]
    SubscriptionPlan,
    #[sea_orm(
        belongs_to = "super::client_app::Entity",
        from = "Column::ClientAppId",
        to = "super::client_app::Column::Id"
    )]
    ClientApp,
    #[sea_orm(
        belongs_to = "super::account::Entity",
        from = "Column::UserId",
        to = "super::account::Column::Id"
    )]
    Account,
}

impl Related<super::subscription_plan::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SubscriptionPlan.def()
    }
}

impl Related<super::client_app::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ClientApp.def()
    }
}

impl Related<super::account::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Account.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
