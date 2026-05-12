// =============================================================================
// Scenario Test: Complete Session Management Flow
// =============================================================================
//
// GWT: Given-When-Then test for session management using Redis
//
// =============================================================================

use crate::infrastructure::redis::{ManagerConfig, RedisConnectionManager};
use redis::AsyncCommands;
use serde_json;

/// Scenario: Complete session management flow
///
/// Given:
///   - Redis connection manager created (test mode)
///   - Session data prepared
///
/// When:
///   - Store session data
///   - Load session data
///   - Delete session data
///
/// Then:
///   - Session data stored successfully
///   - Loaded data matches stored data
///   - After deletion, load returns None
#[tokio::test]
async fn scenario_complete_session_management() {
    // Given: Redis connection manager created (test mode)
    let config = ManagerConfig {
        url: "redis://127.0.0.1:6379/0".to_string(),
        default_db: 0,
        test_mode: true, // Use DB 1
        test_db: 1,
    };
    let manager = RedisConnectionManager::new(config).await;

    // Skip test if Redis is not available
    if manager.is_err() {
        println!("Redis not available, skipping test");
        return;
    }

    let manager = manager.unwrap();

    // Given: Session data
    let token = "test_token_123";
    let session_data = serde_json::json!({
        "user_id": "user_123",
        "realm_id": "realm_456",
        "client_ip": "192.168.1.1",
        "created_at": 1234567890,
        "expires_at": 1234571490,
    });

    // When: Store session
    let mut store_conn = manager.get().await.unwrap();
    let key = format!("sess:{}", token);
    let _: () = store_conn
        .set_ex(key.clone(), session_data.to_string(), 3600)
        .await
        .unwrap();

    // Then: Session stored successfully
    let stored_value: String = store_conn.get(key.clone()).await.unwrap();
    assert_eq!(stored_value, session_data.to_string());

    // When: Load session
    let loaded_value: String = store_conn.get(key.clone()).await.unwrap();
    let loaded_data: serde_json::Value = serde_json::from_str(&loaded_value).unwrap();

    // Then: Loaded data matches stored data
    assert_eq!(loaded_data["user_id"], "user_123");
    assert_eq!(loaded_data["realm_id"], "realm_456");
    assert_eq!(loaded_data["client_ip"], "192.168.1.1");
    assert_eq!(loaded_data["created_at"], 1234567890);
    assert_eq!(loaded_data["expires_at"], 1234571490);

    // When: Delete session
    let _: () = store_conn.del(key.clone()).await.unwrap();

    // Then: After deletion, load returns None
    let deleted_value: Option<String> = store_conn.get(key).await.unwrap();
    assert!(deleted_value.is_none());

    // Cleanup: Ensure no leftover data
    let mut cleanup_conn = manager.get().await.unwrap();
    let keys: Vec<String> = cleanup_conn.keys("sess:*").await.unwrap();
    if !keys.is_empty() {
        let _: () = cleanup_conn.del(keys).await.unwrap();
    }
}

/// Scenario: Session TTL expiration
///
/// Given:
///   - Redis connection manager created
///   - Session data with TTL
///
/// When:
///   - Store session with short TTL
///   - Wait for expiration
///   - Try to load expired session
///
/// Then:
///   - Session data accessible before expiration
///   - Session data returns None after expiration
#[tokio::test]
async fn scenario_session_ttl_expiration() {
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

    // Given: Session data with 2 second TTL
    let token = "test_token_ttl";
    let session_data = serde_json::json!({
        "user_id": "user_ttl",
        "realm_id": "realm_ttl",
    });

    // When: Store session with 2 second TTL
    let mut conn = manager.get().await.unwrap();
    let key = format!("sess:{}", token);
    let _: () = conn
        .set_ex(key.clone(), session_data.to_string(), 2)
        .await
        .unwrap();

    // Then: Session accessible immediately
    let loaded_value: Option<String> = conn.get(key.clone()).await.unwrap();
    assert!(loaded_value.is_some());

    // Wait for expiration
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Then: Session expired (returns None)
    let expired_value: Option<String> = conn.get(key.clone()).await.unwrap();
    assert!(expired_value.is_none());
}

/// Scenario: Multiple concurrent sessions
///
/// Given:
///   - Redis connection manager created
///   - Multiple session tokens
///
/// When:
///   - Store multiple sessions concurrently
///   - Load all sessions
///
/// Then:
///   - All sessions stored successfully
///   - All sessions loaded correctly
///   - No data corruption
#[tokio::test]
async fn scenario_multiple_concurrent_sessions() {
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

    // Given: Multiple session tokens
    let tokens = vec!["token_1", "token_2", "token_3", "token_4", "token_5"];

    // When: Store multiple sessions
    let mut conn = manager.get().await.unwrap();
    for token in &tokens {
        let session_data = serde_json::json!({
            "user_id": format!("user_{}", token),
            "realm_id": format!("realm_{}", token),
        });
        let key = format!("sess:{}", token);
        let _: () = conn
            .set_ex(key, session_data.to_string(), 3600)
            .await
            .unwrap();
    }

    // Then: All sessions loaded correctly
    for token in &tokens {
        let key = format!("sess:{}", token);
        let loaded_value: Option<String> = conn.get(key.clone()).await.unwrap();
        assert!(loaded_value.is_some(), "Session {} should exist", token);

        let loaded_data: serde_json::Value = serde_json::from_str(&loaded_value.unwrap()).unwrap();
        assert_eq!(loaded_data["user_id"], format!("user_{}", token));
        assert_eq!(loaded_data["realm_id"], format!("realm_{}", token));
    }

    // Cleanup: Delete all sessions
    let keys: Vec<String> = conn.keys("sess:token_*").await.unwrap();
    if !keys.is_empty() {
        let _: () = conn.del(keys).await.unwrap();
    }
}

/// Scenario: Session update
///
/// Given:
///   - Redis connection manager created
///   - Existing session data
///
/// When:
///   - Store initial session
///   - Update session data
///   - Load updated session
///
/// Then:
///   - Updated data reflects changes
///   - Old data is replaced
#[tokio::test]
async fn scenario_session_update() {
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

    // Given: Existing session data
    let token = "test_token_update";
    let initial_data = serde_json::json!({
        "user_id": "user_initial",
        "realm_id": "realm_initial",
        "counter": 1,
    });

    // When: Store initial session
    let mut conn = manager.get().await.unwrap();
    let key = format!("sess:{}", token);
    let _: () = conn
        .set_ex(key.clone(), initial_data.to_string(), 3600)
        .await
        .unwrap();

    // When: Update session data
    let updated_data = serde_json::json!({
        "user_id": "user_updated",
        "realm_id": "realm_updated",
        "counter": 2,
    });
    let _: () = conn
        .set_ex(key.clone(), updated_data.to_string(), 3600)
        .await
        .unwrap();

    // Then: Updated data reflects changes
    let loaded_value: String = conn.get(key.clone()).await.unwrap();
    let loaded_data: serde_json::Value = serde_json::from_str(&loaded_value).unwrap();

    assert_eq!(loaded_data["user_id"], "user_updated");
    assert_eq!(loaded_data["realm_id"], "realm_updated");
    assert_eq!(loaded_data["counter"], 2);

    // Cleanup
    let _: () = conn.del(key).await.unwrap();
}
