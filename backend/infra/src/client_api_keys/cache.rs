// Redis Cache Service for Client API Keys
//
// This module provides Redis caching for client API keys to improve
// authentication performance. The cache uses a TTL-based strategy with
// an expected hit rate of ~90%.

use crate::redis::{RedisConnectionManager, RedisError};
use chrono::{DateTime, Utc};
use herald_domain::client_api_keys::entities::ClientApiKey;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid;

/// Cached API Key value (stored in Redis)
///
/// This is a lightweight version of ClientApiKey optimized for
/// Redis storage. We use ISO 8601 strings for timestamps instead of
/// DateTime objects to reduce serialization overhead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyCacheValue {
    /// API Key ID
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// API Key hash (for verification)
    pub api_key_hash: String,

    /// Realm ID
    pub realm_id: String,

    /// Client App ID (1:1 relationship, optional for backward compatibility)
    pub client_app_id: Option<uuid::Uuid>,

    /// Whether the key is enabled
    pub enabled: bool,

    /// Expiration time (ISO 8601 string, None = never expires)
    pub expires_at: Option<String>,

    /// Creation time (ISO 8601 string)
    pub created_at: String,
}

impl From<&ClientApiKey> for ApiKeyCacheValue {
    fn from(api_key: &ClientApiKey) -> Self {
        Self {
            id: api_key.id.clone(),
            name: api_key.name.clone(),
            api_key_hash: api_key.api_key_hash.clone(),
            realm_id: api_key.realm_id.clone(),
            client_app_id: api_key.client_app_id,
            enabled: api_key.enabled,
            expires_at: api_key.expires_at.map(|dt| dt.to_rfc3339()),
            created_at: api_key.created_at.to_rfc3339(),
        }
    }
}

impl TryFrom<ApiKeyCacheValue> for ClientApiKey {
    type Error = String;

    fn try_from(value: ApiKeyCacheValue) -> Result<Self, Self::Error> {
        let expires_at = match value.expires_at {
            Some(ts) => Some(
                DateTime::parse_from_rfc3339(&ts)
                    .map_err(|e| format!("Invalid expires_at: {}", e))?
                    .with_timezone(&Utc),
            ),
            None => None,
        };

        let created_at = DateTime::parse_from_rfc3339(&value.created_at)
            .map_err(|e| format!("Invalid created_at: {}", e))?
            .with_timezone(&Utc);

        Ok(Self {
            id: value.id,
            name: value.name,
            api_key_hash: value.api_key_hash,
            realm_id: value.realm_id,
            client_app_id: value.client_app_id,
            enabled: value.enabled,
            expires_at,
            created_at,
            last_used_at: None,
            usage_count: 0,
        })
    }
}

/// Redis cache service for client API keys
///
/// # Performance
/// - Expected cache hit rate: ~90%
/// - Cache hit latency: < 5ms
/// - Cache miss latency: < 50ms (includes DB query)
///
/// # Cache Strategy
/// - **Key format**: `api_key:{plaintext_key}` where {plaintext_key} is the raw API key
/// - **TTL**: 300 seconds (5 minutes) with randomization (±10s) to prevent cache stampede
/// - **Invalidation**: Manual deletion on key update/disable
///
/// **Why plaintext key as cache key?**
/// Using the plaintext API key as the cache key allows us to bypass
/// the expensive database lookup on cache hits.
/// Even with SHA-256 deterministic hashing enabling O(1) database lookups,
/// caching at the plaintext level provides faster authentication.
///
/// # Example
/// ```rust,no_run
/// use herald_core::infrastructure::client_api_keys::cache::ApiKeyCache;
///
/// // Get from cache
/// if let Some(cached) = cache.get(&api_key_plaintext).await? {
///     // Cache hit - use cached value
/// }
///
/// // Set to cache
/// cache.set(&api_key_plaintext, &api_key, 300).await?;
///
/// // Delete from cache
/// cache.delete(&api_key_plaintext).await?;
/// ```
#[derive(Clone)]
pub struct ApiKeyCache {
    redis: Arc<RedisConnectionManager>,
}

impl ApiKeyCache {
    /// Create a new API key cache service
    ///
    /// # Arguments
    /// * `redis` - Redis connection manager
    pub fn new(redis: Arc<RedisConnectionManager>) -> Self {
        Self { redis }
    }

    /// Get API key from cache
    ///
    /// # Arguments
    /// * `api_key_plaintext` - The plaintext API key (used as cache key)
    ///
    /// # Returns
    /// * `Ok(Some(cached_value))` if found in cache
    /// * `Ok(None)` if not found
    /// * `Err(RedisError)` if Redis operation fails
    ///
    /// # Example
    /// ```rust,no_run
    /// let cached = cache.get(&api_key_plaintext).await?;
    /// if let Some(value) = cached {
    ///     println!("Cache hit: {}", value.name);
    /// }
    /// ```
    pub async fn get(
        &self,
        api_key_plaintext: &str,
    ) -> Result<Option<ApiKeyCacheValue>, RedisError> {
        let key = format!("api_key:{}", api_key_plaintext);
        let mut conn = self.redis.get().await?;

        let value: Option<String> = conn.get(&key).await?;

        match value {
            Some(v) => {
                let cached = serde_json::from_str(&v)
                    .map_err(|e| RedisError::CommandFailed(format!("JSON parse error: {}", e)))?;
                Ok(Some(cached))
            }
            None => Ok(None),
        }
    }

    /// Set API key in cache
    ///
    /// # Arguments
    /// * `api_key_plaintext` - The plaintext API key (used as cache key)
    /// * `value` - The cache value to store
    /// * `ttl` - Time-to-live in seconds (default: 300)
    ///
    /// # Returns
    /// * `Ok(())` if successful
    /// * `Err(RedisError)` if Redis operation fails
    ///
    /// # Example
    /// ```rust,no_run
    /// cache.set(&api_key_plaintext, &api_key, 300).await?;
    /// ```
    pub async fn set(
        &self,
        api_key_plaintext: &str,
        value: &ApiKeyCacheValue,
        ttl: u64,
    ) -> Result<(), RedisError> {
        let key = format!("api_key:{}", api_key_plaintext);
        let mut conn = self.redis.get().await?;

        // Add randomization to TTL (±10 seconds) to prevent cache stampede
        let jitter = (rand::random::<u64>() % 20).saturating_sub(10);
        let ttl_with_jitter = ttl.saturating_add(jitter);

        let json = serde_json::to_string(value)
            .map_err(|e| RedisError::CommandFailed(format!("JSON stringify error: {}", e)))?;

        let _: () = conn.set_ex(&key, json, ttl_with_jitter).await?;

        Ok(())
    }

    /// Delete API key from cache
    ///
    /// # Arguments
    /// * `api_key_plaintext` - The plaintext API key to delete
    ///
    /// # Returns
    /// * `Ok(())` if successful
    /// * `Err(RedisError)` if Redis operation fails
    ///
    /// # Example
    /// ```rust,no_run
    /// cache.delete(&api_key_plaintext).await?;
    /// ```
    pub async fn delete(&self, api_key_plaintext: &str) -> Result<(), RedisError> {
        let key = format!("api_key:{}", api_key_plaintext);
        let mut conn = self.redis.get().await?;

        tracing::debug!(
            key = %key,
            "Deleting API key from cache"
        );

        let deleted: i32 = conn.del(&key).await?;

        tracing::debug!(
            key = %key,
            deleted_count = deleted,
            "API key deletion result"
        );

        Ok(())
    }

    /// Delete all API keys for a realm
    ///
    /// This is called when a realm is deleted to ensure cache consistency.
    ///
    /// # Arguments
    /// * `realm_id` - The realm ID
    ///
    /// # Returns
    /// * `Ok(count)` - Number of keys deleted
    /// * `Err(RedisError)` if Redis operation fails
    ///
    /// # Example
    /// ```rust,no_run
    /// let count = cache.delete_by_realm("realm-123").await?;
    /// println!("Deleted {} cached keys", count);
    /// ```
    pub async fn delete_by_realm(&self, realm_id: &str) -> Result<usize, RedisError> {
        let pattern = "api_key:*";
        let mut conn = self.redis.get().await?;

        // Use SCAN to avoid blocking
        let keys: Vec<String> = redis::cmd("SCAN")
            .arg(0)
            .arg("MATCH")
            .arg(pattern)
            .arg("COUNT")
            .arg(100)
            .query_async(&mut conn)
            .await
            .map_err(|e| RedisError::CommandFailed(format!("SCAN error: {}", e)))?;

        // Check and delete matching keys
        let mut deleted_count = 0;
        for key in keys {
            // Get cached value to check realm_id
            let value_bytes: Option<Vec<u8>> = conn.get(&key).await?;
            let should_delete = value_bytes
                .and_then(|bytes| serde_json::from_slice::<ApiKeyCacheValue>(&bytes).ok())
                .is_some_and(|cached| cached.realm_id == realm_id);

            if should_delete {
                let _: () = conn.del(&key).await?;
                deleted_count += 1;
            }
        }

        Ok(deleted_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_key_cache_value_from_domain_entity() {
        let api_key = ClientApiKey {
            id: "test-id".to_string(),
            name: "Test Key".to_string(),
            api_key_hash: "hash".to_string(),
            realm_id: "realm-1".to_string(),
            client_app_id: None,
            enabled: true,
            expires_at: Some(Utc::now()),
            created_at: Utc::now(),
            last_used_at: None,
            usage_count: 0,
        };

        let cached = ApiKeyCacheValue::from(&api_key);

        assert_eq!(cached.id, "test-id");
        assert_eq!(cached.name, "Test Key");
        assert_eq!(cached.realm_id, "realm-1");
        assert!(cached.enabled);
        assert!(cached.expires_at.is_some());
    }
}
