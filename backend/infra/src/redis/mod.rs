// =============================================================================
// Redis Connection Management Module - Infrastructure Layer
// =============================================================================
//
// Provides unified Redis connection management with DB isolation for testing
// Wraps redis::aio::ConnectionManager with automatic DB selection
//
// =============================================================================

pub mod distributed_lock;
pub mod error;
pub mod manager;

pub use distributed_lock::{DistributedLock, LockGuard};
pub use error::RedisError;
pub use manager::{ManagerConfig, RedisConnectionManager};
