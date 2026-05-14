//! Rate limiting utilities using Redis
//!
//! Provides thread-safe, atomic rate limiting using Redis Functions.
//! Redis Functions (Redis 7.0+) offer better performance and manageability
//! compared to traditional Lua scripts.
//!
//! Rate limiting can be disabled per-environment or per-request configuration.

use serde::{Deserialize, Serialize};

use crate::application::http::server::api_entities::ApiError;
use crate::application::http::state::AppState;

/// Rate limit configuration
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum number of requests allowed
    pub max_requests: i64,

    /// Time window in seconds
    pub window_secs: usize,

    /// Whether to enforce rate limiting in non-production environments
    /// When false (default), rate limiting is skipped in dev/test
    pub enforce_in_dev: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 10,
            window_secs: 60,
            enforce_in_dev: false,
        }
    }
}

impl RateLimitConfig {
    /// Create a new rate limit configuration
    pub fn new(max_requests: i64, window_secs: usize) -> Self {
        Self {
            max_requests,
            window_secs,
            enforce_in_dev: false,
        }
    }

    /// Enable enforcement in development/test environments
    pub fn with_enforce_in_dev(mut self) -> Self {
        self.enforce_in_dev = true;
        self
    }
}

/// Redis Function library name
const RATE_LIMIT_FUNCTION_LIBRARY: &str = "herald_rate_limit";

/// Redis Function for atomic increment and expiration
///
/// This function ensures that INCR and EXPIRE operations are atomic,
/// preventing race conditions where multiple requests might simultaneously
/// see count == 1 and compete to set expiration.
///
/// Redis Functions (introduced in Redis 7.0) offer several advantages over
/// traditional Lua scripts:
/// - Functions are loaded once and can be called multiple times
/// - Better performance due to persistent function library
/// - Built-in versioning and management via FUNCTION LIST/DELETE
/// - Easier to debug and maintain
const RATE_LIMIT_FUNCTION_CODE: &str = "#!lua name=herald_rate_limit\n\
\n\
local function rate_limit_check(keys, args)\n\
    local key = keys[1]\n\
    local limit = tonumber(args[1])\n\
    local window = tonumber(args[2])\n\
\n\
    local current = redis.call('incr', key)\n\
    if current == 1 then\n\
        redis.call('expire', key, window)\n\
    end\n\
\n\
    if current > limit then\n\
        return {0, current}\n\
    else\n\
        return {1, current}\n\
    end\n\
end\n\
\n\
redis.register_function('rate_limit_check', rate_limit_check)\n\
";

/// Initialize Redis Function library
///
/// This function loads the rate limiting function library into Redis.
/// It should be called during application startup.
///
/// # Returns
/// * `Ok(())` if the function library was loaded successfully
/// * `Err(ApiError)` if loading failed
///
/// # Note
/// This function is idempotent - calling it multiple times is safe.
/// Redis will replace the existing function library if it already exists.
pub async fn init_rate_limit_function(state: &AppState) -> Result<(), ApiError> {
    let mut conn = state
        .redis_manager
        .get()
        .await
        .map_err(|_| ApiError::internal("Internal server error"))?;

    // Load the function library using FUNCTION LOAD
    // The 'REPLACE' flag ensures we can update the function if it already exists
    redis::cmd("FUNCTION")
        .arg("LOAD")
        .arg("REPLACE")
        .arg(RATE_LIMIT_FUNCTION_CODE)
        .query_async::<String>(&mut conn)
        .await
        .map_err(|e| {
            tracing::error!("Failed to load rate limit function library: {e}");
            ApiError::internal("Internal server error")
        })?;

    tracing::info!(
        "Redis Function library '{}' loaded successfully",
        RATE_LIMIT_FUNCTION_LIBRARY
    );

    Ok(())
}

/// Check if a rate limit should be enforced
///
/// Returns an error if the rate limit has been exceeded.
/// Rate limiting is automatically disabled in non-production environments
/// unless `config.enforce_in_dev` is set to true.
///
/// This function uses Redis Functions (FCALL) for better performance
/// compared to traditional Lua scripts.
///
/// # Arguments
/// * `state` - Application state containing Redis client
/// * `key` - Unique key for rate limiting (e.g., "rl:login:ip:1.2.3.4")
/// * `config` - Rate limit configuration
///
/// # Example
/// ```no_run
/// use herald_api::application::http::rate_limit::{rate_limit, RateLimitConfig};
///
/// let config = RateLimitConfig::new(5, 60); // 5 requests per 60 seconds
/// rate_limit(&state, "rl:myfeature:user:123".to_string(), config).await?;
/// ```
pub async fn rate_limit(
    state: &AppState,
    key: String,
    config: RateLimitConfig,
) -> Result<(), ApiError> {
    // Rate limiting is enforced by default, but can be disabled for local testing
    if !config.enforce_in_dev {
        return Ok(());
    }

    let mut conn = state
        .redis_manager
        .get()
        .await
        .map_err(|_| ApiError::internal("Internal server error"))?;

    // Execute Redis Function using FCALL
    // FCALL function_name num_keys key1 [key2 ...] arg1 [arg2 ...]
    let result: (i64, i64) = redis::cmd("FCALL")
        .arg("rate_limit_check")
        .arg(1) // number of keys
        .arg(&key)
        .arg(config.max_requests)
        .arg(config.window_secs)
        .query_async(&mut conn)
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute rate limit function: {e}");
            ApiError::internal("Internal server error")
        })?;

    let (allowed, current_count) = result;

    if allowed == 0 {
        tracing::warn!(
            "Rate limit exceeded for key '{}': {} requests (limit: {})",
            key,
            current_count,
            config.max_requests
        );
        return Err(ApiError::too_many_requests(format!(
            "Rate limit exceeded: {} requests per {} seconds",
            config.max_requests, config.window_secs
        )));
    }

    tracing::debug!(
        "Rate limit check passed for key '{}': {}/{} requests",
        key,
        current_count,
        config.max_requests
    );

    Ok(())
}

/// Check if a rate limit should be enforced (simplified interface)
///
/// This is a simplified version that uses default config.
/// Use `rate_limit` for more control.
///
/// # Arguments
/// * `state` - Application state
/// * `key` - Unique key for rate limiting
/// * `limit` - Maximum number of requests
/// * `window_secs` - Time window in seconds
pub async fn rate_limit_hit(
    state: &AppState,
    key: String,
    limit: i64,
    window_secs: usize,
) -> Result<(), ApiError> {
    rate_limit(state, key, RateLimitConfig::new(limit, window_secs)).await
}

/// Rate limit function that enforces limits even in development/test environments
///
/// This is primarily intended for testing purposes.
///
/// # Arguments
/// * `state` - Application state
/// * `key` - Unique key for rate limiting
/// * `limit` - Maximum number of requests
/// * `window_secs` - Time window in seconds
pub async fn rate_limit_hit_forced(
    state: &AppState,
    key: String,
    limit: i64,
    window_secs: usize,
) -> Result<(), ApiError> {
    rate_limit(
        state,
        key,
        RateLimitConfig::new(limit, window_secs).with_enforce_in_dev(),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_config_builder() {
        let config = RateLimitConfig::new(5, 120).with_enforce_in_dev();
        assert_eq!(config.max_requests, 5);
        assert_eq!(config.window_secs, 120);
        assert!(config.enforce_in_dev);
    }
}
