// =============================================================================
// Redis Cache Implementation - Infrastructure Layer
// =============================================================================
//
// Provides caching layer for permission checking system
// Implements generic get/set operations with TTL support
//
// =============================================================================

use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

use crate::redis::RedisConnectionManager;

/// Redis cache for permission checking system
#[derive(Debug)]
pub struct RedisCache {
    manager: RedisConnectionManager,
}

impl RedisCache {
    /// Create a new Redis cache instance
    ///
    /// # Arguments
    /// * `manager` - Redis connection manager
    pub fn new(
        manager: RedisConnectionManager,
    ) -> Result<Self, herald_domain::common::entities::app_errors::CoreError> {
        Ok(Self { manager })
    }

    /// Get a Redis connection with error handling
    ///
    /// # Returns
    /// * `Ok(ConnectionManager)` if connection succeeds
    /// * `Err(CoreError)` if connection fails
    async fn get_connection(
        &self,
    ) -> Result<redis::aio::ConnectionManager, herald_domain::common::entities::app_errors::CoreError>
    {
        self.manager.get().await.map_err(|e| {
            tracing::error!(error = %e, "Failed to get Redis connection");
            herald_domain::common::entities::app_errors::CoreError::InternalServerError(format!(
                "Redis connection error: {}",
                e
            ))
        })
    }

    /// Get a cached value
    ///
    /// # Type Parameters
    /// * `T` - Type to deserialize into (must implement Deserialize)
    ///
    /// # Returns
    /// * `Ok(Some(T))` if value exists in cache
    /// * `Ok(None)` if key doesn't exist or Redis is unavailable
    /// * `Err(CoreError)` if deserialization fails
    pub async fn get<T>(
        &self,
        key: &str,
    ) -> Result<Option<T>, herald_domain::common::entities::app_errors::CoreError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let mut conn = match self.get_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                tracing::warn!(error = %e, key = %key, "Redis unavailable, returning cache miss");
                return Ok(None);
            }
        };

        let value: Option<String> = match AsyncCommands::get(&mut conn, key).await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %e, key = %key, "Failed to get value from Redis, returning cache miss");
                return Ok(None);
            }
        };

        match value {
            Some(v) => {
                let decoded = serde_json::from_str(&v).map_err(|e| {
                    tracing::error!(error = %e, key = %key, "Failed to deserialize cached value");
                    herald_domain::common::entities::app_errors::CoreError::InternalServerError(
                        format!("Deserialization error: {}", e),
                    )
                })?;
                Ok(Some(decoded))
            }
            None => Ok(None),
        }
    }

    /// Set a cached value with TTL
    ///
    /// # Type Parameters
    /// * `T` - Type to serialize (must implement Serialize)
    ///
    /// # Arguments
    /// * `key` - Cache key
    /// * `value` - Value to cache
    /// * `ttl` - Time to live in seconds
    pub async fn set<T>(
        &self,
        key: &str,
        value: &T,
        ttl: u64,
    ) -> Result<(), herald_domain::common::entities::app_errors::CoreError>
    where
        T: Serialize,
    {
        let mut conn = match self.get_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                tracing::warn!(error = %e, key = %key, "Redis unavailable, skipping cache write");
                return Ok(()); // Return Ok to allow operation to continue without cache
            }
        };

        let serialized = serde_json::to_string(value).map_err(|e| {
            tracing::error!(error = %e, key = %key, "Failed to serialize value");
            herald_domain::common::entities::app_errors::CoreError::InternalServerError(format!(
                "Serialization error: {}",
                e
            ))
        })?;

        // set_ex returns (), use turbofish syntax for explicit type
        match AsyncCommands::set_ex::<_, _, ()>(&mut conn, key, serialized, ttl).await {
            Ok(_) => {
                tracing::debug!(key = %key, ttl = ttl, "Cached value");
                Ok(())
            }
            Err(e) => {
                tracing::warn!(error = %e, key = %key, "Failed to set value in Redis, continuing without cache");
                Ok(()) // Return Ok to allow operation to continue without cache
            }
        }
    }

    /// Delete a cached value
    ///
    /// # Arguments
    /// * `key` - Cache key to delete
    pub async fn delete(
        &self,
        key: &str,
    ) -> Result<(), herald_domain::common::entities::app_errors::CoreError> {
        let mut conn = match self.get_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                tracing::warn!(error = %e, key = %key, "Redis unavailable, skipping cache delete");
                return Ok(()); // Return Ok to allow operation to continue
            }
        };

        // del returns usize, but we don't need the count
        match AsyncCommands::del::<&str, usize>(&mut conn, key).await {
            Ok(_) => {
                tracing::debug!(key = %key, "Deleted cached value");
                Ok(())
            }
            Err(e) => {
                tracing::warn!(error = %e, key = %key, "Failed to delete value from Redis, continuing");
                Ok(())
            }
        }
    }

    /// Delete multiple keys matching a pattern
    ///
    /// # Arguments
    /// * `pattern` - Key pattern (e.g., "user_roles:realm1:*")
    ///
    /// # Notes
    /// Uses KEYS command which is O(N) - use with caution in production
    /// For large datasets, consider using SCAN with cursor
    pub async fn delete_pattern(
        &self,
        pattern: &str,
    ) -> Result<(), herald_domain::common::entities::app_errors::CoreError> {
        let mut conn = match self.get_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                tracing::warn!(error = %e, pattern = %pattern, "Redis unavailable, skipping cache delete");
                return Ok(()); // Return Ok to allow operation to continue
            }
        };

        // Use KEYS to find matching keys
        let keys: Vec<String> = match redis::cmd("KEYS").arg(pattern).query_async(&mut conn).await {
            Ok(keys) => keys,
            Err(e) => {
                tracing::warn!(error = %e, pattern = %pattern, "Failed to execute KEYS command, continuing");
                return Ok(());
            }
        };

        if !keys.is_empty() {
            // del returns usize (count of deleted keys), but we don't need it
            match AsyncCommands::del::<&Vec<String>, usize>(&mut conn, &keys).await {
                Ok(_) => {
                    tracing::debug!(pattern = %pattern, count = keys.len(), "Deleted cached values");
                }
                Err(e) => {
                    tracing::warn!(error = %e, pattern = %pattern, "Failed to delete keys, continuing");
                }
            }
        } else {
            tracing::debug!(pattern = %pattern, "No keys matched pattern");
        }

        Ok(())
    }
}

// Note: Comprehensive tests for RedisCache are now in:
// - backend/core/src/tests/scenarios/redis_permission_cache_test.rs
//
// The old inline tests have been removed in favor of scenario-based testing
