// =============================================================================
// Stripe Webhook Mode Dispatch Scenario Tests
// =============================================================================
//
// Tests that verify the Stripe checkout.session.completed webhook handler
// correctly dispatches based on data.object.mode:
// - mode=payment + attemptId -> fulfills one-time payment attempt
// - mode=payment without attemptId -> logs only, no fulfillment
// - mode=subscription -> existing subscription logic (unchanged)
// - duplicate checkout.session.completed -> no double fulfillment
// - payment_intent.succeeded after checkout -> no double grant
//
// User Story: US-PA-003, US-PU-006
// Covers: Design section 5.1
// =============================================================================

#[cfg(test)]
mod tests {
    use crate::tests::helpers::billing_helpers::{
        setup_stripe_config, setup_test_entitlement_mapping_full,
    };
    use crate::tests::helpers::webhook_helpers::{
        generate_test_event_id, send_stripe_webhook_with_signature,
    };
    use crate::tests::schema_test_context::SchemaTestContext;
    use axum::http::StatusCode;
    use serde_json::json;
    use test_context::test_context;
    use uuid::Uuid;

    use SchemaTestContext as StripeWebhookModeTestContext;

    // =========================================================================
    // Helpers
    // =========================================================================

    /// Create a one-time entitlement mapping with points_per_period.
    /// Returns the mapping ID.
    async fn create_one_time_mapping_with_points(
        ctx: &mut SchemaTestContext,
        realm_id: &str,
        entitlement_key: &str,
        points_per_period: i64,
    ) -> Uuid {
        setup_test_entitlement_mapping_full(
            ctx,
            realm_id,
            "stripe",
            &format!("prod_stripe_{entitlement_key}"),
            None, // external_price_id
            entitlement_key,
            Some("one_time"), // billing_type
            None,             // billing_period
            Some(points_per_period),
            None,  // grant_period_type
            None,  // validity_days
            false, // grant_on_subscribe
            None,  // max_periods
            true,  // enabled
            Some(json!({
                "price": 1000,
                "currency": "usd",
                "name": format!("Points Pack {}", points_per_period)
            })),
        )
        .await
    }

    /// Create a test user in the test realm and return their UUID.
    async fn create_test_user_and_wallet(
        ctx: &SchemaTestContext,
        realm_id: &str,
        email: &str,
    ) -> Uuid {
        let user_id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO account (id, realm_id, email, password, status)
             VALUES ($1, $2, $3, $4, 1)
             ON CONFLICT (realm_id, email) DO NOTHING",
        )
        .bind(user_id)
        .bind(realm_id)
        .bind(email)
        .bind("$2a$12$dummy_password_hash")
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to create test user");

        sqlx::query(
            "INSERT INTO points_wallets (id, user_id, realm_id, topup_balance, subscription_balance, total_topup_granted, total_subscription_granted, total_recharged, total_consumed, status, created_at, updated_at)
             VALUES ($1, $2, $3, 0, 0, 0, 0, 0, 0, 'active', NOW(), NOW())
             ON CONFLICT (user_id) DO NOTHING",
        )
        .bind(Uuid::now_v7())
        .bind(user_id)
        .bind(realm_id)
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to create points wallet");

        user_id
    }

    /// Create a pending payment attempt targeting the given mapping.
    /// Returns the attempt ID.
    async fn create_pending_payment_attempt(
        ctx: &SchemaTestContext,
        realm_id: &str,
        user_id: Uuid,
        mapping_id: Uuid,
    ) -> Uuid {
        let attempt_id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO payment_attempts
                (id, realm_id, user_id, payment_provider, target_type, target_id,
                 amount, currency, status, expires_at, created_at, updated_at)
             VALUES ($1, $2, $3, 'stripe', 'entitlement_mapping', $4,
                 1000, 'usd', 'Pending', NOW() + INTERVAL '1 hour', NOW(), NOW())",
        )
        .bind(attempt_id)
        .bind(realm_id)
        .bind(user_id)
        .bind(mapping_id)
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to create pending payment attempt");

        attempt_id
    }

    /// Get the topup_balance for a user's wallet.
    async fn get_topup_balance(ctx: &SchemaTestContext, user_id: Uuid, realm_id: &str) -> i64 {
        sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(topup_balance, 0) FROM points_wallets WHERE user_id = $1 AND realm_id = $2",
        )
        .bind(user_id)
        .bind(realm_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap_or(0)
    }

    /// Get the subscription_balance for a user's wallet.
    async fn get_subscription_balance(
        ctx: &SchemaTestContext,
        user_id: Uuid,
        realm_id: &str,
    ) -> i64 {
        sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(subscription_balance, 0) FROM points_wallets WHERE user_id = $1 AND realm_id = $2",
        )
        .bind(user_id)
        .bind(realm_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap_or(0)
    }

    /// Count ledger entries for a user with a specific credit_type.
    async fn count_ledger_entries(
        ctx: &SchemaTestContext,
        user_id: Uuid,
        credit_type: &str,
    ) -> i64 {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM points_credit_ledger WHERE user_id = $1 AND credit_type = $2",
        )
        .bind(user_id)
        .bind(credit_type)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap_or(0)
    }

    /// Build a checkout.session.completed payload with mode=payment and attemptId.
    fn build_stripe_checkout_completed_payment_mode(
        event_id: &str,
        realm_id: &str,
        user_id: Uuid,
        client_app_id: Uuid,
        attempt_id: Uuid,
        entitlement_key: &str,
    ) -> serde_json::Value {
        json!({
            "id": event_id,
            "object": "event",
            "type": "checkout.session.completed",
            "api_version": "2020-08-27",
            "created": chrono::Utc::now().timestamp(),
            "data": {
                "object": {
                    "id": format!("cs_test_{}", Uuid::now_v7()),
                    "object": "checkout.session",
                    "status": "complete",
                    "payment_status": "paid",
                    "mode": "payment",
                    "customer": format!("cus_test_{}", Uuid::now_v7()),
                    "payment_intent": format!("pi_test_{}", Uuid::now_v7()),
                    "metadata": {
                        "herald_realm_id": realm_id,
                        "herald_user_id": user_id.to_string(),
                        "herald_client_app_id": client_app_id.to_string(),
                        "herald_entitlement_key": entitlement_key,
                        "clientAppId": client_app_id.to_string(),
                        "userId": user_id.to_string(),
                        "attemptId": attempt_id.to_string(),
                    },
                    "display_items": [{
                        "price": {
                            "product": format!("prod_stripe_{}", entitlement_key)
                        }
                    }]
                }
            }
        })
    }

    /// Build a checkout.session.completed payload with mode=payment but NO attemptId.
    fn build_stripe_checkout_completed_payment_no_attempt(
        event_id: &str,
        realm_id: &str,
        user_id: Uuid,
        client_app_id: Uuid,
        entitlement_key: &str,
    ) -> serde_json::Value {
        json!({
            "id": event_id,
            "object": "event",
            "type": "checkout.session.completed",
            "api_version": "2020-08-27",
            "created": chrono::Utc::now().timestamp(),
            "data": {
                "object": {
                    "id": format!("cs_test_{}", Uuid::now_v7()),
                    "object": "checkout.session",
                    "status": "complete",
                    "payment_status": "paid",
                    "mode": "payment",
                    "customer": format!("cus_test_{}", Uuid::now_v7()),
                    "payment_intent": format!("pi_test_{}", Uuid::now_v7()),
                    "metadata": {
                        "herald_realm_id": realm_id,
                        "herald_user_id": user_id.to_string(),
                        "herald_client_app_id": client_app_id.to_string(),
                        "herald_entitlement_key": entitlement_key,
                        "clientAppId": client_app_id.to_string(),
                        "userId": user_id.to_string(),
                    },
                    "display_items": [{
                        "price": {
                            "product": format!("prod_stripe_{}", entitlement_key)
                        }
                    }]
                }
            }
        })
    }

    /// Build a checkout.session.completed payload with mode=subscription.
    fn build_stripe_checkout_completed_subscription_mode(
        event_id: &str,
        realm_id: &str,
        user_id: Uuid,
        client_app_id: Uuid,
        entitlement_key: &str,
        subscription_id: &str,
    ) -> serde_json::Value {
        json!({
            "id": event_id,
            "object": "event",
            "type": "checkout.session.completed",
            "api_version": "2020-08-27",
            "created": chrono::Utc::now().timestamp(),
            "data": {
                "object": {
                    "id": format!("cs_test_{}", Uuid::now_v7()),
                    "object": "checkout.session",
                    "status": "complete",
                    "payment_status": "paid",
                    "mode": "subscription",
                    "customer": format!("cus_test_{}", Uuid::now_v7()),
                    "subscription": subscription_id,
                    "metadata": {
                        "herald_realm_id": realm_id,
                        "herald_user_id": user_id.to_string(),
                        "herald_client_app_id": client_app_id.to_string(),
                        "herald_entitlement_key": entitlement_key,
                        "clientAppId": client_app_id.to_string(),
                        "userId": user_id.to_string(),
                    },
                    "display_items": [{
                        "price": {
                            "product": format!("prod_stripe_{}", entitlement_key)
                        }
                    }]
                }
            }
        })
    }

    /// Build a payment_intent.succeeded payload with attemptId in metadata.
    fn build_stripe_payment_intent_succeeded(
        event_id: &str,
        attempt_id: Uuid,
    ) -> serde_json::Value {
        json!({
            "id": event_id,
            "object": "event",
            "type": "payment_intent.succeeded",
            "api_version": "2020-08-27",
            "created": chrono::Utc::now().timestamp(),
            "data": {
                "object": {
                    "id": format!("pi_test_{}", Uuid::now_v7()),
                    "object": "payment_intent",
                    "status": "succeeded",
                    "metadata": {
                        "attemptId": attempt_id.to_string(),
                    },
                    "created": chrono::Utc::now().timestamp(),
                }
            }
        })
    }

    /// Get payment attempt status by ID.
    async fn get_payment_attempt_status(
        ctx: &SchemaTestContext,
        attempt_id: Uuid,
    ) -> Option<String> {
        sqlx::query_scalar::<_, String>("SELECT status FROM payment_attempts WHERE id = $1")
            .bind(attempt_id)
            .fetch_optional(&ctx.app_state.pool)
            .await
            .unwrap_or(None)
    }

    // =========================================================================
    // Test 1: mode=payment + attemptId fulfills one-time
    // =========================================================================

    /// User Story: US-PA-003, US-PU-006
    /// Covers: Design 5.1 "checkout.session.completed(mode=payment) + attemptId -> fulfill"
    ///
    /// Given: A one-time entitlement mapping with points_per_period=1000
    /// And: A user with a points wallet
    /// And: A pending payment attempt targeting the mapping
    /// And: Stripe webhook secret configured
    /// When: Receiving checkout.session.completed with mode=payment and the attempt's ID in metadata
    /// Then: Payment attempt status becomes Succeeded
    /// And: User's topup_balance increases by 1000
    /// And: subscription_balance remains 0
    /// And: Points credit ledger has 1 entry with topup_credit type
    #[test_context(StripeWebhookModeTestContext)]
    #[tokio::test]
    async fn test_stripe_webhook_payment_mode_with_attempt_fulfills_one_time(
        ctx: &mut StripeWebhookModeTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_test_mode_fulfill";
        let realm_id = ctx._realm_id.clone();
        let client_app_id = Uuid::parse_str(&ctx._client_app_id).unwrap();
        let entitlement_key = "mode-fulfill-onetime";

        // Setup: Stripe config
        setup_stripe_config(ctx, &realm_id, "sk_test_mode", webhook_secret).await;

        // Setup: one-time mapping with 1000 points
        let mapping_id =
            create_one_time_mapping_with_points(ctx, &realm_id, entitlement_key, 1000).await;

        // Setup: user + wallet
        let user_id = create_test_user_and_wallet(ctx, &realm_id, "mode-fulfill@test.com").await;

        // Setup: pending payment attempt
        let attempt_id = create_pending_payment_attempt(ctx, &realm_id, user_id, mapping_id).await;

        // Exercise: send checkout.session.completed with mode=payment + attemptId
        let event_id = generate_test_event_id();
        let payload = build_stripe_checkout_completed_payment_mode(
            &event_id,
            &realm_id,
            user_id,
            client_app_id,
            attempt_id,
            entitlement_key,
        );

        let response =
            send_stripe_webhook_with_signature(&app, &realm_id, payload, webhook_secret).await;
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Expected 200 OK for payment mode with attemptId, got {}",
            response.status()
        );

        // Verify: payment attempt status becomes Succeeded
        let attempt_status = get_payment_attempt_status(ctx, attempt_id)
            .await
            .expect("Payment attempt should exist");
        assert_eq!(
            attempt_status, "Succeeded",
            "Payment attempt should be Succeeded"
        );

        // Verify: topup_balance increased by 1000
        let topup = get_topup_balance(ctx, user_id, &realm_id).await;
        assert_eq!(topup, 1000, "topup_balance should be 1000, got {}", topup);

        // Verify: subscription_balance remains 0
        let sub_balance = get_subscription_balance(ctx, user_id, &realm_id).await;
        assert_eq!(
            sub_balance, 0,
            "subscription_balance should remain 0, got {}",
            sub_balance
        );

        // Verify: exactly 1 topup_credit ledger entry
        let ledger_count = count_ledger_entries(ctx, user_id, "topup_credit").await;
        assert_eq!(
            ledger_count, 1,
            "Expected exactly 1 topup_credit ledger entry, got {}",
            ledger_count
        );
    }

    // =========================================================================
    // Test 2: mode=payment without attemptId logs only
    // =========================================================================

    /// User Story: US-PA-003
    /// Covers: Design 5.1 "no attemptId -> log only, no fulfillment"
    ///
    /// Given: Stripe webhook secret configured
    /// And: A user with a points wallet exists
    /// When: Receiving checkout.session.completed with mode=payment and no attemptId in metadata
    /// Then: Response is OK (200)
    /// And: No payment attempt is modified
    /// And: No points are granted to any user
    /// And: Payment event is recorded (audit)
    #[test_context(StripeWebhookModeTestContext)]
    #[tokio::test]
    async fn test_stripe_webhook_payment_mode_without_attempt_logs_only(
        ctx: &mut StripeWebhookModeTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_test_mode_no_attempt";
        let realm_id = ctx._realm_id.clone();
        let client_app_id = Uuid::parse_str(&ctx._client_app_id).unwrap();
        let entitlement_key = "mode-no-attempt";

        // Setup: Stripe config
        setup_stripe_config(ctx, &realm_id, "sk_test_no_att", webhook_secret).await;

        // Setup: user + wallet (so we can verify no points are granted)
        let user_id = create_test_user_and_wallet(ctx, &realm_id, "mode-no-attempt@test.com").await;

        // Exercise: send checkout.session.completed with mode=payment, no attemptId
        let event_id = generate_test_event_id();
        let payload = build_stripe_checkout_completed_payment_no_attempt(
            &event_id,
            &realm_id,
            user_id,
            client_app_id,
            entitlement_key,
        );

        let response =
            send_stripe_webhook_with_signature(&app, &realm_id, payload, webhook_secret).await;
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Expected 200 OK for payment mode without attemptId, got {}",
            response.status()
        );

        // Verify: no topup points granted
        let topup = get_topup_balance(ctx, user_id, &realm_id).await;
        assert_eq!(topup, 0, "topup_balance should remain 0, got {}", topup);

        // Verify: no topup_credit ledger entries
        let ledger_count = count_ledger_entries(ctx, user_id, "topup_credit").await;
        assert_eq!(
            ledger_count, 0,
            "Expected 0 topup_credit ledger entries, got {}",
            ledger_count
        );

        // Verify: payment event was recorded (audit trail)
        let event_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM payment_event WHERE external_event_id = $1 AND payment_provider = 'stripe'",
        )
        .bind(&event_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap();
        assert_eq!(
            event_count, 1,
            "Expected 1 payment event for audit, got {}",
            event_count
        );
    }

    // =========================================================================
    // Test 3: mode=subscription unchanged (existing subscription logic)
    // =========================================================================

    /// User Story: US-PA-003
    /// Covers: Design 5.1 "mode=subscription -> existing subscription logic"
    ///
    /// Given: A recurring entitlement mapping exists
    /// And: A user exists
    /// When: Receiving checkout.session.completed with mode=subscription
    /// Then: Response is OK
    /// And: Existing subscription creation logic runs (no crash, no one-time fulfillment)
    #[test_context(StripeWebhookModeTestContext)]
    #[tokio::test]
    async fn test_stripe_webhook_subscription_mode_unchanged(
        ctx: &mut StripeWebhookModeTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_test_mode_sub";
        let realm_id = ctx._realm_id.clone();
        let client_app_id = Uuid::parse_str(&ctx._client_app_id).unwrap();
        let entitlement_key = "mode-sub-unchanged";

        // Setup: Stripe config
        setup_stripe_config(ctx, &realm_id, "sk_test_sub", webhook_secret).await;

        // Setup: recurring entitlement mapping
        setup_test_entitlement_mapping_full(
            ctx,
            &realm_id,
            "stripe",
            &format!("prod_stripe_{entitlement_key}"),
            None,
            entitlement_key,
            Some("recurring"),
            Some("monthly"),
            Some(500),
            None,
            None,
            true,
            None,
            true,
            Some(json!({
                "price": 1200,
                "currency": "usd"
            })),
        )
        .await;

        // Setup: user (no wallet needed, subscription flow does not require it here)
        let user_id = create_test_user_and_wallet(ctx, &realm_id, "mode-sub@test.com").await;

        // Exercise: send checkout.session.completed with mode=subscription
        let event_id = generate_test_event_id();
        let stripe_sub_id = format!("sub_test_{}", Uuid::now_v7());
        let payload = build_stripe_checkout_completed_subscription_mode(
            &event_id,
            &realm_id,
            user_id,
            client_app_id,
            entitlement_key,
            &stripe_sub_id,
        );

        let response =
            send_stripe_webhook_with_signature(&app, &realm_id, payload, webhook_secret).await;
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Expected 200 OK for subscription mode, got {}",
            response.status()
        );

        // Verify: no topup_credit granted (subscription flow grants subscription_credit,
        // but checkout.session.completed alone does not grant points - that happens
        // via customer.subscription.created event)
        let topup = get_topup_balance(ctx, user_id, &realm_id).await;
        assert_eq!(
            topup, 0,
            "topup_balance should remain 0 for subscription mode, got {}",
            topup
        );

        // Verify: a subscription was created with the external_subscription_id
        let sub_exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM subscription WHERE external_subscription_id = $1 AND payment_provider = 'stripe'",
        )
        .bind(&stripe_sub_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap();
        assert_eq!(
            sub_exists, 1,
            "Expected 1 subscription created for subscription mode, got {}",
            sub_exists
        );
    }

    // =========================================================================
    // Test 4: duplicate checkout.session.completed -> no double fulfillment
    // =========================================================================

    /// User Story: US-PA-003, US-PU-006
    /// Covers: Design 5.1 "duplicate checkout.session.completed -> no double-fulfillment"
    ///
    /// Given: A one-time mapping with points_per_period=500
    /// And: A user with a wallet and a pending payment attempt
    /// When: Sending checkout.session.completed(mode=payment) twice with the same event_id
    /// Then: First webhook succeeds and grants 500 topup points
    /// And: Second webhook returns OK (idempotent)
    /// And: User still has exactly 500 topup points (not 1000)
    /// And: Only 1 ledger entry exists
    #[test_context(StripeWebhookModeTestContext)]
    #[tokio::test]
    async fn test_stripe_webhook_duplicate_checkout_no_double_fulfillment(
        ctx: &mut StripeWebhookModeTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_test_mode_dup";
        let realm_id = ctx._realm_id.clone();
        let client_app_id = Uuid::parse_str(&ctx._client_app_id).unwrap();
        let entitlement_key = "mode-dup-onetime";

        // Setup: Stripe config
        setup_stripe_config(ctx, &realm_id, "sk_test_dup", webhook_secret).await;

        // Setup: one-time mapping with 500 points
        let mapping_id =
            create_one_time_mapping_with_points(ctx, &realm_id, entitlement_key, 500).await;

        // Setup: user + wallet
        let user_id = create_test_user_and_wallet(ctx, &realm_id, "mode-dup@test.com").await;

        // Setup: pending payment attempt
        let attempt_id = create_pending_payment_attempt(ctx, &realm_id, user_id, mapping_id).await;

        // Build payload with a fixed event_id for duplication test
        let event_id = generate_test_event_id();
        let payload = build_stripe_checkout_completed_payment_mode(
            &event_id,
            &realm_id,
            user_id,
            client_app_id,
            attempt_id,
            entitlement_key,
        );

        // Exercise: send first webhook
        let response1 =
            send_stripe_webhook_with_signature(&app, &realm_id, payload.clone(), webhook_secret)
                .await;
        assert_eq!(
            response1.status(),
            StatusCode::OK,
            "First webhook should return OK, got {}",
            response1.status()
        );

        // Exercise: send duplicate webhook with same event_id
        let response2 =
            send_stripe_webhook_with_signature(&app, &realm_id, payload, webhook_secret).await;
        assert_eq!(
            response2.status(),
            StatusCode::OK,
            "Duplicate webhook should return OK (idempotent), got {}",
            response2.status()
        );

        // Verify: user has exactly 500 topup points (not 1000)
        let topup = get_topup_balance(ctx, user_id, &realm_id).await;
        assert_eq!(
            topup, 500,
            "topup_balance should be exactly 500 (not doubled), got {}",
            topup
        );

        // Verify: exactly 1 topup_credit ledger entry
        let ledger_count = count_ledger_entries(ctx, user_id, "topup_credit").await;
        assert_eq!(
            ledger_count, 1,
            "Expected exactly 1 topup_credit ledger entry, got {}",
            ledger_count
        );
    }

    // =========================================================================
    // Test 5: payment_intent.succeeded after checkout -> no double grant
    // =========================================================================

    /// User Story: US-PA-003
    /// Covers: Design 5.1 "payment_intent.succeeded after checkout.session.completed
    ///         -> no double-grant"
    ///
    /// Given: A one-time mapping with points_per_period=750
    /// And: A user with a wallet and a pending payment attempt
    /// When: First sending checkout.session.completed(mode=payment) and it succeeds
    /// And: Then sending payment_intent.succeeded for the same payment attempt
    /// Then: User has exactly 750 topup points (not 1500)
    /// And: Only 1 ledger entry exists
    #[test_context(StripeWebhookModeTestContext)]
    #[tokio::test]
    async fn test_stripe_webhook_payment_intent_after_checkout_no_double_grant(
        ctx: &mut StripeWebhookModeTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_test_mode_pi";
        let realm_id = ctx._realm_id.clone();
        let client_app_id = Uuid::parse_str(&ctx._client_app_id).unwrap();
        let entitlement_key = "mode-pi-onetime";

        // Setup: Stripe config
        setup_stripe_config(ctx, &realm_id, "sk_test_pi", webhook_secret).await;

        // Setup: one-time mapping with 750 points
        let mapping_id =
            create_one_time_mapping_with_points(ctx, &realm_id, entitlement_key, 750).await;

        // Setup: user + wallet
        let user_id = create_test_user_and_wallet(ctx, &realm_id, "mode-pi@test.com").await;

        // Setup: pending payment attempt
        let attempt_id = create_pending_payment_attempt(ctx, &realm_id, user_id, mapping_id).await;

        // Exercise: send checkout.session.completed with mode=payment + attemptId
        let checkout_event_id = generate_test_event_id();
        let checkout_payload = build_stripe_checkout_completed_payment_mode(
            &checkout_event_id,
            &realm_id,
            user_id,
            client_app_id,
            attempt_id,
            entitlement_key,
        );

        let checkout_response =
            send_stripe_webhook_with_signature(&app, &realm_id, checkout_payload, webhook_secret)
                .await;
        assert_eq!(
            checkout_response.status(),
            StatusCode::OK,
            "checkout.session.completed should return OK, got {}",
            checkout_response.status()
        );

        // Exercise: send payment_intent.succeeded for the same attempt
        let pi_event_id = generate_test_event_id();
        let pi_payload = build_stripe_payment_intent_succeeded(&pi_event_id, attempt_id);

        let pi_response =
            send_stripe_webhook_with_signature(&app, &realm_id, pi_payload, webhook_secret).await;
        // payment_intent.succeeded attempts to complete the attempt again.
        // Since the attempt is already Succeeded, the fulfill_one_time_purchase path
        // checks the ledger and returns the existing fulfillment (idempotent).
        // The response should be OK regardless.
        assert_eq!(
            pi_response.status(),
            StatusCode::OK,
            "payment_intent.succeeded should return OK, got {}",
            pi_response.status()
        );

        // Verify: user has exactly 750 topup points (not 1500)
        let topup = get_topup_balance(ctx, user_id, &realm_id).await;
        assert_eq!(
            topup, 750,
            "topup_balance should be exactly 750 (not doubled), got {}",
            topup
        );

        // Verify: exactly 1 topup_credit ledger entry
        let ledger_count = count_ledger_entries(ctx, user_id, "topup_credit").await;
        assert_eq!(
            ledger_count, 1,
            "Expected exactly 1 topup_credit ledger entry, got {}",
            ledger_count
        );
    }
}
