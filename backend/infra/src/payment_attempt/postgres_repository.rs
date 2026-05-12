// PostgreSQL implementation for Payment Attempt repository

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use std::sync::Arc;

use herald_domain::common::entities::app_errors::CoreError;
use herald_domain::payment_attempt::{
    CreatePaymentAttemptInput, PaymentAttempt, PaymentAttemptRepository,
};
use herald_entity::payment_attempt as payment_attempt_entity;

/// PostgreSQL implementation of PaymentAttempt repository
pub struct PostgresPaymentAttemptRepository {
    db: Arc<DatabaseConnection>,
}

impl PostgresPaymentAttemptRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
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
}
