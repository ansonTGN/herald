// Purchase domain entities

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Points package purchase record
/// Represents a successful points package purchase by a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointsPackagePurchase {
    pub id: Uuid,
    pub realm_id: String,
    pub user_id: Uuid,
    pub points_package_id: Uuid,
    pub payment_attempt_id: Uuid,
    pub points: i64,
    pub amount: i64,
    pub currency: String,
    pub payment_provider: String,
    pub points_transaction_id: Option<Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
