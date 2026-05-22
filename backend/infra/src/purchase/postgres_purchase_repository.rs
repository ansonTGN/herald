// PostgreSQL implementation of Purchase repository

use std::sync::Arc;

use herald_domain::common::entities::app_errors::CoreError;
use herald_domain::purchase::{
    CreatePointsPackagePurchaseInput, PointsPackagePurchase, PurchaseRepository,
};
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// PostgreSQL implementation of PurchaseRepository
pub struct PostgresPurchaseRepository {
    pool: Arc<PgPool>,
}

impl PostgresPurchaseRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert database row to PointsPackagePurchase
    fn row_to_purchase(row: &sqlx::postgres::PgRow) -> Result<PointsPackagePurchase, sqlx::Error> {
        Ok(PointsPackagePurchase {
            id: row.try_get("id")?,
            realm_id: row.try_get("realm_id")?,
            user_id: row.try_get("user_id")?,
            points_package_id: row.try_get("points_package_id")?,
            payment_attempt_id: row.try_get("payment_attempt_id")?,
            points: row.try_get("points")?,
            amount: row.try_get("amount")?,
            currency: row.try_get("currency")?,
            payment_provider: row.try_get("payment_provider")?,
            points_transaction_id: row.try_get("points_transaction_id")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

#[allow(async_fn_in_trait)]
impl PurchaseRepository for PostgresPurchaseRepository {
    async fn create_points_package_purchase(
        &self,
        input: CreatePointsPackagePurchaseInput,
    ) -> Result<PointsPackagePurchase, CoreError> {
        let id = Uuid::now_v7();

        let row = sqlx::query(
            "INSERT INTO points_package_purchases
                (id, realm_id, user_id, points_package_id, payment_attempt_id,
                 points, amount, currency, payment_provider, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW(), NOW())
             RETURNING *",
        )
        .bind(id)
        .bind(&input.realm_id)
        .bind(input.user_id)
        .bind(input.points_package_id)
        .bind(input.payment_attempt_id)
        .bind(input.points)
        .bind(input.amount)
        .bind(&input.currency)
        .bind(&input.payment_provider)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| CoreError::DatabaseError(format!("Failed to create purchase: {e}")))?;

        Self::row_to_purchase(&row)
            .map_err(|e| CoreError::DatabaseError(format!("Failed to parse purchase: {e}")))
    }

    async fn find_points_package_purchase_by_id(
        &self,
        realm_id: &str,
        purchase_id: Uuid,
    ) -> Result<Option<PointsPackagePurchase>, CoreError> {
        let row_opt = sqlx::query(
            "SELECT * FROM points_package_purchases
             WHERE id = $1 AND realm_id = $2",
        )
        .bind(purchase_id)
        .bind(realm_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| CoreError::DatabaseError(format!("Failed to find purchase: {e}")))?;

        match row_opt {
            Some(row) => {
                let purchase = Self::row_to_purchase(&row).map_err(|e| {
                    CoreError::DatabaseError(format!("Failed to parse purchase: {e}"))
                })?;
                Ok(Some(purchase))
            }
            None => Ok(None),
        }
    }

    async fn find_points_package_purchase_by_attempt_id(
        &self,
        payment_attempt_id: Uuid,
    ) -> Result<Option<PointsPackagePurchase>, CoreError> {
        let row_opt = sqlx::query(
            "SELECT * FROM points_package_purchases
             WHERE payment_attempt_id = $1",
        )
        .bind(payment_attempt_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| CoreError::DatabaseError(format!("Failed to find purchase: {e}")))?;

        match row_opt {
            Some(row) => {
                let purchase = Self::row_to_purchase(&row).map_err(|e| {
                    CoreError::DatabaseError(format!("Failed to parse purchase: {e}"))
                })?;
                Ok(Some(purchase))
            }
            None => Ok(None),
        }
    }

    async fn list_points_package_purchases(
        &self,
        realm_id: &str,
        user_id: Uuid,
        payment_provider: Option<String>,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<PointsPackagePurchase>, CoreError> {
        let limit = limit.unwrap_or(20).min(100); // Max 100 records
        let offset = offset.unwrap_or(0);

        let query = if let Some(provider) = payment_provider {
            sqlx::query(
                "SELECT * FROM points_package_purchases
                 WHERE realm_id = $1 AND user_id = $2 AND payment_provider = $3
                 ORDER BY created_at DESC
                 LIMIT $4 OFFSET $5",
            )
            .bind(realm_id)
            .bind(user_id)
            .bind(provider)
            .bind(limit as i64)
            .bind(offset as i64)
        } else {
            sqlx::query(
                "SELECT * FROM points_package_purchases
                 WHERE realm_id = $1 AND user_id = $2
                 ORDER BY created_at DESC
                 LIMIT $3 OFFSET $4",
            )
            .bind(realm_id)
            .bind(user_id)
            .bind(limit as i64)
            .bind(offset as i64)
        };

        query
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| CoreError::DatabaseError(format!("Failed to list purchases: {e}")))?
            .into_iter()
            .map(|row| Self::row_to_purchase(&row))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| CoreError::DatabaseError(format!("Failed to parse purchase: {e}")))
    }

    async fn count_points_package_purchases(
        &self,
        realm_id: &str,
        user_id: Uuid,
        payment_provider: Option<String>,
    ) -> Result<i64, CoreError> {
        let count: i64 = if let Some(provider) = payment_provider {
            sqlx::query_scalar(
                "SELECT COUNT(*) FROM points_package_purchases
                 WHERE realm_id = $1 AND user_id = $2 AND payment_provider = $3",
            )
            .bind(realm_id)
            .bind(user_id)
            .bind(provider)
            .fetch_one(&*self.pool)
            .await
        } else {
            sqlx::query_scalar(
                "SELECT COUNT(*) FROM points_package_purchases
                 WHERE realm_id = $1 AND user_id = $2",
            )
            .bind(realm_id)
            .bind(user_id)
            .fetch_one(&*self.pool)
            .await
        }
        .map_err(|e| CoreError::DatabaseError(format!("Failed to count purchases: {e}")))?;

        Ok(count)
    }

    async fn update_purchase_transaction_id(
        &self,
        purchase_id: Uuid,
        transaction_id: Uuid,
    ) -> Result<(), CoreError> {
        sqlx::query(
            "UPDATE points_package_purchases
             SET points_transaction_id = $1, updated_at = NOW()
             WHERE id = $2",
        )
        .bind(transaction_id)
        .bind(purchase_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| CoreError::DatabaseError(format!("Failed to update purchase: {e}")))?;

        Ok(())
    }
}
