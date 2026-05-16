// =============================================================================
// Shopify Config Encryption Scenarios
// =============================================================================
//
// Test scenarios for Shopify configuration encryption/decryption functionality.
//
// User Stories:
// - docs/user-stories/08-shopify-pay-user-stories.md
//   - US-PP-007: Realm Admin creates and views Shopify configuration
//   - US-PP-008: View Shopify Payment Provider configuration
//   - US-PP-003: Edit Shopify Payment Provider configuration
//
// Covers:
// - Encryption on save (3 sensitive fields)
// - Decryption on read with masking
// - Backward compatibility with legacy plaintext data
// - Encryption/decryption failures handling
// - Performance and concurrent operations
//
// =============================================================================

use crate::application::http::auth::util::{SessionData, store_session};
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::OnceLock;
    use test_context::test_context;

    /// Ensure ENCRYPTION_KEY is set for billing crypto operations.
    /// Must be called before any request that triggers encrypt_secret/decrypt_secret.
    static ENCRYPTION_KEY_SETUP: OnceLock<()> = OnceLock::new();
    fn ensure_encryption_key() {
        ENCRYPTION_KEY_SETUP.get_or_init(|| unsafe {
            std::env::set_var(
                "ENCRYPTION_KEY",
                "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
            );
        });
    }

    // =============================================================================
    // Test Helper Functions
    // =============================================================================

    /// Create a valid Shopify config request for testing
    fn valid_shopify_config_request() -> serde_json::Value {
        json!({
            "shopDomain": "test-shop.myshopify.com",
            "adminAccessToken": "shpat_test_token_12345",
            "storefrontAccessToken": "shp_test_token_67890",
            "appClientSecret": "test_secret_abcde123456789012345",
            "apiVersion": "2024-01",
            "webhookSubscriptionMode": "admin_api",
            "timeout": 30,
            "skipConnectionTest": true
        })
    }

    /// Send POST request to create Shopify configuration
    async fn send_create_shopify_config(
        app: &axum::Router,
        realm_id: &str,
        admin_token: &str,
        config_payload: &serde_json::Value,
    ) -> axum::response::Response {
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/third/pay/{}/providers/shopify", realm_id))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={}", admin_token))
                    .body(Body::from(config_payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    /// Send GET request to retrieve Shopify configuration
    async fn send_get_shopify_config(
        app: &axum::Router,
        realm_id: &str,
        admin_token: &str,
    ) -> axum::response::Response {
        app.clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/third/pay/{}/providers/shopify", realm_id))
                    .header("cookie", format!("X-Auth={}", admin_token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    /// Send PUT request to update Shopify configuration
    async fn send_update_shopify_config(
        app: &axum::Router,
        realm_id: &str,
        admin_token: &str,
        config_payload: &serde_json::Value,
    ) -> axum::response::Response {
        app.clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/third/pay/{}/providers/shopify", realm_id))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={}", admin_token))
                    .body(Body::from(config_payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    /// Get config value directly from database
    async fn get_config_from_db(
        ctx: &TestContext,
        realm_id: &str,
        config_key: &str,
    ) -> Option<String> {
        sqlx::query_scalar(
            "SELECT config_value FROM realm_config
             WHERE realm_id = $1 AND config_type = 'shopify' AND config_key = $2",
        )
        .bind(realm_id)
        .bind(config_key)
        .fetch_optional(&ctx._app_state.pool)
        .await
        .unwrap()
    }

    /// Insert plaintext Shopify config directly into database (for testing backward compatibility)
    async fn insert_plaintext_shopify_config(
        ctx: &TestContext,
        realm_id: &str,
        shop_domain: &str,
        admin_token: &str,
        storefront_token: &str,
        app_secret: &str,
    ) {
        sqlx::query(
            "INSERT INTO realm_config (realm_id, config_type, config_key, config_value, is_secret, enabled, created_at, updated_at)
             VALUES ($1, 'shopify', 'shop_domain', $2, false, true, NOW(), NOW()),
                    ($1, 'shopify', 'admin_access_token', $3, true, true, NOW(), NOW()),
                    ($1, 'shopify', 'storefront_access_token', $4, true, true, NOW(), NOW()),
                    ($1, 'shopify', 'app_client_secret', $5, true, true, NOW(), NOW()),
                    ($1, 'shopify', 'api_version', '2024-01', false, true, NOW(), NOW())"
        )
        .bind(realm_id)
        .bind(shop_domain)
        .bind(admin_token)
        .bind(storefront_token)
        .bind(app_secret)
        .execute(&ctx._app_state.pool)
        .await
        .expect("Failed to insert plaintext config");
    }

    /// Assert that a value appears to be encrypted (not plaintext)
    fn assert_is_encrypted(value: &str, plaintext: &str) {
        // Encrypted value should be different from plaintext
        assert_ne!(
            value, plaintext,
            "Encrypted value should not match plaintext"
        );

        // Encrypted value should be longer (base64 encoded with nonce)
        assert!(
            value.len() > plaintext.len(),
            "Encrypted value should be longer than plaintext"
        );

        // Encrypted value should contain base64 characters
        assert!(
            value.contains('/') || value.contains('+') || value.contains('=') || value.len() > 40,
            "Encrypted value should appear to be base64 encoded"
        );
    }

    /// Assert that sensitive field is masked in response
    fn assert_field_is_masked(field_value: &str, plaintext: &str) {
        assert!(
            field_value.contains("*****") || field_value.contains("***"),
            "Field should be masked, got: {}",
            field_value
        );

        assert!(
            field_value != plaintext,
            "Field should not contain full plaintext value"
        );
    }

    async fn create_realm_admin_for_realm(
        ctx: &TestContext,
        realm_id: &str,
        email: &str,
    ) -> (String, String) {
        sqlx::query(
            "INSERT INTO realm (id, name, created_at, updated_at)
             VALUES ($1, $2, NOW(), NOW())
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(realm_id)
        .bind(format!("realm-{}", &realm_id[..8]))
        .execute(&ctx._app_state.pool)
        .await
        .expect("Failed to create realm");

        let user_id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO account (id, realm_id, email, password, status)
             VALUES ($1, $2, $3, $4, 1)",
        )
        .bind(user_id)
        .bind(realm_id)
        .bind(email)
        .bind("$2a$12$dummy_password_hash")
        .execute(&ctx._app_state.pool)
        .await
        .expect("Failed to create account");

        let role_id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO roles (id, name, description, realm_id, client_id, is_builtin, created_at, updated_at)
             VALUES ($1, 'shopify-test-admin', 'Shopify encryption test admin', $2, $3, false, NOW(), NOW())",
        )
        .bind(role_id)
        .bind(realm_id)
        .bind(&ctx._client_id)
        .execute(&ctx._app_state.pool)
        .await
        .expect("Failed to create role");

        sqlx::query(
            "INSERT INTO role_policies (id, role_id, realm_id, resource, action)
             VALUES ($1, $2, $3, 'realm', 'admin')",
        )
        .bind(Uuid::now_v7())
        .bind(role_id)
        .bind(realm_id)
        .execute(&ctx._app_state.pool)
        .await
        .expect("Failed to create role policy");

        sqlx::query(
            "INSERT INTO user_roles (id, user_id, role_id, realm_id, client_id)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(Uuid::now_v7())
        .bind(user_id)
        .bind(role_id)
        .bind(realm_id)
        .bind(&ctx._client_id)
        .execute(&ctx._app_state.pool)
        .await
        .expect("Failed to create user role");

        let token = ctx.generate_test_token();
        let session = SessionData {
            realm_id: realm_id.to_string(),
            client_id: ctx._client_id.clone(),
            user_id: user_id.to_string(),
            client_ip: "127.0.0.1".to_string(),
            renewal_ttl_seconds: None,
        };
        store_session(&ctx._app_state, &token, &session, 1800)
            .await
            .expect("Failed to store session");

        (token, user_id.to_string())
    }

    // =============================================================================
    // Scenario 1: Configuration is encrypted on save
    // =============================================================================

    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_shopify_config_encrypted_on_save(ctx: &mut TestContext) {
        // User Story: docs/user-stories/08-shopify-pay-user-stories.md
        // Covers: US-PP-007 - Realm Admin creates and views Shopify configuration
        //          验收标准: 敏感凭据必须加密存储

        ensure_encryption_key();
        let app = ctx.create_unified_test_router();
        let realm_id = &ctx._realm_id;

        // Given: Realm Admin user is logged in
        let (admin_token, user_id) =
            crate::tests::helpers::create_admin_session_with_user(ctx, "admin@test.com", 1800)
                .await;
        crate::tests::helpers::grant_realm_admin_role(ctx, &user_id).await;

        // When: POST create Shopify configuration with sensitive tokens
        let config_payload = valid_shopify_config_request();
        let response =
            send_create_shopify_config(&app, realm_id, &admin_token, &config_payload).await;

        assert!(
            response.status() == StatusCode::OK || response.status() == StatusCode::CREATED,
            "Expected 200 OK or 201 Created, got {}",
            response.status(),
        );

        // And: Database values are encrypted (not plaintext)
        let admin_token_db = get_config_from_db(ctx, realm_id, "admin_access_token")
            .await
            .expect("admin_access_token should exist");
        let storefront_token_db = get_config_from_db(ctx, realm_id, "storefront_access_token")
            .await
            .expect("storefront_access_token should exist");
        let app_secret_db = get_config_from_db(ctx, realm_id, "app_client_secret")
            .await
            .expect("app_client_secret should exist");

        // Verify all three sensitive fields are encrypted
        assert_is_encrypted(
            &admin_token_db,
            config_payload["adminAccessToken"].as_str().unwrap(),
        );
        assert_is_encrypted(
            &storefront_token_db,
            config_payload["storefrontAccessToken"].as_str().unwrap(),
        );
        assert_is_encrypted(
            &app_secret_db,
            config_payload["appClientSecret"].as_str().unwrap(),
        );

        // And: Non-sensitive fields are stored as plaintext
        let shop_domain_db = get_config_from_db(ctx, realm_id, "shop_domain")
            .await
            .expect("shop_domain should exist");
        assert_eq!(
            shop_domain_db,
            config_payload["shopDomain"].as_str().unwrap(),
            "Non-sensitive fields should be stored as plaintext"
        );
    }

    // =============================================================================
    // Scenario 2: Configuration is decrypted on read and masked
    // =============================================================================

    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_shopify_config_decrypted_on_read(ctx: &mut TestContext) {
        // User Story: docs/user-stories/08-shopify-pay-user-stories.md
        // Covers: US-PP-008 - View Shopify Payment Provider configuration
        //          验收标准: 可以查看配置，敏感字段脱敏显示

        ensure_encryption_key();
        let app = ctx.create_unified_test_router();
        let realm_id = &ctx._realm_id;

        // Given: Realm Admin user is logged in
        let (admin_token, user_id) =
            crate::tests::helpers::create_admin_session_with_user(ctx, "admin@test.com", 1800)
                .await;
        crate::tests::helpers::grant_realm_admin_role(ctx, &user_id).await;

        // And: An encrypted Shopify configuration exists
        let config_payload = valid_shopify_config_request();
        let create_response =
            send_create_shopify_config(&app, realm_id, &admin_token, &config_payload).await;
        assert!(
            create_response.status() == StatusCode::OK
                || create_response.status() == StatusCode::CREATED
        );

        // When: GET retrieve Shopify configuration
        let get_response = send_get_shopify_config(&app, realm_id, &admin_token).await;

        // Then: Returns 200 OK
        assert_eq!(
            get_response.status(),
            StatusCode::OK,
            "Expected 200 OK, got {}",
            get_response.status()
        );

        // And: Response contains masked sensitive fields
        let body_bytes = to_bytes(get_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_field_is_masked(
            body["adminAccessToken"].as_str().unwrap(),
            config_payload["adminAccessToken"].as_str().unwrap(),
        );
        assert_field_is_masked(
            body["storefrontAccessToken"].as_str().unwrap(),
            config_payload["storefrontAccessToken"].as_str().unwrap(),
        );
        assert_field_is_masked(
            body["appClientSecret"].as_str().unwrap(),
            config_payload["appClientSecret"].as_str().unwrap(),
        );

        // And: Response does not contain plaintext sensitive values
        let admin_token_response = body["adminAccessToken"].as_str().unwrap();
        assert!(
            !admin_token_response.contains("test_token_12345"),
            "Response should not contain full plaintext token"
        );
    }

    // =============================================================================
    // Scenario 3: Encryption/decryption roundtrip
    // =============================================================================

    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_shopify_config_encryption_roundtrip(ctx: &mut TestContext) {
        // User Story: docs/user-stories/08-shopify-pay-user-stories.md
        // Covers: US-PP-007 - Realm Admin creates and views Shopify configuration
        //          验收标准: 敏感凭据加密存储，往返一致性

        ensure_encryption_key();
        let app = ctx.create_unified_test_router();
        let realm_id = &ctx._realm_id;

        // Given: Realm Admin user is logged in
        let (admin_token, user_id) =
            crate::tests::helpers::create_admin_session_with_user(ctx, "admin@test.com", 1800)
                .await;
        crate::tests::helpers::grant_realm_admin_role(ctx, &user_id).await;

        // When: Create initial configuration
        let initial_config = valid_shopify_config_request();
        let create_response =
            send_create_shopify_config(&app, realm_id, &admin_token, &initial_config).await;
        assert!(
            create_response.status() == StatusCode::OK
                || create_response.status() == StatusCode::CREATED
        );

        // Then: Database contains encrypted values
        let encrypted_admin = get_config_from_db(ctx, realm_id, "admin_access_token")
            .await
            .unwrap();
        assert_is_encrypted(&encrypted_admin, "shpat_test_token_12345");

        // When: Read configuration
        let get_response = send_get_shopify_config(&app, realm_id, &admin_token).await;
        assert_eq!(get_response.status(), StatusCode::OK);

        // Then: Response contains masked values
        let body_bytes = to_bytes(get_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_field_is_masked(
            body["adminAccessToken"].as_str().unwrap(),
            initial_config["adminAccessToken"].as_str().unwrap(),
        );

        // When: Update configuration with new values
        let updated_config = json!({
            "shopDomain": "test-shop.myshopify.com",
            "adminAccessToken": "shpat_updated_token_789",
            "storefrontAccessToken": "shp_updated_token_012",
            "appClientSecret": "updated_secret_fghij123456789012345",
            "apiVersion": "2024-01",
            "webhookSubscriptionMode": "admin_api",
            "timeout": 30,
            "skipConnectionTest": true
        });

        let update_response =
            send_update_shopify_config(&app, realm_id, &admin_token, &updated_config).await;
        assert_eq!(update_response.status(), StatusCode::OK);

        // Then: Database contains new encrypted values
        let new_encrypted_admin = get_config_from_db(ctx, realm_id, "admin_access_token")
            .await
            .unwrap();
        assert_is_encrypted(&new_encrypted_admin, "shpat_updated_token_789");
        assert_ne!(
            encrypted_admin, new_encrypted_admin,
            "Encrypted values should be different after update"
        );

        // And: Read again to verify new masked values
        let final_get_response = send_get_shopify_config(&app, realm_id, &admin_token).await;
        assert_eq!(final_get_response.status(), StatusCode::OK);
    }

    // =============================================================================
    // Scenario 4: Backward compatibility with legacy plaintext data
    // =============================================================================

    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_shopify_config_legacy_plaintext_compatible(ctx: &mut TestContext) {
        // User Story: docs/user-stories/08-shopify-pay-user-stories.md
        // Covers: US-PP-008 - View Shopify Payment Provider configuration
        //          验收标准: 可以查看配置（包括历史数据），降级处理

        ensure_encryption_key();
        let app = ctx.create_unified_test_router();
        let realm_id = &ctx._realm_id;

        // Given: Realm Admin user is logged in
        let (admin_token, user_id) =
            crate::tests::helpers::create_admin_session_with_user(ctx, "admin@test.com", 1800)
                .await;
        crate::tests::helpers::grant_realm_admin_role(ctx, &user_id).await;

        // And: Database contains plaintext Shopify config (legacy data)
        insert_plaintext_shopify_config(
            ctx,
            realm_id,
            "legacy-shop.myshopify.com",
            "shpat_legacy_plaintext",
            "shp_legacy_plaintext",
            "legacy_secret_plaintext",
        )
        .await;

        // When: GET retrieve Shopify configuration
        let get_response = send_get_shopify_config(&app, realm_id, &admin_token).await;

        // Then: Returns 200 OK (should not fail with decryption error)
        assert_eq!(
            get_response.status(),
            StatusCode::OK,
            "Legacy plaintext data should be readable with 200 OK"
        );

        // And: Response contains configuration (graceful degradation)
        let body_bytes = to_bytes(get_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(
            body["shopDomain"].as_str().unwrap(),
            "legacy-shop.myshopify.com"
        );

        // And: Sensitive fields are masked (decryption falls back to masking)
        assert_field_is_masked(
            body["adminAccessToken"].as_str().unwrap(),
            "shpat_legacy_plaintext",
        );
        assert_field_is_masked(
            body["storefrontAccessToken"].as_str().unwrap(),
            "shp_legacy_plaintext",
        );
        assert_field_is_masked(
            body["appClientSecret"].as_str().unwrap(),
            "legacy_secret_plaintext",
        );
    }

    // =============================================================================
    // Scenario 5: Update configuration with new encrypted values
    // =============================================================================

    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_shopify_config_update_encrypted(ctx: &mut TestContext) {
        // User Story: docs/user-stories/08-shopify-pay-user-stories.md
        // Covers: US-PP-003 - Edit Shopify Payment Provider configuration
        //          验收标准: 可以更新配置，新值必须加密

        ensure_encryption_key();
        let app = ctx.create_unified_test_router();
        let realm_id = &ctx._realm_id;

        // Given: Realm Admin user is logged in
        let (admin_token, user_id) =
            crate::tests::helpers::create_admin_session_with_user(ctx, "admin@test.com", 1800)
                .await;
        crate::tests::helpers::grant_realm_admin_role(ctx, &user_id).await;

        // And: An encrypted Shopify configuration exists
        let initial_config = valid_shopify_config_request();
        let create_response =
            send_create_shopify_config(&app, realm_id, &admin_token, &initial_config).await;
        assert!(
            create_response.status() == StatusCode::OK
                || create_response.status() == StatusCode::CREATED
        );

        // Get the encrypted value before update
        let old_encrypted_admin = get_config_from_db(ctx, realm_id, "admin_access_token")
            .await
            .unwrap();

        // When: PUT update configuration with new token
        let updated_config = json!({
            "shopDomain": "test-shop.myshopify.com",
            "adminAccessToken": "shpat_new_updated_token",
            "storefrontAccessToken": "shp_new_updated_token",
            "appClientSecret": "new_updated_secret123456789012345",
            "apiVersion": "2024-01",
            "webhookSubscriptionMode": "admin_api",
            "timeout": 30,
            "skipConnectionTest": true
        });

        let update_response =
            send_update_shopify_config(&app, realm_id, &admin_token, &updated_config).await;
        assert_eq!(update_response.status(), StatusCode::OK);

        // Then: Database contains new encrypted value
        let new_encrypted_admin = get_config_from_db(ctx, realm_id, "admin_access_token")
            .await
            .unwrap();

        // Verify it's encrypted (not plaintext)
        assert_is_encrypted(&new_encrypted_admin, "shpat_new_updated_token");

        // Verify it's different from old encrypted value (different nonce)
        assert_ne!(
            old_encrypted_admin, new_encrypted_admin,
            "Updated value should have different encryption (new nonce)"
        );
    }
    // =============================================================================
    // Scenario 8: Concurrent encryption operations
    // =============================================================================

    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_shopify_config_concurrent_encryption(ctx: &mut TestContext) {
        // User Story: docs/user-stories/08-shopify-pay-user-stories.md
        // Covers: US-PP-007 - Realm Admin creates and views Shopify configuration
        //          验收标准: 并发加密操作安全

        ensure_encryption_key();

        let app = ctx.create_unified_test_router();

        // Given: Multiple realms with Realm Admin users
        let mut realms = Vec::new();
        for i in 0..10 {
            let realm_id = Uuid::now_v7().to_string();
            let (admin_token, _user_id) =
                create_realm_admin_for_realm(ctx, &realm_id, &format!("admin{}@test.com", i)).await;
            realms.push((realm_id, admin_token));
        }

        // When: Concurrently create Shopify configurations for all realms
        use tokio::task::JoinSet;

        let mut join_set = JoinSet::new();

        for (realm_id, admin_token) in realms.clone() {
            let app_clone = app.clone();
            let config_payload = valid_shopify_config_request();

            join_set.spawn(async move {
                let response = send_create_shopify_config(
                    &app_clone,
                    &realm_id,
                    &admin_token,
                    &config_payload,
                )
                .await;
                (realm_id, response)
            });
        }

        // Then: All requests succeed
        let mut results = Vec::new();
        while let Some(result) = join_set.join_next().await {
            let (realm_id, response) = result.unwrap();
            assert!(
                response.status() == StatusCode::OK || response.status() == StatusCode::CREATED,
                "Failed to create config for realm {} with status {}",
                realm_id,
                response.status()
            );
            results.push(realm_id);
        }

        // And: All encrypted values are unique (no nonce reuse)
        let mut encrypted_values = std::collections::HashSet::new();

        for realm_id in results {
            let admin_token_db = get_config_from_db(ctx, &realm_id, "admin_access_token")
                .await
                .expect("Should have admin_access_token");

            // Verify it's encrypted
            assert_is_encrypted(&admin_token_db, "shpat_test_token_12345");

            // Collect for uniqueness check
            encrypted_values.insert(admin_token_db);
        }

        assert_eq!(
            encrypted_values.len(),
            10,
            "All encrypted values should be unique (no nonce reuse)"
        );
    }
}
