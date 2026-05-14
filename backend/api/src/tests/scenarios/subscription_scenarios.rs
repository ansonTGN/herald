/// 场景测试：订阅管理 (简化版 - 仅查看和取消)
///
/// 测试 Realm Admin 查看、取消 Client App 订阅的流程
#[cfg(test)]
mod tests {
    use crate::tests::helpers::*;
    use crate::tests::schema_test_context::SchemaTestContext;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
        http::StatusCode,
    };
    use serde_json::json;
    use test_context::test_context;
    use tower::ServiceExt;
    use uuid::Uuid;

    use SchemaTestContext as SubscriptionTestContext;

    // All billing-specific helpers are now in billing_helpers module

    /// 场景测试：Realm Admin 查看和管理订阅
    #[test_context(SubscriptionTestContext)]
    #[tokio::test]
    async fn test_scenario_realm_admin_manages_subscriptions(ctx: &mut SubscriptionTestContext) {
        let app = ctx.create_unified_test_router();

        let admin_token =
            setup_billing_admin_session(ctx, "test-subscription-admin@test.com").await;
        let realm_id = ctx._realm_id.clone();

        // Step 1: Create Plan
        let plan_id = create_test_plan(ctx, &realm_id, "Basic Plan").await;

        // Step 2: Create Client App
        let (_client_id_str, client_app_id) = create_client_app_via_api(
            ctx,
            &app,
            &admin_token,
            &realm_id,
            "test-subscription-app",
            "Test Subscription App",
        )
        .await;

        // Step 3: Create subscription
        let subscription_id =
            create_test_subscription(ctx, &realm_id, client_app_id, plan_id, "monthly").await;

        // Step 4: View subscription
        let subscription_json =
            get_subscription(&app, &admin_token, &realm_id, &client_app_id).await;

        assert_eq!(
            subscription_json["id"],
            serde_json::json!(subscription_id.to_string())
        );
        assert_eq!(
            subscription_json["clientAppId"],
            serde_json::json!(client_app_id.to_string())
        );
        assert_eq!(
            subscription_json["planId"],
            serde_json::json!(plan_id.to_string())
        );
        assert_eq!(subscription_json["billingPeriod"], "monthly");
        assert_eq!(subscription_json["status"], "active");

        // Step 5: Cancel subscription (immediate)
        let cancel_json = cancel_subscription(
            &app,
            &admin_token,
            &realm_id,
            &client_app_id,
            false, // cancel_at_period_end
        )
        .await;

        assert_eq!(
            cancel_json["subscriptionId"],
            serde_json::json!(subscription_id.to_string())
        );
        assert!(cancel_json["canceledAt"].is_string());

        // Step 6: Verify subscription is canceled
        let canceled_json = get_subscription(&app, &admin_token, &realm_id, &client_app_id).await;

        assert_eq!(canceled_json["status"], "canceled");

        // Cleanup
        delete_test_subscription(ctx, subscription_id).await;
    }

    /// 场景测试：取消订阅 (在计费周期结束时)
    #[test_context(SubscriptionTestContext)]
    #[tokio::test]
    async fn test_scenario_cancel_subscription_at_period_end(ctx: &mut SubscriptionTestContext) {
        let app = ctx.create_unified_test_router();

        let admin_token = setup_billing_admin_session(ctx, "test-period-cancel@test.com").await;
        let realm_id = ctx._realm_id.clone();

        // Create Plan and Client App
        let plan_id = create_test_plan(ctx, &realm_id, "Period Plan").await;
        let (_client_id_str, client_app_id) = create_client_app_via_api(
            ctx,
            &app,
            &admin_token,
            &realm_id,
            "test-period-app",
            "Test Period App",
        )
        .await;

        // Create subscription
        let subscription_id =
            create_test_subscription(ctx, &realm_id, client_app_id, plan_id, "monthly").await;

        // Cancel subscription at period end
        let cancel_json = cancel_subscription(
            &app,
            &admin_token,
            &realm_id,
            &client_app_id,
            true, // cancel_at_period_end
        )
        .await;

        assert_eq!(
            cancel_json["subscriptionId"],
            serde_json::json!(subscription_id.to_string())
        );
        assert_eq!(
            cancel_json["message"],
            "Subscription will be canceled at the end of the billing period"
        );

        // Verify subscription is still active
        let still_active_json =
            get_subscription(&app, &admin_token, &realm_id, &client_app_id).await;

        assert_eq!(still_active_json["status"], "active");
        assert!(!still_active_json["cancelAt"].is_null());

        // Cleanup
        delete_test_subscription(ctx, subscription_id).await;
    }

    /// 场景测试：非管理员无法查看其他应用的订阅
    ///
    /// Tests that users without proper permissions cannot view subscriptions.
    #[test_context(SubscriptionTestContext)]
    #[tokio::test]
    async fn test_scenario_cross_realm_subscription_isolation(ctx: &mut SubscriptionTestContext) {
        let app = ctx.create_unified_test_router();

        // Setup admin and subscription
        let admin_token = setup_billing_admin_session(ctx, "admin1-sub@test.com").await;
        let realm_id = ctx._realm_id.clone();

        let plan1_id = create_test_plan(ctx, &realm_id, "Realm Plan").await;
        let (_client_id_1_str, client_app1_id) =
            create_client_app_via_api(ctx, &app, &admin_token, &realm_id, "realm-app", "Realm App")
                .await;

        let _subscription1_id =
            create_test_subscription(ctx, &realm_id, client_app1_id, plan1_id, "monthly").await;

        // Setup non-admin user without billing permissions
        let (regular_token, _user2_id) =
            create_admin_session_with_user(ctx, "regular-user@test.com", 1800).await;

        // Regular user without billing permissions attempts to view subscription
        // Even though they're in the same realm, they lack the required billing.view permission
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!(
                        "/api/bill/{}/client/{}/subscription",
                        realm_id, client_app1_id
                    ))
                    .header("cookie", format!("X-Auth={}", regular_token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Users without billing.view permission should be denied access
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        // Cleanup
        delete_subscriptions_by_client_app(ctx, client_app1_id).await;
    }

    /// 场景测试：查看不存在的订阅
    #[test_context(SubscriptionTestContext)]
    #[tokio::test]
    async fn test_scenario_view_nonexistent_subscription(ctx: &mut SubscriptionTestContext) {
        let app = ctx.create_unified_test_router();

        let admin_token = setup_billing_admin_session(ctx, "test-no-subscription@test.com").await;
        let realm_id = ctx._realm_id.clone();

        // Create Client App without subscription
        let (_client_id_str, client_app_id) = create_client_app_via_api(
            ctx,
            &app,
            &admin_token,
            &realm_id,
            "test-no-subscription-app",
            "App without subscription",
        )
        .await;

        // Attempt to view nonexistent subscription
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!(
                        "/api/bill/{}/client/{}/subscription",
                        realm_id, client_app_id
                    ))
                    .header("cookie", format!("X-Auth={}", admin_token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should return 404 Not Found
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
    // =============================================================================
    // Local Helper Functions
    // =============================================================================

    /// Create a client app via API and return its ID (client_id string and UUID)
    async fn create_client_app_via_api(
        _ctx: &SubscriptionTestContext,
        app: &axum::Router,
        token: &str,
        realm_id: &str,
        client_id: &str,
        name: &str,
    ) -> (String, Uuid) {
        let request = Request::builder()
            .method("POST")
            .uri(format!("/api/client/{}", realm_id))
            .header("content-type", "application/json")
            .header("cookie", format!("X-Auth={}", token))
            .body(Body::from(
                json!({
                    "clientId": client_id,
                    "name": name,
                    "redirectUris": ["https://example.com/callback"],
                    "enabled": true
                })
                .to_string(),
            ))
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let uuid: Uuid = serde_json::from_value(json["id"].clone()).unwrap();
        (client_id.to_string(), uuid)
    }

    /// Get subscription for a client app
    async fn get_subscription(
        app: &axum::Router,
        token: &str,
        realm_id: &str,
        client_app_id: &Uuid,
    ) -> serde_json::Value {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!(
                        "/api/bill/{}/client/{}/subscription",
                        realm_id, client_app_id
                    ))
                    .header("cookie", format!("X-Auth={}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    /// Cancel a subscription
    async fn cancel_subscription(
        app: &axum::Router,
        token: &str,
        realm_id: &str,
        client_app_id: &Uuid,
        cancel_at_period_end: bool,
    ) -> serde_json::Value {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/bill/{}/client/{}/subscription/cancel",
                        realm_id, client_app_id
                    ))
                    .header("content-type", "application/json")
                    .header("cookie", format!("X-Auth={}", token))
                    .body(Body::from(
                        json!({"cancelAtPeriodEnd": cancel_at_period_end}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&body).unwrap()
    }
}
