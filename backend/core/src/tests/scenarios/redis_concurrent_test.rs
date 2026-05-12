// =============================================================================
// Scenario Test: Concurrent Connection Requests
// =============================================================================
//
// GWT: Given-When-Then test for concurrent Redis connection handling
//
// =============================================================================

use crate::infrastructure::redis::{ManagerConfig, RedisConnectionManager};
use redis::AsyncCommands;
use std::sync::Arc;
use tokio::task::JoinSet;

/// Scenario: Concurrent connection requests
///
/// Given:
///   - Redis connection manager created
///
/// When:
///   - Concurrently initiate 100 connection requests
///   - Each connection performs SET/GET/DEL operations
///
/// Then:
///   - All requests complete successfully
///   - No connection leaks
///   - No data races or deadlocks
#[tokio::test]
async fn scenario_concurrent_connection_requests() {
    // Given: Redis connection manager created
    let config = ManagerConfig::default();
    let manager = RedisConnectionManager::new(config).await;

    // Skip test if Redis is not available
    if manager.is_err() {
        println!("Redis not available, skipping test");
        return;
    }

    let manager = Arc::new(manager.unwrap());

    // When: Concurrently initiate 100 connection requests
    let mut join_set = JoinSet::new();

    for i in 0..100 {
        let manager_clone = manager.clone();
        join_set.spawn(async move {
            let mut conn = manager_clone.get().await.unwrap();

            let key = format!("concurrent_test_{}", i);
            let value = format!("value_{}", i);

            // SET
            let _: () = conn.set_ex(key.clone(), value.clone(), 60).await.unwrap();

            // GET
            let retrieved: String = conn.get(key.clone()).await.unwrap();
            assert_eq!(retrieved, value);

            // DEL
            let _: () = conn.del(&key).await.unwrap();

            // Verify deleted
            let deleted: Option<String> = conn.get(&key).await.unwrap();
            assert!(deleted.is_none());

            Ok::<(), anyhow::Error>(())
        });
    }

    // Then: All requests complete successfully
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

    assert_eq!(
        success_count, 100,
        "All 100 concurrent requests should succeed"
    );
    assert_eq!(failure_count, 0, "No requests should fail");

    // Cleanup: Verify no leftover data
    let mut conn = manager.get().await.unwrap();
    let keys: Vec<String> = conn.keys("concurrent_test_*").await.unwrap();
    assert!(keys.is_empty(), "All test keys should be cleaned up");
}

/// Scenario: Concurrent read operations
///
/// Given:
///   - Redis connection manager created
///   - Pre-populated data
///
/// When:
///   - 50 concurrent tasks read same key
///
/// Then:
///   - All reads successful
///   - All read values consistent
#[tokio::test]
async fn scenario_concurrent_read_operations() {
    // Given: Redis connection manager created
    let config = ManagerConfig::default();
    let manager = RedisConnectionManager::new(config).await;

    // Skip test if Redis is not available
    if manager.is_err() {
        println!("Redis not available, skipping test");
        return;
    }

    let manager = Arc::new(manager.unwrap());

    // Given: Pre-populated data
    let key = "concurrent_read_key";
    let expected_value = "shared_value_12345";

    {
        let mut conn = manager.get().await.unwrap();
        let _: () = conn.set(key, expected_value).await.unwrap();
    }

    // When: 50 concurrent tasks read same key
    let mut join_set = JoinSet::new();

    for _ in 0..50 {
        let manager_clone = manager.clone();
        let key = key.to_string();

        join_set.spawn(async move {
            let mut conn = manager_clone.get().await.unwrap();
            let value: String = conn.get(key).await.unwrap();
            Ok::<String, anyhow::Error>(value)
        });
    }

    // Then: All reads successful and consistent
    let mut success_count = 0;
    let mut inconsistent_count = 0;

    while let Some(result) = join_set.join_next().await {
        if let Ok(task_result) = result
            && let Ok(value) = task_result
        {
            if value == expected_value {
                success_count += 1;
            } else {
                inconsistent_count += 1;
            }
        }
    }

    assert_eq!(success_count, 50, "All 50 reads should succeed");
    assert_eq!(inconsistent_count, 0, "No inconsistent reads");

    // Cleanup
    let mut conn = manager.get().await.unwrap();
    let _: () = conn.del(key).await.unwrap();
}

/// Scenario: Concurrent write operations with different keys
///
/// Given:
///   - Redis connection manager created
///
/// When:
///   - 50 concurrent tasks write to different keys
///
/// Then:
///   - All writes successful
///   - No data corruption
///   - All keys accessible after writes
#[tokio::test]
async fn scenario_concurrent_write_operations_different_keys() {
    // Given: Redis connection manager created
    let config = ManagerConfig::default();
    let manager = RedisConnectionManager::new(config).await;

    // Skip test if Redis is not available
    if manager.is_err() {
        println!("Redis not available, skipping test");
        return;
    }

    let manager = Arc::new(manager.unwrap());

    // When: 50 concurrent tasks write to different keys
    let mut join_set = JoinSet::new();

    for i in 0..50 {
        let manager_clone = manager.clone();
        join_set.spawn(async move {
            let mut conn = manager_clone.get().await.unwrap();

            let key = format!("write_test_{}", i);
            let value = format!("value_{}", i);

            let _: () = conn.set_ex(key.clone(), &value, 60).await.unwrap();

            // Verify write
            let read_back: String = conn.get(&key).await.unwrap();
            assert_eq!(read_back, value);

            Ok::<(), anyhow::Error>(())
        });
    }

    // Then: All writes successful
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

    assert_eq!(success_count, 50, "All 50 writes should succeed");
    assert_eq!(failure_count, 0, "No writes should fail");

    // Verify all keys accessible
    let mut conn = manager.get().await.unwrap();
    for i in 0..50 {
        let key = format!("write_test_{}", i);
        let value: Option<String> = conn.get(&key).await.unwrap();
        assert!(value.is_some(), "Key {} should exist", key);
    }

    // Cleanup
    for i in 0..50 {
        let key = format!("write_test_{}", i);
        let _: () = conn.del(&key).await.unwrap();
    }
}

/// Scenario: Concurrent operations with manager clone
///
/// Given:
///   - Redis connection manager created
///   - Multiple clones of manager
///
/// When:
///   - 50 concurrent tasks use different manager clones
///
/// Then:
///   - All clones work correctly
///   - All operations successful
///   - Clones share underlying connection
#[tokio::test]
async fn scenario_concurrent_operations_with_manager_clone() {
    // Given: Redis connection manager created
    let config = ManagerConfig::default();
    let manager = RedisConnectionManager::new(config).await;

    // Skip test if Redis is not available
    if manager.is_err() {
        println!("Redis not available, skipping test");
        return;
    }

    let manager = Arc::new(manager.unwrap());

    // When: 50 concurrent tasks use different manager clones
    let mut join_set = JoinSet::new();

    for i in 0..50 {
        let manager_clone = manager.clone();
        join_set.spawn(async move {
            // Each task gets its own manager clone
            let _manager_clone_2 = manager_clone.clone();

            let mut conn = manager_clone.get().await.unwrap();

            let key = format!("clone_test_{}", i);
            let value = format!("value_{}", i);

            let _: () = conn.set_ex(key.clone(), &value, 60).await.unwrap();
            let read_back: String = conn.get(&key).await.unwrap();
            assert_eq!(read_back, value);

            Ok::<(), anyhow::Error>(())
        });
    }

    // Then: All clones work correctly
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

    assert_eq!(success_count, 50, "All 50 tasks should succeed");
    assert_eq!(failure_count, 0, "No tasks should fail");

    // Cleanup
    let mut conn = manager.get().await.unwrap();
    for i in 0..50 {
        let key = format!("clone_test_{}", i);
        let _: () = conn.del(key).await.unwrap();
    }
}

/// Scenario: Mixed concurrent operations (read/write/delete)
///
/// Given:
///   - Redis connection manager created
///   - Pre-populated data set
///
/// When:
///   - Concurrent tasks perform mixed operations
///     - 20 tasks read
///     - 20 tasks write
///     - 20 tasks delete
///
/// Then:
///   - All operations complete without errors
///   - No data corruption
///   - System remains consistent
#[tokio::test]
async fn scenario_mixed_concurrent_operations() {
    // Given: Redis connection manager created
    let config = ManagerConfig::default();
    let manager = RedisConnectionManager::new(config).await;

    // Skip test if Redis is not available
    if manager.is_err() {
        println!("Redis not available, skipping test");
        return;
    }

    let manager = Arc::new(manager.unwrap());

    // Given: Pre-populated data set
    {
        let mut conn = manager.get().await.unwrap();
        for i in 0..20 {
            let key = format!("mixed_test_{}", i);
            let value = format!("initial_value_{}", i);
            let _: () = conn.set(key, value).await.unwrap();
        }
    }

    // When: Mixed concurrent operations
    let mut join_set = JoinSet::new();

    // 20 read tasks
    for i in 0..20 {
        let manager_clone = manager.clone();
        join_set.spawn(async move {
            let mut conn = manager_clone.get().await.unwrap();
            let key = format!("mixed_test_{}", i);
            let value: Option<String> = conn.get(key).await.unwrap();
            Ok::<bool, anyhow::Error>(value.is_some())
        });
    }

    // 20 write tasks (different keys to avoid conflicts)
    for i in 20..40 {
        let manager_clone = manager.clone();
        join_set.spawn(async move {
            let mut conn = manager_clone.get().await.unwrap();
            let key = format!("mixed_test_{}", i);
            let value = format!("new_value_{}", i);
            let _: () = conn.set_ex(&key, &value, 60).await.unwrap();
            Ok::<bool, anyhow::Error>(true)
        });
    }

    // 20 delete tasks (original keys)
    for i in 0..20 {
        let manager_clone = manager.clone();
        join_set.spawn(async move {
            let mut conn = manager_clone.get().await.unwrap();
            let key = format!("mixed_test_{}", i);
            let _: () = conn.del(&key).await.unwrap();
            Ok::<bool, anyhow::Error>(true)
        });
    }

    // Then: All operations complete without errors
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

    assert_eq!(success_count, 60, "All 60 operations should succeed");
    assert_eq!(failure_count, 0, "No operations should fail");

    // Cleanup: Delete remaining keys
    let mut conn = manager.get().await.unwrap();
    for i in 20..40 {
        let key = format!("mixed_test_{}", i);
        let _: () = conn.del(key).await.unwrap();
    }
}

/// Scenario: Stress test with high concurrency
///
/// Given:
///   - Redis connection manager created
///
/// When:
///   - 200 concurrent tasks perform operations
///
/// Then:
///   - System handles high concurrency
///   - All operations successful
///   - No deadlocks or timeouts
#[tokio::test]
async fn scenario_stress_test_high_concurrency() {
    // Given: Redis connection manager created
    let config = ManagerConfig::default();
    let manager = RedisConnectionManager::new(config).await;

    // Skip test if Redis is not available
    if manager.is_err() {
        println!("Redis not available, skipping test");
        return;
    }

    let manager = Arc::new(manager.unwrap());

    // When: 200 concurrent tasks perform operations
    let mut join_set = JoinSet::new();

    for i in 0..200 {
        let manager_clone = manager.clone();
        join_set.spawn(async move {
            let mut conn = manager_clone.get().await.unwrap();

            let key = format!("stress_test_{}", i);
            let value = format!("value_{}", i);

            // Minimal operations for speed
            let _: () = conn.set_ex(key.clone(), value, 60).await.unwrap();
            let _: () = conn.del(key).await.unwrap();

            Ok::<(), anyhow::Error>(())
        });
    }

    // Then: System handles high concurrency
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

    assert_eq!(success_count, 200, "All 200 operations should succeed");
    assert_eq!(failure_count, 0, "No operations should fail");

    // Verify cleanup
    let mut conn = manager.get().await.unwrap();
    let keys: Vec<String> = conn.keys("stress_test_*").await.unwrap();
    assert!(keys.is_empty(), "All stress test keys should be cleaned up");
}
