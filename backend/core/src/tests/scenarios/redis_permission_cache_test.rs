// =============================================================================
// Scenario Test: Permission Cache Operations
// =============================================================================
//
// GWT: Given-When-Then test for permission caching using Redis
//
// =============================================================================

use crate::infrastructure::authorization::cache::RedisCache;
use crate::infrastructure::redis::{ManagerConfig, RedisConnectionManager};
use serde_json;

/// Scenario: Permission cache operations
///
/// Given:
///   - Redis connection manager created
///   - RedisCache instance created
///   - Test permission data prepared
///
/// When:
///   - Cache permission check result
///   - Read cached permission
///   - Batch delete permission cache
///
/// Then:
///   - Cache write successful
///   - Cache read returns correct data
///   - Batch delete successful, cache cleared
#[tokio::test]
async fn scenario_permission_cache_operations() {
    // Given: Redis connection manager created
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

    // Given: RedisCache instance created
    let cache = RedisCache::new(manager.clone()).unwrap();

    // Given: Test permission data
    let realm_id = "test_realm";
    let user_id = "test_user";
    let resource = "test_resource";
    let action = "read";
    let key = format!("perm:{}:{}:{}:{}", realm_id, user_id, resource, action);

    // When: Cache permission check result
    let permission_data = serde_json::json!({
        "allowed": true,
        "cached_at": 1234567890,
        "ttl": 300,
    });
    cache.set(&key, &permission_data, 300).await.unwrap();

    // Then: Cache write successful
    let loaded: Option<serde_json::Value> = cache.get(&key).await.unwrap();
    assert!(loaded.is_some());
    let loaded_data = loaded.unwrap();
    assert_eq!(loaded_data["allowed"], true);
    assert_eq!(loaded_data["cached_at"], 1234567890);
    assert_eq!(loaded_data["ttl"], 300);

    // When: Batch delete permission cache
    let pattern = format!("perm:{}:{}:*", realm_id, user_id);
    cache.delete_pattern(&pattern).await.unwrap();

    // Then: Cache cleared
    let deleted: Option<serde_json::Value> = cache.get(&key).await.unwrap();
    assert!(deleted.is_none());
}

/// Scenario: Permission cache with complex data structures
///
/// Given:
///   - RedisCache instance created
///   - Complex permission data with roles and policies
///
/// When:
///   - Cache complex permission data
///   - Load and verify structure
///
/// Then:
///   - Complex data serialized/deserialized correctly
///   - All nested fields preserved
#[tokio::test]
async fn scenario_permission_cache_complex_data() {
    // Given: Redis connection manager created
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
    let cache = RedisCache::new(manager.clone()).unwrap();

    // Given: Complex permission data
    let key = "perm:complex:user123:resource1:action1";
    let complex_data = serde_json::json!({
        "allowed": true,
        "cached_at": 1234567890,
        "roles": ["role1", "role2", "role3"],
        "policies": [
            {
                "policy_id": "policy1",
                "effect": "allow",
                "conditions": {
                    "ip": "192.168.1.1",
                    "time": "09:00-17:00"
                }
            },
            {
                "policy_id": "policy2",
                "effect": "deny",
                "conditions": {
                    "ip": "0.0.0.0/0"
                }
            }
        ],
        "metadata": {
            "checked_by": "system",
            "cache_version": 1
        }
    });

    // When: Cache complex data
    cache.set(key, &complex_data, 300).await.unwrap();

    // Then: Load and verify structure
    let loaded: Option<serde_json::Value> = cache.get(key).await.unwrap();
    assert!(loaded.is_some());

    let loaded_data = loaded.unwrap();
    assert_eq!(loaded_data, complex_data);

    // Verify nested fields
    assert_eq!(loaded_data["roles"].as_array().unwrap().len(), 3);
    assert_eq!(loaded_data["policies"].as_array().unwrap().len(), 2);
    assert_eq!(loaded_data["metadata"]["cache_version"], 1);

    // Cleanup
    cache.delete(key).await.unwrap();
}

/// Scenario: Permission cache TTL expiration
///
/// Given:
///   - RedisCache instance created
///   - Permission data with short TTL
///
/// When:
///   - Cache permission with 2 second TTL
///   - Wait for expiration
///   - Try to load expired cache
///
/// Then:
///   - Data accessible before expiration
///   - Data returns None after expiration
#[tokio::test]
async fn scenario_permission_cache_ttl_expiration() {
    // Given: RedisCache instance created
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
    let cache = RedisCache::new(manager.clone()).unwrap();

    // Given: Permission data with 2 second TTL
    let key = "perm:ttl:user123:resource1:action1";
    let data = serde_json::json!({
        "allowed": true,
        "cached_at": 1234567890,
    });

    // When: Cache with 2 second TTL
    cache.set(key, &data, 2).await.unwrap();

    // Then: Data accessible immediately
    let loaded: Option<serde_json::Value> = cache.get(key).await.unwrap();
    assert!(loaded.is_some());

    // Wait for expiration
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Then: Data expired (returns None)
    let expired: Option<serde_json::Value> = cache.get(key).await.unwrap();
    assert!(expired.is_none());
}

/// Scenario: Batch permission cache operations
///
/// Given:
///   - RedisCache instance created
///   - Multiple permission entries for same user
///
/// When:
///   - Cache multiple permissions
///   - Batch delete using pattern
///   - Verify all entries deleted
///
/// Then:
///   - All permissions cached successfully
///   - Batch delete removes all matching entries
///   - No orphaned data remains
#[tokio::test]
async fn scenario_batch_permission_cache_operations() {
    // Given: RedisCache instance created
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
    let cache = RedisCache::new(manager.clone()).unwrap();

    // Given: Multiple permission entries for same user
    let user_id = "user_batch";
    let resources = vec!["resource1", "resource2", "resource3"];
    let actions = vec!["read", "write", "delete"];

    // Cache multiple permissions
    for resource in &resources {
        for action in &actions {
            let key = format!("perm:batch:{}:{}:{}", user_id, resource, action);
            let data = serde_json::json!({
                "allowed": true,
                "resource": resource,
                "action": action,
            });
            cache.set(&key, &data, 300).await.unwrap();
        }
    }

    // Verify all cached
    let pattern = format!("perm:batch:{}:*", user_id);

    // Get a connection to check keys
    let mut conn = manager.get().await.unwrap();
    let keys: Vec<String> = redis::cmd("KEYS")
        .arg(&pattern)
        .query_async(&mut conn)
        .await
        .unwrap();

    assert_eq!(keys.len(), 9); // 3 resources × 3 actions

    // When: Batch delete using pattern
    cache.delete_pattern(&pattern).await.unwrap();

    // Then: Verify all entries deleted
    let keys_after: Vec<String> = redis::cmd("KEYS")
        .arg(&pattern)
        .query_async(&mut conn)
        .await
        .unwrap();

    assert_eq!(keys_after.len(), 0);
}

/// Scenario: Cache update and overwrite
///
/// Given:
///   - RedisCache instance created
///   - Existing cached permission
///
/// When:
///   - Cache initial permission (allow)
///   - Update with different permission (deny)
///   - Load and verify updated value
///
/// Then:
///   - Updated value reflects new permission
///   - Old value is completely replaced
#[tokio::test]
async fn scenario_cache_update_and_overwrite() {
    // Given: RedisCache instance created
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
    let cache = RedisCache::new(manager.clone()).unwrap();

    // Given: Existing cached permission
    let key = "perm:update:user123:resource1:action1";
    let initial_data = serde_json::json!({
        "allowed": true,
        "reason": "initial grant",
        "version": 1,
    });

    // Cache initial permission
    cache.set(key, &initial_data, 300).await.unwrap();

    // When: Update with different permission
    let updated_data = serde_json::json!({
        "allowed": false,
        "reason": "permission revoked",
        "version": 2,
    });
    cache.set(key, &updated_data, 300).await.unwrap();

    // Then: Load and verify updated value
    let loaded: Option<serde_json::Value> = cache.get(key).await.unwrap();
    assert!(loaded.is_some());

    let loaded_data = loaded.unwrap();
    assert_eq!(loaded_data["allowed"], false);
    assert_eq!(loaded_data["reason"], "permission revoked");
    assert_eq!(loaded_data["version"], 2);

    // Cleanup
    cache.delete(key).await.unwrap();
}

/// Scenario: Cache miss handling
///
/// Given:
///   - RedisCache instance created
///   - No cached data
///
/// When:
///   - Try to load non-existent cache key
///
/// Then:
///   - Returns Ok(None) instead of error
///   - No exception thrown
#[tokio::test]
async fn scenario_cache_miss_handling() {
    // Given: RedisCache instance created
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
    let cache = RedisCache::new(manager.clone()).unwrap();

    // When: Try to load non-existent cache key
    let result = cache.get::<serde_json::Value>("perm:nonexistent:key").await;

    // Then: Returns Ok(None)
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}
