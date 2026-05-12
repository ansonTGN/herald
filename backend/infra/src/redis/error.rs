// =============================================================================
// Redis Error Types - Infrastructure Layer
// =============================================================================
//
// Unified error types for Redis operations
//
// =============================================================================

use thiserror::Error;

/// Redis 错误类型
#[derive(Debug, Error)]
pub enum RedisError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Command failed: {0}")]
    CommandFailed(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

impl From<redis::RedisError> for RedisError {
    fn from(err: redis::RedisError) -> Self {
        // Simplified error conversion - redis-rs 1.0+ has different error kinds
        RedisError::CommandFailed(err.to_string())
    }
}
