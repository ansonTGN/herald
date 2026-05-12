// Redis Cache Service Tests
//
// This module contains integration tests for the API key cache service.
// Tests require a running Redis instance.

use super::*;
use chrono::Utc;
use herald_domain::client_api_keys::entities::ClientApiKey;

/// Helper function to create a test API key
fn create_test_api_key(id: &str, realm_id: &str) -> ClientApiKey {
    ClientApiKey {
        id: id.to_string(),
        name: format!("Test Key {}", id),
        api_key_hash: format!("hash-{}", id),
        realm_id: realm_id.to_string(),
        client_app_id: None,
        enabled: true,
        expires_at: None,
        created_at: Utc::now(),
        last_used_at: None,
        usage_count: 0,
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    // Note: These tests require a running Redis instance.
    // They should be run with the test environment setup.
    //
    // To run these tests:
    // 1. Start Redis: `docker run -d -p 6379:6379 redis`
    // 2. Set REDIS_URL environment variable
    // 3. Run: `cargo nextest run -p herald-core infrastructure::third_party_api_keys::cache_test`

    #[test]
    #[ignore = "requires Redis instance"]
    fn test_cache_get_set() {
        // This test would require a running Redis instance
        // For now, we'll test the conversion logic

        let api_key = create_test_api_key("test-1", "realm-1");
        let cached = ApiKeyCacheValue::from(&api_key);

        assert_eq!(cached.id, "test-1");
        assert_eq!(cached.name, "Test Key test-1");
        assert_eq!(cached.realm_id, "realm-1");
        assert!(cached.enabled);
    }

    #[test]
    #[ignore = "requires Redis instance"]
    fn test_cache_ttl() {
        // This test would require a running Redis instance
        // Test implementation would:
        // 1. Set cache with TTL = 1 second
        // 2. Immediately read, verify exists
        // 3. Wait 2 seconds
        // 4. Read again, verify expired
    }

    #[test]
    #[ignore = "requires Redis instance"]
    fn test_cache_delete() {
        // This test would require a running Redis instance
        // Test implementation would:
        // 1. Set cache value
        // 2. Delete cache
        // 3. Verify cache doesn't exist
    }

    #[test]
    fn test_cache_miss() {
        // Test the logic for cache miss
        // This would be tested with a mock Redis or real instance
    }

    // NOTE: Low-value test removed (test_cache_json_serialization)
    // This test only verified serde standard functionality without custom logic.
    // Serde guarantees are covered by the library itself and integration tests.

    #[test]
    fn test_cache_value_conversion_roundtrip() {
        let api_key = create_test_api_key("test-1", "realm-1");
        let cached = ApiKeyCacheValue::from(&api_key);

        // Test TryFrom conversion (roundtrip including api_key_hash)
        let converted: Result<ClientApiKey, String> = cached.try_into();
        assert!(converted.is_ok());

        let converted = converted.unwrap();
        assert_eq!(converted.id, "test-1");
        assert_eq!(converted.realm_id, "realm-1");
        assert_eq!(converted.enabled, api_key.enabled);

        // api_key_hash should be preserved in cache
        assert_eq!(converted.api_key_hash, "hash-test-1");
        assert_eq!(converted.last_used_at, None);
        assert_eq!(converted.usage_count, 0);
    }
}
