// PostgreSQL implementation for Payment Attempt repository

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use sqlx::{PgPool, Row};
use std::sync::Arc;

use herald_domain::common::entities::app_errors::CoreError;
use herald_domain::payment_attempt::{
    CreatePaymentAttemptInput, PaymentAttempt, PaymentAttemptErrorExt, PaymentAttemptRepository,
    PaymentAttemptStatus, PurchaseHistoryRow, RecordRenewalAttemptInput,
};
use herald_entity::payment_attempt as payment_attempt_entity;

/// PostgreSQL implementation of PaymentAttempt repository
pub struct PostgresPaymentAttemptRepository {
    db: Arc<DatabaseConnection>,
    pool: PgPool,
}

impl PostgresPaymentAttemptRepository {
    pub fn new(db: Arc<DatabaseConnection>, pool: PgPool) -> Self {
        Self { db, pool }
    }

    fn model_to_payment_attempt(
        model: payment_attempt_entity::Model,
    ) -> Result<PaymentAttempt, CoreError> {
        Ok(PaymentAttempt {
            id: model.id,
            realm_id: model.realm_id,
            user_id: model.user_id,
            payment_provider: model.payment_provider,
            target_type: model.target_type.parse()?,
            target_id: model.target_id,
            bucket_id: model.bucket_id,
            amount: model.amount,
            currency: model.currency,
            status: model.status.parse()?,
            provider_reference: model.provider_reference,
            provider_status: model.provider_status,
            metadata: model.metadata,
            expires_at: chrono::DateTime::from(model.expires_at),
            completed_at: model.completed_at.map(chrono::DateTime::from),
            created_at: chrono::DateTime::from(model.created_at),
            updated_at: chrono::DateTime::from(model.updated_at),
        })
    }
}

impl PaymentAttemptRepository for PostgresPaymentAttemptRepository {
    async fn create_payment_attempt(
        &self,
        input: CreatePaymentAttemptInput,
    ) -> Result<PaymentAttempt, CoreError> {
        let now = chrono::Utc::now();
        let expires_at = now + chrono::Duration::hours(2);

        let attempt_model = payment_attempt_entity::ActiveModel {
            id: Set(uuid::Uuid::now_v7()),
            realm_id: Set(input.realm_id),
            user_id: Set(input.user_id),
            payment_provider: Set(input.payment_provider),
            target_type: Set(input.target_type),
            target_id: Set(input.target_id),
            bucket_id: Set(input.bucket_id),
            amount: Set(input.amount),
            currency: Set(input.currency),
            status: Set("Pending".to_string()),
            provider_reference: Set(input.provider_reference),
            provider_status: Set(None),
            metadata: Set(input.metadata),
            expires_at: Set(expires_at.into()),
            completed_at: Set(None),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
        };

        let result = attempt_model.insert(self.db.as_ref()).await.map_err(|e| {
            CoreError::DatabaseError(format!("Failed to create payment attempt: {e}"))
        })?;

        Self::model_to_payment_attempt(result)
    }

    async fn insert_succeeded_renewal_attempt(
        &self,
        input: RecordRenewalAttemptInput,
    ) -> Result<PaymentAttempt, CoreError> {
        // Renewal attempt: already-Succeeded charge, no expiry semantics.
        // expires_at = completed_at (NOT NULL column; already-succeeded has no real expiry).
        let now = chrono::Utc::now();

        let attempt_model = payment_attempt_entity::ActiveModel {
            id: Set(uuid::Uuid::now_v7()),
            realm_id: Set(input.realm_id),
            user_id: Set(input.user_id),
            payment_provider: Set(input.payment_provider),
            target_type: Set("entitlement_mapping".to_string()),
            target_id: Set(input.target_id),
            bucket_id: Set(input.bucket_id),
            amount: Set(input.amount),
            currency: Set(input.currency),
            status: Set("Succeeded".to_string()),
            provider_reference: Set(Some(input.provider_reference)),
            provider_status: Set(Some("succeeded".to_string())),
            metadata: Set(None),
            expires_at: Set(input.completed_at.into()),
            completed_at: Set(Some(input.completed_at.into())),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
        };

        let result = attempt_model.insert(self.db.as_ref()).await.map_err(|e| {
            CoreError::DatabaseError(format!("Failed to insert renewal attempt: {e}"))
        })?;

        Self::model_to_payment_attempt(result)
    }

    async fn find_payment_attempt_by_id(
        &self,
        realm_id: &str,
        attempt_id: uuid::Uuid,
    ) -> Result<Option<PaymentAttempt>, CoreError> {
        let result = payment_attempt_entity::Entity::find_by_id(attempt_id)
            .filter(payment_attempt_entity::Column::RealmId.eq(realm_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| {
                CoreError::DatabaseError(format!("Failed to find payment attempt: {e}"))
            })?;

        match result {
            Some(model) => Self::model_to_payment_attempt(model).map(Some),
            None => Ok(None),
        }
    }

    async fn find_payment_attempt_by_id_only(
        &self,
        attempt_id: uuid::Uuid,
    ) -> Result<Option<PaymentAttempt>, CoreError> {
        let result = payment_attempt_entity::Entity::find_by_id(attempt_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| {
                CoreError::DatabaseError(format!("Failed to find payment attempt: {e}"))
            })?;

        match result {
            Some(model) => Self::model_to_payment_attempt(model).map(Some),
            None => Ok(None),
        }
    }

    async fn find_payment_attempts_by_user(
        &self,
        realm_id: &str,
        user_id: uuid::Uuid,
        limit: u64,
    ) -> Result<Vec<PaymentAttempt>, CoreError> {
        let results = payment_attempt_entity::Entity::find()
            .filter(payment_attempt_entity::Column::RealmId.eq(realm_id))
            .filter(payment_attempt_entity::Column::UserId.eq(user_id))
            .order_by_desc(payment_attempt_entity::Column::CreatedAt)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| {
                CoreError::DatabaseError(format!("Failed to find payment attempts by user: {e}"))
            })?;

        results
            .into_iter()
            .map(Self::model_to_payment_attempt)
            .collect()
    }

    async fn find_payment_attempt_by_provider_reference(
        &self,
        provider: &str,
        reference: &str,
    ) -> Result<Option<PaymentAttempt>, CoreError> {
        let result = payment_attempt_entity::Entity::find()
            .filter(payment_attempt_entity::Column::PaymentProvider.eq(provider))
            .filter(payment_attempt_entity::Column::ProviderReference.eq(reference))
            .one(self.db.as_ref())
            .await
            .map_err(|e| {
                CoreError::DatabaseError(format!(
                    "Failed to find payment attempt by provider reference: {}",
                    e
                ))
            })?;

        match result {
            Some(model) => Self::model_to_payment_attempt(model).map(Some),
            None => Ok(None),
        }
    }

    async fn update_payment_attempt(
        &self,
        attempt: PaymentAttempt,
    ) -> Result<PaymentAttempt, CoreError> {
        let attempt_model = payment_attempt_entity::ActiveModel {
            id: Set(attempt.id),
            realm_id: Set(attempt.realm_id),
            user_id: Set(attempt.user_id),
            payment_provider: Set(attempt.payment_provider),
            target_type: Set(attempt.target_type.to_string()),
            target_id: Set(attempt.target_id),
            bucket_id: Set(attempt.bucket_id),
            amount: Set(attempt.amount),
            currency: Set(attempt.currency),
            status: Set(attempt.status.to_string()),
            provider_reference: Set(attempt.provider_reference),
            provider_status: Set(attempt.provider_status),
            metadata: Set(attempt.metadata),
            expires_at: Set(attempt.expires_at.into()),
            completed_at: Set(attempt.completed_at.map(|dt| dt.into())),
            created_at: Set(attempt.created_at.into()),
            updated_at: Set(chrono::Utc::now().into()),
        };

        let result = attempt_model.update(self.db.as_ref()).await.map_err(|e| {
            CoreError::DatabaseError(format!("Failed to update payment attempt: {e}"))
        })?;

        Self::model_to_payment_attempt(result)
    }

    async fn update_payment_attempt_with_status_guard(
        &self,
        attempt: PaymentAttempt,
        expected_status: PaymentAttemptStatus,
    ) -> Result<PaymentAttempt, CoreError> {
        let target_status = attempt.status.clone();
        let result = sqlx::query(
            "UPDATE payment_attempts
             SET status = $1,
                 provider_reference = $2,
                 provider_status = $3,
                 completed_at = $4,
                 updated_at = NOW()
             WHERE id = $5
               AND realm_id = $6
               AND status = $7",
        )
        .bind(target_status.to_string())
        .bind(attempt.provider_reference.as_deref())
        .bind(attempt.provider_status.as_deref())
        .bind(attempt.completed_at)
        .bind(attempt.id)
        .bind(&attempt.realm_id)
        .bind(expected_status.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            CoreError::DatabaseError(format!("Failed to guarded-update payment attempt: {e}"))
        })?;

        if result.rows_affected() > 0 {
            return self
                .find_payment_attempt_by_id(&attempt.realm_id, attempt.id)
                .await?
                .ok_or_else(|| CoreError::attempt_not_found(&attempt.id.to_string()));
        }

        let current = self
            .find_payment_attempt_by_id(&attempt.realm_id, attempt.id)
            .await?
            .ok_or_else(|| CoreError::attempt_not_found(&attempt.id.to_string()))?;

        if current.status == target_status {
            return Ok(current);
        }

        Err(CoreError::invalid_status_transition(
            &expected_status.to_string(),
            &current.status.to_string(),
        ))
    }

    async fn list_expired_attempts(
        &self,
        before: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<PaymentAttempt>, CoreError> {
        let results = payment_attempt_entity::Entity::find()
            .filter(payment_attempt_entity::Column::ExpiresAt.lt(before))
            .filter(payment_attempt_entity::Column::Status.eq("Pending"))
            .all(self.db.as_ref())
            .await
            .map_err(|e| {
                CoreError::DatabaseError(format!("Failed to list expired attempts: {e}"))
            })?;

        results
            .into_iter()
            .map(Self::model_to_payment_attempt)
            .collect()
    }

    async fn list_purchase_history(
        &self,
        realm_id: &str,
        user_id: uuid::Uuid,
        payment_provider: Option<&str>,
        start_date: Option<&str>,
        end_date: Option<&str>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<PurchaseHistoryRow>, i64), CoreError> {
        let offset = (page - 1) * page_size;
        let provider_filter = payment_provider.unwrap_or("");
        let start_filter = start_date.unwrap_or("");
        let end_filter = end_date.unwrap_or("");

        // Count query
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM payment_attempts pa \
             WHERE pa.realm_id = $1 AND pa.user_id = $2 \
             AND pa.status = 'Succeeded' AND pa.target_type = 'entitlement_mapping' \
             AND ($3 = '' OR pa.payment_provider = $3) \
             AND ($4 = '' OR pa.created_at >= $4::timestamptz) \
             AND ($5 = '' OR pa.created_at <= $5::timestamptz)",
        )
        .bind(realm_id)
        .bind(user_id)
        .bind(provider_filter)
        .bind(start_filter)
        .bind(end_filter)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| CoreError::DatabaseError(format!("Failed to count purchase history: {e}")))?;

        // Data query
        let rows = sqlx::query(
            "SELECT pa.id AS attempt_id, pa.target_id AS target_mapping_id, \
             pem.provider_product_info, pem.points_per_period, \
             pa.amount, pa.currency, pa.payment_provider, pa.status, \
             pa.completed_at, pa.created_at \
             FROM payment_attempts pa \
             LEFT JOIN provider_entitlement_mappings pem ON pa.target_id = pem.id \
             WHERE pa.realm_id = $1 AND pa.user_id = $2 \
             AND pa.status = 'Succeeded' AND pa.target_type = 'entitlement_mapping' \
             AND ($3 = '' OR pa.payment_provider = $3) \
             AND ($4 = '' OR pa.created_at >= $4::timestamptz) \
             AND ($5 = '' OR pa.created_at <= $5::timestamptz) \
             ORDER BY pa.created_at DESC \
             LIMIT $6 OFFSET $7",
        )
        .bind(realm_id)
        .bind(user_id)
        .bind(provider_filter)
        .bind(start_filter)
        .bind(end_filter)
        .bind(page_size as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| CoreError::DatabaseError(format!("Failed to fetch purchase history: {e}")))?;

        let items: Vec<PurchaseHistoryRow> = rows
            .into_iter()
            .map(|row| {
                let product_info: Option<serde_json::Value> = row.get("provider_product_info");
                let product_name = product_info
                    .as_ref()
                    .and_then(|v| v.get("name"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let points_per_period: Option<i32> = row.get("points_per_period");

                PurchaseHistoryRow {
                    attempt_id: row.get("attempt_id"),
                    target_mapping_id: row.get("target_mapping_id"),
                    product_name,
                    points: points_per_period.map(|p| p as i64),
                    amount: row.get("amount"),
                    currency: row.get("currency"),
                    payment_provider: row.get("payment_provider"),
                    status: row.get("status"),
                    completed_at: row.get("completed_at"),
                    created_at: row.get("created_at"),
                }
            })
            .collect();

        Ok((items, count))
    }

    async fn has_succeeded_attempt(
        &self,
        user_id: uuid::Uuid,
        target_id: uuid::Uuid,
    ) -> Result<bool, CoreError> {
        // `status` is stored as the PascalCase string ("Succeeded"), matching
        // `PaymentAttemptStatus::Succeeded`'s `to_string()` and the renewal
        // insert path. `target_id` is the entitlement mapping id.
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM payment_attempts \
             WHERE user_id = $1 AND target_id = $2 AND status = 'Succeeded' \
             LIMIT 1",
        )
        .bind(user_id)
        .bind(target_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| CoreError::DatabaseError(format!("Failed to check succeeded attempt: {e}")))?;

        Ok(row.is_some())
    }
}
