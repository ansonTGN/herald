use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "shopify_subscription_binding")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub subscription_id: Uuid,
    pub realm_id: String,
    pub shop_domain: String,
    pub contract_id: String,
    pub contract_gid: String,
    pub contract_revision_id: i64,
    pub customer_id: Option<String>,
    pub customer_payment_method_id: Option<String>,
    pub last_billing_attempt_id: Option<String>,
    pub last_order_id: Option<String>,
    pub cancel_reason: Option<String>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::subscription::Entity",
        from = "Column::SubscriptionId",
        to = "super::subscription::Column::Id"
    )]
    Subscription,
}

impl Related<super::subscription::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Subscription.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
