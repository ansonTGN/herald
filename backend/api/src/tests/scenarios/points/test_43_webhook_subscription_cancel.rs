// =============================================================================
// Test: Subscription Cancel Webhook
// =============================================================================
//
// Tests for subscription.canceled webhook events.
//
// User Story: docs/user-stories/points-billing-events.md
// Covers: US-BI-011 (Subscription cancel behavior)
//
// =============================================================================

use crate::tests::helpers::points_helpers::*;
use crate::tests::helpers::webhook_helpers::*;
use crate::tests::scenarios::points::fixtures::*;
use crate::tests::schema_test_context::SchemaTestContext;
use chrono::{Duration, Utc};
use herald_core::domain::points::entities::{
    CreditLedgerStatus, CreditSourceType, CreditType, RevocationType,
};
use test_context::test_context;
use uuid::Uuid;

// ============================================================================
// Test 1: Default Cancel (Period End) - Retain Points
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-BI-011 场景 1 - 默认取消保留积分到周期结束
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_subscription_cancel_default_retains_points(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user1@example.com").await;
    let plan_id = Uuid::now_v7();
    let event_id = generate_test_event_id();
    let _period_end = Utc::now() + Duration::days(30);

    // Configure Creem webhook for this realm
    ctx.with_creem_config(&realm_id, None, None, None).await;

    // Setup plan config for the test
    setup_test_plan_config(ctx, &realm_id, plan_id).await;

    create_points_wallet(ctx, user_id, &realm_id).await;

    create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        CreditSourceType::SubscriptionInitial,
        plan_id.to_string(),
        10000,
        None, // No expiry initially
    )
    .await;

    // When: Cancel with cancel_at_period_end = true
    let event = build_subscription_canceled_event(
        event_id, user_id, true, // cancel_at_period_end
        &realm_id,
    );

    let app = ctx.create_unified_test_router();
    let response = send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;

    // Then
    assert_webhook_success(&response);

    // Points should remain
    let ledgers =
        get_user_ledgers_by_credit_type(ctx, user_id, CreditType::SubscriptionCredit).await;
    assert_eq!(ledgers.len(), 1);

    let ledger = &ledgers[0];
    assert_eq!(
        ledger.remaining_amount, 10000,
        "Points should remain unchanged"
    );
    assert_eq!(
        ledger.status,
        CreditLedgerStatus::Active,
        "Should remain active"
    );
    assert_eq!(ledger.revoked_amount, 0, "No revocation");

    // Expiry should be set to period end
    assert!(ledger.expires_at.is_some(), "Expiry should be set");

    // No revocation records
    let revocations = get_revocation_records(ctx, user_id).await;
    assert_eq!(revocations.len(), 0, "No revocation for period-end cancel");
}

// ============================================================================
// Test 2: Immediate Cancel - Revoke Unused Points
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-BI-011 场景 2 - 立即取消回收未使用会员积分
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_subscription_cancel_immediate_revokes_unused(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user2@example.com").await;
    let plan_id = Uuid::now_v7();
    let event_id = generate_test_event_id();
    let period_end = Utc::now() + Duration::days(30);

    // Configure Creem webhook for this realm
    ctx.with_creem_config(&realm_id, None, None, None).await;

    // Setup plan config for the test
    setup_test_plan_config(ctx, &realm_id, plan_id).await;

    create_points_wallet(ctx, user_id, &realm_id).await;

    create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        CreditSourceType::SubscriptionInitial,
        plan_id.to_string(),
        10000,
        Some(period_end),
    )
    .await;

    // When: Cancel immediately (cancel_at_period_end = false)
    let event = build_subscription_canceled_event(
        event_id, user_id, false, // immediate cancel
        &realm_id,
    );

    let app = ctx.create_unified_test_router();
    let response = send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;

    // Then
    assert_webhook_success(&response);

    // All unused points should be revoked
    let ledgers =
        get_user_ledgers_by_credit_type(ctx, user_id, CreditType::SubscriptionCredit).await;
    assert_eq!(ledgers.len(), 1);

    let ledger = &ledgers[0];
    assert_eq!(ledger.remaining_amount, 0, "All unused points revoked");
    assert_eq!(ledger.revoked_amount, 10000, "Full amount revoked");
    assert_eq!(
        ledger.status,
        CreditLedgerStatus::Revoked,
        "Should be revoked"
    );

    // Verify revocation record
    let revocations = get_revocation_records(ctx, user_id).await;
    assert_eq!(revocations.len(), 1);
    assert_eq!(revocations[0].revocation_type, RevocationType::CancelRevoke);
    assert_eq!(revocations[0].revoked_amount, 10000);
}

// ============================================================================
// Test 4: Cancel Event Idempotency
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-BI-011 场景 4 - 取消事件幂等性
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_subscription_cancel_idempotency(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user4@example.com").await;
    let plan_id = Uuid::now_v7();
    let event_id = generate_test_event_id();
    let period_end = Utc::now() + Duration::days(30);

    // Configure Creem webhook for this realm
    ctx.with_creem_config(&realm_id, None, None, None).await;

    // Setup plan config for the test
    setup_test_plan_config(ctx, &realm_id, plan_id).await;

    create_points_wallet(ctx, user_id, &realm_id).await;

    create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        CreditSourceType::SubscriptionInitial,
        plan_id.to_string(),
        10000,
        Some(period_end),
    )
    .await;

    let event = build_subscription_canceled_event(
        event_id.clone(),
        user_id,
        false, // immediate cancel
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

    // Then: Should only create one revocation record
    let revocations = get_revocation_records(ctx, user_id).await;
    assert_eq!(
        revocations.len(),
        1,
        "Should not duplicate revocation on retry"
    );
}

// ============================================================================
// BE-T06: Pre-grant reclaim (row-level positioning, design §5.2 A4 / §6.1 P1)
// ============================================================================
//
// User Story: docs/user-stories/points-billing-events.md
// Covers: design point-time §6.1 "预生成失败回收（P1）" + §5.2 A4 (row-level
// reclaim) + A7 (derived balance, no wallet back-adjust).
//
// These tests isolate the row-level pre-grant reclaim path
// (`reclaim_pregrant_for_subscription` → `revoke_pregrant_ledger_row_atomic`)
// triggered by Creem `subscription.canceled` (Stripe-symmetric). They use
// `cancel_at_period_end = true` so the companion `handle_subscription_cancel`
// path only sets expiry and does NOT bulk-revoke subscription credits — that
// isolates the reclaim as the sole cause of the target row becoming revoked.

/// Seed a chained pre-grant state for reclaim tests.
///
/// Builds the exact production shape `reclaim_pregrant_for_subscription`
/// resolves: a subscription row keyed by `external_subscription_id`, a
/// `points_grant_schedules` row bound to that subscription with
/// `granted_periods = 2` (because `pregrant_next_period_atomic` advances
/// `granted_periods = max(old, period_number)` after writing the chained
/// pre-grant, the pre-grant row for period 2 means `granted_periods` is
/// ALREADY 2, not 1), a pre-granted `points_credit_ledger` row (future-effective
/// or already effective per `effective_at`), optionally partially consumed, and
/// the `points_grant_records` bridge row linking schedule+period to the ledger
/// (the A4 reclaim locator).
///
/// The reclaim target is `granted_periods` (= 2), so the period-2 pre-grant row
/// is the row that gets revoked — matching production post-renewal state.
///
/// Returns `(subscription_id, schedule_id, ledger_id, period_number)`.
async fn seed_pregrant_for_reclaim(
    ctx: &mut SchemaTestContext,
    realm_id: &str,
    user_id: Uuid,
    external_subscription_id: &str,
    entitlement_key: &str,
    amount: i64,
    effective_at: Option<chrono::DateTime<chrono::Utc>>,
    used_amount: i64,
) -> (Uuid, Uuid, Uuid, i64) {
    use chrono::Duration;

    let pool = &ctx.app_state.pool;
    let subscription_id = Uuid::now_v7();
    let client_app_id = Uuid::now_v7();
    let bucket_id = ensure_test_bucket_for_realm(pool, realm_id).await;

    // Client app for the subscription FK (UNIQUE constraint on subscription).
    sqlx::query(
        "INSERT INTO client_app (id, realm_id, client_id, name, enabled)
         VALUES ($1, $2, $3, $4, true)",
    )
    .bind(client_app_id)
    .bind(realm_id)
    .bind(format!("client-{}", client_app_id))
    .bind("BE-T06 reclaim seed")
    .execute(pool)
    .await
    .expect("seed_pregrant: client_app insert");

    // Subscription row keyed by external_subscription_id. `sync_creem_subscription`
    // (invoked by the webhook handler) will UPDATE this row in place, preserving
    // its id — which is what `find_grant_schedule_by_subscription` keys on.
    sqlx::query(
        "INSERT INTO subscription
            (id, realm_id, user_id, external_subscription_id, external_product_id,
             payment_provider, status, entitlement_key, current_period_start,
             current_period_end, cancel_at_period_end, client_app_id, created_at,
             updated_at, bucket_id)
         VALUES ($1, $2, $3, $4, 'prod_test_monthly', 'creem', 'active', $5, NOW(),
                 NOW() + INTERVAL '30 days', false, $6, NOW(), NOW(), $7)",
    )
    .bind(subscription_id)
    .bind(realm_id)
    .bind(user_id)
    .bind(external_subscription_id)
    .bind(entitlement_key)
    .bind(client_app_id)
    .bind(bucket_id)
    .execute(pool)
    .await
    .expect("seed_pregrant: subscription insert");

    // Grant schedule bound to the subscription. Production invariant:
    // `pregrant_next_period_atomic` advances `granted_periods` to the
    // pre-granted period number, so when a pre-grant exists at period 2,
    // `granted_periods = 2` (NOT 1). The reclaim target is `granted_periods`
    // itself, which resolves to the period-2 pre-grant row.
    let first_period_start = Utc::now() - Duration::days(15);
    let next_grant_time = effective_at.unwrap_or_else(Utc::now);
    let schedule_id = create_subscription_grant_schedule(
        ctx,
        user_id,
        realm_id,
        subscription_id,
        entitlement_key,
        amount,
        next_grant_time,
        first_period_start,
        2, // granted_periods — pre-grant bumped it to the pre-granted period
    )
    .await;

    // Pre-granted ledger row for period 2. Source id mirrors the external
    // subscription id (production pre-grant writes the subscription reference
    // here). `expires_at` strictly follows `effective_at` to satisfy the
    // CHECK `points_credit_ledger_effective_before_expires`.
    let expires_at = effective_at.unwrap_or_else(Utc::now) + Duration::days(30);
    let ledger_id = create_credit_ledger_entry_with_effective_at(
        ctx,
        user_id,
        realm_id,
        CreditType::SubscriptionCredit,
        CreditSourceType::SubscriptionRenewal,
        external_subscription_id.to_string(),
        amount,
        Some(expires_at),
        effective_at,
    )
    .await;

    if used_amount > 0 {
        // Partially consume the pre-grant row so reclaim hits the
        // `used_amount > 0` branch and records a PointsRevocationRecord.
        consume_points_from_ledger(ctx, ledger_id, used_amount).await;
    }

    // Bridge row: this is what `ReclaimLocator::BySchedulePeriod` resolves to
    // locate the ledger row to revoke (design A4 / BE-D05).
    let period_number: i64 = 2;
    create_grant_record(
        ctx,
        schedule_id,
        period_number,
        amount,
        next_grant_time,
        ledger_id,
    )
    .await;

    (subscription_id, schedule_id, ledger_id, period_number)
}

// ----------------------------------------------------------------------------
// BE-T06 §6.1 P1 scenario (1): unfulfilled pre-grant row revoked + derived
// balance excludes it. WHY: the chained next-period row was pre-granted with a
// future `effective_at`; a cancel webhook must revoke that row row-precisely
// (via (schedule_id, period_number)), and because available balance is a
// derived SUM, revocation auto-excludes it — no wallet back-adjust needed.
// ----------------------------------------------------------------------------

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_pregrant_reclaim_unfulfilled_row_revoked(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(
        &ctx.app_state.pool,
        &realm_id,
        "reclaim_unfulfilled@example.com",
    )
    .await;
    let plan_id = Uuid::now_v7();
    let event_id = generate_test_event_id();
    let external_sub = format!("sub_{}", event_id);

    ctx.with_creem_config(&realm_id, None, None, None).await;
    setup_test_plan_config(ctx, &realm_id, plan_id).await;

    // Pre-grant a FUTURE-effective row (not yet in the available set).
    let effective_at = Utc::now() + Duration::days(5);
    let (_sub_id, schedule_id, ledger_id, period_number) = seed_pregrant_for_reclaim(
        ctx,
        &realm_id,
        user_id,
        &external_sub,
        &plan_id.to_string(),
        10_000,
        Some(effective_at),
        0,
    )
    .await;

    // Sanity: the row locator the webhook's reclaim path will use resolves to
    // our seeded ledger (proves the schedule+period bridge is wired right).
    let located = find_ledger_id_by_schedule_period(ctx, schedule_id, period_number)
        .await
        .expect("pre-grant row must be locatable via (schedule_id, period_number) before webhook");
    assert_eq!(located, ledger_id);

    // Derived balance excludes the future-effective row already (predicate).
    assert_derived_balance(ctx, user_id, &realm_id, CreditType::SubscriptionCredit, 0).await;

    // Fire subscription.canceled with cancel_at_period_end=true so the cancel
    // path only sets expiry (no bulk revoke) — isolating the reclaim effect.
    let event = build_subscription_canceled_event(event_id, user_id, true, &realm_id);
    let app = ctx.create_unified_test_router();
    let response = send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;
    assert_webhook_success(&response);

    // Row-precise revoke of the target pre-grant row.
    let ledger = get_ledger_by_id(ctx, ledger_id).await;
    assert_eq!(ledger.status, CreditLedgerStatus::Revoked);
    assert_eq!(
        ledger.remaining_amount, 0,
        "remaining yanked into revoked_amount"
    );
    assert_eq!(
        ledger.revoked_amount, 10_000,
        "full pre-grant amount revoked"
    );
    assert_eq!(ledger.used_amount, 0, "nothing was consumed");

    // Derived balance still excludes the now-revoked row (A7: derived SUM
    // auto-excludes revoked; no wallet back-adjust was needed).
    assert_derived_balance(ctx, user_id, &realm_id, CreditType::SubscriptionCredit, 0).await;

    // Fully-unused row ⟹ no PointsRevocationRecord (no shortfall).
    let revocations = get_revocation_records(ctx, user_id).await;
    assert!(
        revocations
            .iter()
            .all(|r| { r.reason != "subscription_pre_grant_reclaim" }),
        "fully-unused pre-grant row must not produce a reclaim debt record; got {revocations:?}"
    );
}

// ----------------------------------------------------------------------------
// BE-T06 §6.1 P1 scenario (2): reclaim does NOT touch other active credits.
// WHY: A4 mandates row-precise locator — reclaiming by (schedule_id, period)
// must not cascade to other active subscription/topup credits the user holds.
// ----------------------------------------------------------------------------

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_pregrant_reclaim_does_not_touch_other_active(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(
        &ctx.app_state.pool,
        &realm_id,
        "reclaim_other_active@example.com",
    )
    .await;
    let plan_id = Uuid::now_v7();
    let event_id = generate_test_event_id();
    let external_sub = format!("sub_{}", event_id);

    ctx.with_creem_config(&realm_id, None, None, None).await;
    setup_test_plan_config(ctx, &realm_id, plan_id).await;
    create_points_wallet(ctx, user_id, &realm_id).await;

    // Pre-grant target row (future-effective).
    let effective_at = Utc::now() + Duration::days(5);
    let (_sub_id, _schedule_id, target_ledger_id, _period) = seed_pregrant_for_reclaim(
        ctx,
        &realm_id,
        user_id,
        &external_sub,
        &plan_id.to_string(),
        10_000,
        Some(effective_at),
        0,
    )
    .await;

    // An unrelated active subscription_credit row from a DIFFERENT source
    // (different subscription / no schedule bound). Reclaim must leave it alone.
    let other_ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        CreditSourceType::SubscriptionInitial,
        format!("other_sub_{}", Uuid::now_v7()),
        7_000,
        Some(Utc::now() + Duration::days(30)),
    )
    .await;

    let other_before = get_ledger_by_id(ctx, other_ledger_id).await;
    assert_eq!(other_before.status, CreditLedgerStatus::Active);
    assert_eq!(other_before.remaining_amount, 7_000);

    let event = build_subscription_canceled_event(event_id, user_id, true, &realm_id);
    let app = ctx.create_unified_test_router();
    let response = send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;
    assert_webhook_success(&response);

    // Target row revoked.
    let target_after = get_ledger_by_id(ctx, target_ledger_id).await;
    assert_eq!(target_after.status, CreditLedgerStatus::Revoked);
    assert_eq!(target_after.revoked_amount, 10_000);

    // Other active credit untouched (row-precise locator).
    let other_after = get_ledger_by_id(ctx, other_ledger_id).await;
    assert_eq!(
        other_after.status,
        CreditLedgerStatus::Active,
        "other active credit not revoked"
    );
    assert_eq!(
        other_after.remaining_amount, 7_000,
        "other active credit remaining unchanged"
    );
    assert_eq!(
        other_after.revoked_amount, 0,
        "other active credit revoked_amount unchanged"
    );
    assert_eq!(
        other_after.used_amount, 0,
        "other active credit used_amount unchanged"
    );

    // The other credit still contributes to the derived balance; the revoked
    // target row does not.
    assert_derived_balance(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        7_000,
    )
    .await;
}

// ----------------------------------------------------------------------------
// BE-T06 §6.1 P1 scenario (3): reclaim does NOT back-adjust the wallet Stored
// columns (A4+A7). WHY: available balance is a derived SUM — revoking a row
// auto-excludes it, so no `apply_wallet_delta(WalletDelta::revoke)` is needed.
// Asserting the wallet analytics columns are unchanged proves no back-adjust
// path was silently invoked.
// ----------------------------------------------------------------------------

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_pregrant_reclaim_no_wallet_reverse(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(
        &ctx.app_state.pool,
        &realm_id,
        "reclaim_no_wallet_reverse@example.com",
    )
    .await;
    let plan_id = Uuid::now_v7();
    let event_id = generate_test_event_id();
    let external_sub = format!("sub_{}", event_id);

    ctx.with_creem_config(&realm_id, None, None, None).await;
    setup_test_plan_config(ctx, &realm_id, plan_id).await;
    create_points_wallet(ctx, user_id, &realm_id).await;

    let effective_at = Utc::now() + Duration::days(5);
    seed_pregrant_for_reclaim(
        ctx,
        &realm_id,
        user_id,
        &external_sub,
        &plan_id.to_string(),
        10_000,
        Some(effective_at),
        0,
    )
    .await;

    // Snapshot the wallet's analytics + Stored balance columns BEFORE reclaim.
    let before: (i64, i64, i64, i64) = sqlx::query_as(
        "SELECT total_consumed, total_recharged, total_topup_granted, total_subscription_granted
         FROM points_wallets
         WHERE realm_id = $1 AND user_id = $2",
    )
    .bind(&realm_id)
    .bind(user_id)
    .fetch_one(&ctx.app_state.pool)
    .await
    .expect("wallet snapshot before");

    let event = build_subscription_canceled_event(event_id, user_id, true, &realm_id);
    let app = ctx.create_unified_test_router();
    let response = send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;
    assert_webhook_success(&response);

    let after: (i64, i64, i64, i64) = sqlx::query_as(
        "SELECT total_consumed, total_recharged, total_topup_granted, total_subscription_granted
         FROM points_wallets
         WHERE realm_id = $1 AND user_id = $2",
    )
    .bind(&realm_id)
    .bind(user_id)
    .fetch_one(&ctx.app_state.pool)
    .await
    .expect("wallet snapshot after");

    assert_eq!(
        before, after,
        "reclaim must NOT back-adjust wallet analytics/Stored columns (A4+A7): before={before:?} after={after:?}"
    );

    // And the revoked row is excluded from the derived balance.
    assert_derived_balance(ctx, user_id, &realm_id, CreditType::SubscriptionCredit, 0).await;
}

// ----------------------------------------------------------------------------
// BE-T06 §6.1 P1 scenario (4): partially-consumed pre-grant row records a
// PointsRevocationRecord on reclaim. WHY: when the pre-granted period was
// already partly spent before the cancel, the consumed portion is a debt the
// user owes — the reclaim path writes a `PointsRevocationRecord` with
// `reason = subscription_pre_grant_reclaim` to make that debt auditable. The
// remaining unused portion is yanked into `revoked_amount`.
//
// NOTE (deviation surfaced, Rule 7): the spec text says "shortfall == 已消费
// 部分" (shortfall == consumed portion). The production implementation
// (`revoke_pregrant_ledger_row_atomic`) instead records `revoked_amount =
// row.remaining_amount` (the yanked unused portion) on the
// PointsRevocationRecord, and keeps `used_amount` on the ledger row itself.
// This mirrors the existing `revoke_topup_proportional` semantics where
// `revoked_amount` is always "what was just yanked". The test follows
// production (the more recent + tested authority) and flags the spec wording
// for cleanup.
// ----------------------------------------------------------------------------

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_pregrant_reclaim_partial_consumed_records_debt(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(
        &ctx.app_state.pool,
        &realm_id,
        "reclaim_partial@example.com",
    )
    .await;
    let plan_id = Uuid::now_v7();
    let event_id = generate_test_event_id();
    let external_sub = format!("sub_{}", event_id);

    ctx.with_creem_config(&realm_id, None, None, None).await;
    setup_test_plan_config(ctx, &realm_id, plan_id).await;
    create_points_wallet(ctx, user_id, &realm_id).await;

    // Pre-grant row already effective (effective_at <= now) and partially
    // consumed — this is the `used_amount > 0` reclaim branch.
    let effective_at = Utc::now() - Duration::days(1);
    let (_sub_id, _schedule_id, ledger_id, _period) = seed_pregrant_for_reclaim(
        ctx,
        &realm_id,
        user_id,
        &external_sub,
        &plan_id.to_string(),
        10_000,
        Some(effective_at),
        4_000, // used_amount — partially consumed
    )
    .await;

    // Pre-conditions: 6_000 remaining, all in the available set.
    let pre = get_ledger_by_id(ctx, ledger_id).await;
    assert_eq!(pre.used_amount, 4_000);
    assert_eq!(pre.remaining_amount, 6_000);
    assert_eq!(pre.status, CreditLedgerStatus::Active);
    assert_derived_balance(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        6_000,
    )
    .await;

    let event = build_subscription_canceled_event(event_id, user_id, true, &realm_id);
    let app = ctx.create_unified_test_router();
    let response = send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;
    assert_webhook_success(&response);

    // Row revoked; remaining unused portion yanked into revoked_amount; the
    // already-consumed portion is preserved as used_amount for audit.
    let post = get_ledger_by_id(ctx, ledger_id).await;
    assert_eq!(post.status, CreditLedgerStatus::Revoked);
    assert_eq!(post.remaining_amount, 0, "remaining unused portion yanked");
    assert_eq!(
        post.revoked_amount, 6_000,
        "revoked_amount = remaining unused portion"
    );
    assert_eq!(
        post.used_amount, 4_000,
        "consumed portion preserved on the ledger row"
    );

    // Debt record written with the reclaim reason + reference_id embedding the
    // source_id (the production `revoke_pregrant_ledger_row_atomic` shape).
    let revocations = get_revocation_records(ctx, user_id).await;
    let debt = revocations
        .iter()
        .find(|r| r.reason == "subscription_pre_grant_reclaim")
        .expect("partially-consumed reclaim must write a PointsRevocationRecord with reason=subscription_pre_grant_reclaim");
    assert_eq!(debt.revocation_type, RevocationType::CancelRevoke);
    assert_eq!(
        debt.revoked_amount, 6_000,
        "debt record revoked_amount mirrors the yanked unused portion"
    );
    assert_eq!(
        debt.reference_id,
        Some(format!("subscription_pre_grant_reclaim:{}", external_sub)),
        "reference_id embeds the source_id"
    );
    assert_eq!(debt.ledger_id, ledger_id);

    // Derived balance excludes the revoked row entirely.
    assert_derived_balance(ctx, user_id, &realm_id, CreditType::SubscriptionCredit, 0).await;
}

// ----------------------------------------------------------------------------
// BE-T06 §6.1 P1 scenario (6): event-level idempotency. WHY: webhook
// providers redeliver; a second cancel event with the same event_id must be a
// no-op — the row is already revoked, and no second PointsRevocationRecord or
// additional revoked_amount may be written.
// ----------------------------------------------------------------------------

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_pregrant_reclaim_event_idempotency(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(
        &ctx.app_state.pool,
        &realm_id,
        "reclaim_idempotent@example.com",
    )
    .await;
    let plan_id = Uuid::now_v7();
    let event_id = generate_test_event_id();
    let external_sub = format!("sub_{}", event_id);

    ctx.with_creem_config(&realm_id, None, None, None).await;
    setup_test_plan_config(ctx, &realm_id, plan_id).await;
    create_points_wallet(ctx, user_id, &realm_id).await;

    let effective_at = Utc::now() - Duration::days(1);
    let (_sub_id, _schedule_id, ledger_id, _period) = seed_pregrant_for_reclaim(
        ctx,
        &realm_id,
        user_id,
        &external_sub,
        &plan_id.to_string(),
        10_000,
        Some(effective_at),
        4_000, // partially consumed — produces a debt record on first fire
    )
    .await;

    let app = ctx.create_unified_test_router();

    // First delivery.
    let event = build_subscription_canceled_event(event_id.clone(), user_id, true, &realm_id);
    let r1 = send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;
    assert_webhook_success(&r1);

    let after_first = get_ledger_by_id(ctx, ledger_id).await;
    assert_eq!(after_first.status, CreditLedgerStatus::Revoked);
    assert_eq!(after_first.revoked_amount, 6_000);
    let revocations_after_first = get_revocation_records(ctx, user_id).await;
    let reclaim_count_first = revocations_after_first
        .iter()
        .filter(|r| r.reason == "subscription_pre_grant_reclaim")
        .count();
    assert_eq!(
        reclaim_count_first, 1,
        "first delivery writes exactly one reclaim debt record"
    );

    // Second delivery with the SAME event_id (provider redelivery).
    let event_again = build_subscription_canceled_event(event_id, user_id, true, &realm_id);
    let r2 = send_webhook_with_signature(&app, &realm_id, event_again, "test_webhook_secret").await;
    assert_webhook_success(&r2);

    let after_second = get_ledger_by_id(ctx, ledger_id).await;
    assert_eq!(
        after_second.revoked_amount, 6_000,
        "no double revoke on redelivery"
    );
    assert_eq!(after_second.remaining_amount, 0);
    assert_eq!(after_second.used_amount, 4_000);

    let revocations_after_second = get_revocation_records(ctx, user_id).await;
    let reclaim_count_second = revocations_after_second
        .iter()
        .filter(|r| r.reason == "subscription_pre_grant_reclaim")
        .count();
    assert_eq!(
        reclaim_count_second, 1,
        "redelivery must not duplicate the reclaim debt record (event-level idempotency)"
    );
}

// ----------------------------------------------------------------------------
// BE-T06 §6.1 P1 regression: off-by-one in `reclaim_pregrant_for_subscription`
// target period. WHY: the reclaim target must be `granted_periods` (the
// highest-numbered period row, which is the future-effective pre-grant),
// NOT `granted_periods + 1`. The earlier `+1` code pointed one period too
// high and left the real pre-grant row active after cancel — a cancelled
// subscription kept its next-period credits.
//
// This test guards the off-by-one going forward by driving the REAL chain
// (activation + renewal via `handle_subscription_paid`, so `granted_periods`
// is bumped by PRODUCTION code, not hand-seeded) and then cancelling. It
// MUST fail against the old `+1` code (which would target a non-existent
// period and leave the real pre-grant active) and pass after the fix.
// ----------------------------------------------------------------------------

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_pregrant_reclaim_targets_granted_periods_after_real_chain(
    ctx: &mut SchemaTestContext,
) {
    let realm_id = ctx._realm_id.clone();
    let user_id = {
        let pool = &ctx.app_state.pool;
        create_test_user(pool, &realm_id, "reclaim_off_by_one@example.com").await
    };
    let plan_id = Uuid::now_v7();
    let entitlement_key = plan_id.to_string();
    let event_id = generate_test_event_id();
    let external_sub = format!("sub_{}", event_id);

    ctx.with_creem_config(&realm_id, None, None, None).await;
    setup_test_plan_config(ctx, &realm_id, plan_id).await;
    create_points_wallet(ctx, user_id, &realm_id).await;

    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;

    // Subscription row keyed by external_subscription_id = sub_<event_id> (the
    // value the cancel webhook builder emits). `sync_creem_subscription`
    // (invoked by the cancel handler) UPDATEs this row in place, preserving
    // its id — which is what `find_grant_schedule_by_subscription` keys on.
    let subscription_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO subscription
            (id, realm_id, user_id, external_subscription_id, external_product_id,
             payment_provider, status, entitlement_key, current_period_start,
             current_period_end, cancel_at_period_end, bucket_id, created_at,
             updated_at)
         VALUES ($1, $2, $3, $4, 'prod_test_monthly', 'creem', 'active', $5, NOW(),
                 NOW() + INTERVAL '30 days', false, $6, NOW(), NOW())",
    )
    .bind(subscription_id)
    .bind(&realm_id)
    .bind(user_id)
    .bind(&external_sub)
    .bind(&entitlement_key)
    .bind(bucket_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("regression: subscription insert");

    // Schedule with granted_periods=0. `handle_subscription_paid` will advance
    // granted_periods via PRODUCTION code: activation period 1 + chained
    // pre-grant period 2 -> granted_periods=2; renewal period 2 (idempotent
    // HIT on the pre-grant) + chained pre-grant period 3 -> granted_periods=3.
    let now = trunc_to_micros(Utc::now());
    let first_period_start = now - Duration::days(60);
    let period1_start = first_period_start;
    let period2_start = first_period_start + Duration::days(30);
    let schedule_id = create_subscription_grant_schedule(
        ctx,
        user_id,
        &realm_id,
        subscription_id,
        &entitlement_key,
        1000,
        period2_start, // next_grant_time (irrelevant once chain runs)
        first_period_start,
        0, // granted_periods — production code advances this
    )
    .await;

    // --- Drive the REAL chain via production code (NOT hand-seeded) --------
    // Activation: grants period 1 + chains pre-grant period 2.
    ctx.app_state
        .subscription_service
        .handle_subscription_paid(
            user_id,
            subscription_id,
            bucket_id,
            &realm_id,
            &entitlement_key,
            false, // activation, not renewal
            period1_start,
            period1_start + Duration::days(30),
            format!("evt_act_{}", event_id),
        )
        .await
        .expect("activation grant should succeed");

    // Renewal: period 2 already pre-granted -> idempotent HIT (no duplicate
    // grant) + chains pre-grant period 3. After this granted_periods == 3.
    ctx.app_state
        .subscription_service
        .handle_subscription_paid(
            user_id,
            subscription_id,
            bucket_id,
            &realm_id,
            &entitlement_key,
            true, // renewal
            period2_start,
            period2_start + Duration::days(30),
            format!("evt_ren_{}", event_id),
        )
        .await
        .expect("renewal grant should succeed");

    // Production invariant: granted_periods is now 3 (the highest period row,
    // which is the period-3 chained pre-grant). This is the row reclaim MUST
    // revoke. The old `+1` code would target period 4 (no such row).
    let granted_periods: i64 =
        sqlx::query_scalar("SELECT granted_periods FROM points_grant_schedules WHERE id = $1")
            .bind(schedule_id)
            .fetch_one(&ctx.app_state.pool)
            .await
            .expect("read granted_periods");
    assert_eq!(
        granted_periods, 3,
        "production chain must advance granted_periods to the pre-granted period (3)"
    );

    // The period-3 pre-grant row exists and is active before cancel.
    let pregrant_ledger_id = find_ledger_id_by_schedule_period(ctx, schedule_id, 3)
        .await
        .expect("period-3 pre-grant row must exist after the chain");
    let before = get_ledger_by_id(ctx, pregrant_ledger_id).await;
    assert_eq!(
        before.status,
        CreditLedgerStatus::Active,
        "period-3 pre-grant row is active before cancel"
    );

    // Sanity: no period-4 row exists (the old `+1` target would be a no-op).
    let period4_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM points_grant_records WHERE schedule_id = $1 AND period_number = 4)",
    )
    .bind(schedule_id)
    .fetch_one(&ctx.app_state.pool)
    .await
    .expect("check period-4 absence");
    assert!(
        !period4_exists,
        "no period-4 row should exist (the old +1 reclaim target was a phantom)"
    );

    // --- Cancel via the real webhook path (triggers reclaim) ---------------
    let event = build_subscription_canceled_event(event_id, user_id, true, &realm_id);
    let app = ctx.create_unified_test_router();
    let response = send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;
    assert_webhook_success(&response);

    // The period-3 pre-grant row MUST be revoked. Against the old `+1` code
    // this assertion FAILS (the row stays active because reclaim targeted the
    // non-existent period 4).
    let after = get_ledger_by_id(ctx, pregrant_ledger_id).await;
    assert_eq!(
        after.status,
        CreditLedgerStatus::Revoked,
        "period-3 pre-grant row (granted_periods) must be revoked after cancel — \
         the old +1 target left it active (off-by-one regression)"
    );
    assert_eq!(after.revoked_amount, 1000, "full pre-grant amount revoked");
    assert_eq!(after.remaining_amount, 0);
}
