// Redis Idempotency Store - Infrastructure Layer
//
// Redis implementation of the IdempotencyStore trait.
// This provides the concrete storage backend for idempotency keys.

use std::sync::Arc;
use std::time::Duration;

use crate::redis::RedisConnectionManager;
use herald_domain::common::entities::app_errors::CoreError;
use herald_domain::points::entities::{IdempotencyStatus, PointsTransaction};
use herald_domain::points::idempotency_service::IdempotencyStore;

const IDEMPOTENCY_TTL: Duration = Duration::from_secs(24 * 60 * 60); // 24 hours
const LOCK_TTL: Duration = Duration::from_secs(60); // 1 minute lock timeout

/// Redis implementation of IdempotencyStore
pub struct RedisIdempotencyStore {
    redis: Arc<RedisConnectionManager>,
}

impl RedisIdempotencyStore {
    pub fn new(redis: Arc<RedisConnectionManager>) -> Self {
        Self { redis }
    }
}

impl IdempotencyStore for RedisIdempotencyStore {
    fn get_from_cache(
        &self,
        cache_key: &str,
    ) -> impl Future<Output = Option<PointsTransaction>> + Send {
        let redis = self.redis.clone();
        let cache_key = cache_key.to_string();

        async move {
            let mut conn = redis.get().await.ok()?;

            let result: Option<String> = redis::cmd("GET")
                .arg(&cache_key)
                .query_async(&mut conn)
                .await
                .ok()?;

            result.and_then(|data| serde_json::from_str(&data).ok())
        }
    }

    fn get_status_from_cache(
        &self,
        cache_key: &str,
    ) -> impl Future<Output = Option<IdempotencyStatus>> + Send {
        let redis = self.redis.clone();
        let cache_key = cache_key.to_string();

        async move {
            let mut conn = redis.get().await.ok()?;

            let result: Option<String> = redis::cmd("GET")
                .arg(format!("{}:status", &cache_key))
                .query_async(&mut conn)
                .await
                .ok()?;

            result.and_then(|status| status.parse().ok())
        }
    }

    fn try_create_lock(
        &self,
        cache_key: &str,
        request_data: &str,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send {
        let redis = self.redis.clone();
        let cache_key = cache_key.to_string();
        let request_data = request_data.to_string();

        async move {
            let mut conn = redis.get().await.map_err(|e| {
                CoreError::DatabaseError(format!("Failed to get Redis connection: {}", e))
            })?;

            // Use SET NX EX for atomic lock acquisition
            let result: bool = redis::cmd("SET")
                .arg(&cache_key)
                .arg(&request_data)
                .arg("NX")
                .arg("EX")
                .arg(IDEMPOTENCY_TTL.as_secs())
                .query_async(&mut conn)
                .await
                .map_err(|e| CoreError::DatabaseError(format!("Failed to create lock: {}", e)))?;

            if result {
                // Also set status to processing
                let status = IdempotencyStatus::Processing.as_str();
                redis::cmd("SETEX")
                    .arg(format!("{}:status", &cache_key))
                    .arg(LOCK_TTL.as_secs())
                    .arg(status)
                    .query_async::<()>(&mut conn)
                    .await
                    .map_err(|e| {
                        CoreError::DatabaseError(format!("Failed to set status: {}", e))
                    })?;
            }

            Ok(result)
        }
    }

    fn save_to_cache(
        &self,
        cache_key: &str,
        transaction: &PointsTransaction,
    ) -> impl Future<Output = Result<(), CoreError>> + Send {
        let redis = self.redis.clone();
        let cache_key = cache_key.to_string();
        let transaction = transaction.clone();

        async move {
            let mut conn = redis.get().await.map_err(|e| {
                CoreError::DatabaseError(format!("Failed to get Redis connection: {}", e))
            })?;

            let data = serde_json::to_string(&transaction).map_err(|e| {
                CoreError::BadRequest(format!("Failed to serialize transaction: {}", e))
            })?;

            // Cache the transaction data
            redis::cmd("SETEX")
                .arg(&cache_key)
                .arg(IDEMPOTENCY_TTL.as_secs())
                .arg(&data)
                .query_async::<()>(&mut conn)
                .await
                .map_err(|e| CoreError::DatabaseError(format!("Failed to save to cache: {}", e)))?;

            // Update status to completed
            let status = IdempotencyStatus::Completed.as_str();
            redis::cmd("SETEX")
                .arg(format!("{}:status", &cache_key))
                .arg(IDEMPOTENCY_TTL.as_secs())
                .arg(status)
                .query_async::<()>(&mut conn)
                .await
                .map_err(|e| CoreError::DatabaseError(format!("Failed to update status: {}", e)))?;

            Ok(())
        }
    }

    fn mark_failed(&self, cache_key: &str) -> impl Future<Output = Result<(), CoreError>> + Send {
        let redis = self.redis.clone();
        let cache_key = cache_key.to_string();

        async move {
            let mut conn = redis.get().await.map_err(|e| {
                CoreError::DatabaseError(format!("Failed to get Redis connection: {}", e))
            })?;

            let status = IdempotencyStatus::Failed.as_str();
            let key = format!("{}:status", &cache_key);

            redis::cmd("SETEX")
                .arg(key)
                .arg(LOCK_TTL.as_secs())
                .arg(status)
                .query_async::<()>(&mut conn)
                .await
                .map_err(|e| CoreError::DatabaseError(format!("Failed to mark failed: {}", e)))?;

            Ok(())
        }
    }
}
