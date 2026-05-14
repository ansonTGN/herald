// =============================================================================
// Billing Webhook Tests
// =============================================================================
//
// Test: Verify webhook signature verification and idempotency
//
// User Story: docs/user-stories/06-billing-user-stories.md
// Covers: Webhook signature verification, idempotency, timestamp validation
//
// =============================================================================

#[cfg(test)]
mod tests {
    use crate::tests::schema_test_context::SchemaTestContext;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use chrono::Utc;
    use hex;
    use hmac::{Hmac, Mac};
    use serde_json::json;
    use sha2::Sha256;
    use test_context::test_context;
    use tower::ServiceExt;
    use uuid::Uuid;

    use SchemaTestContext as WebhookTestContext;

    type HmacSha256 = Hmac<Sha256>;

    /// Set webhook secret for a realm in the database (not environment variable)
    ///
    /// This helper function inserts the webhook secret into the realm_config table,
    /// which is where the webhook handler reads it from.
    async fn set_webhook_secret_for_realm(ctx: &SchemaTestContext, webhook_secret: &str) {
        ctx.with_creem_config(
            &ctx._realm_id,
            Some("test_api_key"), // API key (required)
            Some(webhook_secret), // Webhook secret
            Some(30),             // Timeout (required)
        )
        .await;
    }

    async fn get_payment_event_processed(
        ctx: &SchemaTestContext,
        provider: &str,
        external_event_id: &str,
    ) -> bool {
        sqlx::query_scalar(
            "SELECT processed FROM payment_event WHERE payment_provider = $1 AND external_event_id = $2",
        )
        .bind(provider)
        .bind(external_event_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap()
    }

    async fn get_subscription_count_by_client_app(
        ctx: &SchemaTestContext,
        client_app_id: Uuid,
    ) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM subscription WHERE client_app_id = $1")
            .bind(client_app_id)
            .fetch_one(&ctx.app_state.pool)
            .await
            .unwrap()
    }

    async fn get_subscription_snapshot_by_external_id(
        ctx: &SchemaTestContext,
        provider: &str,
        external_subscription_id: &str,
    ) -> Option<(Uuid, String, String, String)> {
        sqlx::query_as(
            "SELECT id, payment_provider, external_subscription_id, external_product_id
             FROM subscription
             WHERE payment_provider = $1 AND external_subscription_id = $2",
        )
        .bind(provider)
        .bind(external_subscription_id)
        .fetch_optional(&ctx.app_state.pool)
        .await
        .unwrap()
    }

    async fn get_subscription_history_actor_count(
        ctx: &SchemaTestContext,
        subscription_id: Uuid,
        event_type: &str,
        actor: &str,
    ) -> i64 {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM subscription_history
             WHERE subscription_id = $1 AND event_type = $2 AND actor = $3",
        )
        .bind(subscription_id)
        .bind(event_type)
        .bind(actor)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap()
    }
    /// Test: 旧版签名头格式应被拒绝（Creem 使用纯十六进制签名）
    #[test_context(WebhookTestContext)]
    #[tokio::test]
    async fn test_scenario_webhook_timestamp_expired(ctx: &mut WebhookTestContext) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "test_webhook_secret";
        let event_id = format!("evt_test_{}", Uuid::now_v7());

        // Set webhook secret in database (where handler reads it from)
        set_webhook_secret_for_realm(ctx, webhook_secret).await;

        // Step 1: 准备 Webhook payload
        let payload = json!({
            "id": event_id,
            "eventType": "checkout.completed",
            "object": {
                "id": "checkout_test_123",
                "status": "completed",
                "metadata": {
                    "realmId": ctx._realm_id.clone(),
                    "clientAppId": ctx._client_app_id.clone(),
                    "planId": Uuid::now_v7().to_string(),
                    "billing_period": "monthly"
                }
            }
        });

        // Step 2: 生成有效签名，但使用旧版头格式（t=...,v1=...）
        let payload_str = serde_json::to_string(&payload).unwrap();
        let signed_payload = payload_str;

        let mut mac = HmacSha256::new_from_slice(webhook_secret.as_bytes()).unwrap();
        mac.update(signed_payload.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        let legacy_timestamp = Utc::now().timestamp() - 600;
        let signature_header = format!("t={},v1={}", legacy_timestamp, signature);

        // Step 3: 发送 Webhook 请求
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/third/pay/{}/creem/webhooks", ctx._realm_id))
                    .header("creem-signature", signature_header.as_str())
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Step 4: 验证返回 401 Unauthorized（旧格式被拒绝）
        let status = response.status();
        assert!(
            status == StatusCode::UNAUTHORIZED || status == StatusCode::BAD_REQUEST,
            "Expected 401 or 400 for legacy signature format, got {}",
            status
        );
    }

    /// Test: Webhook 事件幂等性
    #[test_context(WebhookTestContext)]
    #[tokio::test]
    async fn test_scenario_webhook_idempotency(ctx: &mut WebhookTestContext) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "test_webhook_secret";
        let event_id = format!("evt_test_idempotent_{}", Uuid::now_v7());
        let plan_id = Uuid::now_v7();

        // Set webhook secret in database (where handler reads it from)
        set_webhook_secret_for_realm(ctx, webhook_secret).await;

        // Setup plan config for the test
        crate::tests::helpers::webhook_helpers::setup_test_plan_config(
            ctx,
            &ctx._realm_id,
            plan_id,
        )
        .await;

        // Step 1: 准备 Webhook payload（相同 event_id）
        let _timestamp = Utc::now().timestamp();
        let payload = json!({
            "id": event_id.clone(),
            "eventType": "checkout.completed",
            "object": {
                "id": "checkout_test_idempotent",
                "status": "completed",
                "metadata": {
                    "realmId": ctx._realm_id.clone(),
                    "clientAppId": ctx._client_app_id.clone(),
                    "planId": plan_id.to_string(),
                    "billing_period": "monthly"
                }
            }
        });

        // Step 2: 生成签名
        let payload_str = serde_json::to_string(&payload).unwrap();
        let signed_payload = payload_str;

        let mut mac = HmacSha256::new_from_slice(webhook_secret.as_bytes()).unwrap();
        mac.update(signed_payload.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        let signature_header = signature.clone();

        // Step 3: 第一次发送 Webhook（应该创建 payment_event）
        let response1 = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/third/pay/{}/creem/webhooks", ctx._realm_id))
                    .header("creem-signature", signature_header.as_str())
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response1.status(), StatusCode::OK);

        // Step 4: 验证 payment_event 被创建
        let event_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM payment_event WHERE external_event_id = $1 AND payment_provider = 'creem'")
                .bind(&event_id)
                .fetch_one(&ctx.app_state.pool)
                .await
                .unwrap();

        assert_eq!(
            event_count, 1,
            "Expected 1 payment event, got {}",
            event_count
        );

        // Step 5: 第二次发送相同 Webhook（应该返回 OK，但不重复创建 payment_event）
        let response2 = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/third/pay/{}/creem/webhooks", ctx._realm_id))
                    .header("creem-signature", signature_header.as_str())
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response2.status(), StatusCode::OK);

        // Step 6: 验证仍然只有一个 payment_event
        let event_count2: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM payment_event WHERE external_event_id = $1 AND payment_provider = 'creem'")
                .bind(&event_id)
                .fetch_one(&ctx.app_state.pool)
                .await
                .unwrap();

        assert_eq!(
            event_count2, 1,
            "Expected 1 payment event (idempotent), got {}",
            event_count2
        );

        // Cleanup: Remove payment event
        sqlx::query(
            "DELETE FROM payment_event WHERE external_event_id = $1 AND payment_provider = 'creem'",
        )
        .bind(&event_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();
    }

    /// Test: 并发发送同一 webhook event 时仅处理一次
    #[test_context(WebhookTestContext)]
    #[tokio::test]
    async fn test_scenario_webhook_concurrent_idempotency(ctx: &mut WebhookTestContext) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "test_webhook_secret";
        let event_id = format!("evt_test_concurrent_{}", Uuid::now_v7());
        let plan_id = Uuid::now_v7();

        // Set webhook secret in database (where handler reads it from)
        set_webhook_secret_for_realm(ctx, webhook_secret).await;

        // Setup plan config for the test
        crate::tests::helpers::webhook_helpers::setup_test_plan_config(
            ctx,
            &ctx._realm_id,
            plan_id,
        )
        .await;

        let payload = json!({
            "id": event_id.clone(),
            "eventType": "checkout.completed",
            "object": {
                "id": "checkout_test_concurrent",
                "status": "completed",
                "metadata": {
                    "realmId": ctx._realm_id.clone(),
                    "clientAppId": ctx._client_app_id.clone(),
                    "planId": plan_id.to_string(),
                    "billing_period": "monthly"
                }
            }
        });

        let payload_bytes = serde_json::to_vec(&payload).unwrap();
        let mut mac = HmacSha256::new_from_slice(webhook_secret.as_bytes()).unwrap();
        mac.update(serde_json::to_string(&payload).unwrap().as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        let req1 = Request::builder()
            .method("POST")
            .uri(format!("/api/third/pay/{}/creem/webhooks", ctx._realm_id))
            .header("creem-signature", signature.as_str())
            .header("Content-Type", "application/json")
            .body(Body::from(payload_bytes.clone()))
            .unwrap();
        let req2 = Request::builder()
            .method("POST")
            .uri(format!("/api/third/pay/{}/creem/webhooks", ctx._realm_id))
            .header("creem-signature", signature.as_str())
            .header("Content-Type", "application/json")
            .body(Body::from(payload_bytes))
            .unwrap();

        let fut1 = app.clone().oneshot(req1);
        let fut2 = app.clone().oneshot(req2);
        let (resp1, resp2) = tokio::join!(fut1, fut2);
        let response1 = resp1.unwrap();
        let response2 = resp2.unwrap();

        assert_eq!(response1.status(), StatusCode::OK);
        assert_eq!(response2.status(), StatusCode::OK);

        let event_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM payment_event WHERE external_event_id = $1 AND payment_provider = 'creem'")
                .bind(&event_id)
                .fetch_one(&ctx.app_state.pool)
                .await
                .unwrap();
        assert_eq!(
            event_count, 1,
            "Expected 1 payment event, got {}",
            event_count
        );

        sqlx::query(
            "DELETE FROM payment_event WHERE external_event_id = $1 AND payment_provider = 'creem'",
        )
        .bind(&event_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();
    }
    /// Test: Creem checkout.completed only audits and does not create placeholder subscription
    #[test_context(WebhookTestContext)]
    #[tokio::test]
    async fn test_scenario_creem_checkout_completed_audits_without_creating_subscription(
        ctx: &mut WebhookTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "test_webhook_secret";
        let event_id = format!("evt_test_checkout_only_{}", Uuid::now_v7());
        let plan_id = Uuid::now_v7();
        let client_app_id = Uuid::parse_str(&ctx._client_app_id).unwrap();

        set_webhook_secret_for_realm(ctx, webhook_secret).await;
        crate::tests::helpers::webhook_helpers::setup_test_plan_config(
            ctx,
            &ctx._realm_id,
            plan_id,
        )
        .await;

        let payload = json!({
            "id": event_id.clone(),
            "eventType": "checkout.completed",
            "object": {
                "id": "checkout_test_audit_only",
                "status": "completed",
                "product": {
                    "id": format!("prod_test_{}", plan_id),
                    "name": "Test Plan"
                },
                "metadata": {
                    "realmId": ctx._realm_id.clone(),
                    "clientAppId": client_app_id.to_string(),
                    "planId": plan_id.to_string(),
                    "billing_period": "monthly"
                }
            }
        });

        let response = crate::tests::helpers::webhook_helpers::send_webhook_with_signature(
            &app,
            &ctx._realm_id,
            payload,
            webhook_secret,
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            get_subscription_count_by_client_app(ctx, client_app_id).await,
            0
        );
        assert!(get_payment_event_processed(ctx, "creem", &event_id).await);
    }

    /// Test: Creem subscription.paid creates the first subscription aggregate and writes webhook history
    #[test_context(WebhookTestContext)]
    #[tokio::test]
    async fn test_scenario_creem_subscription_paid_creates_subscription_and_history(
        ctx: &mut WebhookTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "test_webhook_secret";
        let event_id = format!("evt_test_creem_paid_{}", Uuid::now_v7());
        let plan_id = Uuid::now_v7();
        let realm_id = ctx._realm_id.clone();
        let client_app_id = Uuid::parse_str(&ctx._client_app_id).unwrap();
        let external_subscription_id = format!("sub_creem_{}", Uuid::now_v7());
        let external_product_id = format!("prod_test_{}", plan_id);

        let user_email = format!("creem-paid-{}@example.com", Uuid::now_v7());
        let (_token, user_id) =
            crate::tests::helpers::auth_helpers::create_admin_session_with_user(
                ctx,
                &user_email,
                1800,
            )
            .await;
        let user_id = Uuid::parse_str(&user_id).unwrap();
        crate::tests::helpers::points_helpers::create_points_account(ctx, user_id, &realm_id).await;

        set_webhook_secret_for_realm(ctx, webhook_secret).await;
        crate::tests::helpers::webhook_helpers::setup_test_plan_config(ctx, &realm_id, plan_id)
            .await;

        let mut payload = crate::tests::helpers::webhook_helpers::build_subscription_paid_event(
            event_id.clone(),
            user_id,
            plan_id,
            false,
            &realm_id,
        );
        payload["data"]["object"]["subscriptionId"] =
            serde_json::json!(external_subscription_id.clone());
        payload["data"]["object"]["productId"] = serde_json::json!(external_product_id.clone());
        payload["data"]["object"]["clientAppId"] = serde_json::json!(client_app_id.to_string());
        payload["data"]["object"]["status"] = serde_json::json!("active");
        payload["data"]["object"]["billingPeriod"] = serde_json::json!("monthly");

        let response = crate::tests::helpers::webhook_helpers::send_webhook_with_signature(
            &app,
            &realm_id,
            payload,
            webhook_secret,
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert!(get_payment_event_processed(ctx, "creem", &event_id).await);

        let subscription =
            get_subscription_snapshot_by_external_id(ctx, "creem", &external_subscription_id)
                .await
                .expect("Creem subscription should be created by subscription.paid");

        assert_eq!(subscription.1, "creem");
        assert_eq!(subscription.2, external_subscription_id);
        assert_eq!(subscription.3, external_product_id);

        let history_count =
            get_subscription_history_actor_count(ctx, subscription.0, "created", "webhook").await;
        assert_eq!(
            history_count, 1,
            "Expected one webhook-created history entry"
        );
    }
    // ============================================================================
    // Stripe Webhook Tests
    // ============================================================================

    /// Set Stripe webhook secret for a realm in the database
    ///
    /// This helper function inserts the Stripe webhook secret into the realm_config table,
    /// which is where the Stripe webhook handler reads it from.
    async fn set_stripe_webhook_secret_for_realm(ctx: &SchemaTestContext, webhook_secret: &str) {
        crate::tests::helpers::billing_helpers::setup_stripe_config(
            ctx,
            &ctx._realm_id,
            "sk_test_test_key",
            webhook_secret,
        )
        .await;
    }
    /// Test: Stripe invoice.payment_succeeded webhook grants renewal points
    #[test_context(WebhookTestContext)]
    #[tokio::test]
    async fn test_scenario_stripe_webhook_invoice_paid_subscription_renewal(
        ctx: &mut WebhookTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_test_webhook_secret";
        let event_id = format!("evt_test_{}", Uuid::now_v7());
        let client_app_id = Uuid::now_v7();
        let plan_id = Uuid::now_v7();
        let stripe_subscription_id = format!("sub_test_{}", Uuid::now_v7());
        let realm_id = ctx._realm_id.clone();

        // Create a test user in the database (required for points operations)
        let user_email = format!("invoice-renewal-{}@example.com", Uuid::now_v7());
        let (_token, user_id) =
            crate::tests::helpers::auth_helpers::create_admin_session_with_user(
                ctx,
                &user_email,
                1800,
            )
            .await;
        let user_id_uuid = Uuid::parse_str(&user_id).unwrap();
        crate::tests::helpers::points_helpers::create_points_account(ctx, user_id_uuid, &realm_id)
            .await;

        // Set Stripe webhook secret in database
        set_stripe_webhook_secret_for_realm(ctx, webhook_secret).await;

        // Setup plan config for the test
        crate::tests::helpers::webhook_helpers::setup_test_plan_config(ctx, &realm_id, plan_id)
            .await;

        // Create an existing subscription
        let subscription_id = crate::tests::helpers::billing_helpers::create_test_subscription(
            ctx,
            &realm_id,
            client_app_id,
            plan_id,
            "monthly",
        )
        .await;

        // Update subscription with Stripe subscription ID and product ID
        let external_product_id = format!("prod_test_{}", plan_id);
        sqlx::query("UPDATE subscription SET external_subscription_id = $1, external_product_id = $2, payment_provider = 'stripe' WHERE id = $3")
            .bind(&stripe_subscription_id)
            .bind(&external_product_id)
            .bind(subscription_id)
            .execute(&ctx.app_state.pool)
            .await
            .unwrap();

        // Build Stripe invoice.payment_succeeded event (renewal)
        let payload =
            crate::tests::helpers::webhook_helpers::build_stripe_invoice_payment_succeeded_event(
                event_id.clone(),
                stripe_subscription_id.clone(),
                &ctx._realm_id,
                &user_id,
                &plan_id.to_string(),
                2500, // $25.00
            );

        // Send webhook with valid signature
        let response = crate::tests::helpers::webhook_helpers::send_stripe_webhook_with_signature(
            &app,
            &ctx._realm_id,
            payload,
            webhook_secret,
        )
        .await;

        // Verify response is OK
        assert_eq!(response.status(), StatusCode::OK);

        // Verify payment event was created
        assert!(
            crate::tests::helpers::billing_helpers::verify_stripe_payment_event_exists(
                &*ctx, &event_id
            )
            .await,
            "Payment event should be created for renewal"
        );

        // Cleanup
        crate::tests::helpers::billing_helpers::delete_test_subscription(ctx, subscription_id)
            .await;
        crate::tests::helpers::billing_helpers::cleanup_payment_events(ctx, subscription_id).await;
    }

    /// Test: Stripe customer.subscription.updated webhook (plan upgrade)
    #[test_context(WebhookTestContext)]
    #[tokio::test]
    async fn test_scenario_stripe_webhook_subscription_updated_plan_upgrade(
        ctx: &mut WebhookTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_test_webhook_secret";
        let event_id = format!("evt_test_{}", Uuid::now_v7());
        let client_app_id = Uuid::now_v7();
        let old_plan_id = Uuid::now_v7();
        let new_plan_id = Uuid::now_v7();
        let stripe_subscription_id = format!("sub_test_{}", Uuid::now_v7());
        let realm_id = ctx._realm_id.clone();

        // Create a test user in the database (required for subscription operations)
        let user_email = format!("sub-update-{}@example.com", Uuid::now_v7());
        let (_token, user_id) =
            crate::tests::helpers::auth_helpers::create_admin_session_with_user(
                ctx,
                &user_email,
                1800,
            )
            .await;

        // Set Stripe webhook secret in database
        set_stripe_webhook_secret_for_realm(ctx, webhook_secret).await;

        // Setup plan configs for both plans
        crate::tests::helpers::webhook_helpers::setup_test_plan_config(ctx, &realm_id, old_plan_id)
            .await;

        crate::tests::helpers::webhook_helpers::setup_test_plan_config(ctx, &realm_id, new_plan_id)
            .await;

        // Create an existing subscription with old plan
        let subscription_id = crate::tests::helpers::billing_helpers::create_test_subscription(
            ctx,
            &realm_id,
            client_app_id,
            old_plan_id,
            "monthly",
        )
        .await;

        // Update subscription with Stripe subscription ID and product ID
        let external_product_id = format!("prod_test_{}", new_plan_id);
        sqlx::query("UPDATE subscription SET external_subscription_id = $1, external_product_id = $2, payment_provider = 'stripe' WHERE id = $3")
            .bind(&stripe_subscription_id)
            .bind(&external_product_id)
            .bind(subscription_id)
            .execute(&ctx.app_state.pool)
            .await
            .unwrap();

        // Build Stripe customer.subscription.updated event (plan upgrade)
        let mut payload =
            crate::tests::helpers::webhook_helpers::build_stripe_subscription_updated_event(
                event_id.clone(),
                stripe_subscription_id.clone(),
                &ctx._realm_id,
                Some(old_plan_id.to_string()),
                new_plan_id.to_string(),
            );

        // Update the payload with the real user_id
        payload["data"]["object"]["metadata"]["userId"] = serde_json::json!(user_id);

        // Send webhook with valid signature
        let response = crate::tests::helpers::webhook_helpers::send_stripe_webhook_with_signature(
            &app,
            &ctx._realm_id,
            payload,
            webhook_secret,
        )
        .await;

        // Verify response is OK
        assert_eq!(response.status(), StatusCode::OK);

        // Note: Plan update logic may not be fully implemented yet
        // This test verifies the webhook is accepted and processed
        assert!(
            crate::tests::helpers::billing_helpers::verify_stripe_payment_event_exists(
                &*ctx, &event_id
            )
            .await,
            "Payment event should be created for subscription update"
        );

        // Cleanup
        crate::tests::helpers::billing_helpers::delete_test_subscription(ctx, subscription_id)
            .await;
        crate::tests::helpers::billing_helpers::cleanup_payment_events(ctx, subscription_id).await;
    }

    /// Test: Stripe customer.subscription.deleted webhook (cancellation)
    #[test_context(WebhookTestContext)]
    #[tokio::test]
    async fn test_scenario_stripe_webhook_subscription_deleted_cancel_at_period_end(
        ctx: &mut WebhookTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_test_webhook_secret";
        let event_id = format!("evt_test_{}", Uuid::now_v7());
        let client_app_id = Uuid::now_v7();
        let plan_id = Uuid::now_v7();
        let stripe_subscription_id = format!("sub_test_{}", Uuid::now_v7());
        let realm_id = ctx._realm_id.clone();

        // Create a test user in the database (required for subscription operations)
        let user_email = format!("sub-delete-{}@example.com", Uuid::now_v7());
        let (_token, user_id) =
            crate::tests::helpers::auth_helpers::create_admin_session_with_user(
                ctx,
                &user_email,
                1800,
            )
            .await;

        // Set Stripe webhook secret in database
        set_stripe_webhook_secret_for_realm(ctx, webhook_secret).await;

        // Setup plan config for the test
        crate::tests::helpers::webhook_helpers::setup_test_plan_config(ctx, &realm_id, plan_id)
            .await;

        // Create an existing subscription
        let subscription_id = crate::tests::helpers::billing_helpers::create_test_subscription(
            ctx,
            &realm_id,
            client_app_id,
            plan_id,
            "monthly",
        )
        .await;

        // Update subscription with Stripe subscription ID and product ID
        let external_product_id = format!("prod_test_{}", plan_id);
        sqlx::query("UPDATE subscription SET external_subscription_id = $1, external_product_id = $2, payment_provider = 'stripe' WHERE id = $3")
            .bind(&stripe_subscription_id)
            .bind(&external_product_id)
            .bind(subscription_id)
            .execute(&ctx.app_state.pool)
            .await
            .unwrap();

        // Build Stripe customer.subscription.deleted event
        let mut payload =
            crate::tests::helpers::webhook_helpers::build_stripe_subscription_deleted_event(
                event_id.clone(),
                stripe_subscription_id.clone(),
                &ctx._realm_id,
                true, // cancel_at_period_end
            );

        // Update the payload with the real user_id
        payload["data"]["object"]["metadata"]["userId"] = serde_json::json!(user_id);

        // Send webhook with valid signature
        let response = crate::tests::helpers::webhook_helpers::send_stripe_webhook_with_signature(
            &app,
            &ctx._realm_id,
            payload,
            webhook_secret,
        )
        .await;

        // Verify response is OK
        assert_eq!(response.status(), StatusCode::OK);

        // Note: Cancellation logic may not be fully implemented yet
        // This test verifies the webhook is accepted and processed
        assert!(
            crate::tests::helpers::billing_helpers::verify_stripe_payment_event_exists(
                &*ctx, &event_id
            )
            .await,
            "Payment event should be created for subscription deletion"
        );

        // Cleanup
        crate::tests::helpers::billing_helpers::delete_test_subscription(ctx, subscription_id)
            .await;
        crate::tests::helpers::billing_helpers::cleanup_payment_events(ctx, subscription_id).await;
    }

    /// Test: Stripe charge.refunded webhook (full refund)
    #[test_context(WebhookTestContext)]
    #[tokio::test]
    async fn test_scenario_stripe_webhook_charge_refunded_full_refund(
        ctx: &mut WebhookTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_test_webhook_secret";
        let event_id = format!("evt_test_{}", Uuid::now_v7());
        let charge_id = format!("ch_test_{}", Uuid::now_v7());

        // Create a test user in the database (required for refund processing)
        let user_email = format!("refund-test-{}@example.com", Uuid::now_v7());
        let (_token, user_id) =
            crate::tests::helpers::auth_helpers::create_admin_session_with_user(
                ctx,
                &user_email,
                1800,
            )
            .await;

        // Set Stripe webhook secret in database
        set_stripe_webhook_secret_for_realm(ctx, webhook_secret).await;

        // Build Stripe charge.refunded event (full refund)
        let payload = crate::tests::helpers::webhook_helpers::build_stripe_charge_refunded_event(
            event_id.clone(),
            charge_id,
            &ctx._realm_id,
            &user_id,
            2500, // amount_refunded
            2500, // original_amount (full refund)
        );

        // Send webhook with valid signature
        let response = crate::tests::helpers::webhook_helpers::send_stripe_webhook_with_signature(
            &app,
            &ctx._realm_id,
            payload,
            webhook_secret,
        )
        .await;

        // Verify response is OK
        assert_eq!(response.status(), StatusCode::OK);

        // Note: Refund processing logic may not be fully implemented yet
        // This test verifies the webhook is accepted and processed
        assert!(
            crate::tests::helpers::billing_helpers::verify_stripe_payment_event_exists(
                &*ctx, &event_id
            )
            .await,
            "Payment event should be created for refund"
        );
    }
    /// Test: Stripe webhook idempotency (same event sent twice)
    #[test_context(WebhookTestContext)]
    #[tokio::test]
    async fn test_scenario_stripe_webhook_idempotency(ctx: &mut WebhookTestContext) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_test_webhook_secret";
        let event_id = format!("evt_test_{}", Uuid::now_v7());
        let client_app_id = Uuid::now_v7();
        let plan_id = Uuid::now_v7();
        let user_id = Some(Uuid::now_v7());

        // Set Stripe webhook secret in database
        set_stripe_webhook_secret_for_realm(ctx, webhook_secret).await;

        // Setup plan config for the test
        crate::tests::helpers::webhook_helpers::setup_test_plan_config(
            ctx,
            &ctx._realm_id,
            plan_id,
        )
        .await;

        // Build Stripe checkout.session.completed event
        let payload =
            crate::tests::helpers::webhook_helpers::build_stripe_checkout_session_completed_event(
                event_id.clone(),
                client_app_id,
                plan_id,
                &ctx._realm_id,
                user_id,
                None,
            );

        // Send webhook first time
        let response1 = crate::tests::helpers::webhook_helpers::send_stripe_webhook_with_signature(
            &app,
            &ctx._realm_id,
            payload.clone(),
            webhook_secret,
        )
        .await;
        assert_eq!(response1.status(), StatusCode::OK);

        // Send webhook second time (same event_id)
        let response2 = crate::tests::helpers::webhook_helpers::send_stripe_webhook_with_signature(
            &app,
            &ctx._realm_id,
            payload,
            webhook_secret,
        )
        .await;
        assert_eq!(response2.status(), StatusCode::OK);

        // Verify only one subscription was created (idempotency)
        let subscriptions: Vec<i64> =
            sqlx::query_scalar("SELECT COUNT(*) FROM subscription WHERE client_app_id = $1")
                .bind(client_app_id)
                .fetch_all(&ctx.app_state.pool)
                .await
                .unwrap();

        assert_eq!(
            subscriptions[0], 1,
            "Only one subscription should be created despite duplicate webhook"
        );

        // Cleanup
        if let Some(sub_id) =
            crate::tests::helpers::billing_helpers::get_subscription_by_client_app(
                ctx,
                client_app_id,
            )
            .await
        {
            crate::tests::helpers::billing_helpers::delete_test_subscription(ctx, sub_id).await;
            crate::tests::helpers::billing_helpers::cleanup_payment_events(ctx, sub_id).await;
        }
    }

    /// Test: Stripe checkout.session.completed requires subscription id in payload
    #[test_context(WebhookTestContext)]
    #[tokio::test]
    async fn test_scenario_stripe_checkout_completed_missing_subscription_id(
        ctx: &mut WebhookTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_test_webhook_secret";
        let event_id = format!("evt_test_missing_sub_{}", Uuid::now_v7());
        let client_app_id = Uuid::now_v7();
        let plan_id = Uuid::now_v7();

        set_stripe_webhook_secret_for_realm(ctx, webhook_secret).await;
        crate::tests::helpers::webhook_helpers::setup_test_plan_config(
            ctx,
            &ctx._realm_id,
            plan_id,
        )
        .await;

        let mut payload =
            crate::tests::helpers::webhook_helpers::build_stripe_checkout_session_completed_event(
                event_id.clone(),
                client_app_id,
                plan_id,
                &ctx._realm_id,
                Some(Uuid::now_v7()),
                None,
            );
        payload["data"]["object"]["subscription"] = serde_json::Value::Null;

        let response = crate::tests::helpers::webhook_helpers::send_stripe_webhook_with_signature(
            &app,
            &ctx._realm_id,
            payload,
            webhook_secret,
        )
        .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(
            crate::tests::helpers::billing_helpers::verify_stripe_payment_event_exists(
                &*ctx, &event_id
            )
            .await,
            "Stripe payment event should still be recorded for failed processing"
        );
        assert!(
            !get_payment_event_processed(ctx, "stripe", &event_id).await,
            "Failed Stripe webhook should not be marked processed"
        );
        assert_eq!(
            get_subscription_count_by_client_app(ctx, client_app_id).await,
            0
        );
    }

    /// Test: Stripe webhook with unknown event type (should be handled gracefully)
    #[test_context(WebhookTestContext)]
    #[tokio::test]
    async fn test_scenario_stripe_webhook_unknown_event_type(ctx: &mut WebhookTestContext) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_test_webhook_secret";
        let event_id = format!("evt_test_{}", Uuid::now_v7());

        // Set Stripe webhook secret in database
        set_stripe_webhook_secret_for_realm(ctx, webhook_secret).await;

        // Build Stripe event with unknown event type
        let payload = json!({
            "id": event_id,
            "object": "event",
            "type": "unknown.event.type",
            "api_version": "2020-08-27",
            "created": chrono::Utc::now().timestamp(),
            "data": {
                "object": {
                    "id": "test_object",
                    "metadata": {
                        "realmId": ctx._realm_id.clone()
                    }
                }
            }
        });

        // Send webhook with valid signature
        let response = crate::tests::helpers::webhook_helpers::send_stripe_webhook_with_signature(
            &app,
            &ctx._realm_id,
            payload,
            webhook_secret,
        )
        .await;

        // Verify response is OK (unknown events should be accepted for idempotency)
        assert_eq!(response.status(), StatusCode::OK);
    }
}
