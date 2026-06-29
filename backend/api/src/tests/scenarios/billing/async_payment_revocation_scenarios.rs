// =============================================================================
// Async Payment Revocation Scenario Tests
// =============================================================================
//
// Tests for async_payment_succeeded idempotency and async_payment_failed
// revocation logic under the async payment points toggle feature:
// - US-AP-002: Idempotent async_payment_succeeded (skip when already Succeeded)
// - US-AP-003: Points revocation on async_payment_failed
//   - One-time: revoke TopupCredit
//   - Subscription: cancel + revoke SubscriptionCredit
// - US-AP-004: Debt recording when insufficient balance
//
// User Story: US-AP-002, US-AP-003, US-AP-004
// Covers: Design sections 4.1, 4.3, 5.1
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

    use SchemaTestContext as RevocationTestContext;

    // =========================================================================
    // Revocation-Specific Helpers
    // =========================================================================

    /// Create a payment attempt that is already Succeeded (simulating eager fulfillment).
    /// Sets status to Succeeded directly via SQL, bypassing the production status machine.
    /// Also creates a TopupCredit ledger entry and updates wallet balance.
    /// Returns the attempt ID.
    async fn create_succeeded_payment_attempt_with_topup(
        ctx: &RevocationTestContext,
        realm_id: &str,
        user_id: Uuid,
        mapping_id: Uuid,
        points: i64,
    ) -> Uuid {
        // Create the attempt in Pending first
        let attempt_id = create_pending_payment_attempt(ctx, realm_id, user_id, mapping_id).await;

        // Bypass status machine: set directly to Succeeded
        sqlx::query(
            "UPDATE payment_attempts SET status = 'Succeeded', completed_at = NOW(), updated_at = NOW() WHERE id = $1",
        )
        .bind(attempt_id)
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to set payment attempt to Succeeded");

        // Create TopupCredit ledger entry
        let bucket_id = crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm(
            &ctx.app_state.pool,
            realm_id,
        )
        .await;
        sqlx::query(
            "INSERT INTO points_credit_ledger
                (id, user_id, realm_id, bucket_id, credit_type, source_type, source_id,
                 granted_amount, used_amount, revoked_amount, status, created_at, updated_at)
             VALUES ($1, $2, $3, $4, 'topup_credit', 'topup', $5,
                     $6, 0, 0, 'active', NOW(), NOW())",
        )
        .bind(Uuid::now_v7())
        .bind(user_id)
        .bind(realm_id)
        .bind(bucket_id)
        .bind(attempt_id.to_string())
        .bind(points)
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to create topup credit ledger entry");

        // Update wallet lifetime analytics (derived balance comes from ledger)
        sqlx::query(
            "UPDATE points_wallets
             SET total_topup_granted = total_topup_granted + $1,
                 updated_at = NOW()
             WHERE user_id = $2 AND realm_id = $3",
        )
        .bind(points)
        .bind(user_id)
        .bind(realm_id)
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to update wallet topup balance");

        attempt_id
    }

    /// Simulate point consumption by updating wallet and ledger.
    /// Reduces topup_balance and marks the consumed amount as used in the ledger.
    async fn consume_topup_points(
        ctx: &RevocationTestContext,
        user_id: Uuid,
        realm_id: &str,
        attempt_id: Uuid,
        consume_amount: i64,
    ) {
        // Update wallet lifetime analytics (derived balance comes from ledger)
        sqlx::query(
            "UPDATE points_wallets SET total_consumed = total_consumed + $1, updated_at = NOW()
             WHERE user_id = $2 AND realm_id = $3",
        )
        .bind(consume_amount)
        .bind(user_id)
        .bind(realm_id)
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to consume topup points from wallet");

        // Update ledger used_amount to reflect consumption
        sqlx::query(
            "UPDATE points_credit_ledger SET used_amount = used_amount + $1, updated_at = NOW()
             WHERE realm_id = $2 AND user_id = $3 AND source_id = $4 AND credit_type = 'topup_credit'",
        )
        .bind(consume_amount)
        .bind(realm_id)
        .bind(user_id)
        .bind(attempt_id.to_string())
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to update credit ledger for consumption");
    }

    /// Create a quota-entitlement-backed subscription credit entry.
    async fn create_subscription_credit_with_ledger(
        ctx: &RevocationTestContext,
        user_id: Uuid,
        realm_id: &str,
        amount: i64,
        source_id: &str,
    ) {
        let bucket_id = crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm(
            &ctx.app_state.pool,
            realm_id,
        )
        .await;
        let entitlement_source_id = source_id
            .rsplit(':')
            .next()
            .filter(|part| Uuid::parse_str(part).is_ok())
            .unwrap_or(source_id);
        let quota_windows = crate::tests::helpers::points_helpers::quota_windows_jsonb(&[(
            2_592_000, amount, "period",
        )]);

        sqlx::query(
            "INSERT INTO points_quota_entitlements
                (id, user_id, realm_id, bucket_id, credit_type, source_type, source_id,
                 quota_windows, effective_from, effective_until, status, idempotency_key,
                 created_at, updated_at)
             VALUES ($1, $2, $3, $4, 'subscription_credit', 'subscription_initial', $5,
                     $6, NOW(), NOW() + INTERVAL '30 days', 'active', $7, NOW(), NOW())",
        )
        .bind(Uuid::now_v7())
        .bind(user_id)
        .bind(realm_id)
        .bind(bucket_id)
        .bind(entitlement_source_id)
        .bind(quota_windows)
        .bind(format!("test-sub-entitlement:{}", entitlement_source_id))
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to create subscription quota entitlement");

        sqlx::query(
            "UPDATE points_wallets
             SET total_subscription_granted = total_subscription_granted + $1,
                 updated_at = NOW()
             WHERE user_id = $2 AND realm_id = $3",
        )
        .bind(amount)
        .bind(user_id)
        .bind(realm_id)
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to update wallet balance");
    }

    /// Pre-create a subscription via SQL with the given fields.
    /// Returns the subscription ID (internal UUID).
    async fn pre_create_subscription(
        ctx: &RevocationTestContext,
        realm_id: &str,
        user_id: Uuid,
        client_app_id: Uuid,
        external_subscription_id: &str,
        external_product_id: &str,
        payment_provider: &str,
        status: &str,
        entitlement_key: &str,
    ) -> Uuid {
        let subscription_id = Uuid::now_v7();

        // subscription.bucket_id is NOT NULL (eager binding); bind the realm's
        // legacy test bucket so the pre-created row satisfies the constraint.
        let bucket_id = crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm(
            &ctx.app_state.pool,
            realm_id,
        )
        .await;

        sqlx::query(
            "INSERT INTO subscription
                (id, realm_id, user_id, external_subscription_id, external_product_id,
                 payment_provider, status, entitlement_key, external_price_id,
                 provider_metadata, synced_at, current_period_start, current_period_end,
                 cancel_at_period_end, client_app_id, cancel_at, created_at, updated_at,
                 bucket_id)
             VALUES ($1, $2, $3, $4, $5,
                     $6, $7, $8, NULL,
                     NULL, NOW(), NOW(), NOW() + INTERVAL '30 days',
                     false, $9, NULL, NOW(), NOW(), $10)",
        )
        .bind(subscription_id)
        .bind(realm_id)
        .bind(user_id)
        .bind(external_subscription_id)
        .bind(external_product_id)
        .bind(payment_provider)
        .bind(status)
        .bind(entitlement_key)
        .bind(client_app_id)
        .bind(bucket_id)
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to pre-create subscription");
        subscription_id
    }

    /// Get subscription status by internal ID.
    async fn get_subscription_status(ctx: &RevocationTestContext, subscription_id: Uuid) -> String {
        sqlx::query_scalar::<_, String>("SELECT status FROM subscription WHERE id = $1")
            .bind(subscription_id)
            .fetch_one(&ctx.app_state.pool)
            .await
            .expect("Subscription not found")
    }

    /// Count credit ledger entries for a given user, realm, and source.
    async fn count_credit_ledger_entries(
        ctx: &RevocationTestContext,
        user_id: Uuid,
        realm_id: &str,
        source_id: &str,
        credit_type: &str,
    ) -> i64 {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM points_credit_ledger
             WHERE user_id = $1 AND realm_id = $2 AND source_id = $3 AND credit_type = $4",
        )
        .bind(user_id)
        .bind(realm_id)
        .bind(source_id)
        .bind(credit_type)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap_or(0)
    }

    /// Check if a debt record exists in points_revocation_records with a debt: prefix
    /// and matching reference_id.
    async fn find_debt_record(
        ctx: &RevocationTestContext,
        user_id: Uuid,
        realm_id: &str,
        reference_id: &str,
    ) -> Option<(i64, String)> {
        sqlx::query_as::<_, (i64, String)>(
            "SELECT revoked_amount, reason FROM points_revocation_records
             WHERE user_id = $1 AND realm_id = $2 AND reference_id = $3 AND reason LIKE 'debt:%'",
        )
        .bind(user_id)
        .bind(realm_id)
        .bind(reference_id)
        .fetch_optional(&ctx.app_state.pool)
        .await
        .unwrap_or(None)
    }

    // =========================================================================
    // Payload Builders
    // =========================================================================

    fn build_stripe_async_payment_succeeded(
        event_id: &str,
        realm_id: &str,
        attempt_id: Uuid,
    ) -> serde_json::Value {
        json!({
            "id": event_id,
            "object": "event",
            "type": "checkout.session.async_payment_succeeded",
            "api_version": "2020-08-27",
            "created": chrono::Utc::now().timestamp(),
            "data": {
                "object": {
                    "id": format!("cs_test_{}", Uuid::now_v7()),
                    "object": "checkout.session",
                    "mode": "payment",
                    "payment_status": "paid",
                    "metadata": {
                        "attemptId": attempt_id.to_string(),
                        "herald_realm_id": realm_id,
                    },
                    "created": chrono::Utc::now().timestamp(),
                }
            }
        })
    }

    fn build_stripe_async_payment_failed(
        event_id: &str,
        realm_id: &str,
        attempt_id: Uuid,
        mode: &str,
    ) -> serde_json::Value {
        json!({
            "id": event_id,
            "object": "event",
            "type": "checkout.session.async_payment_failed",
            "api_version": "2020-08-27",
            "created": chrono::Utc::now().timestamp(),
            "data": {
                "object": {
                    "id": format!("cs_test_{}", Uuid::now_v7()),
                    "object": "checkout.session",
                    "mode": mode,
                    "payment_status": "unpaid",
                    "metadata": {
                        "attemptId": attempt_id.to_string(),
                        "herald_realm_id": realm_id,
                    },
                    "created": chrono::Utc::now().timestamp(),
                }
            }
        })
    }

    // =========================================================================
    // Test 1: async_succeeded skips when attempt already Succeeded (US-AP-002)
    // =========================================================================

    /// User Story: US-AP-002
    /// Covers: Design 5.1 "Idempotent async_payment_succeeded skips when already Succeeded"
    ///
    /// Given: A realm with eager strategy, a one-time mapping (500 points), a user + wallet,
    ///        and a payment attempt already Succeeded from eager fulfillment (topup_balance = 500)
    /// When: checkout.session.async_payment_succeeded arrives
    /// Then: No double fulfillment — balance stays at 500, no duplicate credit ledger entry.
    /// Why: The core idempotency guarantee: eager strategy fulfills immediately on checkout.completed,
    ///      and when async_payment_succeeded arrives later, it must be a no-op.
    #[test_context(RevocationTestContext)]
    #[tokio::test]
    async fn test_async_succeeded_skips_when_attempt_already_succeeded(
        ctx: &mut RevocationTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_rev_idemp_skip";
        let realm_id = ctx._realm_id.clone();
        let entitlement_key = "rev-idemp-skip";

        // Setup: Stripe config + eager strategy
        setup_stripe_config(ctx, &realm_id, "sk_test_rev_skip", webhook_secret).await;
        set_async_points_strategy(ctx, &realm_id, "eager").await;
        let mapping_id = create_one_time_mapping(ctx, &realm_id, entitlement_key, 500).await;
        let user_id = create_test_user(ctx, &realm_id, "rev-idemp-skip@test.com").await;
        create_points_wallet(ctx, user_id, &realm_id).await;

        // Simulate eager fulfillment: create a Succeeded attempt with 500 topup points
        let attempt_id =
            create_succeeded_payment_attempt_with_topup(ctx, &realm_id, user_id, mapping_id, 500)
                .await;

        // Verify pre-condition: topup_balance = 500, attempt is Succeeded
        let topup_before = get_topup_balance(ctx, user_id, &realm_id).await;
        assert_eq!(topup_before, 500, "Pre-condition: topup should be 500");
        let status_before = get_payment_attempt_status(ctx, attempt_id)
            .await
            .expect("Attempt should exist");
        assert_eq!(
            status_before, "Succeeded",
            "Pre-condition: attempt should be Succeeded"
        );
        let ledger_count_before = count_credit_ledger_entries(
            ctx,
            user_id,
            &realm_id,
            &attempt_id.to_string(),
            "topup_credit",
        )
        .await;

        // Exercise: send async_payment_succeeded for the already-Succeeded attempt
        let event_id = generate_test_event_id();
        let payload = build_stripe_async_payment_succeeded(&event_id, &realm_id, attempt_id);
        let response =
            send_stripe_webhook_with_signature(&app, &realm_id, payload, webhook_secret).await;
        assert_eq!(response.status(), StatusCode::OK, "Expected 200 OK");

        // Verify: balance unchanged (no double grant)
        let topup_after = get_topup_balance(ctx, user_id, &realm_id).await;
        assert_eq!(
            topup_after, 500,
            "Balance should stay at 500 — no double grant"
        );

        // Verify: no new ledger entry created
        let ledger_count_after = count_credit_ledger_entries(
            ctx,
            user_id,
            &realm_id,
            &attempt_id.to_string(),
            "topup_credit",
        )
        .await;
        assert_eq!(
            ledger_count_after, ledger_count_before,
            "No new credit ledger entries should be created"
        );
    }

    // =========================================================================
    // Test 2: async_succeeded fulfills when attempt Pending (regression)
    // =========================================================================

    /// User Story: US-AP-002
    /// Covers: Design 5.1 "Conservative strategy: async_payment_succeeded fulfills Pending attempt"
    ///
    /// Given: A realm with conservative strategy (no eager config), a one-time mapping (500 points),
    ///        a user + wallet, and a Pending payment attempt
    /// When: checkout.session.async_payment_succeeded arrives
    /// Then: Normal fulfillment — attempt becomes Succeeded, topup_balance = 500.
    /// Why: Regression guard: without eager strategy, async_payment_succeeded must still
    ///      perform normal fulfillment. Ensures the idempotency skip logic does not
    ///      accidentally block the conservative path.
    #[test_context(RevocationTestContext)]
    #[tokio::test]
    async fn test_async_succeeded_fulfills_when_attempt_pending(ctx: &mut RevocationTestContext) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_rev_pending";
        let realm_id = ctx._realm_id.clone();
        let entitlement_key = "rev-pending";

        // Setup: Stripe config only (conservative = no async_points_strategy set)
        setup_stripe_config(ctx, &realm_id, "sk_test_rev_pending", webhook_secret).await;
        // Do NOT set eager strategy — default conservative
        let mapping_id = create_one_time_mapping(ctx, &realm_id, entitlement_key, 500).await;
        let user_id = create_test_user(ctx, &realm_id, "rev-pending@test.com").await;
        create_points_wallet(ctx, user_id, &realm_id).await;
        let attempt_id = create_pending_payment_attempt(ctx, &realm_id, user_id, mapping_id).await;

        // Verify pre-condition: attempt is Pending, balance is 0
        let status_before = get_payment_attempt_status(ctx, attempt_id)
            .await
            .expect("Attempt should exist");
        assert_eq!(
            status_before, "Pending",
            "Pre-condition: attempt should be Pending"
        );
        let topup_before = get_topup_balance(ctx, user_id, &realm_id).await;
        assert_eq!(topup_before, 0, "Pre-condition: balance should be 0");

        // Exercise: send async_payment_succeeded for the Pending attempt
        let event_id = generate_test_event_id();
        let payload = build_stripe_async_payment_succeeded(&event_id, &realm_id, attempt_id);
        let response =
            send_stripe_webhook_with_signature(&app, &realm_id, payload, webhook_secret).await;
        assert_eq!(response.status(), StatusCode::OK, "Expected 200 OK");

        // Verify: attempt becomes Succeeded
        let status_after = get_payment_attempt_status(ctx, attempt_id)
            .await
            .expect("Attempt should exist");
        assert_eq!(
            status_after, "Succeeded",
            "Attempt should be Succeeded after async payment succeeded"
        );

        // Verify: topup_balance = 500 (normal fulfillment)
        let topup_after = get_topup_balance(ctx, user_id, &realm_id).await;
        assert_eq!(
            topup_after, 500,
            "topup_balance should be 500 after normal fulfillment"
        );
    }

    // =========================================================================
    // Test 3: async_failed marks Pending attempt Failed, no revocation (regression)
    // =========================================================================

    /// User Story: US-AP-003
    /// Covers: Design 5.1 "Conservative: async_payment_failed marks Pending attempt Failed, no revocation"
    ///
    /// Given: A realm with conservative strategy, a one-time mapping, a user + wallet,
    ///        and a Pending payment attempt
    /// When: checkout.session.async_payment_failed arrives
    /// Then: Attempt becomes Failed, no points are revoked, balance stays at 0.
    /// Why: Regression guard: under conservative strategy, no points were ever granted,
    ///      so async_payment_failed must NOT trigger revocation. Only the attempt status changes.
    #[test_context(RevocationTestContext)]
    #[tokio::test]
    async fn test_async_failed_marks_pending_attempt_failed_no_revocation(
        ctx: &mut RevocationTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_rev_no_revoke";
        let realm_id = ctx._realm_id.clone();
        let entitlement_key = "rev-no-revoke";

        // Setup: Stripe config only (conservative = no async_points_strategy)
        setup_stripe_config(ctx, &realm_id, "sk_test_rev_no_revoke", webhook_secret).await;
        let mapping_id = create_one_time_mapping(ctx, &realm_id, entitlement_key, 500).await;
        let user_id = create_test_user(ctx, &realm_id, "rev-no-revoke@test.com").await;
        create_points_wallet(ctx, user_id, &realm_id).await;
        let attempt_id = create_pending_payment_attempt(ctx, &realm_id, user_id, mapping_id).await;

        // Verify pre-condition
        let status_before = get_payment_attempt_status(ctx, attempt_id)
            .await
            .expect("Attempt should exist");
        assert_eq!(
            status_before, "Pending",
            "Pre-condition: attempt should be Pending"
        );

        // Exercise: send async_payment_failed for the Pending attempt
        let event_id = generate_test_event_id();
        let payload =
            build_stripe_async_payment_failed(&event_id, &realm_id, attempt_id, "payment");
        let response =
            send_stripe_webhook_with_signature(&app, &realm_id, payload, webhook_secret).await;
        assert_eq!(response.status(), StatusCode::OK, "Expected 200 OK");

        // Verify: attempt becomes Failed
        let status_after = get_payment_attempt_status(ctx, attempt_id)
            .await
            .expect("Attempt should exist");
        assert_eq!(
            status_after, "Failed",
            "Attempt should be Failed after async payment failed"
        );

        // Verify: no revocation — balance stays at 0
        let topup_after = get_topup_balance(ctx, user_id, &realm_id).await;
        assert_eq!(
            topup_after, 0,
            "No points should be revoked — balance stays at 0"
        );
    }

    // =========================================================================
    // Test 4: async_failed revokes TopupCredit for Succeeded one-time (US-AP-003)
    // =========================================================================

    /// User Story: US-AP-003
    /// Covers: Design 5.1 "One-time: revoke TopupCredit when async_payment_failed on Succeeded attempt"
    ///
    /// Given: A realm with eager strategy, a one-time mapping (500 points), a user + wallet,
    ///        and a payment attempt already Succeeded from eager fulfillment (topup_balance = 500)
    /// When: checkout.session.async_payment_failed arrives with mode=payment
    /// Then: Attempt becomes Failed, topup_balance = 0 (points revoked).
    /// Why: The core revocation path for one-time purchases under eager strategy.
    ///      When the async payment ultimately fails, the points granted during eager
    ///      fulfillment must be fully revoked.
    #[test_context(RevocationTestContext)]
    #[tokio::test]
    async fn test_async_failed_revokes_topup_for_succeeded_one_time(
        ctx: &mut RevocationTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_rev_topup";
        let realm_id = ctx._realm_id.clone();
        let entitlement_key = "rev-topup";

        // Setup: Stripe config + eager strategy
        setup_stripe_config(ctx, &realm_id, "sk_test_rev_topup", webhook_secret).await;
        set_async_points_strategy(ctx, &realm_id, "eager").await;
        let mapping_id = create_one_time_mapping(ctx, &realm_id, entitlement_key, 500).await;
        let user_id = create_test_user(ctx, &realm_id, "rev-topup@test.com").await;
        create_points_wallet(ctx, user_id, &realm_id).await;

        // Simulate eager fulfillment: Succeeded attempt with 500 topup points
        let attempt_id =
            create_succeeded_payment_attempt_with_topup(ctx, &realm_id, user_id, mapping_id, 500)
                .await;

        // Verify pre-condition: topup_balance = 500
        let topup_before = get_topup_balance(ctx, user_id, &realm_id).await;
        assert_eq!(topup_before, 500, "Pre-condition: topup should be 500");

        // Exercise: send async_payment_failed with mode=payment
        let event_id = generate_test_event_id();
        let payload =
            build_stripe_async_payment_failed(&event_id, &realm_id, attempt_id, "payment");
        let response =
            send_stripe_webhook_with_signature(&app, &realm_id, payload, webhook_secret).await;
        assert_eq!(response.status(), StatusCode::OK, "Expected 200 OK");

        // Verify: attempt becomes Failed
        let status_after = get_payment_attempt_status(ctx, attempt_id)
            .await
            .expect("Attempt should exist");
        assert_eq!(
            status_after, "Failed",
            "Attempt should be Failed after async payment failed revocation"
        );

        // Verify: topup_balance = 0 (all points revoked)
        let topup_after = get_topup_balance(ctx, user_id, &realm_id).await;
        assert_eq!(
            topup_after, 0,
            "topup_balance should be 0 — all TopupCredit revoked"
        );

        // Verify: no debt record (full revocation, no shortfall)
        let debt = find_debt_record(ctx, user_id, &realm_id, &attempt_id.to_string()).await;
        assert!(
            debt.is_none(),
            "No debt record should exist when full revocation succeeds"
        );
    }

    // =========================================================================
    // Test 5: async_failed cancels subscription and revokes for Succeeded recurring (US-AP-003)
    // =========================================================================

    /// User Story: US-AP-003
    /// Covers: Design 5.1 "Subscription: cancel + revoke SubscriptionCredit on async_payment_failed"
    ///
    /// Given: A realm with eager strategy, a recurring mapping (500 points), a user + wallet,
    ///        a payment attempt already Succeeded, an active subscription, and subscription_balance = 500
    /// When: checkout.session.async_payment_failed arrives with mode=subscription
    /// Then: Attempt becomes Failed, subscription status = "canceled", subscription_balance = 0.
    /// Why: The subscription revocation path: under eager strategy, when async payment fails
    ///      for a subscription purchase, both the subscription must be canceled and
    ///      subscription credits revoked.
    #[test_context(RevocationTestContext)]
    #[tokio::test]
    async fn test_async_failed_cancels_subscription_and_revokes_for_succeeded_recurring(
        ctx: &mut RevocationTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_rev_sub";
        let realm_id = ctx._realm_id.clone();
        let client_app_id = Uuid::parse_str(&ctx._client_app_id).unwrap();
        let entitlement_key = "rev-sub";

        // Setup: Stripe config + eager strategy
        setup_stripe_config(ctx, &realm_id, "sk_test_rev_sub", webhook_secret).await;
        set_async_points_strategy(ctx, &realm_id, "eager").await;
        let mapping_id = create_recurring_mapping(ctx, &realm_id, entitlement_key, 500).await;
        let user_id = create_test_user(ctx, &realm_id, "rev-sub@test.com").await;
        create_points_wallet(ctx, user_id, &realm_id).await;

        // Pre-create an active subscription
        let ext_sub_id = format!("sub_stripe_{}", Uuid::now_v7());
        let sub_id = pre_create_subscription(
            ctx,
            &realm_id,
            user_id,
            client_app_id,
            &ext_sub_id,
            &format!("prod_stripe_{}", entitlement_key),
            "stripe",
            "active",
            entitlement_key,
        )
        .await;

        // Create the payment attempt in Pending, then set to Succeeded via SQL
        let attempt_id = create_pending_payment_attempt(ctx, &realm_id, user_id, mapping_id).await;
        sqlx::query(
            "UPDATE payment_attempts SET status = 'Succeeded', completed_at = NOW(), updated_at = NOW() WHERE id = $1",
        )
        .bind(attempt_id)
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to set payment attempt to Succeeded");

        // Grant subscription credits via ledger + wallet
        create_subscription_credit_with_ledger(ctx, user_id, &realm_id, 500, &sub_id.to_string())
            .await;

        // Verify pre-conditions
        let sub_status_before = get_subscription_status(ctx, sub_id).await;
        assert_eq!(
            sub_status_before, "active",
            "Pre-condition: subscription should be active"
        );
        let sub_balance_before = get_subscription_balance(ctx, user_id, &realm_id).await;
        assert_eq!(
            sub_balance_before, 500,
            "Pre-condition: subscription_balance should be 500"
        );

        // Exercise: send async_payment_failed with mode=subscription
        let event_id = generate_test_event_id();
        let payload =
            build_stripe_async_payment_failed(&event_id, &realm_id, attempt_id, "subscription");
        let response =
            send_stripe_webhook_with_signature(&app, &realm_id, payload, webhook_secret).await;
        assert_eq!(response.status(), StatusCode::OK, "Expected 200 OK");

        // Verify: attempt becomes Failed
        let attempt_status = get_payment_attempt_status(ctx, attempt_id)
            .await
            .expect("Attempt should exist");
        assert_eq!(
            attempt_status, "Failed",
            "Attempt should be Failed after async payment failed"
        );

        // Verify: subscription status = "canceled"
        let sub_status_after = get_subscription_status(ctx, sub_id).await;
        assert_eq!(
            sub_status_after, "canceled",
            "Subscription should be canceled after async payment failed"
        );

        // Verify: subscription_balance = 0 (credits revoked)
        let sub_balance_after = get_subscription_balance(ctx, user_id, &realm_id).await;
        assert_eq!(
            sub_balance_after, 0,
            "subscription_balance should be 0 — all SubscriptionCredit revoked"
        );
    }

    // =========================================================================
    // Test 6: async_failed records debt when insufficient balance (US-AP-004)
    // =========================================================================

    /// User Story: US-AP-004
    /// Covers: Design 4.3 "Debt recording when total_revoked < original_points"
    ///
    /// Given: A realm with eager strategy, a one-time mapping (500 points), a user + wallet,
    ///        a payment attempt already Succeeded (topup_balance = 500), but the user
    ///        has consumed 300 of 500 points (topup_balance = 200)
    /// When: checkout.session.async_payment_failed arrives with mode=payment
    /// Then: Attempt becomes Failed, topup_balance = 0, a debt record exists in
    ///       points_revocation_records with reason containing "debt:" prefix,
    ///       reference_id = attempt_id, and debt amount = 300 (500 original - 200 recovered).
    /// Why: The partial consumption debt scenario: when a user has spent some of the eagerly
    ///      granted points and the async payment fails, the system can only revoke what remains.
    ///      The unrecoverable portion must be recorded as debt for later reconciliation.
    #[test_context(RevocationTestContext)]
    #[tokio::test]
    async fn test_async_failed_records_debt_when_insufficient_balance(
        ctx: &mut RevocationTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_rev_debt";
        let realm_id = ctx._realm_id.clone();
        let entitlement_key = "rev-debt";

        // Setup: Stripe config + eager strategy
        setup_stripe_config(ctx, &realm_id, "sk_test_rev_debt", webhook_secret).await;
        set_async_points_strategy(ctx, &realm_id, "eager").await;
        let mapping_id = create_one_time_mapping(ctx, &realm_id, entitlement_key, 500).await;
        let user_id = create_test_user(ctx, &realm_id, "rev-debt@test.com").await;
        create_points_wallet(ctx, user_id, &realm_id).await;

        // Simulate eager fulfillment: Succeeded attempt with 500 topup points
        let attempt_id =
            create_succeeded_payment_attempt_with_topup(ctx, &realm_id, user_id, mapping_id, 500)
                .await;

        // Simulate point consumption: user spent 300 of 500 points
        consume_topup_points(ctx, user_id, &realm_id, attempt_id, 300).await;

        // Verify pre-condition: topup_balance = 200 (500 - 300 consumed)
        let topup_before = get_topup_balance(ctx, user_id, &realm_id).await;
        assert_eq!(
            topup_before, 200,
            "Pre-condition: topup should be 200 after consuming 300 of 500"
        );

        // Exercise: send async_payment_failed with mode=payment
        let event_id = generate_test_event_id();
        let payload =
            build_stripe_async_payment_failed(&event_id, &realm_id, attempt_id, "payment");
        let response =
            send_stripe_webhook_with_signature(&app, &realm_id, payload, webhook_secret).await;
        assert_eq!(response.status(), StatusCode::OK, "Expected 200 OK");

        // Verify: attempt becomes Failed
        let status_after = get_payment_attempt_status(ctx, attempt_id)
            .await
            .expect("Attempt should exist");
        assert_eq!(
            status_after, "Failed",
            "Attempt should be Failed after async payment failed"
        );

        // Verify: topup_balance = 0 (revoked to zero)
        let topup_after = get_topup_balance(ctx, user_id, &realm_id).await;
        assert_eq!(
            topup_after, 0,
            "topup_balance should be 0 — revoked remaining 200"
        );

        // Verify: debt record exists with debt: prefix
        let debt = find_debt_record(ctx, user_id, &realm_id, &attempt_id.to_string())
            .await
            .expect("Debt record should exist when insufficient balance for full revocation");

        // Verify: debt amount is 300 (500 original - 200 recovered)
        assert_eq!(
            debt.0, 300,
            "Debt amount should be 300 (500 original - 200 recovered)"
        );

        // Verify: reason contains debt: prefix and key fields
        assert!(
            debt.1.starts_with("debt:"),
            "Debt reason should start with 'debt:' prefix, got: {}",
            debt.1
        );
        assert!(
            debt.1.contains("original=500"),
            "Debt reason should contain original=500, got: {}",
            debt.1
        );
        assert!(
            debt.1.contains("recovered=200"),
            "Debt reason should contain recovered=200, got: {}",
            debt.1
        );
        assert!(
            debt.1.contains("shortfall=300"),
            "Debt reason should contain shortfall=300, got: {}",
            debt.1
        );
        assert!(
            debt.1.contains("async_payment_failed_insufficient_balance"),
            "Debt reason should contain async_payment_failed_insufficient_balance, got: {}",
            debt.1
        );
    }
}
