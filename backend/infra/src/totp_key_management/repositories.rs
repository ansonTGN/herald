use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use herald_domain::common::entities::app_errors::CoreError;
use herald_domain::totp_key_management::ports::RealmTotpKeyRepository;
use sea_orm::DatabaseConnection;
use sqlx;
use std::sync::Arc;

/// PostgreSQL implementation of RealmTotpKeyRepository
///
/// This repository stores realm TOTP encryption keys in the realm_config table.
/// Keys are base64-encoded and marked as sensitive (is_secret=true).
pub struct PostgresRealmTotpKeyRepository {
    db: Arc<DatabaseConnection>,
}

impl PostgresRealmTotpKeyRepository {
    /// Create a new PostgresRealmTotpKeyRepository instance
    ///
    /// # Arguments
    /// * `db` - Database connection
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

impl RealmTotpKeyRepository for PostgresRealmTotpKeyRepository {
    async fn get_active_key(&self, realm_id: &str) -> Result<Option<[u8; 32]>, CoreError> {
        let config_value: Option<String> = sqlx::query_scalar(
            r#"
            SELECT config_value
            FROM realm_config
            WHERE realm_id = $1
              AND config_type = 'totp_key'
              AND enabled = true
            LIMIT 1
            "#,
        )
        .bind(realm_id)
        .fetch_optional(self.db.get_postgres_connection_pool())
        .await
        .map_err(|e| {
            tracing::error!(
                realm_id = %realm_id,
                error = %e,
                "Failed to get active realm TOTP key"
            );
            CoreError::InternalServerError(e.to_string())
        })?;

        match config_value {
            Some(base64_key) => {
                // Decode base64 to get raw key bytes
                let key_bytes = STANDARD.decode(&base64_key).map_err(|e| {
                    tracing::error!(
                        error = %e,
                        "Failed to decode base64 realm TOTP key"
                    );
                    CoreError::InternalServerError("Failed to decode realm key".to_string())
                })?;

                if key_bytes.len() != 32 {
                    return Err(CoreError::InternalServerError(
                        "Invalid realm key length: expected 32 bytes".to_string(),
                    ));
                }

                // Convert to fixed-size array
                let mut key = [0u8; 32];
                key.copy_from_slice(&key_bytes);
                Ok(Some(key))
            }
            None => {
                tracing::debug!(
                    realm_id = %realm_id,
                    "No active realm TOTP key found"
                );
                Ok(None)
            }
        }
    }

    async fn create_key(&self, realm_id: &str, key: [u8; 32]) -> Result<(), CoreError> {
        let key_base64 = STANDARD.encode(key);
        let config_id = herald_domain::common::generate_uuid_v7();

        sqlx::query(
            r#"
            INSERT INTO realm_config
                (id, realm_id, config_type, config_key, config_value, is_secret, enabled, metadata, created_at, updated_at)
            VALUES ($1, $2, 'totp_key', 'version_1', $3, true, true, $4, NOW(), NOW())
            ON CONFLICT (realm_id, config_type, config_key) DO NOTHING
            "#
        )
        .bind(config_id)
        .bind(realm_id)
        .bind(key_base64)
        .bind(serde_json::json!({ "version": 1 }))
        .execute(self.db.get_postgres_connection_pool())
        .await
        .map_err(|e| {
            tracing::error!(
                realm_id = %realm_id,
                error = %e,
                "Failed to create realm TOTP key"
            );
            CoreError::InternalServerError(e.to_string())
        })?;

        tracing::debug!(
            realm_id = %realm_id,
            "TOTP realm key created successfully (version: 1)"
        );

        Ok(())
    }
}
