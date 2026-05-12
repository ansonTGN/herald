// Purchase repository ports

use uuid::Uuid;

use super::entities::PointsPackagePurchase;
use crate::common::entities::app_errors::CoreError;

/// Input for creating a points package purchase record
#[derive(Debug, Clone)]
pub struct CreatePointsPackagePurchaseInput {
    pub realm_id: String,
    pub user_id: Uuid,
    pub points_package_id: Uuid,
    pub payment_attempt_id: Uuid,
    pub points: i64,
    pub amount: i64,
    pub currency: String,
    pub payment_provider: String,
}

/// Repository trait for Purchase operations
#[allow(async_fn_in_trait)]
pub trait PurchaseRepository: Send + Sync {
    /// Create a points package purchase record
    async fn create_points_package_purchase(
        &self,
        input: CreatePointsPackagePurchaseInput,
    ) -> Result<PointsPackagePurchase, CoreError>;

    /// Find a points package purchase by ID
    async fn find_points_package_purchase_by_id(
        &self,
        realm_id: &str,
        purchase_id: Uuid,
    ) -> Result<Option<PointsPackagePurchase>, CoreError>;

    /// Find a points package purchase by payment attempt ID
    async fn find_points_package_purchase_by_attempt_id(
        &self,
        payment_attempt_id: Uuid,
    ) -> Result<Option<PointsPackagePurchase>, CoreError>;

    /// List points package purchases for a user
    async fn list_points_package_purchases(
        &self,
        realm_id: &str,
        user_id: Uuid,
        payment_provider: Option<String>,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<PointsPackagePurchase>, CoreError>;

    /// Count points package purchases for a user
    async fn count_points_package_purchases(
        &self,
        realm_id: &str,
        user_id: Uuid,
        payment_provider: Option<String>,
    ) -> Result<i64, CoreError>;

    /// Update purchase with transaction ID
    async fn update_purchase_transaction_id(
        &self,
        purchase_id: Uuid,
        transaction_id: Uuid,
    ) -> Result<(), CoreError>;
}
