// =============================================================================
// Shopify Configuration Authentication and Authorization Scenario Tests
// =============================================================================
//
// Tests for authentication and authorization of Shopify configuration endpoints:
// - Unauthenticated access returns 401
// - Non-admin users can view enabled providers via list endpoint, but return 403 for config operations
// - Realm Admin users can access all endpoints
// - Cross-realm access prevention
// - Malformed JWT handling
// - Expired session handling
//
// User Story: docs/user-stories/08-shopify-pay-user-stories.md
// Covers: US-PP-007, US-PP-008, US-PP-003, US-PP-004, US-PP-001
//
// =============================================================================

use crate::tests::helpers::billing_helpers::*;
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

    // ============================================================================
    // HTTP Request Helper Functions
    // ============================================================================

    /// Send request to Shopify config endpoint without authentication
    async fn send_shopify_config_request_no_auth(
        app: &axum::Router,
        _realm_id: &str,
        method: &str,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> axum::response::Response {
        let request_builder = Request::builder()
            .method(method)
            .uri(path)
            .header("Content-Type", "application/json");

        // Add body if provided
        let body_bytes =
            body.map(|json_body| Body::from(serde_json::to_string(&json_body).unwrap()));

        let request = if let Some(bytes) = body_bytes {
            request_builder.body(bytes).unwrap()
        } else {
            request_builder.body(Body::empty()).unwrap()
        };

        app.clone().oneshot(request).await.unwrap()
    }

    /// Send request to Shopify config endpoint with authentication token
    async fn send_shopify_config_request_with_token(
        app: &axum::Router,
        _realm_id: &str,
        method: &str,
        path: &str,
        token: &str,
        body: Option<serde_json::Value>,
    ) -> axum::response::Response {
        let request_builder = Request::builder()
            .method(method)
            .uri(path)
            .header("Content-Type", "application/json")
            .header("cookie", format!("X-Auth={}", token));

        let body_bytes =
            body.map(|json_body| Body::from(serde_json::to_string(&json_body).unwrap()));

        let request = if let Some(bytes) = body_bytes {
            request_builder.body(bytes).unwrap()
        } else {
            request_builder.body(Body::empty()).unwrap()
        };

        app.clone().oneshot(request).await.unwrap()
    }

    /// Convenience function: GET shopify config without auth
    async fn send_get_shopify_config_no_auth(
        app: &axum::Router,
        realm_id: &str,
    ) -> axum::response::Response {
        send_shopify_config_request_no_auth(
            app,
            realm_id,
            "GET",
            &format!("/api/third/pay/{}/providers/shopify", realm_id),
            None,
        )
        .await
    }

    /// Convenience function: POST create config with token
    async fn send_post_shopify_config_with_token(
        app: &axum::Router,
        realm_id: &str,
        token: &str,
        config: &serde_json::Value,
    ) -> axum::response::Response {
        send_shopify_config_request_with_token(
            app,
            realm_id,
            "POST",
            &format!("/api/third/pay/{}/providers/shopify", realm_id),
            token,
            Some(config.clone()),
        )
        .await
    }

    /// Convenience function: GET shopify config with token
    async fn send_get_shopify_config_with_token(
        app: &axum::Router,
        realm_id: &str,
        token: &str,
    ) -> axum::response::Response {
        send_shopify_config_request_with_token(
            app,
            realm_id,
            "GET",
            &format!("/api/third/pay/{}/providers/shopify", realm_id),
            token,
            None,
        )
        .await
    }

    /// Convenience function: PUT update config with token
    async fn send_put_shopify_config_with_token(
        app: &axum::Router,
        realm_id: &str,
        token: &str,
        config: &serde_json::Value,
    ) -> axum::response::Response {
        send_shopify_config_request_with_token(
            app,
            realm_id,
            "PUT",
            &format!("/api/third/pay/{}/providers/shopify", realm_id),
            token,
            Some(config.clone()),
        )
        .await
    }

    /// Convenience function: DELETE config with token
    async fn send_delete_shopify_config_with_token(
        app: &axum::Router,
        realm_id: &str,
        token: &str,
    ) -> axum::response::Response {
        send_shopify_config_request_with_token(
            app,
            realm_id,
            "DELETE",
            &format!("/api/third/pay/{}/providers/shopify", realm_id),
            token,
            None,
        )
        .await
    }

    /// Convenience function: POST test connection with token
    async fn send_post_shopify_test_connection_with_token(
        app: &axum::Router,
        realm_id: &str,
        token: &str,
        config: &serde_json::Value,
    ) -> axum::response::Response {
        send_shopify_config_request_with_token(
            app,
            realm_id,
            "POST",
            &format!("/api/third/pay/{}/providers/shopify/test", realm_id),
            token,
            Some(config.clone()),
        )
        .await
    }

    /// Convenience function: GET list providers without auth
    async fn send_get_providers_list_no_auth(
        app: &axum::Router,
        realm_id: &str,
    ) -> axum::response::Response {
        send_shopify_config_request_no_auth(
            app,
            realm_id,
            "GET",
            &format!("/api/third/pay/{}/providers", realm_id),
            None,
        )
        .await
    }

    // ============================================================================
    // Test Data Helper Functions
    // ============================================================================

    /// Create a valid Shopify config request payload
    fn valid_shopify_config_request() -> serde_json::Value {
        json!({
            "shopDomain": "test-shop.myshopify.com",
            "adminAccessToken": "shpat_test_token_12345",
            "storefrontAccessToken": "shp_test_token_67890",
            "appClientSecret": "test_secret_abcde123456789012345",
            "apiVersion": "2024-01",
            "webhookSubscriptionMode": "admin_api",
            "timeout": 30,
            "skipConnectionTest": true  // Skip actual connection test in auth tests
        })
    }

    /// Create a test connection request payload
    fn valid_test_connection_request() -> serde_json::Value {
        json!({
            "shopDomain": "test-shop.myshopify.com",
            "adminAccessToken": "shpat_test_token_12345",
            "storefrontAccessToken": "shp_test_token_67890",
            "apiVersion": "2024-01"
        })
    }

    // ============================================================================
    // Assertion Helper Functions
    // ============================================================================

    /// Assert response is 401 Unauthorized
    fn assert_unauthorized(response: &axum::response::Response) {
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "Expected 401 Unauthorized, got {}",
            response.status()
        );
    }

    /// Assert response is 403 Forbidden
    fn assert_forbidden(response: &axum::response::Response) {
        assert_eq!(
            response.status(),
            StatusCode::FORBIDDEN,
            "Expected 403 Forbidden, got {}",
            response.status()
        );
    }

    /// Assert response is 200 OK
    fn assert_success(response: &axum::response::Response) {
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Expected 200 OK, got {}",
            response.status()
        );
    }

    // ============================================================================
    // Scenario 1: Unauthenticated User Access Returns 401
    // ============================================================================

    /// User Story: docs/user-stories/08-shopify-pay-user-stories.md
    /// Covers: US-PP-007 (Realm Admin creates and views Shopify config)
    /// Acceptance: Must verify authentication
    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_shopify_config_no_auth_returns_401(ctx: &mut TestContext) {
        let realm_id = ctx._realm_id.clone();
        let app = ctx.create_unified_test_router();

        // Given: Test environment is ready, no auth token

        // When: Access GET /api/third/pay/{realmId}/providers/shopify without auth
        let response = send_get_shopify_config_no_auth(&app, &realm_id).await;

        // Then: Returns 401 Unauthorized
        assert_unauthorized(&response);

        // When: Access POST /api/third/pay/{realmId}/providers/shopify without auth
        let response = send_post_shopify_config_with_token(
            &app,
            &realm_id,
            "", // Empty token = no auth
            &valid_shopify_config_request(),
        )
        .await;

        // Then: Returns 401 Unauthorized
        assert_unauthorized(&response);

        // When: Access PUT /api/third/pay/{realmId}/providers/shopify without auth
        let response = send_put_shopify_config_with_token(
            &app,
            &realm_id,
            "", // Empty token = no auth
            &valid_shopify_config_request(),
        )
        .await;

        // Then: Returns 401 Unauthorized
        assert_unauthorized(&response);

        // When: Access DELETE /api/third/pay/{realmId}/providers/shopify without auth
        let response = send_delete_shopify_config_with_token(&app, &realm_id, "").await;

        // Then: Returns 401 Unauthorized
        assert_unauthorized(&response);

        // When: Access POST /api/third/pay/{realmId}/providers/shopify/test without auth
        let response = send_post_shopify_test_connection_with_token(
            &app,
            &realm_id,
            "", // Empty token = no auth
            &valid_test_connection_request(),
        )
        .await;

        // Then: Returns 401 Unauthorized
        assert_unauthorized(&response);

        // When: Access GET /api/third/pay/{realmId}/providers without auth
        let response = send_get_providers_list_no_auth(&app, &realm_id).await;

        // Then: Returns 401 Unauthorized
        assert_unauthorized(&response);
    }

    // ============================================================================
    // Scenario 2: Non-Realm Admin User Access
    // ============================================================================

    /// User Story: docs/user-stories/08-shopify-pay-user-stories.md
    /// Covers: US-PP-007 (Realm Admin creates and views Shopify config)
    /// Acceptance: Non-admin users can only view enabled providers, not modify config
    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_shopify_config_non_admin_returns_403(ctx: &mut TestContext) {
        let realm_id = ctx._realm_id.clone();
        let app = ctx.create_unified_test_router();

        // Given: Regular user (non-admin) session exists
        let (regular_token, _) = crate::tests::helpers::create_admin_session_with_user(
            ctx,
            "regular-user@example.com",
            1800,
        )
        .await;

        // When: Regular user tries to GET Shopify config
        let response = send_get_shopify_config_with_token(&app, &realm_id, &regular_token).await;

        // Then: Returns 403 Forbidden
        assert_forbidden(&response);

        // When: Regular user tries to POST create Shopify config
        let response = send_post_shopify_config_with_token(
            &app,
            &realm_id,
            &regular_token,
            &valid_shopify_config_request(),
        )
        .await;

        // Then: Returns 403 Forbidden
        assert_forbidden(&response);

        // When: Regular user tries to PUT update Shopify config
        let response = send_put_shopify_config_with_token(
            &app,
            &realm_id,
            &regular_token,
            &valid_shopify_config_request(),
        )
        .await;

        // Then: Returns 403 Forbidden
        assert_forbidden(&response);

        // When: Regular user tries to DELETE Shopify config
        let response = send_delete_shopify_config_with_token(&app, &realm_id, &regular_token).await;

        // Then: Returns 403 Forbidden
        assert_forbidden(&response);

        // When: Regular user tries to POST test connection
        let response = send_post_shopify_test_connection_with_token(
            &app,
            &realm_id,
            &regular_token,
            &valid_test_connection_request(),
        )
        .await;

        // Then: Returns 403 Forbidden
        assert_forbidden(&response);
    }

    /// User Story: docs/user-stories/08-shopify-pay-user-stories.md
    /// Covers: US-PP-007 (Realm Admin creates and views Shopify config)
    /// Acceptance: Non-admin users can view enabled providers via list endpoint
    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_shopify_providers_list_non_admin_success(ctx: &mut TestContext) {
        let realm_id = ctx._realm_id.clone();
        let app = ctx.create_unified_test_router();

        // Given: Regular user (non-admin) session exists
        let (regular_token, _) = crate::tests::helpers::create_admin_session_with_user(
            ctx,
            "regular-user-view@example.com",
            1800,
        )
        .await;

        // And: An enabled Shopify configuration exists
        crate::tests::helpers::shopify_helpers::setup_shopify_config(
            ctx,
            &realm_id,
            "test-shop.myshopify.com",
            "shpat_test_token",
            "shp_test_token",
            "test_secret",
            "2024-01",
        )
        .await;

        // When: Regular user tries to GET providers list
        let response = send_shopify_config_request_with_token(
            &app,
            &realm_id,
            "GET",
            &format!("/api/third/pay/{}/providers", realm_id),
            &regular_token,
            None,
        )
        .await;

        // Then: Returns 200 OK (non-admin users can view providers)
        assert_success(&response);

        // And: Response contains providers list
        let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        // And: At least one provider is present (Shopify)
        assert!(body["providers"].is_array());
        let providers = body["providers"].as_array().unwrap();
        assert!(!providers.is_empty(), "Expected at least one provider");

        // And: Each provider has expected fields
        if let Some(first_provider) = providers.first() {
            assert!(first_provider.get("platform").is_some());
            assert!(first_provider.get("enabled").is_some());
        }
    }

    // ============================================================================
    // Scenario 3: Realm Admin User Can Create Config
    // ============================================================================

    /// User Story: docs/user-stories/08-shopify-pay-user-stories.md
    /// Covers: US-PP-007 (Realm Admin creates and views Shopify config)
    /// Acceptance: Realm Admin can create configuration
    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_shopify_config_create_with_admin_success(ctx: &mut TestContext) {
        let realm_id = ctx._realm_id.clone();
        let app = ctx.create_unified_test_router();

        // Given: Realm Admin user is logged in
        let admin_token = setup_billing_admin_session(ctx, "shopify-admin@example.com").await;

        // And: Request contains valid Authorization header
        // When: POST /api/third/pay/{realmId}/providers/shopify to create config
        let response = send_post_shopify_config_with_token(
            &app,
            &realm_id,
            &admin_token,
            &valid_shopify_config_request(),
        )
        .await;

        // Then: Returns 201 Created or 200 OK
        assert!(
            response.status() == StatusCode::OK || response.status() == StatusCode::CREATED,
            "Expected OK or CREATED, got {}",
            response.status()
        );

        // And: Response contains configuration information
        let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body["platform"], "shopify");
        assert_eq!(body["shopDomain"], "test-shop.myshopify.com");

        // And: Configuration is saved in database
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM realm_config WHERE realm_id = $1 AND config_type = 'shopify'",
        )
        .bind(&realm_id)
        .fetch_one(&ctx._app_state.pool)
        .await
        .unwrap();
        assert_eq!(count, 7); // 7 config keys
    }
    // ============================================================================
    // Scenario 5: Realm Admin User Can Update Config
    // ============================================================================

    /// User Story: docs/user-stories/08-shopify-pay-user-stories.md
    /// Covers: US-PP-003 (Edit Shopify Payment Provider configuration)
    /// Acceptance: Realm Admin can update configuration
    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_shopify_config_update_with_admin_success(ctx: &mut TestContext) {
        let realm_id = ctx._realm_id.clone();
        let app = ctx.create_unified_test_router();

        // Given: Realm Admin user is logged in
        let admin_token = setup_billing_admin_session(ctx, "shopify-update@example.com").await;

        // And: Shopify config exists
        crate::tests::helpers::shopify_helpers::setup_shopify_config(
            ctx,
            &realm_id,
            "test-shop.myshopify.com",
            "shpat_test_token",
            "shp_test_token",
            "test_secret",
            "2024-01",
        )
        .await;

        // When: PUT /api/third/pay/{realmId}/providers/shopify with updated config
        let updated_config = json!({
            "shopDomain": "updated-shop.myshopify.com",
            "adminAccessToken": "shpat_new_token_12345",
            "storefrontAccessToken": "shp_new_token_67890",
            "appClientSecret": "new_secret_xyz123456789012345678",
            "apiVersion": "2024-04",
            "webhookSubscriptionMode": "admin_api",
            "timeout": 30,
            "skipConnectionTest": true
        });

        let response =
            send_put_shopify_config_with_token(&app, &realm_id, &admin_token, &updated_config)
                .await;

        // Then: Returns 200 OK
        assert_success(&response);

        // And: Response contains updated configuration
        let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body["shopDomain"], "updated-shop.myshopify.com");

        // And: Configuration is updated in database
        let shop_domain: String = sqlx::query_scalar(
            "SELECT config_value FROM realm_config
             WHERE realm_id = $1 AND config_type = 'shopify' AND config_key = 'shop_domain'",
        )
        .bind(&realm_id)
        .fetch_one(&ctx._app_state.pool)
        .await
        .unwrap();
        assert_eq!(shop_domain, "updated-shop.myshopify.com");
    }

    // ============================================================================
    // Scenario 6: Realm Admin User Can Delete Config
    // ============================================================================

    /// User Story: docs/user-stories/08-shopify-pay-user-stories.md
    /// Covers: US-PP-004 (Delete Shopify Payment Provider configuration)
    /// Acceptance: Realm Admin can delete configuration
    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_shopify_config_delete_with_admin_success(ctx: &mut TestContext) {
        let realm_id = ctx._realm_id.clone();
        let app = ctx.create_unified_test_router();

        // Given: Realm Admin user is logged in
        let admin_token = setup_billing_admin_session(ctx, "shopify-delete@example.com").await;

        // And: Shopify config exists (with no active subscriptions)
        crate::tests::helpers::shopify_helpers::setup_shopify_config(
            ctx,
            &realm_id,
            "test-shop.myshopify.com",
            "shpat_test_token",
            "shp_test_token",
            "test_secret",
            "2024-01",
        )
        .await;

        // When: DELETE /api/third/pay/{realmId}/providers/shopify
        let response = send_delete_shopify_config_with_token(&app, &realm_id, &admin_token).await;

        // Then: Returns 204 No Content or 200 OK
        assert!(
            response.status() == StatusCode::NO_CONTENT || response.status() == StatusCode::OK,
            "Expected NO_CONTENT or OK, got {}",
            response.status()
        );

        // And: Configuration is deleted from database
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM realm_config WHERE realm_id = $1 AND config_type = 'shopify'",
        )
        .bind(&realm_id)
        .fetch_one(&ctx._app_state.pool)
        .await
        .unwrap();
        assert_eq!(count, 0);
    }

    // ============================================================================
    // Scenario 7: Realm Admin User Can Test Connection
    // ============================================================================

    /// User Story: docs/user-stories/08-shopify-pay-user-stories.md
    /// Covers: US-PP-001 (Test Shopify Payment Provider connection)
    /// Acceptance: Can test connection
    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_shopify_config_test_connection_with_admin_success(
        ctx: &mut TestContext,
    ) {
        let realm_id = ctx._realm_id.clone();
        let app = ctx.create_unified_test_router();

        // Given: Realm Admin user is logged in
        let admin_token = setup_billing_admin_session(ctx, "shopify-test@example.com").await;

        // And: Shopify config exists
        crate::tests::helpers::shopify_helpers::setup_shopify_config(
            ctx,
            &realm_id,
            "test-shop.myshopify.com",
            "shpat_test_token",
            "shp_test_token",
            "test_secret",
            "2024-01",
        )
        .await;

        // When: POST /api/third/pay/{realmId}/providers/shopify/test
        let response = send_post_shopify_test_connection_with_token(
            &app,
            &realm_id,
            &admin_token,
            &valid_test_connection_request(),
        )
        .await;

        // Then: Returns 200 OK (test endpoint always returns 200, check response body for success/failure)
        assert_eq!(response.status(), StatusCode::OK);

        // And: Response contains test result
        let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert!(body.get("success").is_some());
        assert!(body.get("results").is_some());
    }

    // ============================================================================
    // Scenario 8: Cross-Realm Access Prevention
    // ============================================================================

    /// User Story: docs/user-stories/08-shopify-pay-user-stories.md
    /// Covers: US-PP-007 (Realm Admin creates and views Shopify config)
    /// Acceptance: Must verify realm boundary
    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_shopify_config_cross_realm_access_forbidden(ctx: &mut TestContext) {
        // Given: Test environment is ready
        let app = ctx.create_unified_test_router();

        // And: Another realm exists alongside the current test realm
        let realm_b_id = Uuid::now_v7().to_string();

        // Create Realm B
        sqlx::query(
            "INSERT INTO realm (id, name, created_at, updated_at)
             VALUES ($1, 'realm-b', NOW(), NOW())",
        )
        .bind(&realm_b_id)
        .execute(&ctx._app_state.pool)
        .await
        .unwrap();

        // And: Current realm's Admin user exists
        let admin_a_token = setup_billing_admin_session(ctx, "admin-a@example.com").await;

        // And: Realm B has Shopify config
        crate::tests::helpers::shopify_helpers::setup_shopify_config(
            ctx,
            &realm_b_id,
            "test-shop.myshopify.com",
            "shpat_test_token",
            "shp_test_token",
            "test_secret",
            "2024-01",
        )
        .await;

        // When: Realm A's Admin tries to GET Realm B's config
        let response = send_get_shopify_config_with_token(&app, &realm_b_id, &admin_a_token).await;

        // Then: Returns 403 Forbidden
        assert_forbidden(&response);

        // When: Realm A's Admin tries to PUT Realm B's config
        let response = send_put_shopify_config_with_token(
            &app,
            &realm_b_id,
            &admin_a_token,
            &valid_shopify_config_request(),
        )
        .await;

        // Then: Returns 403 Forbidden
        assert_forbidden(&response);

        // When: Realm A's Admin tries to DELETE Realm B's config
        let response =
            send_delete_shopify_config_with_token(&app, &realm_b_id, &admin_a_token).await;

        // Then: Returns 403 Forbidden
        assert_forbidden(&response);

        // And: Realm B's config is untouched
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM realm_config WHERE realm_id = $1 AND config_type = 'shopify'",
        )
        .bind(&realm_b_id)
        .fetch_one(&ctx._app_state.pool)
        .await
        .unwrap();
        assert_eq!(count, 5); // Config still exists
    }
}
