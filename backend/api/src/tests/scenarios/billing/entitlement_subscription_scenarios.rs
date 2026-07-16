// =============================================================================
// Entitlement Subscription Endpoint Scenario Tests
// =============================================================================
//
// Tests for subscription list and detail endpoints with the new schema
// (entitlement_key, external_price_id, provider_metadata, synced_at).
//
// User Story: US-EM-006 (Subscription Projection List)
// Covers: GET subscriptions list with entitlement_key filter, status filter;
//         GET subscription detail with new fields, not found
//
// =============================================================================

#[cfg(test)]
mod tests {
    use crate::tests::helpers::billing_helpers::setup_billing_admin_session;
    use crate::tests::helpers::subscription_test_helpers::{
        create_test_subscription_full, create_test_subscription_with_entitlement_key,
    };
    use crate::tests::schema_test_context::SchemaTestContext;
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode, header},
    };
    use serde_json::json;
    use test_context::test_context;
    use tower::ServiceExt;
    use uuid::Uuid;

    use SchemaTestContext as SubTestContext;

    // Helper: build request with admin auth cookie
    fn auth_request(method: &str, uri: String, token: &str, body: Option<Body>) -> Request<Body> {
        let mut builder = Request::builder()
            .method(method)
            .uri(uri)
            .header(header::AUTHORIZATION, format!("Bearer {}", token));
        if let Some(b) = body {
            builder = builder.header("Content-Type", "application/json");
            builder.body(b).unwrap()
        } else {
            builder.body(Body::empty()).unwrap()
        }
    }

    // =========================================================================
    // GET /api/bill/{realmId}/subscriptions (US-EM-006)
    // =========================================================================

    /// User Story: US-EM-006
    /// Covers: List returns entitlement_key, no planId/tier/billingPeriod
    #[test_context(SubTestContext)]
    #[tokio::test]
    async fn test_list_subscriptions_returns_entitlement_key(ctx: &mut SubTestContext) {
        let app = ctx.create_unified_test_router();
        let token = setup_billing_admin_session(ctx, "sub-list-ek@test.com").await;
        let realm_id = ctx._realm_id.clone();
        let client_app_id = Uuid::parse_str(&ctx._client_app_id).unwrap();

        // Create a subscription with entitlement_key
        let sub_id = create_test_subscription_with_entitlement_key(
            ctx,
            &realm_id,
            client_app_id,
            "pro-plan",
            "price_abc123",
            "stripe",
            "active",
        )
        .await;

        let response = app
            .clone()
            .oneshot(auth_request(
                "GET",
                format!("/api/bill/{}/subscriptions", realm_id),
                &token,
                None,
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        // Should have items with entitlement_key
        let items = json["items"].as_array().expect("items should be an array");
        assert!(!items.is_empty(), "Should have at least one subscription");

        // Find our subscription
        let sub = items
            .iter()
            .find(|item| item["id"] == sub_id.to_string())
            .expect("Created subscription should appear in list");

        // Verify new fields are present
        assert_eq!(sub["entitlementKey"], "pro-plan");
        assert_eq!(sub["paymentProvider"], "stripe");

        // Verify old fields are NOT present
        assert!(
            sub.get("planId").is_none(),
            "planId should not be in response"
        );
        assert!(sub.get("tier").is_none(), "tier should not be in response");
        assert!(
            sub.get("billingPeriod").is_none(),
            "billingPeriod should not be in response"
        );
    }

    /// User Story: US-EM-006
    /// Covers: Filter by entitlementKey returns matching subscriptions only
    #[test_context(SubTestContext)]
    #[tokio::test]
    async fn test_list_subscriptions_filter_by_entitlement_key(ctx: &mut SubTestContext) {
        let app = ctx.create_unified_test_router();
        let token = setup_billing_admin_session(ctx, "sub-filter-ek@test.com").await;
        let realm_id = ctx._realm_id.clone();
        let client_app_id = Uuid::parse_str(&ctx._client_app_id).unwrap();

        // Create subscriptions with different entitlement_keys
        create_test_subscription_with_entitlement_key(
            ctx,
            &realm_id,
            client_app_id,
            "basic-plan",
            "price_basic",
            "creem",
            "active",
        )
        .await;

        // Use a different client_app_id for the second subscription to avoid UNIQUE conflict
        let client_app_id_2 = Uuid::now_v7();
        // Ensure a user row exists for the second client app
        sqlx::query(
            "INSERT INTO client_apps (id, realm_id, client_id, name, redirect_uris, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, 'test-app-2', '[]', true, NOW(), NOW())
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(client_app_id_2)
        .bind(&realm_id)
        .bind(format!("client-{}", client_app_id_2))
        .execute(&ctx.app_state.pool)
        .await
        .ok(); // Ignore error if already exists

        create_test_subscription_with_entitlement_key(
            ctx,
            &realm_id,
            client_app_id_2,
            "premium-plan",
            "price_premium",
            "stripe",
            "active",
        )
        .await;

        // Filter by entitlementKey=basic-plan
        let response = app
            .clone()
            .oneshot(auth_request(
                "GET",
                format!(
                    "/api/bill/{}/subscriptions?entitlementKey=basic-plan",
                    realm_id
                ),
                &token,
                None,
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        let items = json["items"].as_array().expect("items should be an array");
        assert_eq!(items.len(), 1, "Should return exactly 1 subscription");
        assert_eq!(items[0]["entitlementKey"], "basic-plan");
    }

    /// User Story: US-EM-006
    /// Covers: Filter by status=active returns only active subscriptions
    #[test_context(SubTestContext)]
    #[tokio::test]
    async fn test_list_subscriptions_filter_by_status(ctx: &mut SubTestContext) {
        let app = ctx.create_unified_test_router();
        let token = setup_billing_admin_session(ctx, "sub-filter-status@test.com").await;
        let realm_id = ctx._realm_id.clone();
        let client_app_id = Uuid::parse_str(&ctx._client_app_id).unwrap();

        // Create an active subscription
        create_test_subscription_with_entitlement_key(
            ctx,
            &realm_id,
            client_app_id,
            "active-plan",
            "price_active",
            "creem",
            "active",
        )
        .await;

        // Create a canceled subscription with a different client_app_id
        let client_app_id_2 = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO client_apps (id, realm_id, client_id, name, redirect_uris, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, 'test-app-canceled', '[]', true, NOW(), NOW())
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(client_app_id_2)
        .bind(&realm_id)
        .bind(format!("client-{}", client_app_id_2))
        .execute(&ctx.app_state.pool)
        .await
        .ok();

        create_test_subscription_with_entitlement_key(
            ctx,
            &realm_id,
            client_app_id_2,
            "canceled-plan",
            "price_canceled",
            "creem",
            "canceled",
        )
        .await;

        // Filter by status=active
        let response = app
            .clone()
            .oneshot(auth_request(
                "GET",
                format!("/api/bill/{}/subscriptions?status=active", realm_id),
                &token,
                None,
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        let items = json["items"].as_array().expect("items should be an array");
        // All returned subscriptions should be active
        for item in items {
            assert_eq!(item["status"], "active");
        }
    }

    // =========================================================================
    // GET /api/bill/{realmId}/subscriptions/{subscriptionId} (US-EM-006)
    // =========================================================================

    /// User Story: US-EM-006
    /// Covers: Subscription detail includes entitlement_key, external_price_id,
    ///         provider_metadata, synced_at; no planId/tier/billingPeriod
    #[test_context(SubTestContext)]
    #[tokio::test]
    async fn test_get_subscription_detail_new_fields(ctx: &mut SubTestContext) {
        let app = ctx.create_unified_test_router();
        let token = setup_billing_admin_session(ctx, "sub-detail-new@test.com").await;
        let realm_id = ctx._realm_id.clone();
        let client_app_id = Uuid::parse_str(&ctx._client_app_id).unwrap();

        let metadata = json!({
            "stripe_product_name": "Pro Plan",
            "interval": "month"
        });

        let sub_id = create_test_subscription_full(
            ctx,
            &realm_id,
            client_app_id,
            "detail-plan",
            "price_detail_123",
            "stripe",
            "active",
            Some(metadata.clone()),
        )
        .await;

        let response = app
            .clone()
            .oneshot(auth_request(
                "GET",
                format!("/api/bill/{}/subscriptions/{}", realm_id, sub_id),
                &token,
                None,
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        // Verify new fields
        assert_eq!(json["entitlementKey"], "detail-plan");
        assert_eq!(json["externalPriceId"], "price_detail_123");
        assert_eq!(json["paymentProvider"], "stripe");
        assert_eq!(json["status"], "active");

        // providerMetadata should be present
        assert!(json.get("providerMetadata").is_some());

        // syncedAt should be present
        assert!(
            json.get("syncedAt").is_some(),
            "syncedAt should be present in response"
        );

        // Verify old fields are NOT present
        assert!(
            json.get("planId").is_none(),
            "planId should not be in response"
        );
        assert!(json.get("tier").is_none(), "tier should not be in response");
        assert!(
            json.get("billingPeriod").is_none(),
            "billingPeriod should not be in response"
        );
    }

    /// User Story: US-EM-006
    /// Covers: Non-existent subscription ID returns 404
    #[test_context(SubTestContext)]
    #[tokio::test]
    async fn test_get_subscription_detail_not_found(ctx: &mut SubTestContext) {
        let app = ctx.create_unified_test_router();
        let token = setup_billing_admin_session(ctx, "sub-notfound@test.com").await;
        let realm_id = ctx._realm_id.clone();
        let fake_id = Uuid::now_v7();

        let response = app
            .clone()
            .oneshot(auth_request(
                "GET",
                format!("/api/bill/{}/subscriptions/{}", realm_id, fake_id),
                &token,
                None,
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
