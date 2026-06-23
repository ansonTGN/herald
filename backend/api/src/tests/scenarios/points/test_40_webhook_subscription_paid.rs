// =============================================================================
// Test: Subscription Paid Webhook
// =============================================================================
//
// Tests for subscription.paid webhook events (initial subscription and renewals).
//
// User Story: docs/user-stories/points-billing-events.md
// Covers: US-PO-06 (Subscription grants and renewals)
//
// =============================================================================

use crate::tests::helpers::points_helpers::*;
use crate::tests::helpers::webhook_helpers::*;
use crate::tests::scenarios::points::fixtures::*;
use crate::tests::schema_test_context::SchemaTestContext;
use herald_core::domain::points::entities::{CreditLedgerStatus, CreditSourceType, CreditType};
use test_context::test_context;
use uuid::Uuid;

// ============================================================================
// Test 1: Initial Subscription Grant
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-PO-06 场景 - Initial subscription grants points
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_subscription_paid_initial_grant(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone(); // Clone to avoid borrow issues
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user1@example.com").await;
    let plan_id = Uuid::now_v7();
    let event_id = generate_test_event_id();

    // Configure Creem webhook for this realm
    ctx.with_creem_config(&realm_id, None, None, None).await;

    // Setup plan config for the test
    setup_test_plan_config(ctx, &realm_id, plan_id).await;

    // Create points account for user
    create_points_wallet(ctx, user_id, &realm_id).await;

    // Build subscription.paid event (is_renewal = false)
    let event = build_subscription_paid_event(
        event_id, user_id, plan_id, false, // initial subscription
        &realm_id,
    );

    // When
    let app = ctx.create_unified_test_router();
    let response = send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;

    // Then
    assert_webhook_success(&response);

    // Verify subscription_credit ledger was created
    let ledgers =
        get_user_ledgers_by_credit_type(ctx, user_id, CreditType::SubscriptionCredit).await;
    assert_eq!(
        ledgers.len(),
        1,
        "Should create one subscription credit ledger"
    );

    let ledger = &ledgers[0];
    assert_eq!(ledger.credit_type, CreditType::SubscriptionCredit);
    assert_eq!(ledger.source_type, CreditSourceType::SubscriptionInitial);
    assert_eq!(ledger.granted_amount, 1000); // Amount from setup_test_plan_config
    assert_eq!(ledger.remaining_amount, 1000);
    assert_eq!(ledger.status, CreditLedgerStatus::Active);
    assert!(
        ledger.expires_at.is_some(),
        "Subscription credits should have expiry"
    );

    // Verify transaction record
    assert_transaction_exists_by_type(
        ctx,
        user_id,
        herald_core::domain::points::entities::TransactionType::SubscriptionGrant,
        1000,
    )
    .await;
}

// ============================================================================
// Test 2: Subscription Renewal
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-PO-06 场景 - Subscription renewal grants points
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_subscription_paid_renewal_grant(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone(); // Clone to avoid borrow issues
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user2@example.com").await;
    let plan_id = Uuid::now_v7();
    let event_id = generate_test_event_id();

    // Configure Creem webhook for this realm
    ctx.with_creem_config(&realm_id, None, None, None).await;

    // Setup plan config for the test
    setup_test_plan_config(ctx, &realm_id, plan_id).await;

    create_points_wallet(ctx, user_id, &realm_id).await;

    // Build subscription.paid event (is_renewal = true)
    let event = build_subscription_paid_event(
        event_id, user_id, plan_id, true, // renewal
        &realm_id,
    );

    // When
    let app = ctx.create_unified_test_router();
    let response = send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;

    // Then
    assert_webhook_success(&response);

    // Verify subscription_credit ledger was created with renewal source type
    let ledgers =
        get_user_ledgers_by_credit_type(ctx, user_id, CreditType::SubscriptionCredit).await;
    assert_eq!(ledgers.len(), 1);

    let ledger = &ledgers[0];
    assert_eq!(ledger.credit_type, CreditType::SubscriptionCredit);
    assert_eq!(ledger.source_type, CreditSourceType::SubscriptionRenewal);
    assert_eq!(ledger.granted_amount, 1000);
    assert_eq!(ledger.remaining_amount, 1000);
    assert_eq!(ledger.status, CreditLedgerStatus::Active);

    // Verify transaction record
    assert_transaction_exists_by_type(
        ctx,
        user_id,
        herald_core::domain::points::entities::TransactionType::SubscriptionRenewal,
        1000,
    )
    .await;
}

// ============================================================================
// Test 3: Subscription Paid Event Idempotency
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-PO-06 场景 - subscription.paid 幂等性，相同 event_id 不重复发放积分
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_subscription_paid_idempotency(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user3@example.com").await;
    let plan_id = Uuid::now_v7();
    let event_id = generate_test_event_id();

    // Configure Creem webhook for this realm
    ctx.with_creem_config(&realm_id, None, None, None).await;

    // Setup plan config for the test
    setup_test_plan_config(ctx, &realm_id, plan_id).await;

    create_points_wallet(ctx, user_id, &realm_id).await;

    // Build subscription.paid event with a shared event_id
    let event = build_subscription_paid_event(
        event_id.clone(),
        user_id,
        plan_id,
        false, // initial subscription
        &realm_id,
    );

    let app = ctx.create_unified_test_router();

    // When: First processing
    let response1 =
        send_webhook_with_signature(&app, &realm_id, event.clone(), "test_webhook_secret").await;
    assert_webhook_success(&response1);

    // When: Second processing (same event_id)
    let response2 =
        send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;
    assert_webhook_success(&response2);

    // Then: Should only create one subscription credit ledger entry
    let ledgers =
        get_user_ledgers_by_credit_type(ctx, user_id, CreditType::SubscriptionCredit).await;
    assert_eq!(
        ledgers.len(),
        1,
        "Should not duplicate credit ledger on retry"
    );

    let ledger = &ledgers[0];
    assert_eq!(
        ledger.granted_amount, 1000,
        "Granted amount should be exactly one plan allocation"
    );
    assert_eq!(ledger.remaining_amount, 1000);
}

// ============================================================================
// point-time BE-T04 — Subscription pre-grant, period-level idempotency,
// chained pre-grant, expires_at correction (design §5.2 / §6.1 P0)
// ============================================================================
//
// These tests exercise the period-aware `handle_subscription_paid` path
// (design §5.2). They invoke the real `subscription_service.handle_subscription_paid`
// directly with a pre-seeded `subscription` + `points_grant_schedules` row
// (`base_time` = first_period_start, `subscription_id` bound) so the
// period-level business idempotency gate (`points_grant_records(schedule_id,
// period_number)` UNIQUE) and the chained next-period pre-grant
// (`pregrant_next_period_atomic`) are exercised. Provider event-level
// idempotency is covered separately in `test_subscription_renewal_event_idempotency`
// via the webhook HTTP path.
//
// Why direct service invocation (not webhook HTTP):
//   - `handle_subscription_paid`'s new period-level behavior is the unit under
//     test; the HTTP/webhook plumbing is already covered by tests 1-3 above.
//   - The `find_grant_schedule_by_subscription(subscription_id)` lookup keys
//     on the subscription id, which is minted inside `sync_creem_subscription`
//     during webhook processing and not knowable ahead of time. Direct service
//     invocation lets the test bind a known `subscription_id` to the seeded
//     schedule, exercising the period path deterministically.

/// Seed a `subscription` row bound to the realm's legacy test bucket and
/// return its id. Used by BE-T04 tests so the schedule's `subscription_id`
/// and the grant target `bucket_id` are known ahead of the service call.
async fn seed_subscription_row(
    ctx: &mut SchemaTestContext,
    user_id: Uuid,
    realm_id: &str,
    entitlement_key: &str,
) -> Uuid {
    use crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm;
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, realm_id).await;
    let subscription_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO subscription
            (id, realm_id, user_id, status, entitlement_key,
             external_subscription_id, external_product_id, payment_provider,
             current_period_start, current_period_end, cancel_at_period_end,
             bucket_id, created_at, updated_at)
         VALUES ($1, $2, $3, 'active', $4, $5, $6, 'creem',
                 NOW(), NOW() + INTERVAL '30 days', false, $7, NOW(), NOW())",
    )
    .bind(subscription_id)
    .bind(realm_id)
    .bind(user_id)
    .bind(entitlement_key)
    .bind(format!("sub_be_t04_{}", subscription_id))
    .bind(format!("prod_be_t04_{}", entitlement_key))
    .bind(bucket_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to seed subscription row for BE-T04");
    subscription_id
}

/// User Story: US-PU-009 (use current-period points without distribution delay).
/// Covers (design §6.1 P0 — 订阅预生成):
///   - Subscription activation grants the CURRENT period (`effective_at =
///     period_start <= now` ⟺ immediately available) AND pre-grants the NEXT
///     period (`effective_at = next_period_start`, future) atomically.
///   - Derived available balance EXCLUDES the next-period row before its
///     `effective_at` arrives (availability predicate), and INCLUDES it after
///     the clock advances past `effective_at` (zero-delay availability: only
///     time-advance, no worker/state transition).
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_subscription_activation_writes_current_and_pregrants_next(
    ctx: &mut SchemaTestContext,
) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(
        &ctx.app_state.pool,
        &realm_id,
        "be_t04_activate@example.com",
    )
    .await;

    let entitlement_key = format!("be-t04-act-{}", Uuid::now_v7());
    let points_per_period: i64 = 1000;

    // Entitlement mapping (grant_on_subscribe=true, monthly). Routed to the
    // realm's legacy bucket — same one `ensure_test_bucket_for_realm` returns,
    // which `seed_subscription_row` also binds.
    crate::tests::helpers::webhook_helpers::setup_test_entitlement_mapping_for_webhook(
        ctx,
        &realm_id,
        "creem",
        &format!("prod_be_t04_{}", entitlement_key),
        &entitlement_key,
        points_per_period,
        true,
        true,
    )
    .await;

    let subscription_id = seed_subscription_row(ctx, user_id, &realm_id, &entitlement_key).await;

    // Schedule anchored to a first_period_start 30 days ago — so the current
    // period (period_number=2) has period_start = -30d + 30d ≈ now and the
    // next period starts ~30 days in the future.
    let now = chrono::Utc::now();
    let first_period_start = now - chrono::Duration::days(30);
    let current_period_start = now - chrono::Duration::seconds(10); // <= now ⟺ immediately available
    let current_period_end = current_period_start + chrono::Duration::days(30);

    let _schedule_id = create_subscription_grant_schedule(
        ctx,
        user_id,
        &realm_id,
        subscription_id,
        &entitlement_key,
        points_per_period,
        current_period_start, // next_grant_time = current period start
        first_period_start,
        0, // granted_periods
    )
    .await;

    // --- When: subscription activation fires handle_subscription_paid -------
    let result = ctx
        .app_state
        .subscription_service
        .handle_subscription_paid(
            user_id,
            subscription_id,
            crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm(
                &ctx.app_state.pool,
                &realm_id,
            )
            .await,
            &realm_id,
            &entitlement_key,
            false, // initial activation
            current_period_start,
            current_period_end,
            format!("evt_be_t04_act_{}", Uuid::now_v7()),
        )
        .await;
    assert!(result.is_ok(), "activation grant failed: {:?}", result);

    // --- Then: ledger has current period (available) + next period (future) --
    let ledgers =
        get_user_ledgers_by_credit_type(ctx, user_id, CreditType::SubscriptionCredit).await;
    assert!(
        ledgers.len() >= 2,
        "activation should write current period AND pre-grant next period, got {} ledgers",
        ledgers.len()
    );

    // Derived available balance must be EXACTLY one period's worth — the
    // future-effective next-period row is excluded by the predicate.
    assert_derived_balance(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        points_per_period,
    )
    .await;

    let future_before = count_future_effective_active_rows(ctx, user_id, &realm_id).await;
    assert_eq!(
        future_before, 1,
        "exactly one future-effective active row (the pre-granted next period)"
    );

    // --- When: clock advances past the next period's effective_at (SQL UPDATE
    // simulates time-advance; NO worker / state transition runs).
    let next_period_row_id: Uuid = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM points_credit_ledger
         WHERE user_id = $1 AND realm_id = $2
           AND credit_type = 'subscription_credit'
           AND effective_at IS NOT NULL AND effective_at > NOW()
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(user_id)
    .bind(&realm_id)
    .fetch_one(&ctx.app_state.pool)
    .await
    .expect("future-effective row must exist before clock-advance");

    // Bring the row's effective_at into the past while honoring the DB CHECK
    // `effective_at <= expires_at` (keep expires_at unchanged).
    sqlx::query(
        "UPDATE points_credit_ledger
         SET effective_at = NOW() - INTERVAL '1 second', updated_at = NOW()
         WHERE id = $1 AND (expires_at IS NULL OR expires_at > NOW() - INTERVAL '1 second')",
    )
    .bind(next_period_row_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("clock-advance UPDATE failed");

    // --- Then: derived balance now includes BOTH periods (zero-delay) -------
    assert_derived_balance(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        points_per_period * 2,
    )
    .await;
    let future_after = count_future_effective_active_rows(ctx, user_id, &realm_id).await;
    assert_eq!(
        future_after, 0,
        "no future-effective rows remain after clock-advance"
    );
}

/// User Story: US-PU-009 (renewal must not double-grant).
/// Covers (design §6.1 P0 — 续费幂等 + §6.3 P1 business idempotency dimension
/// shift): when a pre-grant and a formal renewal webhook converge on the same
/// `(schedule_id, period_number)`, the `points_grant_records` UNIQUE constraint
/// is the primary dedup. The renewal must NOT re-grant; it only CORRECTS the
/// pre-granted ledger's `expires_at` to the provider's actual `period_end`.
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_subscription_renewal_period_idempotency(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(
        &ctx.app_state.pool,
        &realm_id,
        "be_t04_period_idem@example.com",
    )
    .await;

    let entitlement_key = format!("be-t04-pi-{}", Uuid::now_v7());
    let points_per_period: i64 = 500;

    crate::tests::helpers::webhook_helpers::setup_test_entitlement_mapping_for_webhook(
        ctx,
        &realm_id,
        "creem",
        &format!("prod_be_t04_{}", entitlement_key),
        &entitlement_key,
        points_per_period,
        true,
        true,
    )
    .await;

    let subscription_id = seed_subscription_row(ctx, user_id, &realm_id, &entitlement_key).await;
    let bucket_id = crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm(
        &ctx.app_state.pool,
        &realm_id,
    )
    .await;

    // Truncate to microsecond precision: Postgres `TIMESTAMPTZ` stores
    // microseconds, so values derived from `chrono::Utc::now()` (nanosecond)
    // lose sub-microsecond nanos on the DB round-trip. Truncating the seed
    // keeps strict equality assertions (e.g. `expires_at == provider_actual_period_end`)
    // exact without loosening them.
    let now = trunc_to_micros(chrono::Utc::now());
    let first_period_start = now - chrono::Duration::days(60);
    let period_start = now - chrono::Duration::days(30);
    let estimate_expires = period_start + chrono::Duration::days(30);

    let schedule_id = create_subscription_grant_schedule(
        ctx,
        user_id,
        &realm_id,
        subscription_id,
        &entitlement_key,
        points_per_period,
        period_start,
        first_period_start,
        0,
    )
    .await;

    // --- Given: a PRE-GRANT for period_number=2 already exists (estimated
    // expires_at, future effective_at is irrelevant for idempotency; we set it
    // in the past so the pre-grant is "available").
    let pregrant_ledger_id = create_credit_ledger_entry_with_effective_at(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        CreditSourceType::SubscriptionRenewal,
        format!("schedule:{}:period:2", schedule_id),
        points_per_period,
        Some(estimate_expires),
        Some(period_start),
    )
    .await;

    create_grant_record(
        ctx,
        schedule_id,
        2, // period_number
        points_per_period,
        period_start,
        pregrant_ledger_id,
    )
    .await;

    let ledger_count_before =
        get_user_ledgers_by_credit_type(ctx, user_id, CreditType::SubscriptionCredit)
            .await
            .len();

    // --- When: formal renewal webhook fires for the SAME period/schedule -----
    let provider_actual_period_end = estimate_expires + chrono::Duration::days(2);
    let result = ctx
        .app_state
        .subscription_service
        .handle_subscription_paid(
            user_id,
            subscription_id,
            bucket_id,
            &realm_id,
            &entitlement_key,
            true, // renewal
            period_start,
            provider_actual_period_end,
            format!("evt_be_t04_pi_{}", Uuid::now_v7()),
        )
        .await;
    assert!(result.is_ok(), "renewal should succeed: {:?}", result);

    // --- Then: NO duplicate grant for the current period --------------------
    let ledger_count_after =
        get_user_ledgers_by_credit_type(ctx, user_id, CreditType::SubscriptionCredit)
            .await
            .len();
    // The renewal chains a pre-grant for the NEXT period (period_number=3,
    // future-effective). So ledger count grows by exactly 1 (the next-period
    // pre-grant), NOT by a duplicate current-period grant.
    assert_eq!(
        ledger_count_after,
        ledger_count_before + 1,
        "renewal must not duplicate current-period grant; only the chained next-period pre-grant may add a row"
    );

    assert!(
        grant_record_exists(ctx, schedule_id, 2).await,
        "period_number=2 grant_record must still exist"
    );

    // --- And: the pre-granted ledger's expires_at was CORRECTED to the
    // provider's actual period_end (design §5.2).
    let corrected_ledger = get_ledger_by_id(ctx, pregrant_ledger_id).await;
    assert_eq!(
        corrected_ledger.expires_at,
        Some(provider_actual_period_end),
        "pre-grant expires_at must be corrected to provider period_end on renewal hit"
    );

    // --- And: the chained next-period pre-grant (period_number=3) exists ----
    assert!(
        grant_record_exists(ctx, schedule_id, 3).await,
        "chained pre-grant for period_number=3 must be written"
    );

    // Derived balance: current period (available) only; next-period chained
    // pre-grant is future-effective and excluded.
    assert_derived_balance(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        points_per_period,
    )
    .await;
}

/// User Story: US-PU-009 (duplicate provider webhook delivery must not
/// double-grant).
/// Covers (design §6.1 P0 — provider event-level idempotency preserved):
/// when the SAME `event_id` is delivered twice, the webhook layer's
/// idempotency_service (`creem_{event_id}` key) returns the cached result on
/// the second hit and does NOT re-enter `handle_subscription_paid`. This is
/// the defense-in-depth backstop ABOVE the period-level business idempotency.
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_subscription_renewal_event_idempotency(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(
        &ctx.app_state.pool,
        &realm_id,
        "be_t04_event_idem@example.com",
    )
    .await;
    let plan_id = Uuid::now_v7();
    let event_id = generate_test_event_id();

    ctx.with_creem_config(&realm_id, None, None, None).await;
    setup_test_plan_config(ctx, &realm_id, plan_id).await;
    create_points_wallet(ctx, user_id, &realm_id).await;

    // Build a subscription.paid webhook with `currentPeriodStart` so the
    // period-normalization layer resolves a real window (A8 P0). The default
    // `build_subscription_paid_event` helper omits `currentPeriodStart`.
    let now = chrono::Utc::now();
    let period_start_str = now.to_rfc3339();
    let period_end_str = (now + chrono::Duration::days(30)).to_rfc3339();
    let base = build_subscription_paid_event(event_id.clone(), user_id, plan_id, false, &realm_id);
    let mut event = base.clone();
    event["data"]["object"]["currentPeriodStart"] = serde_json::Value::String(period_start_str);
    event["data"]["object"]["currentPeriodEnd"] = serde_json::Value::String(period_end_str);

    let app = ctx.create_unified_test_router();

    // --- When: first webhook delivery ---------------------------------------
    let response1 =
        send_webhook_with_signature(&app, &realm_id, event.clone(), "test_webhook_secret").await;
    assert_webhook_success(&response1);

    // Capture ledger state AFTER the first delivery but BEFORE the duplicate.
    // The first delivery may produce 1 (current period only) or 2 (current +
    // chained next-period pre-grant) ledgers.
    let ledgers_after_first =
        get_user_ledgers_by_credit_type(ctx, user_id, CreditType::SubscriptionCredit)
            .await
            .len();
    assert!(
        ledgers_after_first == 1 || ledgers_after_first == 2,
        "first delivery should produce 1 (current period only) or 2 (current + chained next-period pre-grant) ledgers; got {}",
        ledgers_after_first
    );

    // --- When: second webhook delivery (SAME event_id) ----------------------
    let response2 =
        send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;
    assert_webhook_success(&response2);

    // --- Then: the duplicate delivery must NOT add any additional row -------
    let ledgers_after_second =
        get_user_ledgers_by_credit_type(ctx, user_id, CreditType::SubscriptionCredit)
            .await
            .len();
    assert_eq!(
        ledgers_after_first, ledgers_after_second,
        "duplicate webhook event_id must not create additional ledger rows (event-level idempotency backstop)"
    );

    // --- And: the event-level idempotency key is recorded in the SQL
    // `idempotency_keys` table. The outer Creem webhook handler keys its Redis
    // cache on `creem_{event_id}`, but the SQL row (the durable, dup-delivery
    // backstop asserted here) is written by the inner
    // `handle_subscription_paid_atomic` under the `sub_paid:{event_id}` key
    // (IDEMPOTENCY_KEY_SUBSCRIPTION_PAID prefix, subscription_service.rs:387).
    assert_idempotency_key_exists(ctx, &format!("sub_paid:{}", event_id)).await;
}

/// User Story: US-PU-009 (always one period ahead — no distribution vacuum).
/// Covers (design §5.2 / §6.1 P0 — 链式预生成): after a renewal webhook hits
/// (whether it grants the current period fresh or hits an existing
/// pre-grant), the service CHAINS a pre-grant for `period_number + 1` so
/// there is always one future-period ledger row waiting.
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_subscription_renewal_chains_next_period_pregrant(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id =
        create_test_user(&ctx.app_state.pool, &realm_id, "be_t04_chain@example.com").await;

    let entitlement_key = format!("be-t04-chain-{}", Uuid::now_v7());
    let points_per_period: i64 = 800;

    crate::tests::helpers::webhook_helpers::setup_test_entitlement_mapping_for_webhook(
        ctx,
        &realm_id,
        "creem",
        &format!("prod_be_t04_{}", entitlement_key),
        &entitlement_key,
        points_per_period,
        true,
        true,
    )
    .await;

    let subscription_id = seed_subscription_row(ctx, user_id, &realm_id, &entitlement_key).await;
    let bucket_id = crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm(
        &ctx.app_state.pool,
        &realm_id,
    )
    .await;

    let now = chrono::Utc::now();
    let first_period_start = now - chrono::Duration::days(60);
    // Renewal grants the CURRENT period, which must still be ongoing for the
    // granted credit to be available under the derived predicate
    // (`expires_at > NOW()`). Anchor the current period to have started ~10s
    // ago and end ~30d in the future, so the just-granted current-period row
    // is available AND the chained next-period pre-grant lands at
    // `period_end` (≈ now+30d) which is unambiguously future-effective.
    // (Mirrors the activation test's windowing convention.)
    let current_period_start = now - chrono::Duration::seconds(10);
    let current_period_end = now + chrono::Duration::days(30);

    let schedule_id = create_subscription_grant_schedule(
        ctx,
        user_id,
        &realm_id,
        subscription_id,
        &entitlement_key,
        points_per_period,
        current_period_start,
        first_period_start,
        0,
    )
    .await;

    // --- Given: NO pre-grant for the current period exists ------------------
    assert!(
        !grant_record_exists(ctx, schedule_id, 2).await,
        "precondition: current period grant_record should not exist yet"
    );

    // --- When: renewal webhook fires (no pre-grant → fresh grant path) ------
    let result = ctx
        .app_state
        .subscription_service
        .handle_subscription_paid(
            user_id,
            subscription_id,
            bucket_id,
            &realm_id,
            &entitlement_key,
            true,
            current_period_start,
            current_period_end,
            format!("evt_be_t04_chain_{}", Uuid::now_v7()),
        )
        .await;
    assert!(result.is_ok(), "renewal grant failed: {:?}", result);

    // --- Then: current period grant_record is written -----------------------
    assert!(
        grant_record_exists(ctx, schedule_id, 2).await,
        "current period (period_number=2) grant_record must exist after renewal"
    );

    // --- And: chained pre-grant for period_number=3 is written (future) -----
    assert!(
        grant_record_exists(ctx, schedule_id, 3).await,
        "chained pre-grant for period_number=3 must exist after renewal"
    );

    // --- And: the chained pre-grant ledger is future-effective --------------
    let chained_ledger_id = find_ledger_id_by_schedule_period(ctx, schedule_id, 3)
        .await
        .expect("chained pre-grant ledger must be resolvable via grant_record FK");
    let chained_ledger = get_ledger_by_id(ctx, chained_ledger_id).await;
    assert!(
        chained_ledger
            .effective_at
            .map(|t| t > now)
            .unwrap_or(false),
        "chained next-period pre-grant must be future-effective (effective_at > now); got {:?}",
        chained_ledger.effective_at
    );

    // Derived balance excludes the chained pre-grant.
    assert_derived_balance(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        points_per_period,
    )
    .await;
}
