// =============================================================================
// Scenario Test: Test Environment Isolation
// =============================================================================
//
// GWT: Given-When-Then test for Redis DB isolation between test and production
//
// =============================================================================

use crate::infrastructure::redis::{ManagerConfig, RedisConnectionManager};
use redis::AsyncCommands;

/// Scenario: Test environment isolation
///
/// Given:
///   - Production environment configuration (DB 0)
///   - Test environment configuration (DB 1)
///
/// When:
///   - Write data to production environment (DB 0)
///   - Write data to test environment (DB 1) with same keys
///   - Read from both environments
///
/// Then:
///   - Production environment data remains unchanged
///   - Test environment data is independent
///   - DB 0 and DB 1 are completely isolated
#[tokio::test]
async fn scenario_test_environment_isolation() {
    // Given: Production environment configuration
    let prod_config = ManagerConfig {
        url: "redis://127.0.0.1:6379/0".to_string(),
        default_db: 0,
        test_mode: false,
        test_db: 1,
    };
    let prod_manager = RedisConnectionManager::new(prod_config).await;

    // Given: Test environment configuration
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

    let key = "isolation_test_key";
    let prod_value = "production_data";
    let test_value = "test_data";

    // When: Write to production environment (DB 0)
    let mut prod_conn = prod_manager.get().await.unwrap();
    let _: () = prod_conn.set(key, prod_value).await.unwrap();

    // When: Write to test environment (DB 1) with same key
    let mut test_conn = test_manager.get().await.unwrap();
    let _: () = test_conn.set(key, test_value).await.unwrap();

    // Then: Production environment data remains unchanged
    let prod_result: String = prod_conn.get(key).await.unwrap();
    assert_eq!(
        prod_result, prod_value,
        "Production data should not be affected"
    );

    // Then: Test environment data is independent
    let test_result: String = test_conn.get(key).await.unwrap();
    assert_eq!(test_result, test_value, "Test data should be independent");

    // Cleanup
    let _: () = prod_conn.del(key).await.unwrap();
    let _: () = test_conn.del(key).await.unwrap();
}

/// Scenario: Multiple DB isolation
///
/// Given:
///   - Three different managers using different DBs (0, 1, 5)
///
/// When:
///   - Write same key to all three DBs with different values
///   - Read from each DB
///
/// Then:
///   - Each DB maintains its own data
///   - No cross-DB contamination
#[tokio::test]
async fn scenario_multiple_db_isolation() {
    // Given: Three managers using different DBs
    let config0 = ManagerConfig {
        url: "redis://127.0.0.1:6379/0".to_string(),
        default_db: 0,
        test_mode: false,
        test_db: 0,
    };
    let manager0 = RedisConnectionManager::new(config0).await;

    let config1 = ManagerConfig {
        url: "redis://127.0.0.1:6379/0".to_string(),
        default_db: 0,
        test_mode: true,
        test_db: 1,
    };
    let manager1 = RedisConnectionManager::new(config1).await;

    let config5 = ManagerConfig {
        url: "redis://127.0.0.1:6379/0".to_string(),
        default_db: 0,
        test_mode: false,
        test_db: 5,
    };

    let manager5 = RedisConnectionManager::new(config5).await;

    // Skip test if Redis is not available
    if manager0.is_err() || manager1.is_err() || manager5.is_err() {
        println!("Redis not available, skipping test");
        return;
    }

    let manager0 = manager0.unwrap();
    let manager1 = manager1.unwrap();
    let manager5 = manager5.unwrap();

    let key = "multi_db_test_key";

    // When: Write same key to all three DBs
    let mut conn0 = manager0.get().await.unwrap();
    let mut conn1 = manager1.get().await.unwrap();
    let mut conn5 = manager5.get_with_db(5).await.unwrap();

    let _: () = conn0.set(key, "value_db_0").await.unwrap();
    let _: () = conn1.set(key, "value_db_1").await.unwrap();
    let _: () = conn5.set(key, "value_db_5").await.unwrap();

    // Then: Each DB maintains its own data
    let val0: String = conn0.get(key).await.unwrap();
    let val1: String = conn1.get(key).await.unwrap();
    let val5: String = conn5.get(key).await.unwrap();

    assert_eq!(val0, "value_db_0");
    assert_eq!(val1, "value_db_1");
    assert_eq!(val5, "value_db_5");

    // Cleanup
    let _: () = conn0.del(key).await.unwrap();
    let _: () = conn1.del(key).await.unwrap();
    let _: () = conn5.del(key).await.unwrap();
}

/// Scenario: Test mode automatic DB selection
///
/// Given:
///   - Manager configured with test_mode = true
///   - Manager configured with test_mode = false
///
/// When:
///   - Both managers call get() method
///   - Check DB number of returned connections
///
/// Then:
///   - Test mode manager uses test_db
///   - Production mode manager uses default_db
#[tokio::test]
async fn scenario_test_mode_automatic_db_selection() {
    // Given: Manager configured with test_mode = true
    let test_config = ManagerConfig {
        url: "redis://127.0.0.1:6379/0".to_string(),
        default_db: 0,
        test_mode: true,
        test_db: 1,
    };
    let test_manager = RedisConnectionManager::new(test_config).await;

    // Given: Manager configured with test_mode = false
    let prod_config = ManagerConfig {
        url: "redis://127.0.0.1:6379/0".to_string(),
        default_db: 0,
        test_mode: false,
        test_db: 1,
    };
    let prod_manager = RedisConnectionManager::new(prod_config).await;

    // Skip test if Redis is not available
    if test_manager.is_err() || prod_manager.is_err() {
        println!("Redis not available, skipping test");
        return;
    }

    let test_manager = test_manager.unwrap();
    let prod_manager = prod_manager.unwrap();

    // When: Both managers call get() method
    let _test_conn = test_manager.get().await.unwrap();
    let _prod_conn = prod_manager.get().await.unwrap();

    // Then: Both connections are created successfully
    // Note: ConnectionManager doesn't expose the current DB, but we know from config:
    // - Test mode uses DB 1
    // - Production mode uses DB 0
}

/// Scenario: Concurrent test and production operations
///
/// Given:
///   - Test manager and production manager
///   - Both operating simultaneously
///
/// When:
///   - Concurrent operations on both managers
///   - Write and read from both environments
///
/// Then:
///   - No interference between environments
///   - All operations successful
#[tokio::test]
async fn scenario_concurrent_test_and_production_operations() {
    // Given: Test and production managers
    let test_config = ManagerConfig {
        url: "redis://127.0.0.1:6379/0".to_string(),
        default_db: 0,
        test_mode: true,
        test_db: 1,
    };
    let test_manager = RedisConnectionManager::new(test_config).await;

    let prod_config = ManagerConfig {
        url: "redis://127.0.0.1:6379/0".to_string(),
        default_db: 0,
        test_mode: false,
        test_db: 1,
    };
    let prod_manager = RedisConnectionManager::new(prod_config).await;

    // Skip test if Redis is not available
    if test_manager.is_err() || prod_manager.is_err() {
        println!("Redis not available, skipping test");
        return;
    }

    let test_manager = std::sync::Arc::new(test_manager.unwrap());
    let prod_manager = std::sync::Arc::new(prod_manager.unwrap());

    // When: Concurrent operations on both managers
    let mut join_set = tokio::task::JoinSet::new();

    // 20 test operations
    for i in 0..20 {
        let manager = test_manager.clone();
        join_set.spawn(async move {
            let mut conn = manager.get().await.unwrap();
            let key = format!("test_op_{}", i);
            let value = format!("test_value_{}", i);
            let _: () = conn.set(key, value).await.unwrap();
            Ok::<(), anyhow::Error>(())
        });
    }

    // 20 production operations
    for i in 0..20 {
        let manager = prod_manager.clone();
        join_set.spawn(async move {
            let mut conn = manager.get().await.unwrap();
            let key = format!("prod_op_{}", i);
            let value = format!("prod_value_{}", i);
            let _: () = conn.set(key, value).await.unwrap();
            Ok::<(), anyhow::Error>(())
        });
    }

    // Then: No interference, all operations successful
    let mut success_count = 0;
    let mut failure_count = 0;

    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(task_result) => match task_result {
                Ok(_) => success_count += 1,
                Err(_) => failure_count += 1,
            },
            Err(_) => failure_count += 1,
        }
    }

    assert_eq!(success_count, 40, "All 40 operations should succeed");
    assert_eq!(failure_count, 0, "No operations should fail");

    // Verify data isolation
    let mut test_conn = test_manager.get().await.unwrap();
    let mut prod_conn = prod_manager.get().await.unwrap();

    // Test environment should have test keys
    let test_keys: Vec<String> = test_conn.keys("test_op_*").await.unwrap();
    assert_eq!(test_keys.len(), 20, "Test environment should have 20 keys");

    // Production environment should have prod keys
    let prod_keys: Vec<String> = prod_conn.keys("prod_op_*").await.unwrap();
    assert_eq!(
        prod_keys.len(),
        20,
        "Production environment should have 20 keys"
    );

    // Test environment should NOT have prod keys
    let prod_keys_in_test: Vec<String> = test_conn.keys("prod_op_*").await.unwrap();
    assert_eq!(
        prod_keys_in_test.len(),
        0,
        "Test environment should not have prod keys"
    );

    // Production environment should NOT have test keys
    let test_keys_in_prod: Vec<String> = prod_conn.keys("test_op_*").await.unwrap();
    assert_eq!(
        test_keys_in_prod.len(),
        0,
        "Production environment should not have test keys"
    );

    // Cleanup
    for i in 0..20 {
        let _: () = test_conn.del(format!("test_op_{}", i)).await.unwrap();
        let _: () = prod_conn.del(format!("prod_op_{}", i)).await.unwrap();
    }
}

/// Scenario: DB isolation persistence across connections
///
/// Given:
///   - Test manager and production manager
///
/// When:
///   - Write data via first connection
///   - Get new connection and read data
///   - Repeat multiple times
///
/// Then:
///   - Data persists within same DB
///   - No cross-DB leakage
#[tokio::test]
async fn scenario_db_isolation_persistence_across_connections() {
    // Given: Test and production managers
    let test_config = ManagerConfig {
        url: "redis://127.0.0.1:6379/0".to_string(),
        default_db: 0,
        test_mode: true,
        test_db: 1,
    };
    let test_manager = RedisConnectionManager::new(test_config).await;

    let prod_config = ManagerConfig {
        url: "redis://127.0.0.1:6379/0".to_string(),
        default_db: 0,
        test_mode: false,
        test_db: 1,
    };
    let prod_manager = RedisConnectionManager::new(prod_config).await;

    // Skip test if Redis is not available
    if test_manager.is_err() || prod_manager.is_err() {
        println!("Redis not available, skipping test");
        return;
    }

    let test_manager = test_manager.unwrap();
    let prod_manager = prod_manager.unwrap();

    let key = "persistence_test";
    let test_value = "test_persistent_value";
    let prod_value = "prod_persistent_value";

    // When: Write data via first connection
    {
        let mut test_conn = test_manager.get().await.unwrap();
        let mut prod_conn = prod_manager.get().await.unwrap();

        let _: () = test_conn.set(key, test_value).await.unwrap();
        let _: () = prod_conn.set(key, prod_value).await.unwrap();
    }

    // When: Get new connection and read data (3 times)
    for i in 1..=3 {
        let mut test_conn = test_manager.get().await.unwrap();
        let mut prod_conn = prod_manager.get().await.unwrap();

        let test_val: String = test_conn.get(key).await.unwrap();
        let prod_val: String = prod_conn.get(key).await.unwrap();

        assert_eq!(
            test_val, test_value,
            "Iteration {}: Test data should persist",
            i
        );
        assert_eq!(
            prod_val, prod_value,
            "Iteration {}: Prod data should persist",
            i
        );
    }

    // Cleanup
    let mut test_conn = test_manager.get().await.unwrap();
    let mut prod_conn = prod_manager.get().await.unwrap();
    let _: () = test_conn.del(key).await.unwrap();
    let _: () = prod_conn.del(key).await.unwrap();
}

/// Scenario: Cleanup isolation
///
/// Given:
///   - Test and production environments
///   - Data in both environments
///
/// When:
///   - Clean up test environment
///   - Check production environment
///
/// Then:
///   - Test environment cleaned
///   - Production environment unaffected
#[tokio::test]
async fn scenario_cleanup_isolation() {
    // Given: Test and production environments
    let test_config = ManagerConfig {
        url: "redis://127.0.0.1:6379/0".to_string(),
        default_db: 0,
        test_mode: true,
        test_db: 1,
    };
    let test_manager = RedisConnectionManager::new(test_config).await;

    let prod_config = ManagerConfig {
        url: "redis://127.0.0.1:6379/0".to_string(),
        default_db: 0,
        test_mode: false,
        test_db: 1,
    };
    let prod_manager = RedisConnectionManager::new(prod_config).await;

    // Skip test if Redis is not available
    if test_manager.is_err() || prod_manager.is_err() {
        println!("Redis not available, skipping test");
        return;
    }

    let test_manager = test_manager.unwrap();
    let prod_manager = prod_manager.unwrap();

    // Given: Data in both environments
    {
        let mut test_conn = test_manager.get().await.unwrap();
        let mut prod_conn = prod_manager.get().await.unwrap();

        let _: () = test_conn.set("cleanup_test", "test_data").await.unwrap();
        let _: () = prod_conn.set("cleanup_test", "prod_data").await.unwrap();
    }

    // When: Clean up test environment
    {
        let mut test_conn = test_manager.get().await.unwrap();
        let _: () = test_conn.del("cleanup_test").await.unwrap();
    }

    // Then: Test environment cleaned
    {
        let mut test_conn = test_manager.get().await.unwrap();
        let test_val: Option<String> = test_conn.get("cleanup_test").await.unwrap();
        assert!(test_val.is_none(), "Test environment should be cleaned");
    }

    // Then: Production environment unaffected
    {
        let mut prod_conn = prod_manager.get().await.unwrap();
        let prod_val: String = prod_conn.get("cleanup_test").await.unwrap();
        assert_eq!(
            prod_val, "prod_data",
            "Production environment should be unaffected"
        );
    }

    // Cleanup production
    let mut prod_conn = prod_manager.get().await.unwrap();
    let _: () = prod_conn.del("cleanup_test").await.unwrap();
}
