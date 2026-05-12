// =============================================================================
// Redis Connection Manager - Infrastructure Layer
// =============================================================================
//
// Wraps redis::aio::ConnectionManager with automatic DB selection
// Provides elegant test isolation using separate Redis DB
//
// =============================================================================

use redis::aio::ConnectionManager;

use super::error::RedisError;

/// Redis 连接管理器配置
#[derive(Clone, Debug)]
pub struct ManagerConfig {
    /// Redis 连接 URL
    pub url: String,
    /// 默认 DB 编号（默认 0）
    pub default_db: u8,
    /// 测试模式（使用不同 DB 隔离）
    pub test_mode: bool,
    /// 测试 DB 编号（默认 1，测试环境使用 DB 1）
    pub test_db: u8,
}

impl Default for ManagerConfig {
    fn default() -> Self {
        Self {
            url: "redis://127.0.0.1:6379/0".to_string(),
            default_db: 0,
            test_mode: false,
            test_db: 1,
        }
    }
}

/// Redis 连接管理器（包装器）
///
/// 基于 redis::aio::ConnectionManager，添加 DB 选择和测试隔离功能
#[derive(Debug)]
pub struct RedisConnectionManager {
    manager: ConnectionManager,
    config: ManagerConfig,
}

impl RedisConnectionManager {
    /// 创建新的连接管理器
    pub async fn new(config: ManagerConfig) -> Result<Self, RedisError> {
        let client = redis::Client::open(config.url.clone())
            .map_err(|e| RedisError::ConnectionFailed(e.to_string()))?;

        let manager = ConnectionManager::new(client)
            .await
            .map_err(|e| RedisError::ConnectionFailed(e.to_string()))?;

        // 健康检查
        let mut conn = manager.clone();
        redis::cmd("PING")
            .query_async::<String>(&mut conn)
            .await
            .map_err(|e| RedisError::CommandFailed(e.to_string()))?;

        Ok(Self { manager, config })
    }

    /// 获取连接（自动选择 DB）
    ///
    /// - 生产环境：使用 default_db（默认 0）
    /// - 测试环境：使用 test_db（默认 1）
    pub async fn get(&self) -> Result<ConnectionManager, RedisError> {
        let target_db = if self.config.test_mode {
            self.config.test_db
        } else {
            self.config.default_db
        };

        let mut conn = self.manager.clone();
        redis::cmd("SELECT")
            .arg(target_db)
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| RedisError::CommandFailed(e.to_string()))?;

        Ok(conn)
    }

    /// 获取连接（指定 DB）
    pub async fn get_with_db(&self, db: u8) -> Result<ConnectionManager, RedisError> {
        let mut conn = self.manager.clone();
        redis::cmd("SELECT")
            .arg(db)
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| RedisError::CommandFailed(e.to_string()))?;

        Ok(conn)
    }

    /// 健康检查
    pub async fn health_check(&self) -> Result<(), RedisError> {
        let mut conn = self.manager.clone();
        redis::cmd("PING")
            .query_async::<String>(&mut conn)
            .await
            .map_err(|e| RedisError::CommandFailed(e.to_string()))?;
        Ok(())
    }

    /// 获取配置
    pub fn config(&self) -> &ManagerConfig {
        &self.config
    }
}

impl Clone for RedisConnectionManager {
    fn clone(&self) -> Self {
        Self {
            manager: self.manager.clone(),
            config: self.config.clone(),
        }
    }
}

// Tests are in manager_test.rs to allow for comprehensive testing
#[path = "manager_test.rs"]
#[cfg(test)]
mod tests;
