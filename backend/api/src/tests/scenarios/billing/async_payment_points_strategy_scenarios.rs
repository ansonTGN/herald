// =============================================================================
// Async Payment Points Strategy Scenario Tests
// =============================================================================
//
// Tests for the async payment points distribution strategy toggle:
// - US-AP-001: Strategy config reading (conservative default, eager, invalid fallback)
// - US-AP-002: Eager fulfillment on checkout.session.completed(unpaid)
//   - One-time: topup_balance granted
//   - Subscription: subscription_balance granted
//   - Paid: regression (eager does not interfere with normal paid path)
//
// User Story: US-AP-001, US-AP-002
// Covers: Design sections 4.1, 5.1
// =============================================================================

#[cfg(test)]
mod tests {
    use crate::tests::helpers::async_payment_helpers::{
        create_one_time_mapping, create_pending_payment_attempt, create_points_wallet,
        create_recurring_mapping, create_test_user, get_payment_attempt_status,
        get_subscription_balance, get_topup_balance, set_async_points_strategy,
    };
    use crate::tests::helpers::billing_helpers::setup_stripe_config;
    use crate::tests::helpers::webhook_helpers::{
        generate_test_event_id, send_stripe_webhook_with_signature,
    };
    use crate::tests::schema_test_context::SchemaTestContext;
    use axum::http::StatusCode;
    use serde_json::json;
    use test_context::test_context;
    use uuid::Uuid;

    use SchemaTestContext as StrategyTestContext;

    // =========================================================================
    // Payload Builders
    // =========================================================================

    /// Build a checkout.session.completed payload with payment_status=unpaid, mode=payment.
    fn build_stripe_checkout_completed_unpaid(
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
                    "payment_status": "unpaid",
                    "mode": "payment",
                    "metadata": {
                        "attemptId": attempt_id.to_string(),
                        "herald_realm_id": realm_id,
                        "herald_user_id": user_id.to_string(),
                        "herald_client_app_id": client_app_id.to_string(),
                        "herald_entitlement_key": entitlement_key,
                        "clientAppId": client_app_id.to_string(),
                    },
                    "display_items": [{
                        "price": {
                            "product": format!("prod_stripe_{}", entitlement_key),
                        }
                    }]
                }
            }
        })
    }

    /// Build a checkout.session.completed payload with payment_status=unpaid, mode=subscription.
    fn build_stripe_checkout_completed_unpaid_subscription(
        event_id: &str,
        realm_id: &str,
        user_id: Uuid,
        client_app_id: Uuid,
        attempt_id: Uuid,
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
                    "payment_status": "unpaid",
                    "mode": "subscription",
                    "subscription": subscription_id,
                    "metadata": {
                        "attemptId": attempt_id.to_string(),
                        "herald_realm_id": realm_id,
                        "herald_user_id": user_id.to_string(),
                        "herald_client_app_id": client_app_id.to_string(),
                        "herald_entitlement_key": entitlement_key,
                        "clientAppId": client_app_id.to_string(),
                    },
                    "display_items": [{
                        "price": {
                            "product": format!("prod_stripe_{}", entitlement_key),
                        }
                    }]
                }
            }
        })
    }

    /// Build a checkout.session.completed payload with payment_status=paid, mode=payment.
    fn build_stripe_checkout_completed_paid(
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
                        "attemptId": attempt_id.to_string(),
                        "herald_realm_id": realm_id,
                        "herald_user_id": user_id.to_string(),
                        "herald_client_app_id": client_app_id.to_string(),
                        "herald_entitlement_key": entitlement_key,
                        "clientAppId": client_app_id.to_string(),
                    },
                    "display_items": [{
                        "price": {
                            "product": format!("prod_stripe_{}", entitlement_key),
                        }
                    }]
                }
            }
        })
    }

    // =========================================================================
    // Test 1: No config -> Conservative behavior (no fulfillment on unpaid)
    // =========================================================================

    /// User Story: US-AP-001
    /// Covers: Design 4.1 "default Conservative when no config exists"
    ///
    /// Given: A realm with NO async_points_strategy config row
    /// And: A one-time mapping with 500 points, a user + wallet, and a pending payment attempt
    /// When: checkout.session.completed arrives with payment_status=unpaid
    /// Then: Attempt status remains Pending, topup_balance stays 0
    /// Why: Validates the default conservative strategy when no config is set at all,
    ///      ensuring existing behavior is unchanged without explicit configuration.
    #[test_context(StrategyTestContext)]
    #[tokio::test]
    async fn test_strategy_default_is_conservative(ctx: &mut StrategyTestContext) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_strategy_default";
        let realm_id = ctx._realm_id.clone();
        let client_app_id = Uuid::parse_str(&ctx._client_app_id).unwrap();
        let entitlement_key = "strategy-default";

        // Setup: Stripe config (no async_points_strategy set)
        setup_stripe_config(ctx, &realm_id, "sk_test_default", webhook_secret).await;
        let mapping_id = create_one_time_mapping(ctx, &realm_id, entitlement_key, 500).await;
        let user_id = create_test_user(ctx, &realm_id, "strategy-default@test.com").await;
        create_points_wallet(ctx, user_id, &realm_id).await;
        let attempt_id = create_pending_payment_attempt(ctx, &realm_id, user_id, mapping_id).await;

        // Exercise: send checkout.session.completed with payment_status=unpaid
        let event_id = generate_test_event_id();
        let payload = build_stripe_checkout_completed_unpaid(
            &event_id,
            &realm_id,
            user_id,
            client_app_id,
            attempt_id,
            entitlement_key,
        );
        let response =
            send_stripe_webhook_with_signature(&app, &realm_id, payload, webhook_secret).await;
        assert_eq!(response.status(), StatusCode::OK, "Expected 200 OK");

        // Verify: attempt stays Pending (conservative = no fulfillment on unpaid)
        let attempt_status = get_payment_attempt_status(ctx, attempt_id)
            .await
            .expect("Payment attempt should exist");
        assert_eq!(
            attempt_status, "Pending",
            "Attempt should remain Pending under default conservative strategy"
        );

        // Verify: no points granted
        let topup = get_topup_balance(ctx, user_id, &realm_id).await;
        assert_eq!(
            topup, 0,
            "No points should be granted under conservative strategy"
        );
    }

    // =========================================================================
    // Test 2: Explicit eager config -> fulfillment on unpaid
    // =========================================================================

    /// User Story: US-AP-001, US-AP-002
    /// Covers: Design 4.1 "eager strategy", 5.1 "eager fulfillment on unpaid"
    ///
    /// Given: A realm with async_points_strategy = "eager"
    /// And: A one-time mapping with 500 points, a user + wallet, and a pending payment attempt
    /// When: checkout.session.completed arrives with payment_status=unpaid
    /// Then: Attempt status becomes Succeeded, topup_balance = 500
    /// Why: Validates that explicit eager strategy causes immediate fulfillment
    ///      even when payment has not yet settled.
    #[test_context(StrategyTestContext)]
    #[tokio::test]
    async fn test_strategy_eager_configured(ctx: &mut StrategyTestContext) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_strategy_eager";
        let realm_id = ctx._realm_id.clone();
        let client_app_id = Uuid::parse_str(&ctx._client_app_id).unwrap();
        let entitlement_key = "strategy-eager";

        // Setup: Stripe config + eager strategy
        setup_stripe_config(ctx, &realm_id, "sk_test_eager", webhook_secret).await;
        set_async_points_strategy(ctx, &realm_id, "eager").await;
        let mapping_id = create_one_time_mapping(ctx, &realm_id, entitlement_key, 500).await;
        let user_id = create_test_user(ctx, &realm_id, "strategy-eager@test.com").await;
        create_points_wallet(ctx, user_id, &realm_id).await;
        let attempt_id = create_pending_payment_attempt(ctx, &realm_id, user_id, mapping_id).await;

        // Exercise: send checkout.session.completed with payment_status=unpaid
        let event_id = generate_test_event_id();
        let payload = build_stripe_checkout_completed_unpaid(
            &event_id,
            &realm_id,
            user_id,
            client_app_id,
            attempt_id,
            entitlement_key,
        );
        let response =
            send_stripe_webhook_with_signature(&app, &realm_id, payload, webhook_secret).await;
        assert_eq!(response.status(), StatusCode::OK, "Expected 200 OK");

        // Verify: attempt becomes Succeeded (eager = immediate fulfillment)
        let attempt_status = get_payment_attempt_status(ctx, attempt_id)
            .await
            .expect("Payment attempt should exist");
        assert_eq!(
            attempt_status, "Succeeded",
            "Attempt should be Succeeded under eager strategy"
        );

        // Verify: points granted
        let topup = get_topup_balance(ctx, user_id, &realm_id).await;
        assert_eq!(
            topup, 500,
            "topup_balance should be 500 under eager strategy"
        );
    }

    // =========================================================================
    // Test 3: Invalid strategy value -> Conservative (no fulfillment)
    // =========================================================================

    /// User Story: US-AP-001
    /// Covers: Design 4.1 "invalid values treated as Conservative"
    ///
    /// Given: A realm with async_points_strategy = "garbage" (invalid value)
    /// And: A one-time mapping with 500 points, a user + wallet, and a pending payment attempt
    /// When: checkout.session.completed arrives with payment_status=unpaid
    /// Then: Attempt status remains Pending, topup_balance stays 0
    /// Why: Validates safety net: explicitly set but invalid config values
    ///      are treated as conservative rather than causing errors or eager behavior.
    #[test_context(StrategyTestContext)]
    #[tokio::test]
    async fn test_strategy_invalid_value_treated_as_conservative(ctx: &mut StrategyTestContext) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_strategy_invalid";
        let realm_id = ctx._realm_id.clone();
        let client_app_id = Uuid::parse_str(&ctx._client_app_id).unwrap();
        let entitlement_key = "strategy-invalid";

        // Setup: Stripe config + invalid strategy value
        setup_stripe_config(ctx, &realm_id, "sk_test_invalid", webhook_secret).await;
        set_async_points_strategy(ctx, &realm_id, "garbage").await;
        let mapping_id = create_one_time_mapping(ctx, &realm_id, entitlement_key, 500).await;
        let user_id = create_test_user(ctx, &realm_id, "strategy-invalid@test.com").await;
        create_points_wallet(ctx, user_id, &realm_id).await;
        let attempt_id = create_pending_payment_attempt(ctx, &realm_id, user_id, mapping_id).await;

        // Exercise: send checkout.session.completed with payment_status=unpaid
        let event_id = generate_test_event_id();
        let payload = build_stripe_checkout_completed_unpaid(
            &event_id,
            &realm_id,
            user_id,
            client_app_id,
            attempt_id,
            entitlement_key,
        );
        let response =
            send_stripe_webhook_with_signature(&app, &realm_id, payload, webhook_secret).await;
        assert_eq!(response.status(), StatusCode::OK, "Expected 200 OK");

        // Verify: attempt stays Pending (invalid = conservative)
        let attempt_status = get_payment_attempt_status(ctx, attempt_id)
            .await
            .expect("Payment attempt should exist");
        assert_eq!(
            attempt_status, "Pending",
            "Attempt should remain Pending with invalid strategy value"
        );

        // Verify: no points granted
        let topup = get_topup_balance(ctx, user_id, &realm_id).await;
        assert_eq!(
            topup, 0,
            "No points should be granted with invalid strategy value"
        );
    }

    // =========================================================================
    // Test 4: Eager + one-time + unpaid -> topup_balance granted
    // =========================================================================

    /// User Story: US-AP-002
    /// Covers: Design 5.1 "eager + one-time + unpaid -> fulfill with topup_balance"
    ///
    /// Given: A realm with eager strategy, a one-time mapping with 500 points,
    ///        a user + wallet, and a pending payment attempt
    /// When: checkout.session.completed arrives with payment_status=unpaid
    /// Then: Attempt status = Succeeded, topup_balance = 500
    /// Why: Validates the core eager fulfillment path for one-time purchases
    ///      when payment has not yet settled.
    #[test_context(StrategyTestContext)]
    #[tokio::test]
    async fn test_checkout_completed_eager_unpaid_fulfills_one_time(ctx: &mut StrategyTestContext) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_eager_onetime";
        let realm_id = ctx._realm_id.clone();
        let client_app_id = Uuid::parse_str(&ctx._client_app_id).unwrap();
        let entitlement_key = "eager-onetime";

        // Setup
        setup_stripe_config(ctx, &realm_id, "sk_test_eager_ot", webhook_secret).await;
        set_async_points_strategy(ctx, &realm_id, "eager").await;
        let mapping_id = create_one_time_mapping(ctx, &realm_id, entitlement_key, 500).await;
        let user_id = create_test_user(ctx, &realm_id, "eager-onetime@test.com").await;
        create_points_wallet(ctx, user_id, &realm_id).await;
        let attempt_id = create_pending_payment_attempt(ctx, &realm_id, user_id, mapping_id).await;

        // Exercise
        let event_id = generate_test_event_id();
        let payload = build_stripe_checkout_completed_unpaid(
            &event_id,
            &realm_id,
            user_id,
            client_app_id,
            attempt_id,
            entitlement_key,
        );
        let response =
            send_stripe_webhook_with_signature(&app, &realm_id, payload, webhook_secret).await;
        assert_eq!(response.status(), StatusCode::OK, "Expected 200 OK");

        // Verify: attempt Succeeded
        let attempt_status = get_payment_attempt_status(ctx, attempt_id)
            .await
            .expect("Payment attempt should exist");
        assert_eq!(
            attempt_status, "Succeeded",
            "One-time attempt should be Succeeded under eager strategy"
        );

        // Verify: topup_balance = 500
        let topup = get_topup_balance(ctx, user_id, &realm_id).await;
        assert_eq!(
            topup, 500,
            "topup_balance should be 500 for one-time eager fulfillment"
        );
    }

    // =========================================================================
    // Test 5: Eager + paid -> normal fulfillment (regression)
    // =========================================================================

    /// User Story: US-AP-002
    /// Covers: Design 5.1 "eager does not interfere with paid path"
    ///
    /// Given: A realm with eager strategy, a one-time mapping with 500 points,
    ///        a user + wallet, and a pending payment attempt
    /// When: checkout.session.completed arrives with payment_status=paid
    /// Then: Attempt status = Succeeded, topup_balance = 500
    /// Why: Regression guard: eager strategy must not interfere with or
    ///      double-grant the normal paid fulfillment path.
    #[test_context(StrategyTestContext)]
    #[tokio::test]
    async fn test_checkout_completed_eager_paid_fulfills_normally(ctx: &mut StrategyTestContext) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_eager_paid";
        let realm_id = ctx._realm_id.clone();
        let client_app_id = Uuid::parse_str(&ctx._client_app_id).unwrap();
        let entitlement_key = "eager-paid";

        // Setup
        setup_stripe_config(ctx, &realm_id, "sk_test_eager_paid", webhook_secret).await;
        set_async_points_strategy(ctx, &realm_id, "eager").await;
        let mapping_id = create_one_time_mapping(ctx, &realm_id, entitlement_key, 500).await;
        let user_id = create_test_user(ctx, &realm_id, "eager-paid@test.com").await;
        create_points_wallet(ctx, user_id, &realm_id).await;
        let attempt_id = create_pending_payment_attempt(ctx, &realm_id, user_id, mapping_id).await;

        // Exercise: send checkout.session.completed with payment_status=paid
        let event_id = generate_test_event_id();
        let payload = build_stripe_checkout_completed_paid(
            &event_id,
            &realm_id,
            user_id,
            client_app_id,
            attempt_id,
            entitlement_key,
        );
        let response =
            send_stripe_webhook_with_signature(&app, &realm_id, payload, webhook_secret).await;
        assert_eq!(response.status(), StatusCode::OK, "Expected 200 OK");

        // Verify: attempt Succeeded
        let attempt_status = get_payment_attempt_status(ctx, attempt_id)
            .await
            .expect("Payment attempt should exist");
        assert_eq!(
            attempt_status, "Succeeded",
            "Paid attempt should be Succeeded regardless of strategy"
        );

        // Verify: topup_balance = 500 (exactly once, not doubled)
        let topup = get_topup_balance(ctx, user_id, &realm_id).await;
        assert_eq!(
            topup, 500,
            "topup_balance should be exactly 500 (not doubled) for paid eager checkout"
        );
    }

    // =========================================================================
    // Test 6: Eager + subscription + unpaid -> subscription_balance granted
    // =========================================================================

    /// User Story: US-AP-002
    /// Covers: Design 5.1 "eager + subscription + unpaid -> fulfill with subscription_balance"
    ///
    /// Given: A realm with eager strategy, a recurring mapping with 500 points/month,
    ///        a user + wallet, and a pending payment attempt
    /// When: checkout.session.completed arrives with payment_status=unpaid + mode=subscription
    /// Then: Attempt status = Succeeded, subscription_balance = 500
    /// Why: Validates that eager strategy also applies to subscription purchases,
    ///      granting subscription_balance immediately on unpaid async payment.
    #[test_context(StrategyTestContext)]
    #[tokio::test]
    async fn test_checkout_completed_eager_unpaid_fulfills_subscription(
        ctx: &mut StrategyTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_eager_sub";
        let realm_id = ctx._realm_id.clone();
        let client_app_id = Uuid::parse_str(&ctx._client_app_id).unwrap();
        let entitlement_key = "eager-sub";

        // Setup
        setup_stripe_config(ctx, &realm_id, "sk_test_eager_sub", webhook_secret).await;
        set_async_points_strategy(ctx, &realm_id, "eager").await;
        let mapping_id = create_recurring_mapping(ctx, &realm_id, entitlement_key, 500).await;
        let user_id = create_test_user(ctx, &realm_id, "eager-sub@test.com").await;
        create_points_wallet(ctx, user_id, &realm_id).await;
        let attempt_id = create_pending_payment_attempt(ctx, &realm_id, user_id, mapping_id).await;

        // Exercise: send checkout.session.completed with payment_status=unpaid + mode=subscription
        let event_id = generate_test_event_id();
        let stripe_sub_id = format!("sub_test_{}", Uuid::now_v7());
        let payload = build_stripe_checkout_completed_unpaid_subscription(
            &event_id,
            &realm_id,
            user_id,
            client_app_id,
            attempt_id,
            entitlement_key,
            &stripe_sub_id,
        );
        let response =
            send_stripe_webhook_with_signature(&app, &realm_id, payload, webhook_secret).await;
        assert_eq!(response.status(), StatusCode::OK, "Expected 200 OK");

        // Verify: attempt Succeeded
        let attempt_status = get_payment_attempt_status(ctx, attempt_id)
            .await
            .expect("Payment attempt should exist");
        assert_eq!(
            attempt_status, "Succeeded",
            "Subscription attempt should be Succeeded under eager strategy"
        );

        // Verify: subscription_balance = 500
        let sub_balance = get_subscription_balance(ctx, user_id, &realm_id).await;
        assert_eq!(
            sub_balance, 500,
            "subscription_balance should be 500 for subscription eager fulfillment"
        );
    }
}
