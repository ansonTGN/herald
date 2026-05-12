// =============================================================================
// Redis Connection Manager Tests - Infrastructure Layer
// =============================================================================
//
// Unit tests for Redis connection manager functionality
//
// =============================================================================

use super::*;
use redis::AsyncCommands;
use tokio;
use tokio::sync::OnceCell;

/// Shared Redis connection manager for tests
/// Created once and reused across all tests to avoid connection overhead
static SHARED_MANAGER: OnceCell<Option<RedisConnectionManager>> = OnceCell::const_new();

/// Initialize the shared Redis manager
async fn init_shared_manager() -> Option<RedisConnectionManager> {
    let config = ManagerConfig::default();
    let manager = RedisConnectionManager::new(config).await;

    // Return None if Redis is not available, allowing tests to skip
    match manager {
        Ok(m) => {
            tracing::info!("✅ Shared Redis manager initialized");
            Some(m)
        }
        Err(e) => {
            tracing::warn!("⚠️ Redis not available: {}", e);
            None
        }
    }
}

/// Get or create the shared Redis manager
async fn get_shared_manager() -> Option<&'static RedisConnectionManager> {
    SHARED_MANAGER
        .get_or_init(|| async { init_shared_manager().await })
        .await
        .as_ref()
}

/// Test manager creation with default config
#[tokio::test]
async fn test_manager_creation_default_config() {
    let config = ManagerConfig::default();
    let manager = RedisConnectionManager::new(config).await;

    // Skip test if Redis is not available
    if manager.is_err() {
        println!("Redis not available, skipping test");
        return;
    }

    assert!(manager.is_ok());
}

/// Test manager creation with custom config
#[tokio::test]
async fn test_manager_creation_custom_config() {
    let config = ManagerConfig {
        url: "redis://127.0.0.1:6379/0".to_string(),
        default_db: 0,
        test_mode: false,
        test_db: 1,
    };
    let manager = RedisConnectionManager::new(config).await;

    // Skip test if Redis is not available
    if manager.is_err() {
        println!("Redis not available, skipping test");
        return;
    }

    assert!(manager.is_ok());
}

/// Test manager creation with test mode config
#[tokio::test]
async fn test_manager_creation_test_mode_config() {
    let config = ManagerConfig {
        url: "redis://127.0.0.1:6379/0".to_string(),
        default_db: 0,
        test_mode: true,
        test_db: 1,
    };
    let manager = RedisConnectionManager::new(config).await;

    // Skip test if Redis is not available
    if manager.is_err() {
        println!("Redis not available, skipping test");
        return;
    }

    assert!(manager.is_ok());
    let manager = manager.unwrap();

    // Verify config is stored correctly
    assert!(manager.config().test_mode);
    assert_eq!(manager.config().test_db, 1);
}

/// Test get connection in production mode
#[tokio::test]
async fn test_get_connection_production() {
    let config = ManagerConfig {
        url: "redis://127.0.0.1:6379/0".to_string(),
        default_db: 0,
        test_mode: false,
        test_db: 1,
    };
    let manager = RedisConnectionManager::new(config).await;

    // Skip test if Redis is not available
    if manager.is_err() {
        println!("Redis not available, skipping test");
        return;
    }

    let manager = manager.unwrap();
    let conn = manager.get().await;

    assert!(conn.is_ok());
    // ConnectionManager doesn't have db() method, but we know it's DB 0 from config
}

/// Test get connection in test mode
#[tokio::test]
async fn test_get_connection_test_mode() {
    let config = ManagerConfig {
        url: "redis://127.0.0.1:6379/0".to_string(),
        default_db: 0,
        test_mode: true,
        test_db: 1,
    };
    let manager = RedisConnectionManager::new(config).await;

    // Skip test if Redis is not available
    if manager.is_err() {
        println!("Redis not available, skipping test");
        return;
    }

    let manager = manager.unwrap();
    let conn = manager.get().await;

    assert!(conn.is_ok());
    // ConnectionManager doesn't have db() method, but we know it's DB 1 from config
}

/// Test get connection with specific DB
#[tokio::test]
async fn test_get_connection_with_specific_db() {
    let config = ManagerConfig::default();
    let manager = RedisConnectionManager::new(config).await;

    // Skip test if Redis is not available
    if manager.is_err() {
        println!("Redis not available, skipping test");
        return;
    }

    let manager = manager.unwrap();

    // Test DB 5
    let conn = manager.get_with_db(5).await;
    assert!(conn.is_ok());
    // We requested DB 5

    // Test DB 10
    let conn = manager.get_with_db(10).await;
    assert!(conn.is_ok());
    // We requested DB 10
}

/// Test health check (uses shared manager)
#[tokio::test]
async fn test_health_check() {
    let manager = match get_shared_manager().await {
        Some(m) => m,
        None => {
            println!("Redis not available, skipping test");
            return;
        }
    };

    let result = manager.health_check().await;
    assert!(result.is_ok());
}

/// Test basic SET/GET operations (uses shared manager)
#[tokio::test]
async fn test_basic_set_get() {
    let manager = match get_shared_manager().await {
        Some(m) => m,
        None => {
            println!("Redis not available, skipping test");
            return;
        }
    };

    let mut conn = manager.get().await.unwrap();

    // SET
    let _: () = conn.set("test_key", "test_value").await.unwrap();

    // GET
    let value: String = conn.get("test_key").await.unwrap();
    assert_eq!(value, "test_value");

    // Cleanup
    let _: () = conn.del("test_key").await.unwrap();
}

/// Test basic SETEX (SET with expiry) (uses shared manager)
#[tokio::test]
async fn test_basic_setex() {
    let manager = match get_shared_manager().await {
        Some(m) => m,
        None => {
            println!("Redis not available, skipping test");
            return;
        }
    };

    let mut conn = manager.get().await.unwrap();

    // SETEX with 2 seconds TTL
    let _: () = conn
        .set_ex("test_key_ex", "test_value_ex", 2)
        .await
        .unwrap();

    // GET immediately
    let value: String = conn.get("test_key_ex").await.unwrap();
    assert_eq!(value, "test_value_ex");

    // Cleanup
    let _: () = conn.del("test_key_ex").await.unwrap();
}

/// Test basic DEL operation (uses shared manager)
#[tokio::test]
async fn test_basic_del() {
    let manager = match get_shared_manager().await {
        Some(m) => m,
        None => {
            println!("Redis not available, skipping test");
            return;
        }
    };

    let mut conn = manager.get().await.unwrap();

    // SET
    let _: () = conn.set("test_key_del", "test_value_del").await.unwrap();

    // Verify exists
    let value: Option<String> = conn.get("test_key_del").await.unwrap();
    assert!(value.is_some());

    // DEL
    let _: () = conn.del("test_key_del").await.unwrap();

    // Verify deleted
    let value: Option<String> = conn.get("test_key_del").await.unwrap();
    assert!(value.is_none());
}

/// Test GET of non-existent key returns None (uses shared manager)
#[tokio::test]
async fn test_get_nonexistent_key() {
    let manager = match get_shared_manager().await {
        Some(m) => m,
        None => {
            println!("Redis not available, skipping test");
            return;
        }
    };

    let mut conn = manager.get().await.unwrap();

    // GET non-existent key
    let value: Option<String> = conn.get("nonexistent_key_xyz123").await.unwrap();
    assert!(value.is_none());
}

/// Test DB isolation between production and test environments
#[tokio::test]
async fn test_db_isolation() {
    // Production environment (DB 0)
    let prod_config = ManagerConfig {
        url: "redis://127.0.0.1:6379/0".to_string(),
        default_db: 0,
        test_mode: false,
        test_db: 1,
    };
    let prod_manager = RedisConnectionManager::new(prod_config).await;

    // Test environment (DB 1)
    let test_config = ManagerConfig {
        url: "redis://127.0.0.1:6379/0".to_string(),
        default_db: 0,
        test_mode: true,
        test_db: 1,
    };
    let test_manager = RedisConnectionManager::new(test_config).await;

    // Skip test if Redis is not available
    if prod_manager.is_err() || test_manager.is_err() {
        println!("Redis not available, skipping test");
        return;
    }

    let prod_manager = prod_manager.unwrap();
    let test_manager = test_manager.unwrap();

    // Write to DB 0
    let mut prod_conn = prod_manager.get().await.unwrap();
    let _: () = prod_conn.set("isolation_key", "prod_value").await.unwrap();

    // Write to DB 1 with same key
    let mut test_conn = test_manager.get().await.unwrap();
    let _: () = test_conn.set("isolation_key", "test_value").await.unwrap();

    // Verify DB 0 data
    let prod_value: String = prod_conn.get("isolation_key").await.unwrap();
    assert_eq!(prod_value, "prod_value");

    // Verify DB 1 data
    let test_value: String = test_conn.get("isolation_key").await.unwrap();
    assert_eq!(test_value, "test_value");

    // Cleanup
    let _: () = prod_conn.del("isolation_key").await.unwrap();
    let _: () = test_conn.del("isolation_key").await.unwrap();
}

/// Test manager clone functionality
#[tokio::test]
async fn test_manager_clone() {
    let config = ManagerConfig::default();
    let manager1 = RedisConnectionManager::new(config).await;

    // Skip test if Redis is not available
    if manager1.is_err() {
        println!("Redis not available, skipping test");
        return;
    }

    let manager1 = manager1.unwrap();
    let manager2 = manager1.clone();

    // Both managers should work
    let conn1 = manager1.get().await;
    let conn2 = manager2.get().await;

    assert!(conn1.is_ok());
    assert!(conn2.is_ok());

    let mut conn1 = conn1.unwrap();
    let mut conn2 = conn2.unwrap();

    // Write using first connection
    let _: () = conn1.set("clone_test", "value1").await.unwrap();

    // Write using second connection (same underlying Redis)
    let _: () = conn2.set("clone_test", "value2").await.unwrap();

    // Read back - should get the last written value
    let value: String = conn1.get("clone_test").await.unwrap();
    assert_eq!(value, "value2");

    // Cleanup
    let _: () = conn1.del("clone_test").await.unwrap();
}

/// Test multiple connections from same manager (uses shared manager)
#[tokio::test]
async fn test_multiple_connections() {
    let manager = match get_shared_manager().await {
        Some(m) => m,
        None => {
            println!("Redis not available, skipping test");
            return;
        }
    };

    // Get multiple connections
    let conn1 = manager.get().await;
    let conn2 = manager.get().await;
    let conn3 = manager.get().await;

    assert!(conn1.is_ok());
    assert!(conn2.is_ok());
    assert!(conn3.is_ok());
}

/// Test connection retrieval after manager clone
#[tokio::test]
async fn test_manager_clone_connection_retrieval() {
    let config = ManagerConfig::default();
    let manager1 = RedisConnectionManager::new(config).await;

    // Skip test if Redis is not available
    if manager1.is_err() {
        println!("Redis not available, skipping test");
        return;
    }

    let manager1 = manager1.unwrap();
    let manager2 = manager1.clone();

    // Both managers should be able to get connections
    let conn1 = manager1.get().await.unwrap();
    let conn2 = manager2.get().await.unwrap();

    // Verify both connections work (they both use default DB from config)
    let mut conn1 = conn1;
    let mut conn2 = conn2;

    let _: () = conn1.set("clone_conn_test", "value1").await.unwrap();
    let value: String = conn2.get("clone_conn_test").await.unwrap();
    assert_eq!(value, "value1");

    // Cleanup
    let _: () = conn1.del("clone_conn_test").await.unwrap();
}

/// Test config accessor
#[tokio::test]
async fn test_config_accessor() {
    let config = ManagerConfig {
        url: "redis://127.0.0.1:6379/0".to_string(),
        default_db: 5,
        test_mode: true,
        test_db: 10,
    };
    let manager = RedisConnectionManager::new(config.clone()).await;

    // Skip test if Redis is not available
    if manager.is_err() {
        println!("Redis not available, skipping test");
        return;
    }

    let manager = manager.unwrap();
    let retrieved_config = manager.config();

    assert_eq!(retrieved_config.url, config.url);
    assert_eq!(retrieved_config.default_db, config.default_db);
    assert_eq!(retrieved_config.test_mode, config.test_mode);
    assert_eq!(retrieved_config.test_db, config.test_db);
}
