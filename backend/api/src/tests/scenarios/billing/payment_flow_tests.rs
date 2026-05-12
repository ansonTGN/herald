// =============================================================================
// Payment Flow Tests
// =============================================================================
//
// Test checkout creation, payment completion, and subscription activation.
//
// User Story: docs/user-stories/06-billing-user-stories.md
// Covers: Checkout creation, metadata preservation, payment → subscription flow
//
// =============================================================================

#[cfg(test)]
mod tests {
    use crate::tests::helpers::creem_mocks::*;
    use crate::tests::helpers::payment_assertions::*;
    use crate::tests::helpers::points_helpers::*;
    use crate::tests::helpers::webhook_helpers::*;
    use crate::tests::helpers::*;
    use crate::tests::scenarios::points::fixtures::*;
    use crate::tests::schema_test_context::SchemaTestContext;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use herald_core::domain::billing::entities::SubscriptionStatus;
    use serde_json::json;
    use test_context::test_context;
    use tower::ServiceExt;
    use uuid::Uuid;

    use SchemaTestContext as PaymentFlowTestContext;

    // Explicitly import create_test_plan to avoid ambiguity
    use crate::tests::helpers::billing_helpers::create_test_plan;

    /// Helper: Setup Creem config for a realm
    async fn setup_creem_config(ctx: &PaymentFlowTestContext, realm_id: &str) {
        ctx.with_creem_config(
            realm_id,
            Some("test_api_key"),
            Some("test_webhook_secret"),
            Some(30),
        )
        .await;
    }

    /// Helper: Create a client app via API and return its ID
    async fn create_client_app_via_api(
        _ctx: &PaymentFlowTestContext,
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

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let uuid: Uuid = serde_json::from_value(json["id"].clone()).unwrap();
        (client_id.to_string(), uuid)
    }

    // =========================================================================
    // Test: Checkout session creation success
    // =========================================================================
    // Given: 激活的 Plan 和 Client App
    // When: 创建 checkout session
    // Then: 返回有效的 checkout URL 和 session ID

    #[test_context(PaymentFlowTestContext)]
    #[tokio::test]
    #[ignore = "Requires Creem API mocking - integration test only"]
    async fn test_checkout_session_creation_success(ctx: &mut PaymentFlowTestContext) {
        let app = ctx.create_unified_test_router();
        // Webhook secret is now configured in database via setup_creem_config
        let realm_id = ctx._realm_id.clone();

        // Set webhook secret
        setup_creem_config(ctx, realm_id.as_str()).await;

        // Given: Setup admin and create plan
        let admin_token = setup_billing_admin_session(ctx, "test-checkout@test.com").await;
        let plan_id = create_test_plan(ctx, realm_id.as_str(), "Basic Plan").await;

        // Given: Create client app
        let (client_id_str, _client_app_id) = create_client_app_via_api(
            ctx,
            &app,
            &admin_token,
            realm_id.as_str(),
            "test-checkout-app",
            "Test Checkout App",
        )
        .await;

        // When: Create checkout session
        let request_payload = json!({
            "planId": plan_id.to_string(),
            "billingPeriod": "monthly",
            "successUrl": "https://example.com/success",
            "cancelUrl": "https://example.com/cancel"
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/bill/{}/client/{}/checkout",
                        realm_id, client_id_str
                    ))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={}", admin_token))
                    .body(Body::from(request_payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Then: Response is successful
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json_response: serde_json::Value = serde_json::from_slice(&body).unwrap();

        // Then: Returns valid checkout URL and session ID
        assert!(json_response["checkoutUrl"].is_string());
        assert!(json_response["sessionId"].is_string());

        let checkout_url = json_response["checkoutUrl"].as_str().unwrap();
        assert!(checkout_url.starts_with("https://"));
    }

    // =========================================================================
    // Test: Checkout metadata preservation
    // =========================================================================
    // Given: 创建 checkout session 时包含 metadata
    // When: Creem 返回 webhook
    // Then: Metadata 被正确传递并保存到 payment_event

    #[test_context(PaymentFlowTestContext)]
    #[tokio::test]
    async fn test_checkout_metadata_preservation(ctx: &mut PaymentFlowTestContext) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();
        let event_id = format!("evt_metadata_{}", Uuid::now_v7());

        setup_creem_config(ctx, realm_id.as_str()).await;

        // Given: Setup admin, plan, and client app
        let admin_token = setup_billing_admin_session(ctx, "test-metadata@test.com").await;
        let plan_id = create_test_plan(ctx, realm_id.as_str(), "Metadata Plan").await;
        let (_client_id_str, client_app_id) = create_client_app_via_api(
            ctx,
            &app,
            &admin_token,
            realm_id.as_str(),
            "test-metadata-app",
            "Test Metadata App",
        )
        .await;

        // Given: Create checkout with metadata
        let custom_metadata = json!({
            "customField": "customValue",
            "userId": "user_123"
        });

        // When: Creem sends webhook with metadata
        let webhook_payload = generate_checkout_completed_webhook(
            &event_id,
            realm_id.as_str(),
            &client_app_id.to_string(),
            &plan_id.to_string(),
            "monthly",
        );

        // Merge custom metadata
        let mut webhook_with_metadata = webhook_payload.clone();
        if let Some(meta_obj) = webhook_with_metadata["object"]["metadata"].as_object_mut() {
            for (key, value) in custom_metadata.as_object().unwrap() {
                meta_obj.insert(key.clone(), value.clone());
            }
        }

        let response = send_webhook_event(
            &app,
            realm_id.as_str(),
            webhook_with_metadata,
            "test_webhook_secret",
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);

        // Then: Metadata is saved to payment_event
        let payment_event: Option<serde_json::Value> =
            sqlx::query_scalar("SELECT payload FROM payment_event WHERE external_event_id = $1 AND payment_provider = 'creem'")
                .bind(&event_id)
                .fetch_optional(&ctx.app_state.pool)
                .await
                .unwrap();

        assert!(payment_event.is_some(), "Payment event should be created");

        let saved_metadata = &payment_event.unwrap()["object"]["metadata"];
        assert_eq!(saved_metadata["customField"], "customValue");
        assert_eq!(saved_metadata["userId"], "user_123");

        // Cleanup
        cleanup_payment_event_by_creem_id(ctx, &event_id).await;
    }

    // =========================================================================
    // Test: Checkout with invalid plan
    // =========================================================================
    // Given: 不存在的 plan_id
    // When: 尝试创建 checkout
    // Then: 返回 404 错误

    #[test_context(PaymentFlowTestContext)]
    #[tokio::test]
    async fn test_checkout_with_invalid_plan(ctx: &mut PaymentFlowTestContext) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();

        // Given: Setup admin
        let admin_token = setup_billing_admin_session(ctx, "test-invalid-plan@test.com").await;

        // Given: Create client app
        let (_client_id_str, client_app_id) = create_client_app_via_api(
            ctx,
            &app,
            &admin_token,
            realm_id.as_str(),
            "test-invalid-plan-app",
            "Test Invalid Plan App",
        )
        .await;

        // Given: Invalid plan ID
        let invalid_plan_id = Uuid::now_v7();

        // When: Attempt to create checkout with invalid plan
        let request_payload = json!({
            "planId": invalid_plan_id.to_string(),
            "paymentProvider": "stripe",
            "billingPeriod": "monthly",
            "successUrl": "https://example.com/success",
            "cancelUrl": "https://example.com/cancel"
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/bill/{}/client/{}/checkout",
                        realm_id, client_app_id
                    ))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={}", admin_token))
                    .body(Body::from(request_payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Then: Returns 404 error
        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "Should return 404 for invalid plan"
        );
    }

    // =========================================================================
    // Test: Checkout with disabled provider
    // =========================================================================
    // Given: Plan 的支付提供商未激活
    // When: 尝试创建 checkout
    // Then: 返回 400 错误

    #[test_context(PaymentFlowTestContext)]
    #[tokio::test]
    async fn test_checkout_with_disabled_provider(ctx: &mut PaymentFlowTestContext) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();

        // Given: Setup admin
        let admin_token = setup_billing_admin_session(ctx, "test-disabled-provider@test.com").await;

        // Given: Create plan with active=false
        let plan_id = create_test_plan_with_attrs(
            ctx,
            realm_id.as_str(),
            "Disabled Provider Plan",
            "monthly",
            2500,
        )
        .await;

        // Deactivate the plan
        sqlx::query("UPDATE plan SET active = false WHERE id = $1")
            .bind(plan_id)
            .execute(&ctx.app_state.pool)
            .await
            .unwrap();

        // Given: Create client app
        let (client_id_str, _client_app_id) = create_client_app_via_api(
            ctx,
            &app,
            &admin_token,
            realm_id.as_str(),
            "test-disabled-provider-app",
            "Test Disabled Provider App",
        )
        .await;

        // When: Attempt to create checkout with disabled provider
        let request_payload = json!({
            "planId": plan_id.to_string(),
            "billingPeriod": "monthly",
            "successUrl": "https://example.com/success",
            "cancelUrl": "https://example.com/cancel"
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/bill/{}/client/{}/checkout",
                        realm_id, client_id_str
                    ))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={}", admin_token))
                    .body(Body::from(request_payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Then: Returns 400 error (or 404, depending on implementation)
        let status = response.status();
        assert!(
            status == StatusCode::BAD_REQUEST || status == StatusCode::NOT_FOUND,
            "Expected 400 or 404 for disabled provider, got {}",
            status
        );
    }

    // =========================================================================
    // Test: Payment completion creates subscription
    // =========================================================================
    // Given: 完成的 checkout session
    // When: 接收 `checkout.completed` webhook
    // Then: 创建新的 active subscription

    #[test_context(PaymentFlowTestContext)]
    #[tokio::test]
    async fn test_payment_completion_creates_subscription(ctx: &mut PaymentFlowTestContext) {
        let app = ctx.create_unified_test_router();
        // Webhook secret is now configured in database via setup_creem_config
        let realm_id = ctx._realm_id.clone();
        let _event_id = format!("evt_payment_{}", Uuid::now_v7());

        setup_creem_config(ctx, realm_id.as_str()).await;

        // Given: Setup admin, plan, and client app
        let admin_token =
            setup_billing_admin_session(ctx, "test-payment-completion@test.com").await;
        let plan_id = Uuid::now_v7(); // Generate a plan ID

        // Setup plan config for the test (this creates both the plan and plan config)
        setup_test_plan_config(ctx, &realm_id, plan_id).await;

        let (_client_id_str, client_app_id) = create_client_app_via_api(
            ctx,
            &app,
            &admin_token,
            realm_id.as_str(),
            "test-payment-completion-app",
            "Test Payment Completion App",
        )
        .await;

        // Given: Completed checkout session
        let checkout_event_id = format!("evt_checkout_{}", Uuid::now_v7());
        let subscription_event_id = format!("evt_sub_{}", Uuid::now_v7());

        let checkout_payload = generate_checkout_completed_webhook(
            &checkout_event_id,
            realm_id.as_str(),
            &client_app_id.to_string(),
            &plan_id.to_string(),
            "monthly",
        );

        // When: Receive checkout.completed webhook (audited, no subscription created)
        let checkout_response = send_webhook_event(
            &app,
            realm_id.as_str(),
            checkout_payload,
            "test_webhook_secret",
        )
        .await;

        assert_eq!(checkout_response.status(), StatusCode::OK);

        // Then: No subscription is created yet
        let subscription_id_before: Option<Uuid> =
            get_subscription_by_client_app(ctx, client_app_id).await;
        assert!(
            subscription_id_before.is_none(),
            "Subscription should not be created yet"
        );

        // When: Receive subscription.paid webhook (this creates the subscription)
        // Use the verified helper function from webhook_helpers
        let test_user_id = create_test_user(
            &ctx.app_state.pool,
            &realm_id,
            "test-payment-sub@example.com",
        )
        .await;

        // Create points account for the user (required for subscription.paid webhook)
        create_points_account(ctx, test_user_id, &realm_id).await;

        let subscription_payload = build_subscription_paid_event_with_client(
            subscription_event_id.clone(),
            test_user_id,
            plan_id,
            false, // is_renewal
            realm_id.as_str(),
            Some(client_app_id), // Pass client_app_id so subscription can be linked
        );

        let subscription_response = send_webhook_event(
            &app,
            realm_id.as_str(),
            subscription_payload,
            "test_webhook_secret",
        )
        .await;

        assert_eq!(subscription_response.status(), StatusCode::OK);

        // Then: New active subscription is created
        let subscription_id: Option<Uuid> =
            get_subscription_by_client_app(ctx, client_app_id).await;
        assert!(subscription_id.is_some(), "Subscription should be created");

        let subscription_id = subscription_id.unwrap();
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Active).await;

        // Cleanup
        delete_test_subscription(ctx, subscription_id).await;
        cleanup_payment_event_by_creem_id(ctx, &checkout_event_id).await;
        cleanup_payment_event_by_creem_id(ctx, &subscription_event_id).await;
    }

    // =========================================================================
    // Test: Yearly billing period checkout
    // =========================================================================
    // Given: Yearly plan
    // When: 创建 checkout 并完成支付
    // Then: Subscription 的 billing_period 为 yearly

    #[test_context(PaymentFlowTestContext)]
    #[tokio::test]
    async fn test_yearly_billing_period_checkout(ctx: &mut PaymentFlowTestContext) {
        let app = ctx.create_unified_test_router();
        // Webhook secret is now configured in database via setup_creem_config
        let realm_id = ctx._realm_id.clone();
        let _event_id = format!("evt_yearly_{}", Uuid::now_v7());

        setup_creem_config(ctx, realm_id.as_str()).await;

        // Given: Setup admin and yearly plan
        let admin_token = setup_billing_admin_session(ctx, "test-yearly@test.com").await;
        let plan_id = Uuid::now_v7(); // Generate a plan ID

        // Setup plan config for the test (this creates both the plan and plan config)
        setup_test_plan_config(ctx, &realm_id, plan_id).await;

        // Given: Create client app
        let (_client_id_str, client_app_id) = create_client_app_via_api(
            ctx,
            &app,
            &admin_token,
            realm_id.as_str(),
            "test-yearly-app",
            "Test Yearly App",
        )
        .await;

        // When: Create checkout and complete payment
        let checkout_event_id = format!("evt_checkout_yearly_{}", Uuid::now_v7());
        let subscription_event_id = format!("evt_sub_yearly_{}", Uuid::now_v7());

        let checkout_payload = generate_checkout_completed_webhook(
            &checkout_event_id,
            realm_id.as_str(),
            &client_app_id.to_string(),
            &plan_id.to_string(),
            "yearly",
        );

        // Send checkout.completed (audited, no subscription)
        let checkout_response = send_webhook_event(
            &app,
            realm_id.as_str(),
            checkout_payload,
            "test_webhook_secret",
        )
        .await;
        assert_eq!(checkout_response.status(), StatusCode::OK);

        // Send subscription.paid (creates subscription)
        let test_user_id = create_test_user(
            &ctx.app_state.pool,
            &realm_id,
            "test-yearly-sub@example.com",
        )
        .await;

        // Create points account for the user (required for subscription.paid webhook)
        create_points_account(ctx, test_user_id, &realm_id).await;

        let subscription_payload = build_subscription_paid_event_full(
            subscription_event_id.clone(),
            test_user_id,
            plan_id,
            false, // is_renewal
            realm_id.as_str(),
            Some(client_app_id), // Pass client_app_id so subscription can be linked
            "active",            // status
            "yearly",            // billing period
        );

        let subscription_response = send_webhook_event(
            &app,
            realm_id.as_str(),
            subscription_payload,
            "test_webhook_secret",
        )
        .await;

        assert_eq!(subscription_response.status(), StatusCode::OK);

        // Then: Subscription has yearly billing_period
        let subscription_id = get_subscription_by_client_app(ctx, client_app_id).await;
        assert!(subscription_id.is_some());

        let subscription_id = subscription_id.unwrap();
        let billing_period: String =
            sqlx::query_scalar("SELECT billing_period FROM subscription WHERE id = $1")
                .bind(subscription_id)
                .fetch_one(&ctx.app_state.pool)
                .await
                .unwrap();

        assert_eq!(billing_period, "yearly");

        // Cleanup
        delete_test_subscription(ctx, subscription_id).await;
        cleanup_payment_event_by_creem_id(ctx, &checkout_event_id).await;
        cleanup_payment_event_by_creem_id(ctx, &subscription_event_id).await;
    }

    // =========================================================================
    // Test: Trial period checkout
    // =========================================================================
    // Given: 带 trial days 的 plan
    // When: 创建 checkout 并完成支付
    // Then: Subscription 状态为 trialing

    #[test_context(PaymentFlowTestContext)]
    #[tokio::test]
    async fn test_trial_period_checkout(ctx: &mut PaymentFlowTestContext) {
        let app = ctx.create_unified_test_router();
        // Webhook secret is now configured in database via setup_creem_config
        let realm_id = ctx._realm_id.clone();
        let _event_id = format!("evt_trial_{}", Uuid::now_v7());

        setup_creem_config(ctx, realm_id.as_str()).await;

        // Given: Setup admin and plan with trial
        let admin_token = setup_billing_admin_session(ctx, "test-trial@test.com").await;
        let plan_id = Uuid::now_v7(); // Generate a plan ID

        // Setup plan config for the test (this creates both the plan and plan config)
        setup_test_plan_config(ctx, &realm_id, plan_id).await;

        // Given: Create client app
        let (_client_id_str, client_app_id) = create_client_app_via_api(
            ctx,
            &app,
            &admin_token,
            realm_id.as_str(),
            "test-trial-app",
            "Test Trial App",
        )
        .await;

        // When: Create checkout and complete payment
        let checkout_event_id = format!("evt_checkout_trial_{}", Uuid::now_v7());
        let subscription_event_id = format!("evt_sub_trial_{}", Uuid::now_v7());

        let checkout_payload = generate_checkout_completed_webhook_with_trial(
            &checkout_event_id,
            realm_id.as_str(),
            &client_app_id.to_string(),
            &plan_id.to_string(),
            "monthly",
            Some(14), // 14 days trial
        );

        // Send checkout.completed (audited, no subscription)
        let checkout_response = send_webhook_event(
            &app,
            realm_id.as_str(),
            checkout_payload,
            "test_webhook_secret",
        )
        .await;
        assert_eq!(checkout_response.status(), StatusCode::OK);

        // Send subscription.paid with trial (creates subscription)
        let test_user_id =
            create_test_user(&ctx.app_state.pool, &realm_id, "test-trial-sub@example.com").await;

        // Create points account for the user (required for subscription.paid webhook)
        create_points_account(ctx, test_user_id, &realm_id).await;

        let subscription_payload = build_subscription_paid_event_with_client_and_status(
            subscription_event_id.clone(),
            test_user_id,
            plan_id,
            false, // is_renewal
            realm_id.as_str(),
            Some(client_app_id), // Pass client_app_id so subscription can be linked
            "trialing",          // Set status to trialing for trial period
        );

        let subscription_response = send_webhook_event(
            &app,
            realm_id.as_str(),
            subscription_payload,
            "test_webhook_secret",
        )
        .await;

        assert_eq!(subscription_response.status(), StatusCode::OK);

        // Then: Subscription status is trialing
        let subscription_id = get_subscription_by_client_app(ctx, client_app_id).await;
        assert!(subscription_id.is_some());

        let subscription_id = subscription_id.unwrap();
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Trialing).await;

        // Cleanup
        delete_test_subscription(ctx, subscription_id).await;
        cleanup_payment_event_by_creem_id(ctx, &checkout_event_id).await;
        cleanup_payment_event_by_creem_id(ctx, &subscription_event_id).await;
    }

    // =========================================================================
    // Test: Checkout webhook idempotency
    // =========================================================================
    // Given: 重复的 `checkout.completed` webhook
    // When: 发送相同 event_id 两次
    // Then: 只创建一个 subscription

    #[test_context(PaymentFlowTestContext)]
    #[tokio::test]
    async fn test_checkout_webhook_idempotency(ctx: &mut PaymentFlowTestContext) {
        let app = ctx.create_unified_test_router();
        // Webhook secret is now configured in database via setup_creem_config
        let realm_id = ctx._realm_id.clone();
        let event_id = format!("evt_idempotent_{}", Uuid::now_v7());

        setup_creem_config(ctx, realm_id.as_str()).await;

        // Given: Setup admin, plan, and client app
        let admin_token = setup_billing_admin_session(ctx, "test-idempotent@test.com").await;
        let plan_id = Uuid::now_v7(); // Generate a plan ID

        // Setup plan config for the test (this creates both the plan and plan config)
        setup_test_plan_config(ctx, &realm_id, plan_id).await;
        let (_client_id_str, client_app_id) = create_client_app_via_api(
            ctx,
            &app,
            &admin_token,
            realm_id.as_str(),
            "test-idempotent-app",
            "Test Idempotent App",
        )
        .await;

        let webhook_payload = generate_checkout_completed_webhook(
            &event_id,
            realm_id.as_str(),
            &client_app_id.to_string(),
            &plan_id.to_string(),
            "monthly",
        );

        // When: Send same checkout webhook twice
        let response1 = send_webhook_event(
            &app,
            realm_id.as_str(),
            webhook_payload.clone(),
            "test_webhook_secret",
        )
        .await;

        assert_eq!(response1.status(), StatusCode::OK);

        let response2 = send_webhook_event(
            &app,
            realm_id.as_str(),
            webhook_payload,
            "test_webhook_secret",
        )
        .await;

        assert_eq!(response2.status(), StatusCode::OK);

        // Then: No subscription is created (checkout.completed doesn't create subscriptions)
        let subscription_id_before = get_subscription_by_client_app(ctx, client_app_id).await;
        assert!(
            subscription_id_before.is_none(),
            "Checkout should not create subscription"
        );

        // When: Send subscription.paid webhook once
        let subscription_event_id = format!("evt_sub_idempotent_{}", Uuid::now_v7());

        let test_user_id = create_test_user(
            &ctx.app_state.pool,
            &realm_id,
            "test-idempotent-sub@example.com",
        )
        .await;

        // Create points account for the user (required for subscription.paid webhook)
        create_points_account(ctx, test_user_id, &realm_id).await;

        let subscription_payload = build_subscription_paid_event_with_client(
            subscription_event_id.clone(),
            test_user_id,
            plan_id,
            false, // is_renewal
            realm_id.as_str(),
            Some(client_app_id), // Pass client_app_id so subscription can be linked
        );

        let subscription_response = send_webhook_event(
            &app,
            realm_id.as_str(),
            subscription_payload,
            "test_webhook_secret",
        )
        .await;

        assert_eq!(subscription_response.status(), StatusCode::OK);

        // Then: Subscription is created
        let subscription_id = get_subscription_by_client_app(ctx, client_app_id).await;
        assert!(subscription_id.is_some());

        let subscription_id = subscription_id.unwrap();

        // Then: Payment event is idempotent
        assert_payment_idempotent(ctx, &event_id).await;

        // Cleanup
        delete_test_subscription(ctx, subscription_id).await;
        cleanup_payment_event_by_creem_id(ctx, &event_id).await;
        cleanup_payment_event_by_creem_id(ctx, &subscription_event_id).await;
    }
}
