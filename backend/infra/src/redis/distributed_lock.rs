// Distributed Lock Service - Infrastructure Layer
//
// Provides distributed locking using Redis for concurrent operation control
// Prevents race conditions in multi-instance deployments

use std::sync::Arc;
use std::time::Duration;

use crate::redis::RedisConnectionManager;
use herald_domain::common::entities::app_errors::CoreError;

const DEFAULT_TTL: Duration = Duration::from_secs(30); // 30 second default lock timeout
const LOCK_RETRY_DELAY: Duration = Duration::from_millis(50); // 50ms between retries

/// Distributed Lock
///
/// Uses Redis SET NX EX pattern for atomic lock acquisition
/// Supports automatic release via Lua script
pub struct DistributedLock {
    redis: Arc<RedisConnectionManager>,
}

impl DistributedLock {
    pub fn new(redis: Arc<RedisConnectionManager>) -> Self {
        Self { redis }
    }

    /// Helper method to get a Redis connection
    async fn get_conn(&self) -> Result<redis::aio::ConnectionManager, CoreError> {
        self.redis
            .get()
            .await
            .map_err(|e| CoreError::DatabaseError(format!("Failed to get Redis connection: {}", e)))
    }

    /// Try to acquire a lock for the given key
    ///
    /// Returns None if the lock cannot be acquired (already held)
    /// Returns Some(LockGuard) if the lock is acquired
    pub async fn try_acquire(
        &self,
        key: &str,
        ttl: Option<Duration>,
    ) -> Result<Option<LockGuard>, CoreError> {
        let lock_ttl = ttl.unwrap_or(DEFAULT_TTL);
        let lock_value = uuid::Uuid::now_v7().to_string(); // Unique value for this lock attempt

        let mut conn = self.get_conn().await?;

        // Use SET NX EX for atomic lock acquisition
        let acquired: bool = redis::cmd("SET")
            .arg(key)
            .arg(&lock_value)
            .arg("NX")
            .arg("EX")
            .arg(lock_ttl.as_secs())
            .query_async(&mut conn)
            .await
            .map_err(|e| CoreError::DatabaseError(format!("Failed to acquire lock: {}", e)))?;

        if acquired {
            tracing::debug!(key = %key, ttl_secs = lock_ttl.as_secs(), "Distributed lock acquired");
            Ok(Some(LockGuard::new(
                Arc::clone(&self.redis),
                key.to_string(),
                lock_value,
            )))
        } else {
            tracing::debug!(key = %key, "Distributed lock already held");
            Ok(None)
        }
    }

    /// Acquire a lock with automatic retries
    ///
    /// Will retry up to max_retries times with retry_delay between attempts
    pub async fn acquire_with_retry(
        &self,
        key: &str,
        ttl: Option<Duration>,
        max_retries: usize,
        retry_delay: Option<Duration>,
    ) -> Result<LockGuard, CoreError> {
        let delay = retry_delay.unwrap_or(LOCK_RETRY_DELAY);

        for attempt in 1..=max_retries {
            match self.try_acquire(key, ttl).await? {
                Some(guard) => {
                    if attempt > 1 {
                        tracing::info!(
                            key = %key,
                            attempt,
                            "Distributed lock acquired after retries"
                        );
                    }
                    return Ok(guard);
                }
                None => {
                    if attempt == max_retries {
                        tracing::warn!(
                            key = %key,
                            max_retries,
                            "Failed to acquire distributed lock after retries"
                        );
                        return Err(CoreError::Conflict(format!(
                            "Failed to acquire lock for key '{}' after {} attempts",
                            key, max_retries
                        )));
                    }

                    tracing::debug!(
                        key = %key,
                        attempt,
                        "Lock acquisition failed, retrying..."
                    );

                    tokio::time::sleep(delay).await;
                }
            }
        }

        // This should never be reached, but for type safety
        Err(CoreError::Conflict(format!(
            "Failed to acquire lock for key '{}'",
            key
        )))
    }
}

/// Lock Guard that automatically releases the lock when dropped
///
/// Uses a Lua script for atomic check-and-release to prevent
/// accidentally releasing someone else's lock
pub struct LockGuard {
    redis: Arc<RedisConnectionManager>,
    key: String,
    value: String,
}

impl LockGuard {
    fn new(redis: Arc<RedisConnectionManager>, key: String, value: String) -> Self {
        Self { redis, key, value }
    }

    /// Manually release the lock
    ///
    /// Returns true if the lock was released, false if it was already released
    /// or held by a different process
    pub async fn release(&mut self) -> Result<bool, CoreError> {
        let mut conn = self.redis.get().await.map_err(|e| {
            CoreError::DatabaseError(format!("Failed to get Redis connection: {}", e))
        })?;

        // Lua script for atomic check-and-release
        // Only releases if the key still exists and has the same value
        let lua_script = r#"
            if redis.call("GET", KEYS[1]) == ARGV[1] then
                return redis.call("DEL", KEYS[1])
            else
                return 0
            end
        "#;

        let released: u32 = redis::cmd("EVAL")
            .arg(lua_script)
            .arg(1) // Number of keys
            .arg(&self.key)
            .arg(&self.value)
            .query_async(&mut conn)
            .await
            .map_err(|e| CoreError::DatabaseError(format!("Failed to release lock: {}", e)))?;

        if released > 0 {
            tracing::debug!(key = %self.key, "Distributed lock released");
            Ok(true)
        } else {
            tracing::debug!(key = %self.key, "Lock already released or held by another process");
            Ok(false)
        }
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        // Note: Drop is synchronous, but release() is async
        // We use a blocking task for cleanup, but it's a best-effort operation
        // If it fails, Redis will expire the lock naturally based on TTL
        let redis = Arc::clone(&self.redis);
        let key = self.key.clone();
        let value = self.value.clone();

        // Spawn a background task to release the lock
        tokio::spawn(async move {
            // Get connection gracefully, never panic in Drop
            if let Ok(mut conn) = redis.get().await {
                // Use Lua script for atomic check-and-release (same as release())
                let lua_script = r#"
                    if redis.call("GET", KEYS[1]) == ARGV[1] then
                        return redis.call("DEL", KEYS[1])
                    else
                        return 0
                    end
                "#;

                if let Err(e) = redis::cmd("EVAL")
                    .arg(lua_script)
                    .arg(1)
                    .arg(&key)
                    .arg(&value)
                    .query_async::<()>(&mut conn)
                    .await
                {
                    tracing::warn!(
                        key = %key,
                        error = %e,
                        "Failed to release lock in Drop, will expire naturally"
                    );
                }
            } else {
                tracing::warn!(
                    key = %key,
                    "Failed to get Redis connection in Drop, lock will expire naturally"
                );
            }
        });
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_lock_guard_creation() {
        // This is a placeholder test
        // Real tests would require Redis instance
    }
}
