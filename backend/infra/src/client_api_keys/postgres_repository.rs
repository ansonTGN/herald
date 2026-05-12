// PostgreSQL Repository for Client API Keys
//
// This module provides database operations for client API keys using Sea-ORM.

use chrono::{DateTime, Utc};
use herald_domain::client_api_keys::entities::ClientApiKey;
use herald_entity::client_api_key;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect, Set,
};
use std::sync::Arc;
use thiserror::Error;

/// Error type for client API key repository operations
#[derive(Debug, Error)]
pub enum ClientApiKeyRepositoryError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sea_orm::DbErr),

    #[error("API key not found: {0}")]
    NotFound(String),

    #[error("API key already exists: {0}")]
    AlreadyExists(String),
}

/// PostgreSQL repository for client API keys
///
/// Provides CRUD operations and specialized queries for API key management.
///
/// # Example
/// ```rust,no_run
/// use herald_core::infrastructure::client_api_keys::postgres_repository::ClientApiKeyRepository;
///
/// let repo = ClientApiKeyRepository::new(db.clone());
///
/// // Create API key
/// let api_key = repo.create(&api_key).await?;
///
/// // Find by hash (authentication)
/// let found = repo.find_by_hash(&hash).await?;
///
/// // Update usage stats
/// repo.update_usage_stats(&api_key.id, Utc::now()).await?;
/// ```
pub struct ClientApiKeyRepository {
    db: Arc<DatabaseConnection>,
}

impl ClientApiKeyRepository {
    /// Create a new repository instance
    ///
    /// # Arguments
    /// * `db` - Database connection
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Create a new API key
    ///
    /// # Arguments
    /// * `api_key` - The API key entity to create
    ///
    /// # Returns
    /// * `Ok(created_api_key)` if successful
    /// * `Err(ClientApiKeyRepositoryError)` if creation fails
    pub async fn create(
        &self,
        api_key: &ClientApiKey,
    ) -> Result<ClientApiKey, ClientApiKeyRepositoryError> {
        let active_model = client_api_key::ActiveModel {
            id: Set(api_key.id.clone()),
            name: Set(api_key.name.clone()),
            api_key_hash: Set(api_key.api_key_hash.clone()),
            realm_id: Set(api_key.realm_id.clone()),
            client_app_id: Set(api_key.client_app_id),
            enabled: Set(api_key.enabled),
            expires_at: Set(api_key.expires_at.map(|dt| dt.into())),
            created_at: Set(api_key.created_at.into()),
            last_used_at: Set(api_key.last_used_at.map(|dt| dt.into())),
            usage_count: Set(api_key.usage_count),
        };

        let inserted = active_model.insert(self.db.as_ref()).await?;

        Ok(Self::model_to_entity(inserted))
    }

    /// Find API key by ID
    ///
    /// # Arguments
    /// * `id` - The API key ID
    ///
    /// # Returns
    /// * `Ok(Some(api_key))` if found
    /// * `Ok(None)` if not found
    /// * `Err(ClientApiKeyRepositoryError)` if query fails
    pub async fn find_by_id(
        &self,
        id: &str,
    ) -> Result<Option<ClientApiKey>, ClientApiKeyRepositoryError> {
        let result = client_api_key::Entity::find_by_id(id.to_string())
            .one(self.db.as_ref())
            .await?;

        Ok(result.map(Self::model_to_entity))
    }

    /// Find API key by hash (used for authentication)
    ///
    /// This is an O(1) query that uses the hash as a direct lookup key.
    ///
    /// # Arguments
    /// * `hash` - The API key hash
    ///
    /// # Returns
    /// * `Ok(Some(api_key))` if found
    /// * `Ok(None)` if not found
    /// * `Err(ClientApiKeyRepositoryError)` if query fails
    ///
    /// # Performance Note
    /// With SHA-256 hashing and deterministic salt, we can perform O(1)
    /// database lookups using the hash column as an indexed key.
    pub async fn find_by_hash(
        &self,
        hash: &str,
    ) -> Result<Option<ClientApiKey>, ClientApiKeyRepositoryError> {
        let result = client_api_key::Entity::find()
            .filter(client_api_key::Column::ApiKeyHash.eq(hash))
            .one(self.db.as_ref())
            .await?;

        Ok(result.map(Self::model_to_entity))
    }

    /// List API keys for a realm
    ///
    /// # Arguments
    /// * `realm_id` - The realm ID
    /// * `offset` - Pagination offset
    /// * `limit` - Maximum number of results
    ///
    /// # Returns
    /// * `Ok(api_keys)` - List of API keys
    /// * `Err(ClientApiKeyRepositoryError)` if query fails
    pub async fn list(
        &self,
        realm_id: &str,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<ClientApiKey>, ClientApiKeyRepositoryError> {
        let results = client_api_key::Entity::find()
            .filter(client_api_key::Column::RealmId.eq(realm_id))
            .offset(offset)
            .limit(limit)
            .all(self.db.as_ref())
            .await?;

        Ok(results.into_iter().map(Self::model_to_entity).collect())
    }

    /// Update usage statistics (called asynchronously after successful auth)
    ///
    /// # Arguments
    /// * `id` - The API key ID
    /// * `last_used_at` - Timestamp of the current usage
    ///
    /// # Returns
    /// * `Ok(())` if successful
    /// * `Err(ClientApiKeyRepositoryError)` if update fails
    ///
    /// # Note
    /// This is typically called using `tokio::spawn` to avoid blocking the request:
    /// ```rust,no_run
    /// tokio::spawn(async move {
    ///     let _ = repo.update_usage_stats(&api_key_id, Utc::now()).await;
    /// });
    /// ```
    pub async fn update_usage_stats(
        &self,
        id: &str,
        last_used_at: DateTime<Utc>,
    ) -> Result<(), ClientApiKeyRepositoryError> {
        use sea_orm::EntityTrait;
        use sea_orm::sea_query::Expr;

        // Perform atomic UPDATE with increment (avoids SELECT-UPDATE anti-pattern)
        // SQL: UPDATE "client_api_key" SET "last_used_at" = $1, "usage_count" = "usage_count" + 1 WHERE "id" = $2
        let result = client_api_key::Entity::update_many()
            .col_expr(
                client_api_key::Column::LastUsedAt,
                Expr::val(last_used_at).into(),
            )
            .col_expr(
                client_api_key::Column::UsageCount,
                Expr::col(client_api_key::Column::UsageCount).add(1),
            )
            .filter(client_api_key::Column::Id.eq(id))
            .exec(self.db.as_ref())
            .await?;

        // Check if the API key exists by verifying rows affected
        if result.rows_affected == 0 {
            return Err(ClientApiKeyRepositoryError::NotFound(id.to_string()));
        }

        Ok(())
    }

    /// Enable or disable an API key
    ///
    /// # Arguments
    /// * `id` - The API key ID
    /// * `enabled` - Whether to enable or disable
    ///
    /// # Returns
    /// * `Ok(())` if successful
    /// * `Err(ClientApiKeyRepositoryError)` if update fails
    pub async fn update_enabled(
        &self,
        id: &str,
        enabled: bool,
    ) -> Result<(), ClientApiKeyRepositoryError> {
        let api_key = self
            .find_by_id(id)
            .await?
            .ok_or_else(|| ClientApiKeyRepositoryError::NotFound(id.to_string()))?;

        let mut active_model = Self::entity_to_active_model(api_key);
        active_model.enabled = Set(enabled);

        active_model.update(self.db.as_ref()).await?;

        Ok(())
    }

    /// Delete an API key
    ///
    /// # Arguments
    /// * `id` - The API key ID
    ///
    /// # Returns
    /// * `Ok(())` if successful
    /// * `Err(ClientApiKeyRepositoryError)` if deletion fails
    pub async fn delete(&self, id: &str) -> Result<(), ClientApiKeyRepositoryError> {
        client_api_key::Entity::delete_by_id(id.to_string())
            .exec(self.db.as_ref())
            .await?;

        Ok(())
    }

    /// List hot keys (most recently used) for cache warmup
    ///
    /// # Arguments
    /// * `limit` - Maximum number of results
    ///
    /// # Returns
    /// * `Ok(api_keys)` - List of most recently used API keys
    /// * `Err(ClientApiKeyRepositoryError)` if query fails
    ///
    /// # Note
    /// This is used during application startup to warm up the cache:
    /// ```rust,no_run
    /// let hot_keys = repo.list_hot_keys(100).await?;
    /// for key in hot_keys {
    ///     cache.set(&key.api_key_hash, &key, 300).await?;
    /// }
    /// ```
    pub async fn list_hot_keys(
        &self,
        limit: u64,
    ) -> Result<Vec<ClientApiKey>, ClientApiKeyRepositoryError> {
        let results = client_api_key::Entity::find()
            .filter(client_api_key::Column::Enabled.eq(true))
            .limit(limit)
            .all(self.db.as_ref())
            .await?;

        Ok(results.into_iter().map(Self::model_to_entity).collect())
    }

    /// Convert Sea-ORM model to domain entity
    fn model_to_entity(model: client_api_key::Model) -> ClientApiKey {
        ClientApiKey {
            id: model.id,
            name: model.name,
            api_key_hash: model.api_key_hash,
            realm_id: model.realm_id,
            client_app_id: model.client_app_id,
            enabled: model.enabled,
            expires_at: model.expires_at.map(|dt| dt.into()),
            created_at: model.created_at.into(),
            last_used_at: model.last_used_at.map(|dt| dt.into()),
            usage_count: model.usage_count,
        }
    }

    /// Convert domain entity to Sea-ORM ActiveModel for updates
    fn entity_to_active_model(api_key: ClientApiKey) -> client_api_key::ActiveModel {
        client_api_key::ActiveModel {
            id: Set(api_key.id),
            name: Set(api_key.name),
            api_key_hash: Set(api_key.api_key_hash),
            realm_id: Set(api_key.realm_id),
            client_app_id: Set(api_key.client_app_id),
            enabled: Set(api_key.enabled),
            expires_at: Set(api_key.expires_at.map(|dt| dt.into())),
            created_at: Set(api_key.created_at.into()),
            last_used_at: Set(api_key.last_used_at.map(|dt| dt.into())),
            usage_count: Set(api_key.usage_count),
        }
    }
}

#[cfg(test)]
mod tests {

    // Note: Integration tests should be in a separate test file
    // that sets up a test database using testcontainers
}
