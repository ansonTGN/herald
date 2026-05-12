// Realm configuration repository implementation

use sea_orm::DatabaseConnection;
use std::future::Future;
use std::sync::Arc;

use herald_domain::common::entities::app_errors::CoreError;
use herald_domain::realm_config::{RealmConfig, RealmConfigRepository, UpsertRealmConfigRequest};

type RealmConfigRow = (
    uuid::Uuid,
    String,
    String,
    String,
    String,
    bool,
    bool,
    Option<serde_json::Value>,
    chrono::DateTime<chrono::Utc>,
    chrono::DateTime<chrono::Utc>,
);

pub struct PostgresRealmConfigRepository {
    db: Arc<DatabaseConnection>,
}

impl PostgresRealmConfigRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    fn row_to_entity(row: RealmConfigRow) -> Result<RealmConfig, CoreError> {
        let (
            id,
            realm_id,
            config_type_str,
            config_key,
            config_value,
            is_secret,
            enabled,
            metadata,
            created_at,
            updated_at,
        ) = row;

        let config_type = config_type_str.into();

        Ok(RealmConfig {
            id,
            realm_id,
            config_type,
            config_key,
            config_value,
            is_secret,
            enabled,
            metadata,
            created_at,
            updated_at,
        })
    }
}

impl RealmConfigRepository for PostgresRealmConfigRepository {
    fn upsert(
        &self,
        realm_id: &str,
        request: UpsertRealmConfigRequest,
    ) -> impl Future<Output = Result<RealmConfig, CoreError>> + Send {
        let realm_id = realm_id.to_string();
        let db = self.db.clone();

        async move {
            let config_type_str: String = request.config_type.clone().into();
            let now = chrono::Utc::now();

            let row: RealmConfigRow = sqlx::query_as(
                "INSERT INTO realm_config (id, realm_id, config_type, config_key, config_value, is_secret, enabled, metadata, created_at, updated_at)
                 VALUES (uuidv7(), $1, $2, $3, $4, $5, $6, $7, $8, $8)
                 ON CONFLICT (realm_id, config_type, config_key)
                 DO UPDATE SET
                     config_value = EXCLUDED.config_value,
                     is_secret = COALESCE(EXCLUDED.is_secret, realm_config.is_secret),
                     enabled = COALESCE(EXCLUDED.enabled, realm_config.enabled),
                     metadata = COALESCE(EXCLUDED.metadata, realm_config.metadata),
                     updated_at = EXCLUDED.updated_at
                 RETURNING id, realm_id, config_type, config_key, config_value, is_secret, enabled, metadata, created_at, updated_at",
            )
            .bind(&realm_id)
            .bind(&config_type_str)
            .bind(&request.config_key)
            .bind(&request.config_value)
            .bind(request.is_secret.unwrap_or(false))
            .bind(request.enabled.unwrap_or(true))
            .bind(request.metadata.clone().unwrap_or(serde_json::json!({})))
            .bind(now)
            .fetch_one(db.get_postgres_connection_pool())
            .await
            .map_err(|e| {
                tracing::error!("Failed to upsert realm config: {e}");
                CoreError::InternalServerError(e.to_string())
            })?;

            Self::row_to_entity(row)
        }
    }

    fn batch_upsert(
        &self,
        realm_id: &str,
        requests: Vec<UpsertRealmConfigRequest>,
    ) -> impl Future<Output = Result<Vec<RealmConfig>, CoreError>> + Send {
        let realm_id = realm_id.to_string();
        let db = self.db.clone();

        async move {
            let mut results = Vec::with_capacity(requests.len());

            tracing::debug!(
                realm_id = %realm_id,
                config_count = requests.len(),
                "Starting batch upsert of realm configs"
            );

            // 使用事务来保证批量操作的原子性
            let mut tx = db
                .get_postgres_connection_pool()
                .begin()
                .await
                .map_err(|e| {
                    tracing::error!(realm_id = %realm_id, error = %e, "Failed to begin transaction");
                    CoreError::InternalServerError(e.to_string())
                })?;

            tracing::debug!(realm_id = %realm_id, "Transaction started successfully");

            for (index, request) in requests.iter().enumerate() {
                let config_type_str: String = request.config_type.clone().into();
                let now = chrono::Utc::now();

                let enabled_value = request.enabled.unwrap_or(true);

                tracing::debug!(
                    index = index,
                    realm_id = %realm_id,
                    config_type = %config_type_str,
                    config_key = %request.config_key,
                    enabled = enabled_value,
                    is_secret = request.is_secret,
                    "Upserting realm config"
                );

                let row: RealmConfigRow = sqlx::query_as(
                    "INSERT INTO realm_config (id, realm_id, config_type, config_key, config_value, is_secret, enabled, metadata, created_at, updated_at)
                     VALUES (uuidv7(), $1, $2, $3, $4, $5, $6, $7, $8, $8)
                     ON CONFLICT (realm_id, config_type, config_key)
                     DO UPDATE SET
                         config_value = EXCLUDED.config_value,
                         is_secret = COALESCE(EXCLUDED.is_secret, realm_config.is_secret),
                         enabled = COALESCE(EXCLUDED.enabled, realm_config.enabled),
                         metadata = COALESCE(EXCLUDED.metadata, realm_config.metadata),
                         updated_at = EXCLUDED.updated_at
                     RETURNING id, realm_id, config_type, config_key, config_value, is_secret, enabled, metadata, created_at, updated_at",
                )
                .bind(&realm_id)
                .bind(&config_type_str)
                .bind(&request.config_key)
                .bind(&request.config_value)
                .bind(request.is_secret.unwrap_or(false))
                .bind(enabled_value)
                .bind(request.metadata.clone().unwrap_or(serde_json::json!({})))
                .bind(now)
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| {
                    tracing::error!(
                        index = index,
                        realm_id = %realm_id,
                        config_type = %config_type_str,
                        config_key = %request.config_key,
                        error = %e,
                        "Failed to batch upsert realm config"
                    );
                    CoreError::InternalServerError(e.to_string())
                })?;

                let config = Self::row_to_entity(row)?;
                tracing::debug!(
                    index = index,
                    realm_id = %realm_id,
                    config_id = %config.id,
                    config_key = %config.config_key,
                    enabled = config.enabled,
                    updated_at = %config.updated_at,
                    "Successfully upserted realm config (from RETURNING clause)"
                );
                results.push(config);
            }

            tracing::debug!(
                realm_id = %realm_id,
                config_count = results.len(),
                "Committing transaction"
            );

            tx.commit().await.map_err(|e| {
                tracing::error!(realm_id = %realm_id, error = %e, "Failed to commit transaction");
                CoreError::InternalServerError(e.to_string())
            })?;

            tracing::debug!(
                realm_id = %realm_id,
                config_count = results.len(),
                "Transaction committed successfully"
            );

            Ok(results)
        }
    }

    fn get(
        &self,
        realm_id: String,
        config_type: String,
        config_key: String,
    ) -> impl Future<Output = Result<Option<RealmConfig>, CoreError>> + Send {
        let db = self.db.clone();

        async move {
            let row = sqlx::query_as(
                "SELECT id, realm_id, config_type, config_key, config_value, is_secret, enabled, metadata, created_at, updated_at
                 FROM realm_config
                 WHERE realm_id = $1 AND config_type = $2 AND config_key = $3",
            )
            .bind(&realm_id)
            .bind(&config_type)
            .bind(&config_key)
            .fetch_optional(db.get_postgres_connection_pool())
            .await
            .map_err(|e| {
                tracing::error!("Failed to get realm config: {e}");
                CoreError::InternalServerError(e.to_string())
            })?;

            match row {
                Some(r) => Self::row_to_entity(r).map(Some),
                None => Ok(None),
            }
        }
    }

    fn get_by_type(
        &self,
        realm_id: String,
        config_type: String,
    ) -> impl Future<Output = Result<Vec<RealmConfig>, CoreError>> + Send {
        let db = self.db.clone();

        async move {
            let rows = sqlx::query_as(
                "SELECT id, realm_id, config_type, config_key, config_value, is_secret, enabled, metadata, created_at, updated_at
                 FROM realm_config
                 WHERE realm_id = $1 AND config_type = $2
                 ORDER BY config_key",
            )
            .bind(&realm_id)
            .bind(&config_type)
            .fetch_all(db.get_postgres_connection_pool())
            .await
            .map_err(|e| {
                tracing::error!("Failed to get realm configs by type: {e}");
                CoreError::InternalServerError(e.to_string())
            })?;

            rows.into_iter().map(Self::row_to_entity).collect()
        }
    }

    fn get_all(
        &self,
        realm_id: String,
    ) -> impl Future<Output = Result<Vec<RealmConfig>, CoreError>> + Send {
        let db = self.db.clone();

        async move {
            let rows = sqlx::query_as(
                "SELECT id, realm_id, config_type, config_key, config_value, is_secret, enabled, metadata, created_at, updated_at
                 FROM realm_config
                 WHERE realm_id = $1
                 ORDER BY config_type, config_key",
            )
            .bind(&realm_id)
            .fetch_all(db.get_postgres_connection_pool())
            .await
            .map_err(|e| {
                tracing::error!("Failed to get all realm configs: {e}");
                CoreError::InternalServerError(e.to_string())
            })?;

            rows.into_iter().map(Self::row_to_entity).collect()
        }
    }

    fn delete(
        &self,
        realm_id: String,
        config_type: String,
        config_key: String,
    ) -> impl Future<Output = Result<(), CoreError>> + Send {
        let db = self.db.clone();

        async move {
            let result = sqlx::query(
                "DELETE FROM realm_config WHERE realm_id = $1 AND config_type = $2 AND config_key = $3",
            )
            .bind(&realm_id)
            .bind(&config_type)
            .bind(&config_key)
            .execute(db.get_postgres_connection_pool())
            .await
            .map_err(|e| {
                tracing::error!("Failed to delete realm config: {e}");
                CoreError::InternalServerError(e.to_string())
            })?;

            if result.rows_affected() == 0 {
                return Err(CoreError::NotFound);
            }

            Ok(())
        }
    }

    fn exists(
        &self,
        realm_id: String,
        config_type: String,
        config_key: String,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send {
        let db = self.db.clone();

        async move {
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(
                    SELECT 1 FROM realm_config
                    WHERE realm_id = $1 AND config_type = $2 AND config_key = $3
                )",
            )
            .bind(&realm_id)
            .bind(&config_type)
            .bind(&config_key)
            .fetch_one(db.get_postgres_connection_pool())
            .await
            .map_err(|e| {
                tracing::error!("Failed to check realm config exists: {e}");
                CoreError::InternalServerError(e.to_string())
            })?;

            Ok(exists)
        }
    }
}

impl std::fmt::Debug for PostgresRealmConfigRepository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostgresRealmConfigRepository").finish()
    }
}
