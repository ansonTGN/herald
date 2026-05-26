// =============================================================================
// Shopify Config Scenarios
// =============================================================================
//
// Test scenarios for Shopify configuration storage and masking functionality.
//
// User Stories:
// - docs/user-stories/08-shopify-pay-user-stories.md
//   - US-PP-007: Realm Admin creates and views Shopify configuration
//   - US-PP-008: View Shopify Payment Provider configuration
//   - US-PP-003: Edit Shopify Payment Provider configuration
//
// Covers:
// - Plaintext storage of all config fields
// - Sensitive field masking in API responses
// - Update and concurrent operations
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
    use test_context::test_context;

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

    /// Insert plaintext Shopify config directly into database
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
             VALUES ($1, 'shopify-test-admin', 'Shopify test admin', $2, $3, false, NOW(), NOW())",
        )
        .bind(role_id)
        .bind(realm_id)
        .bind(&ctx._client_id)
        .execute(&ctx._app_state.pool)
        .await
        .expect("Failed to create role");

        let permissions = vec![
            ("realm", "view"),
            ("dashboard", "view"),
            ("audit", "view"),
            ("users", "view"),
            ("users", "manage"),
            ("clients", "view"),
            ("clients", "manage"),
            ("roles", "view"),
            ("roles", "manage"),
            ("permissions", "view"),
            ("permissions", "manage"),
            ("policies", "view"),
            ("policies", "manage"),
            ("settings", "view"),
            ("settings", "manage"),
            ("api_keys", "view"),
            ("api_keys", "manage"),
            ("billing", "view"),
            ("billing", "manage"),
            ("points", "view"),
            ("points", "manage"),
        ];

        for (resource, action) in &permissions {
            sqlx::query(
                "INSERT INTO role_policies (id, role_id, realm_id, resource, action)
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(Uuid::now_v7())
            .bind(role_id)
            .bind(realm_id)
            .bind(resource)
            .bind(action)
            .execute(&ctx._app_state.pool)
            .await
            .expect("Failed to create role policy");
        }

        sqlx::query(
            "INSERT INTO user_roles (id, user_id, role_id, realm_id, client_id, principal_type, principal_id)
             VALUES ($1, $2, $3, $4, $5, $6, $2::text)",
        )
        .bind(Uuid::now_v7())
        .bind(user_id)
        .bind(role_id)
        .bind(realm_id)
        .bind(&ctx._client_id)
        .bind(herald_core::domain::authorization::principal_types::USER)
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
    // Scenario 1: Configuration is stored as plaintext in database
    // =============================================================================

    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_shopify_config_plaintext_storage(ctx: &mut TestContext) {
        // User Story: docs/user-stories/08-shopify-pay-user-stories.md
        // Covers: US-PP-007 - Realm Admin creates and views Shopify configuration

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

        // Then: Database values are stored as plaintext
        let admin_token_db = get_config_from_db(ctx, realm_id, "admin_access_token")
            .await
            .expect("admin_access_token should exist");
        assert_eq!(
            admin_token_db,
            config_payload["adminAccessToken"].as_str().unwrap(),
            "Sensitive fields should be stored as plaintext"
        );

        let storefront_token_db = get_config_from_db(ctx, realm_id, "storefront_access_token")
            .await
            .expect("storefront_access_token should exist");
        assert_eq!(
            storefront_token_db,
            config_payload["storefrontAccessToken"].as_str().unwrap(),
            "Sensitive fields should be stored as plaintext"
        );

        // And: Non-sensitive fields are also stored as plaintext
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
    // Scenario 2: Configuration response masks sensitive fields
    // =============================================================================

    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_shopify_config_masked_on_read(ctx: &mut TestContext) {
        // User Story: docs/user-stories/08-shopify-pay-user-stories.md
        // Covers: US-PP-008 - View Shopify Payment Provider configuration

        let app = ctx.create_unified_test_router();
        let realm_id = &ctx._realm_id;

        // Given: Realm Admin user is logged in
        let (admin_token, user_id) =
            crate::tests::helpers::create_admin_session_with_user(ctx, "admin@test.com", 1800)
                .await;
        crate::tests::helpers::grant_realm_admin_role(ctx, &user_id).await;

        // And: A Shopify configuration exists
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
    // Scenario 3: Configuration update roundtrip
    // =============================================================================

    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_shopify_config_update_roundtrip(ctx: &mut TestContext) {
        // User Story: docs/user-stories/08-shopify-pay-user-stories.md
        // Covers: US-PP-007 - Realm Admin creates and views Shopify configuration

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

        // Then: Database contains plaintext values
        let stored_admin = get_config_from_db(ctx, realm_id, "admin_access_token")
            .await
            .unwrap();
        assert_eq!(stored_admin, "shpat_test_token_12345");

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

        // Then: Database contains new plaintext values
        let new_stored_admin = get_config_from_db(ctx, realm_id, "admin_access_token")
            .await
            .unwrap();
        assert_eq!(new_stored_admin, "shpat_updated_token_789");

        // And: Read again to verify masked values
        let final_get_response = send_get_shopify_config(&app, realm_id, &admin_token).await;
        assert_eq!(final_get_response.status(), StatusCode::OK);
    }

    // =============================================================================
    // Scenario 4: Direct database insert is readable
    // =============================================================================

    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_shopify_config_direct_db_readable(ctx: &mut TestContext) {
        // User Story: docs/user-stories/08-shopify-pay-user-stories.md
        // Covers: US-PP-008 - View Shopify Payment Provider configuration

        let app = ctx.create_unified_test_router();
        let realm_id = &ctx._realm_id;

        // Given: Realm Admin user is logged in
        let (admin_token, user_id) =
            crate::tests::helpers::create_admin_session_with_user(ctx, "admin@test.com", 1800)
                .await;
        crate::tests::helpers::grant_realm_admin_role(ctx, &user_id).await;

        // And: Database contains directly inserted Shopify config
        insert_plaintext_shopify_config(
            ctx,
            realm_id,
            "direct-shop.myshopify.com",
            "shpat_direct_insert",
            "shp_direct_insert",
            "direct_secret_plaintext",
        )
        .await;

        // When: GET retrieve Shopify configuration
        let get_response = send_get_shopify_config(&app, realm_id, &admin_token).await;

        // Then: Returns 200 OK
        assert_eq!(
            get_response.status(),
            StatusCode::OK,
            "Direct DB insert should be readable"
        );

        // And: Response contains configuration
        let body_bytes = to_bytes(get_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(
            body["shopDomain"].as_str().unwrap(),
            "direct-shop.myshopify.com"
        );

        // And: Sensitive fields are masked
        assert_field_is_masked(
            body["adminAccessToken"].as_str().unwrap(),
            "shpat_direct_insert",
        );
    }

    // =============================================================================
    // Scenario 5: Concurrent operations
    // =============================================================================

    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_shopify_config_concurrent_operations(ctx: &mut TestContext) {
        // User Story: docs/user-stories/08-shopify-pay-user-stories.md
        // Covers: US-PP-007 - Realm Admin creates and views Shopify configuration

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

        // And: All stored values are correct
        for realm_id in results {
            let admin_token_db = get_config_from_db(ctx, &realm_id, "admin_access_token")
                .await
                .expect("Should have admin_access_token");

            assert_eq!(
                admin_token_db, "shpat_test_token_12345",
                "Stored value should match plaintext"
            );
        }
    }
}
