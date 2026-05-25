//! WeChat Pay order repository
//!
//! Provides database access methods for `wechat_payment_order` table
//! and WeChat configuration stored in `realm_config`.

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::models::{WechatOrderStatus, WechatPaymentOrder};
use herald_domain::common::entities::app_errors::CoreError;

/// Repository for WeChat payment order database operations
pub struct WechatOrderRepository {
    pool: PgPool,
}

static ENCRYPTION_KEY: Lazy<Result<[u8; 32], String>> = Lazy::new(|| {
    let key_str = std::env::var("ENCRYPTION_KEY")
        .map_err(|_| "ENCRYPTION_KEY environment variable not set".to_string())?;
    let key_bytes = STANDARD
        .decode(&key_str)
        .map_err(|e| format!("ENCRYPTION_KEY must be valid base64: {e}"))?;

    if key_bytes.len() != 32 {
        return Err(format!(
            "ENCRYPTION_KEY must be 32 bytes (256 bits), got {} bytes",
            key_bytes.len()
        ));
    }

    let mut key = [0u8; 32];
    key.copy_from_slice(&key_bytes);
    Ok(key)
});

const NONCE_SIZE: usize = 12;

fn encryption_key() -> Result<&'static [u8; 32], CoreError> {
    ENCRYPTION_KEY
        .as_ref()
        .map_err(|e| CoreError::InternalServerError(e.clone()))
}

fn is_wechat_secret_key(key: &str) -> bool {
    matches!(key, "private_key" | "v3_key")
}

fn encrypt_secret(secret: &str) -> Result<String, CoreError> {
    let cipher = Aes256Gcm::new_from_slice(encryption_key()?).map_err(|e| {
        tracing::error!("Failed to create WeChat secret cipher: {}", e);
        CoreError::InternalServerError(format!("Cipher init failed: {}", e))
    })?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher.encrypt(&nonce, secret.as_bytes()).map_err(|e| {
        tracing::error!("Failed to encrypt WeChat config value: {}", e);
        CoreError::InternalServerError(format!("Encryption failed: {}", e))
    })?;

    Ok(STANDARD.encode([nonce.as_slice(), &ciphertext].concat()))
}

fn decrypt_secret(encrypted_secret: &str) -> Result<String, CoreError> {
    let combined = STANDARD.decode(encrypted_secret).map_err(|e| {
        tracing::error!("Failed to decode WeChat config value: {}", e);
        CoreError::InvalidWebhookSecret
    })?;

    if combined.len() < NONCE_SIZE + 16 {
        return Err(CoreError::InvalidWebhookSecret);
    }

    let cipher = Aes256Gcm::new_from_slice(encryption_key()?).map_err(|e| {
        tracing::error!("Failed to create WeChat secret cipher: {}", e);
        CoreError::InternalServerError(format!("Cipher init failed: {}", e))
    })?;
    let (nonce_bytes, ciphertext) = combined.split_at(NONCE_SIZE);
    let plaintext = cipher
        .decrypt(Nonce::from_slice(nonce_bytes), ciphertext)
        .map_err(|e| {
            tracing::debug!("Failed to decrypt WeChat config value: {}", e);
            CoreError::InvalidWebhookSecret
        })?;

    String::from_utf8(plaintext).map_err(|e| {
        tracing::error!("Decrypted WeChat config value is not valid UTF-8: {}", e);
        CoreError::InvalidWebhookSecret
    })
}

fn store_config_value(key: &str, value: &str) -> Result<String, CoreError> {
    if is_wechat_secret_key(key) {
        encrypt_secret(value)
    } else {
        Ok(value.to_string())
    }
}

fn load_config_value(key: &str, value: String) -> Result<String, CoreError> {
    if is_wechat_secret_key(key) {
        decrypt_secret(&value)
    } else {
        Ok(value)
    }
}

impl WechatOrderRepository {
    /// Create a new repository instance
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insert a new payment order
    pub async fn create_order(
        &self,
        order: &WechatPaymentOrder,
    ) -> Result<WechatPaymentOrder, CoreError> {
        sqlx::query(
            r#"
            INSERT INTO wechat_payment_order
                (id, realm_id, user_id, plan_id, client_app_id, out_trade_no,
                 transaction_id, amount, currency, code_url, status, description,
                 paid_at, closed_at, expires_at, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
            "#,
        )
        .bind(order.id)
        .bind(&order.realm_id)
        .bind(order.user_id)
        .bind(order.plan_id)
        .bind(order.client_app_id)
        .bind(&order.out_trade_no)
        .bind(&order.transaction_id)
        .bind(order.amount)
        .bind(&order.currency)
        .bind(&order.code_url)
        .bind(order.status.as_str()) // Convert enum to string
        .bind(&order.description)
        .bind(order.paid_at)
        .bind(order.closed_at)
        .bind(order.expires_at)
        .bind(order.created_at)
        .bind(order.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to insert wechat_payment_order");
            CoreError::InternalServerError(format!("Database error: {}", e))
        })?;

        Ok(order.clone())
    }

    /// Find order by primary key
    pub async fn find_order_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<WechatPaymentOrder>, CoreError> {
        let row = sqlx::query(
            r#"
            SELECT id, realm_id, user_id, plan_id, client_app_id, out_trade_no,
                   transaction_id, amount, currency, code_url, status, description,
                   paid_at, closed_at, expires_at, created_at, updated_at
            FROM wechat_payment_order
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(order_id = %id, error = %e, "Failed to query wechat_payment_order by id");
            CoreError::InternalServerError(format!("Database error: {}", e))
        })?;

        let Some(row) = row else { return Ok(None) };
        Ok(Some(Self::row_to_order(&row)))
    }

    /// Find order by merchant order number (out_trade_no)
    pub async fn find_order_by_out_trade_no(
        &self,
        out_trade_no: &str,
    ) -> Result<Option<WechatPaymentOrder>, CoreError> {
        let row = sqlx::query(
            r#"
            SELECT id, realm_id, user_id, plan_id, client_app_id, out_trade_no,
                   transaction_id, amount, currency, code_url, status, description,
                   paid_at, closed_at, expires_at, created_at, updated_at
            FROM wechat_payment_order
            WHERE out_trade_no = $1
            "#,
        )
        .bind(out_trade_no)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(out_trade_no = %out_trade_no, error = %e, "Failed to query wechat_payment_order by out_trade_no");
            CoreError::InternalServerError(format!("Database error: {}", e))
        })?;

        let Some(row) = row else { return Ok(None) };
        Ok(Some(Self::row_to_order(&row)))
    }

    /// Update order status (generic)
    pub async fn update_order_status(
        &self,
        id: Uuid,
        status: WechatOrderStatus,
        transaction_id: Option<&str>,
    ) -> Result<(), CoreError> {
        sqlx::query(
            r#"
            UPDATE wechat_payment_order
            SET status = $1,
                transaction_id = COALESCE($2, transaction_id),
                updated_at = NOW()
            WHERE id = $3
            "#,
        )
        .bind(status.as_str())
        .bind(transaction_id)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(order_id = %id, error = %e, "Failed to update wechat order status");
            CoreError::InternalServerError(format!("Database error: {}", e))
        })?;

        Ok(())
    }

    /// Mark order as paid
    pub async fn mark_order_paid(&self, id: Uuid, transaction_id: &str) -> Result<(), CoreError> {
        sqlx::query(
            r#"
            UPDATE wechat_payment_order
            SET status = 'paid',
                transaction_id = $1,
                paid_at = NOW(),
                updated_at = NOW()
            WHERE id = $2
            "#,
        )
        .bind(transaction_id)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(order_id = %id, error = %e, "Failed to mark wechat order as paid");
            CoreError::InternalServerError(format!("Database error: {}", e))
        })?;

        Ok(())
    }

    /// Mark order as closed
    pub async fn mark_order_closed(&self, id: Uuid) -> Result<(), CoreError> {
        sqlx::query(
            r#"
            UPDATE wechat_payment_order
            SET status = 'closed',
                closed_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(order_id = %id, error = %e, "Failed to mark wechat order as closed");
            CoreError::InternalServerError(format!("Database error: {}", e))
        })?;

        Ok(())
    }

    // ===== Configuration (realm_config with config_type='wechat') =====

    /// Get WeChat config for a realm
    ///
    /// Returns all config keys for `config_type='wechat'` from `realm_config`.
    /// Sensitive fields (`private_key`, `v3_key`) are returned in plaintext
    /// (masking is the caller's responsibility).
    pub async fn get_wechat_config(
        &self,
        realm_id: &str,
    ) -> Result<Option<WechatConfigRow>, CoreError> {
        let rows = sqlx::query(
            r#"
            SELECT config_key, config_value, created_at, updated_at
            FROM realm_config
            WHERE realm_id = $1 AND config_type = 'wechat'
            "#,
        )
        .bind(realm_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(realm_id = %realm_id, error = %e, "Failed to load wechat config");
            CoreError::InternalServerError(format!("Database error: {}", e))
        })?;

        if rows.is_empty() {
            return Ok(None);
        }

        let mut config = WechatConfigRow::default();
        for row in rows {
            let key: String = row.get("config_key");
            let value: String = row.get("config_value");

            match key.as_str() {
                "app_id" => config.app_id = Some(value),
                "mch_id" => config.mch_id = Some(value),
                "private_key" => config.private_key = Some(load_config_value(&key, value)?),
                "serial_no" => config.serial_no = Some(value),
                "v3_key" => config.v3_key = Some(load_config_value(&key, value)?),
                "platform_public_key" => config.platform_public_key = Some(value),
                "notify_url" => config.notify_url = Some(value),
                "mock_base_url" => config.mock_base_url = Some(value),
                _ => {}
            }

            let created_at: Option<DateTime<Utc>> = row.get("created_at");
            let updated_at: Option<DateTime<Utc>> = row.get("updated_at");

            if let Some(ca) = created_at {
                config.created_at = Some(
                    config
                        .created_at
                        .map_or(ca, |existing: DateTime<Utc>| existing.min(ca)),
                );
            }
            if let Some(ua) = updated_at {
                config.updated_at = Some(
                    config
                        .updated_at
                        .map_or(ua, |existing: DateTime<Utc>| existing.max(ua)),
                );
            }
        }

        Ok(Some(config))
    }

    /// Save WeChat config for a realm (create)
    #[allow(clippy::too_many_arguments)]
    pub async fn save_wechat_config(
        &self,
        realm_id: &str,
        app_id: &str,
        mch_id: &str,
        private_key: &str,
        serial_no: &str,
        v3_key: &str,
        notify_url: &str,
    ) -> Result<(), CoreError> {
        let config_items: Vec<(&str, &str, bool)> = vec![
            ("app_id", app_id, false),
            ("mch_id", mch_id, false),
            ("private_key", private_key, true),
            ("serial_no", serial_no, false),
            ("v3_key", v3_key, true),
            ("notify_url", notify_url, false),
        ];

        for (key, value, is_secret) in config_items {
            let stored_value = store_config_value(key, value)?;
            sqlx::query(
                r#"
                INSERT INTO realm_config (id, realm_id, config_type, config_key, config_value, is_secret, created_at, updated_at)
                VALUES ($1, $2, 'wechat', $3, $4, $5, NOW(), NOW())
                "#,
            )
            .bind(Uuid::now_v7())
            .bind(realm_id)
            .bind(key)
            .bind(&stored_value)
            .bind(is_secret)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!(realm_id = %realm_id, key = %key, error = %e, "Failed to save wechat config");
                CoreError::InternalServerError(format!("Database error: {}", e))
            })?;
        }

        Ok(())
    }

    /// Update WeChat config for a realm (partial update)
    #[allow(clippy::too_many_arguments)]
    pub async fn update_wechat_config(
        &self,
        realm_id: &str,
        app_id: Option<&str>,
        mch_id: Option<&str>,
        private_key: Option<&str>,
        serial_no: Option<&str>,
        v3_key: Option<&str>,
        notify_url: Option<&str>,
    ) -> Result<(), CoreError> {
        let updates: Vec<(&str, &str)> = [
            app_id.map(|v| ("app_id", v)),
            mch_id.map(|v| ("mch_id", v)),
            private_key.map(|v| ("private_key", v)),
            serial_no.map(|v| ("serial_no", v)),
            v3_key.map(|v| ("v3_key", v)),
            notify_url.map(|v| ("notify_url", v)),
        ]
        .into_iter()
        .flatten()
        .collect();

        for (key, value) in updates {
            let stored_value = store_config_value(key, value)?;
            sqlx::query(
                r#"
                UPDATE realm_config
                SET config_value = $1, updated_at = NOW()
                WHERE realm_id = $2
                  AND config_type = 'wechat'
                  AND config_key = $3
                "#,
            )
            .bind(&stored_value)
            .bind(realm_id)
            .bind(key)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!(realm_id = %realm_id, key = %key, error = %e, "Failed to update wechat config");
                CoreError::InternalServerError(format!("Database error: {}", e))
            })?;
        }

        Ok(())
    }

    /// Delete WeChat config for a realm
    pub async fn delete_wechat_config(&self, realm_id: &str) -> Result<(), CoreError> {
        sqlx::query(
            r#"
            DELETE FROM realm_config
            WHERE realm_id = $1
              AND config_type = 'wechat'
            "#,
        )
        .bind(realm_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(realm_id = %realm_id, error = %e, "Failed to delete wechat config");
            CoreError::InternalServerError(format!("Database error: {}", e))
        })?;

        Ok(())
    }

    /// Count active wechat subscriptions for a realm
    pub async fn count_active_wechat_subscriptions(
        &self,
        realm_id: &str,
    ) -> Result<i64, CoreError> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM subscription
            WHERE realm_id = $1
              AND payment_provider = 'wechat'
              AND status = 'active'
            "#,
        )
        .bind(realm_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(realm_id = %realm_id, error = %e, "Failed to count wechat subscriptions");
            CoreError::InternalServerError(format!("Database error: {}", e))
        })?;

        Ok(count)
    }

    /// Convert a database row to a WechatPaymentOrder
    fn row_to_order(row: &sqlx::postgres::PgRow) -> WechatPaymentOrder {
        let status_str: String = row.get("status");
        let status = status_str
            .parse::<WechatOrderStatus>()
            .unwrap_or(WechatOrderStatus::Pending); // Default to pending if invalid

        WechatPaymentOrder {
            id: row.get("id"),
            realm_id: row.get("realm_id"),
            user_id: row.get("user_id"),
            plan_id: row.get("plan_id"),
            client_app_id: row.get("client_app_id"),
            out_trade_no: row.get("out_trade_no"),
            transaction_id: row.get("transaction_id"),
            amount: row.get("amount"),
            currency: row.get("currency"),
            code_url: row.get("code_url"),
            status,
            description: row.get("description"),
            paid_at: row.get("paid_at"),
            closed_at: row.get("closed_at"),
            expires_at: row.get("expires_at"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}

/// Raw WeChat config row from realm_config
#[derive(Debug, Clone, Default)]
pub struct WechatConfigRow {
    pub app_id: Option<String>,
    pub mch_id: Option<String>,
    pub private_key: Option<String>,
    pub serial_no: Option<String>,
    pub v3_key: Option<String>,
    pub platform_public_key: Option<String>,
    pub notify_url: Option<String>,
    pub mock_base_url: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}
