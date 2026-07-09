use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "user_roles")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub realm_id: String,
    pub user_id: Option<Uuid>,
    pub role_id: Uuid,
    pub client_id: Option<String>,
    pub principal_type: String,
    pub principal_id: String,
    /// Grant origin: `manual` (hand-assigned) or `payment` (design §4.3.2).
    #[sea_orm(default_value = "manual")]
    pub source: String,
    /// Payment origin id: one_time = attempt_id, subscription = subscription_id
    /// (design §4.3.2). NULL for manual grants.
    #[sea_orm(nullable)]
    pub source_id: Option<String>,
    /// INFORMATIONAL/PROVENANCE ONLY: subscription billing-period-end snapshot
    /// taken at grant time. NOT an authz TTL — no background sweep removes rows
    /// after this timestamp. Role removal is event-driven (ImmediateCancel
    /// webhook → revoke_roles_by_payment_source). NULL for manual or permanent
    /// one-time grants (design §4.3.2).
    #[sea_orm(nullable)]
    pub expires_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::roles::Entity",
        from = "Column::RoleId",
        to = "super::roles::Column::Id"
    )]
    Role,
}

impl Related<super::roles::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Role.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
