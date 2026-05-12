// =============================================================================
// Webhook Infrastructure Tests (Stage 1)
// =============================================================================
//
// Test webhook infrastructure components:
// - Signature verification
// - Idempotency key generation
// - Event routing
// - Async processing with tokio::spawn
// - Error handling
//
// User Story: docs/user-stories/points-billing-events.md
// Covers: Webhook infrastructure (P0)
//
// =============================================================================

#[cfg(test)]
mod tests {
    use crate::tests::helpers::webhook_helpers::*;
    use crate::tests::schema_test_context::SchemaTestContext;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use hex;
    use hmac::{Hmac, Mac};
    use serde_json::json;
    use sha2::Sha256;
    use test_context::test_context;
    use tower::ServiceExt;
    use uuid::Uuid;

    type HmacSha256 = Hmac<Sha256>;

    // User Story: docs/user-stories/points-billing-events.md
    // Covers: Webhook 事件处理 - Stage 1: Infrastructure

    /// Set webhook secret for a realm in the database
    async fn set_webhook_secret_for_realm(ctx: &SchemaTestContext, webhook_secret: &str) {
        ctx.with_creem_config(
            &ctx._realm_id,
            Some("test_api_key"),
            Some(webhook_secret),
            Some(30),
        )
        .await;
    }

    // ============================================================================
    // Test Category 1: Webhook Signature Verification (P0)
    // ============================================================================

    /// Test: Webhook signature verification succeeds with valid signature
    #[test_context(SchemaTestContext)]
    #[tokio::test]
    async fn test_scenario_webhook_signature_valid(ctx: &mut SchemaTestContext) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "test_webhook_secret";
        let event_id = generate_test_event_id();
        let plan_id = generate_test_plan_id();

        set_webhook_secret_for_realm(ctx, webhook_secret).await;

        // Given: Create a test user
        let user_id = crate::tests::helpers::test_setup_helpers::create_test_user(
            ctx,
            "test-webhook-signature@example.com",
            "password123",
        )
        .await;
        let realm_id = ctx._realm_id.clone();
        crate::tests::helpers::points_helpers::create_points_account(ctx, user_id, &realm_id).await;

        // Given: Setup plan config for the test
        setup_test_plan_config(ctx, &ctx._realm_id, plan_id).await;

        // Given: A valid webhook payload
        let payload = build_subscription_paid_event(
            event_id.clone(),
            user_id,
            plan_id,
            false,
            &ctx._realm_id,
        );

        // When: Sending webhook with valid signature
        let response =
            send_webhook_with_signature(&app, &ctx._realm_id, payload, webhook_secret).await;

        // Then: Response should be 200 OK (processing completed)
        assert_eq!(response.status(), StatusCode::OK);
    }

    /// Test: Webhook signature verification fails with invalid signature
    #[test_context(SchemaTestContext)]
    #[tokio::test]
    async fn test_scenario_webhook_signature_invalid(ctx: &mut SchemaTestContext) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "test_webhook_secret";
        let event_id = generate_test_event_id();

        set_webhook_secret_for_realm(ctx, webhook_secret).await;

        // Given: A webhook payload
        let payload = build_subscription_paid_event(
            event_id,
            generate_test_user_id(),
            generate_test_plan_id(),
            false,
            &ctx._realm_id,
        );

        // When: Sending webhook with invalid signature
        let response = send_webhook_with_invalid_signature(&app, &ctx._realm_id, payload).await;

        // Then: Response should be 401 Unauthorized or 400 Bad Request
        let status = response.status();
        assert!(
            status == StatusCode::UNAUTHORIZED || status == StatusCode::BAD_REQUEST,
            "Expected 401 or 400, got {}",
            status
        );
    }

    /// Test: Webhook without signature header is rejected
    #[test_context(SchemaTestContext)]
    #[tokio::test]
    async fn test_scenario_webhook_missing_signature(ctx: &mut SchemaTestContext) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "test_webhook_secret";
        let event_id = generate_test_event_id();

        set_webhook_secret_for_realm(ctx, webhook_secret).await;

        // Given: A webhook payload
        let payload = build_subscription_paid_event(
            event_id,
            generate_test_user_id(),
            generate_test_plan_id(),
            false,
            &ctx._realm_id,
        );

        // When: Sending webhook without signature
        let response = send_webhook_without_signature(&app, &ctx._realm_id, payload).await;

        // Then: Response should be 401 Unauthorized or 400 Bad Request
        let status = response.status();
        assert!(
            status == StatusCode::UNAUTHORIZED || status == StatusCode::BAD_REQUEST,
            "Expected 401 or 400, got {}",
            status
        );
    }

    // ============================================================================
    // Test Category 2: Idempotency Key Generation (P0)
    // ============================================================================

    /// Test: Idempotency key format is correct (creem_{event_id})
    #[test_context(SchemaTestContext)]
    #[tokio::test]
    async fn test_scenario_webhook_idempotency_key_format(_ctx: &mut SchemaTestContext) {
        // Given: A webhook event ID
        let event_id = "evt_test_12345";

        // When: Generating idempotency key
        let key = generate_webhook_idempotency_key(event_id);

        // Then: Key should be in format "creem_{event_id}"
        assert_eq!(key, "creem_evt_test_12345");
    }

    /// Test: Idempotency key is unique for each event
    #[test_context(SchemaTestContext)]
    #[tokio::test]
    async fn test_scenario_webhook_idempotency_key_uniqueness(_ctx: &mut SchemaTestContext) {
        // Given: Two different webhook event IDs
        let event_id_1 = generate_test_event_id();
        let event_id_2 = generate_test_event_id();

        // When: Generating idempotency keys
        let key_1 = generate_webhook_idempotency_key(&event_id_1);
        let key_2 = generate_webhook_idempotency_key(&event_id_2);

        // Then: Keys should be different
        assert_ne!(key_1, key_2, "Idempotency keys should be unique");
    }

    // ============================================================================
    // Test Category 3: Event Routing (P0)
    // ============================================================================

    /// Test: subscription.paid event is routed correctly
    #[test_context(SchemaTestContext)]
    #[tokio::test]
    async fn test_scenario_webhook_routing_subscription_paid(ctx: &mut SchemaTestContext) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "test_webhook_secret";
        let event_id = generate_test_event_id();
        let plan_id = generate_test_plan_id();

        set_webhook_secret_for_realm(ctx, webhook_secret).await;

        // Given: Create a test user
        let user_id = crate::tests::helpers::test_setup_helpers::create_test_user(
            ctx,
            "test-webhook-paid@example.com",
            "password123",
        )
        .await;
        let realm_id = ctx._realm_id.clone();
        crate::tests::helpers::points_helpers::create_points_account(ctx, user_id, &realm_id).await;

        // Given: Setup plan config for the test
        setup_test_plan_config(ctx, &ctx._realm_id, plan_id).await;

        // Given: A subscription.paid event
        let payload =
            build_subscription_paid_event(event_id, user_id, plan_id, false, &ctx._realm_id);

        // When: Sending webhook
        let response =
            send_webhook_with_signature(&app, &ctx._realm_id, payload, webhook_secret).await;

        // Then: Response should be 200 OK (event routed and processed)
        assert_eq!(response.status(), StatusCode::OK);
    }

    /// Test: subscription.update event is routed correctly
    #[test_context(SchemaTestContext)]
    #[tokio::test]
    async fn test_scenario_webhook_routing_subscription_updated(ctx: &mut SchemaTestContext) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "test_webhook_secret";
        let event_id = generate_test_event_id();
        let previous_plan_id = generate_test_plan_id();
        let current_plan_id = generate_test_plan_id();

        set_webhook_secret_for_realm(ctx, webhook_secret).await;

        // Given: Create a test user
        let user_id = crate::tests::helpers::test_setup_helpers::create_test_user(
            ctx,
            "test-webhook-updated@example.com",
            "password123",
        )
        .await;

        // Given: Setup plan configs for the test
        setup_test_plan_config(ctx, &ctx._realm_id, previous_plan_id).await;
        setup_test_plan_config(ctx, &ctx._realm_id, current_plan_id).await;

        // Given: A subscription.update event
        let payload = build_subscription_updated_event(
            event_id,
            user_id,
            previous_plan_id,
            current_plan_id,
            &ctx._realm_id,
        );

        // When: Sending webhook
        let response =
            send_webhook_with_signature(&app, &ctx._realm_id, payload, webhook_secret).await;

        // Then: Response should be 200 OK
        assert_eq!(response.status(), StatusCode::OK);
    }

    /// Test: subscription.canceled event is routed correctly
    #[test_context(SchemaTestContext)]
    #[tokio::test]
    async fn test_scenario_webhook_routing_subscription_canceled(ctx: &mut SchemaTestContext) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "test_webhook_secret";
        let event_id = generate_test_event_id();
        let user_id = generate_test_user_id();

        set_webhook_secret_for_realm(ctx, webhook_secret).await;

        // Given: A subscription.canceled event
        let payload = build_subscription_canceled_event(event_id, user_id, false, &ctx._realm_id);

        // When: Sending webhook
        let response =
            send_webhook_with_signature(&app, &ctx._realm_id, payload, webhook_secret).await;

        // Then: Response should be 200 OK
        assert_eq!(response.status(), StatusCode::OK);
    }

    /// Test: refund.created event is routed correctly
    #[test_context(SchemaTestContext)]
    #[tokio::test]
    async fn test_scenario_webhook_routing_refund_created(ctx: &mut SchemaTestContext) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "test_webhook_secret";
        let event_id = generate_test_event_id();
        let refund_id = format!("reft_{}", Uuid::now_v7());
        let payment_id = format!("pay_{}", Uuid::now_v7());
        let user_id = generate_test_user_id();

        set_webhook_secret_for_realm(ctx, webhook_secret).await;

        // Given: A refund.created event with user ID in metadata
        let payload = build_refund_created_event_with_user(
            event_id,
            refund_id,
            payment_id,
            2500,
            2500,
            &ctx._realm_id,
            user_id,
        );

        // When: Sending webhook
        let response =
            send_webhook_with_signature(&app, &ctx._realm_id, payload, webhook_secret).await;

        // Then: Response should be 200 OK
        assert_eq!(response.status(), StatusCode::OK);
    }

    /// Test: Unknown event type is handled gracefully
    #[test_context(SchemaTestContext)]
    #[tokio::test]
    async fn test_scenario_webhook_routing_unknown_event(ctx: &mut SchemaTestContext) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "test_webhook_secret";
        let event_id = generate_test_event_id();

        set_webhook_secret_for_realm(ctx, webhook_secret).await;

        // Given: An unknown event type
        let payload = json!({
            "id": event_id,
            "eventType": "unknown.event",
            "data": {},
            "metadata": {
                "realmId": ctx._realm_id.clone()
            }
        });

        // When: Sending webhook
        let response =
            send_webhook_with_signature(&app, &ctx._realm_id, payload, webhook_secret).await;

        // Then: Response should still be 200 OK (processing completed, error logged)
        assert_eq!(response.status(), StatusCode::OK);
    }

    // ============================================================================
    // Test Category 4: Event Parsing and Validation (P0)
    // ============================================================================

    /// Test: Webhook with missing event ID returns error
    #[test_context(SchemaTestContext)]
    #[tokio::test]
    async fn test_scenario_webhook_validation_missing_event_id(ctx: &mut SchemaTestContext) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "test_webhook_secret";

        set_webhook_secret_for_realm(ctx, webhook_secret).await;

        // Given: A webhook payload without event ID
        let payload = json!({
            "eventType": "subscription.paid",
            "data": {
                "object": {
                    "userId": generate_test_user_id().to_string(),
                    "planId": generate_test_plan_id().to_string()
                }
            },
            "metadata": {
                "realmId": ctx._realm_id.clone()
            }
        });

        // When: Sending webhook
        let response =
            send_webhook_with_signature(&app, &ctx._realm_id, payload, webhook_secret).await;

        // Then: Response should be 400 Bad Request
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    /// Test: Webhook with missing event type returns error
    #[test_context(SchemaTestContext)]
    #[tokio::test]
    async fn test_scenario_webhook_validation_missing_event_type(ctx: &mut SchemaTestContext) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "test_webhook_secret";
        let event_id = generate_test_event_id();

        set_webhook_secret_for_realm(ctx, webhook_secret).await;

        // Given: A webhook payload without event type
        let payload = json!({
            "id": event_id,
            "data": {
                "object": {
                    "userId": generate_test_user_id().to_string(),
                    "planId": generate_test_plan_id().to_string()
                }
            },
            "metadata": {
                "realmId": ctx._realm_id.clone()
            }
        });

        // When: Sending webhook
        let response =
            send_webhook_with_signature(&app, &ctx._realm_id, payload, webhook_secret).await;

        // Then: Response should be 400 Bad Request
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    /// Test: Webhook with invalid JSON returns error
    #[test_context(SchemaTestContext)]
    #[tokio::test]
    async fn test_scenario_webhook_validation_invalid_json(ctx: &mut SchemaTestContext) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "test_webhook_secret";

        set_webhook_secret_for_realm(ctx, webhook_secret).await;

        // Given: Invalid JSON payload
        let invalid_json = "{invalid json}";

        // Generate signature for invalid JSON
        let mut mac = HmacSha256::new_from_slice(webhook_secret.as_bytes()).unwrap();
        mac.update(invalid_json.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        // When: Sending webhook
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/third/pay/{}/creem/webhooks", ctx._realm_id))
                    .header("creem-signature", signature)
                    .header("Content-Type", "application/json")
                    .body(Body::from(invalid_json.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Then: Response should be 400 Bad Request
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    // ============================================================================
    // Test Category 5: Placeholder Transaction Creation (P0)
    // ============================================================================

    /// Test: Placeholder transactions are created with correct structure
    #[test_context(SchemaTestContext)]
    #[tokio::test]
    async fn test_scenario_webhook_placeholder_transaction_structure(_ctx: &mut SchemaTestContext) {
        // This test verifies that the placeholder transaction function works correctly
        // The actual webhook handlers use this internally

        use herald_core::domain::points::entities::TransactionType;

        // Given: A user ID and realm ID
        let user_id = generate_test_user_id();
        let realm_id = "test-realm";
        let transaction_type = TransactionType::SubscriptionGrant;

        // When: Creating a placeholder transaction
        let tx =
            crate::application::http::billing::webhook_handlers::create_placeholder_transaction(
                user_id,
                realm_id,
                transaction_type.clone(),
            );

        // Then: Transaction should have correct structure
        assert_eq!(tx.user_id, user_id);
        assert_eq!(tx.realm_id, realm_id);
        assert_eq!(tx.transaction_type, transaction_type);
        assert_eq!(tx.amount, 0);
        assert_eq!(tx.balance_after, 0);
    }
}
