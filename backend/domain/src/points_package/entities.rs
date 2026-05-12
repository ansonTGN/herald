// Points Package domain entities

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Points package - a purchasable product that grants topup_credit
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PointsPackage {
    pub id: Uuid,
    pub realm_id: String,
    pub name: String,
    pub title: String,
    pub description: Option<String>,
    pub points: i64,
    pub price: i64,
    pub currency: String,
    pub sort_order: i32,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Payment provider mapping for a points package
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PointsPackagePaymentProvider {
    pub id: Uuid,
    pub points_package_id: Uuid,
    pub payment_provider: String, // "wechat", "stripe", "creem"
    pub enabled: bool,
    pub external_product_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
