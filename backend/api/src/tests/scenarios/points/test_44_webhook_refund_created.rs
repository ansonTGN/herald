// =============================================================================
// Test: Refund Created Webhook
// =============================================================================
//
// Tests for refund.created webhook events.
//
// User Story: docs/user-stories/points-billing-events.md
// Covers: US-PO-06 (Refund revokes unused points proportionally)
//
// =============================================================================

use crate::tests::helpers::points_helpers::*;
use crate::tests::helpers::webhook_helpers::*;
use crate::tests::scenarios::points::fixtures::*;
use crate::tests::schema_test_context::SchemaTestContext;
use herald_core::domain::points::entities::{
    CreditLedgerStatus, CreditSourceType, CreditType, RevocationType,
};
use sqlx::Row;
use test_context::test_context;
use uuid::Uuid;

// ============================================================================
// Test 1: Topup Refund - Proportional Recovery
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-PO-06 场景 1 - 充值退款按未使用比例回收
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_refund_topup_proportional_recovery(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user1@example.com").await;
    let event_id = generate_test_event_id();
    let refund_id = format!("refund_{}", Uuid::now_v7());
    let payment_id = format!("payment_{}", Uuid::now_v7());

    create_points_wallet(ctx, user_id, &realm_id).await;

    // Configure Creem webhook for this realm
    ctx.with_creem_config(&realm_id, None, None, None).await;

    // Grant 10000 topup credits
    let ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::TopupCredit,
        CreditSourceType::Topup,
        payment_id.clone(),
        10000,
        None, // No expiry
    )
    .await;

    // Consume 3000, remaining 7000
    consume_points_from_ledger(ctx, ledger_id, 3000).await;

    // Seed the payment_attempts snapshot the Creem refund webhook resolves the
    // routing bucket from (design A8). Without it the handler fails loud with
    // "no payment_attempt for payment_id".
    let bucket_id = get_wallet_bucket_id(ctx, &realm_id, user_id).await;
    create_payment_attempt_snapshot(ctx, &realm_id, user_id, &payment_id, bucket_id, 10000).await;

    // DEBUG: Verify ledger was created
    let all_ledgers = get_all_ledgers_for_user(ctx, user_id, &realm_id)
        .await
        .expect("Failed to query ledgers");

    println!("DEBUG: Total ledgers for user: {}", all_ledgers.len());
    for (id, credit_type, status, remaining) in &all_ledgers {
        println!(
            "  Ledger: id={}, credit_type={}, status={}, remaining={}",
            id, credit_type, status, remaining
        );
    }

    // When: Refund of 5000 (50% of original 10000)
    let event = build_refund_created_event_with_user(
        event_id,
        refund_id.clone(),
        payment_id.clone(),
        5000,  // refund amount
        10000, // original amount
        &realm_id,
        user_id, // Use the actual user_id from the test
    );

    let app = ctx.create_unified_test_router();
    let response = send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;

    // Then
    assert_webhook_success(&response);

    // Should revoke 50% of remaining: 7000 * 0.5 = 3500
    let ledger = get_ledger_by_id(ctx, ledger_id).await;
    assert_eq!(
        ledger.remaining_amount, 3500,
        "7000 - 3500 = 3500 remaining"
    );
    assert_eq!(ledger.revoked_amount, 3500, "50% of remaining revoked");
    assert_eq!(ledger.used_amount, 3000, "Used amount unchanged");

    // Verify revocation record
    let revocations = get_revocation_records(ctx, user_id).await;
    assert_eq!(revocations.len(), 1);
    assert_eq!(revocations[0].revocation_type, RevocationType::RefundRevoke);
    assert_eq!(revocations[0].revoked_amount, 3500);
    assert_eq!(revocations[0].reference_id, Some(refund_id));
}

// ============================================================================
// Test 2: Subscription Refund - Only Revoke Unused Subscription Credits
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-PO-06 场景 2 - 会员退款仅回收未使用会员积分
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_refund_subscription_only_unused(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user@example.com").await;
    let plan_id = Uuid::now_v7();
    let event_id = generate_test_event_id();
    let refund_id = format!("refund_{}", Uuid::now_v7());
    let payment_id = format!("payment_{}", Uuid::now_v7());

    create_points_wallet(ctx, user_id, &realm_id).await;

    // Configure Creem webhook for this realm
    ctx.with_creem_config(&realm_id, None, None, None).await;

    // Grant 5000 subscription credits
    let ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        CreditSourceType::SubscriptionInitial,
        plan_id.to_string(),
        5000,
        None,
    )
    .await;

    // Consume 2000, remaining 3000
    consume_points_from_ledger(ctx, ledger_id, 2000).await;

    // Seed the payment_attempts snapshot the Creem refund webhook resolves the
    // routing bucket from (design A8). Without it the handler fails loud with
    // "no payment_attempt for payment_id".
    let bucket_id = get_wallet_bucket_id(ctx, &realm_id, user_id).await;
    create_payment_attempt_snapshot(ctx, &realm_id, user_id, &payment_id, bucket_id, 5000).await;

    // When: Full refund (5000) - subscription type
    let event = build_refund_created_event_with_user_and_type(
        event_id,
        refund_id.clone(),
        payment_id.clone(),
        5000,
        5000,
        &realm_id,
        user_id,
        "subscription", // Subscription refund type
    );

    let app = ctx.create_unified_test_router();
    let response = send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;

    // Then
    assert_webhook_success(&response);

    // Should revoke all remaining 3000
    let ledger = get_ledger_by_id(ctx, ledger_id).await;
    assert_eq!(ledger.remaining_amount, 0, "All remaining revoked");
    assert_eq!(ledger.revoked_amount, 3000, "Only unused portion revoked");
    assert_eq!(ledger.used_amount, 2000, "Used amount unchanged");
    assert_eq!(ledger.status, CreditLedgerStatus::Revoked);

    // Verify revocation record
    let revocations = get_revocation_records(ctx, user_id).await;
    assert_eq!(revocations.len(), 1);
    assert_eq!(revocations[0].revocation_type, RevocationType::RefundRevoke);
    assert_eq!(revocations[0].revoked_amount, 3000);
}

/// Get all ledgers for user (debug function)
async fn get_all_ledgers_for_user(
    ctx: &SchemaTestContext,
    user_id: Uuid,
    realm_id: &str,
) -> Result<Vec<(Uuid, String, String, i64)>, Box<dyn std::error::Error>> {
    // DEBUG: Check schema
    let (current_schema, backend_pid): (Option<String>, Option<i32>) =
        sqlx::query_as("SELECT current_schema(), pg_backend_pid()")
            .fetch_one(&ctx.app_state.pool)
            .await
            .ok()
            .unwrap_or((None, None));
    println!(
        "DEBUG get_all_ledgers_for_user: schema={:?}, pid={:?}",
        current_schema, backend_pid
    );

    let rows = sqlx::query(
        "SELECT id, credit_type, status, remaining_amount FROM points_credit_ledger
         WHERE user_id = $1 AND realm_id = $2",
    )
    .bind(user_id)
    .bind(realm_id)
    .fetch_all(&ctx.app_state.pool)
    .await?;

    let result = rows
        .iter()
        .map(|row| {
            (
                row.get("id"),
                row.get("credit_type"),
                row.get("status"),
                row.get("remaining_amount"),
            )
        })
        .collect();

    Ok(result)
}

// ============================================================================
// Test 3: Refund Created Event Idempotency
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-PO-06 场景 - refund.created 幂等性，相同 event_id 不重复回收积分
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_refund_created_idempotency(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user3@example.com").await;
    let event_id = generate_test_event_id();
    let refund_id = format!("refund_{}", Uuid::now_v7());
    let payment_id = format!("payment_{}", Uuid::now_v7());

    create_points_wallet(ctx, user_id, &realm_id).await;

    // Configure Creem webhook for this realm
    ctx.with_creem_config(&realm_id, None, None, None).await;

    // Grant 10000 topup credits
    let ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::TopupCredit,
        CreditSourceType::Topup,
        payment_id.clone(),
        10000,
        None,
    )
    .await;

    // Consume 3000, remaining 7000
    consume_points_from_ledger(ctx, ledger_id, 3000).await;

    // Seed the payment_attempts snapshot the Creem refund webhook resolves the
    // routing bucket from (design A8).
    let bucket_id = get_wallet_bucket_id(ctx, &realm_id, user_id).await;
    create_payment_attempt_snapshot(ctx, &realm_id, user_id, &payment_id, bucket_id, 10000).await;

    // Build refund event with a shared event_id
    let event = build_refund_created_event_with_user(
        event_id.clone(),
        refund_id.clone(),
        payment_id.clone(),
        5000,  // refund amount
        10000, // original amount
        &realm_id,
        user_id,
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
    assert_eq!(revocations[0].revocation_type, RevocationType::RefundRevoke);
    assert_eq!(
        revocations[0].revoked_amount, 3500,
        "50% of 7000 remaining = 3500"
    );

    // Verify ledger state is correct (not double-revoked)
    let ledger = get_ledger_by_id(ctx, ledger_id).await;
    assert_eq!(
        ledger.revoked_amount, 3500,
        "Should revoke exactly 3500, not double"
    );
    assert_eq!(ledger.remaining_amount, 3500);
}

// Covers retry after outer webhook bookkeeping fails: a different webhook event id
// carrying the same refund id must not revoke topup credits again.
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_refund_topup_same_refund_id_different_event_id_is_idempotent(
    ctx: &mut SchemaTestContext,
) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user4@example.com").await;
    let refund_id = format!("refund_{}", Uuid::now_v7());
    let payment_id = format!("payment_{}", Uuid::now_v7());

    create_points_wallet(ctx, user_id, &realm_id).await;
    ctx.with_creem_config(&realm_id, None, None, None).await;

    let ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::TopupCredit,
        CreditSourceType::Topup,
        payment_id.clone(),
        10000,
        None,
    )
    .await;
    consume_points_from_ledger(ctx, ledger_id, 3000).await;

    let bucket_id = get_wallet_bucket_id(ctx, &realm_id, user_id).await;
    create_payment_attempt_snapshot(ctx, &realm_id, user_id, &payment_id, bucket_id, 10000).await;

    let app = ctx.create_unified_test_router();
    for _ in 0..2 {
        let event = build_refund_created_event_with_user(
            generate_test_event_id(),
            refund_id.clone(),
            payment_id.clone(),
            5000,
            10000,
            &realm_id,
            user_id,
        );
        let response =
            send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;
        assert_webhook_success(&response);
    }

    let revocations = get_revocation_records(ctx, user_id).await;
    assert_eq!(revocations.len(), 1, "same refund id must not revoke twice");
    assert_eq!(revocations[0].revoked_amount, 3500);

    let ledger = get_ledger_by_id(ctx, ledger_id).await;
    assert_eq!(ledger.revoked_amount, 3500);
    assert_eq!(ledger.remaining_amount, 3500);
}

// Covers retry after outer webhook bookkeeping fails: subscription refund revoke
// must use the refund id as business idempotency, not only the webhook event id.
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_refund_subscription_same_refund_id_different_event_id_is_idempotent(
    ctx: &mut SchemaTestContext,
) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user5@example.com").await;
    let refund_id = format!("refund_{}", Uuid::now_v7());
    let payment_id = format!("payment_{}", Uuid::now_v7());

    create_points_wallet(ctx, user_id, &realm_id).await;
    ctx.with_creem_config(&realm_id, None, None, None).await;

    let ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        CreditSourceType::SubscriptionInitial,
        payment_id.clone(),
        5000,
        None,
    )
    .await;
    consume_points_from_ledger(ctx, ledger_id, 2000).await;

    let bucket_id = get_wallet_bucket_id(ctx, &realm_id, user_id).await;
    create_payment_attempt_snapshot(ctx, &realm_id, user_id, &payment_id, bucket_id, 5000).await;

    let app = ctx.create_unified_test_router();
    for _ in 0..2 {
        let event = build_refund_created_event_with_user_and_type(
            generate_test_event_id(),
            refund_id.clone(),
            payment_id.clone(),
            5000,
            5000,
            &realm_id,
            user_id,
            "subscription",
        );
        let response =
            send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;
        assert_webhook_success(&response);
    }

    let revocations = get_revocation_records(ctx, user_id).await;
    assert_eq!(revocations.len(), 1, "same refund id must not revoke twice");
    assert_eq!(revocations[0].revoked_amount, 3000);
    assert_eq!(revocations[0].reference_id, Some(refund_id));

    let ledger = get_ledger_by_id(ctx, ledger_id).await;
    assert_eq!(ledger.revoked_amount, 3000);
    assert_eq!(ledger.remaining_amount, 0);
}

// ============================================================================
// BE-T06: Creem refund.created reclaim (row-level, design §5.2 A4 / §6.1 P1)
// ============================================================================
//
// User Story: docs/user-stories/points-billing-events.md
// Covers: design point-time §6.1 "预生成失败回收（P1）" Creem refund.created
// branch + §5.2 A4 (row-level reclaim via (schedule_id, period_number)) + A7
// (derived balance, no wallet back-adjust).
//
// The Creem refund.created handler invokes the same
// `reclaim_pregrant_for_subscription` row-level path as Stripe
// invoice.payment_failed / subscription.canceled (design §5.2: "Creem
// subscription.canceled / refund.created, 与 Stripe 对称"). For a subscription
// refund (refundType != "topup") it first resolves the routing bucket via the
// payment_attempt snapshot, then for each active subscription schedule in that
// bucket reclaims the chained pre-grant row, and finally calls
// `revoke_subscription_unused` to revoke any still-active subscription credits.
// These tests isolate the reclaim row-level effect by seeding only the
// pre-grant row as active subscription_credit (after reclaim it is revoked, so
// `revoke_subscription_unused` finds nothing else to revoke).

/// Seed a Creem subscription + chained pre-grant row for refund reclaim tests.
///
/// Mirrors `seed_pregrant_for_reclaim` in test_43 but returns the pieces the
/// refund test needs (external_sub for the event, schedule_id + ledger_id +
/// period_number for assertions, payment_id for the payment_attempt snapshot).
async fn seed_creem_pregrant_for_refund_reclaim(
    ctx: &mut SchemaTestContext,
    realm_id: &str,
    user_id: Uuid,
    external_subscription_id: &str,
    entitlement_key: &str,
    payment_id: &str,
    amount: i64,
    effective_at: Option<chrono::DateTime<chrono::Utc>>,
    used_amount: i64,
) -> (Uuid, Uuid, Uuid, i64) {
    use chrono::Duration;

    let pool = &ctx.app_state.pool;
    let subscription_id = Uuid::now_v7();
    let client_app_id = Uuid::now_v7();
    let bucket_id = ensure_test_bucket_for_realm(pool, realm_id).await;

    sqlx::query(
        "INSERT INTO client_app (id, realm_id, client_id, name, enabled)
         VALUES ($1, $2, $3, $4, true)",
    )
    .bind(client_app_id)
    .bind(realm_id)
    .bind(format!("client-{}", client_app_id))
    .bind("BE-T06 creem refund seed")
    .execute(pool)
    .await
    .expect("creem seed: client_app insert");

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
    .expect("creem seed: subscription insert");

    let first_period_start = chrono::Utc::now() - Duration::days(15);
    let next_grant_time = effective_at.unwrap_or_else(chrono::Utc::now);
    let schedule_id = create_subscription_grant_schedule(
        ctx,
        user_id,
        realm_id,
        subscription_id,
        entitlement_key,
        amount,
        next_grant_time,
        first_period_start,
        // Production invariant: `pregrant_next_period_atomic` advances
        // `granted_periods = max(old, period_number)` after writing the chained
        // pre-grant, so when a pre-grant exists at period 2, `granted_periods`
        // is ALREADY 2 (not 1). Both reclaim paths (Creem cancel via
        // `handle_subscription_canceled` → `reclaim_pregrant_for_subscription`,
        // and Creem refund via `handle_refund_created` →
        // `reclaim_pregrant_for_subscription`) resolve the reclaim target as
        // `granted_periods` itself, which resolves to the period-2 pre-grant
        // row. Mirrors `seed_pregrant_for_reclaim` in test_43.
        2, // granted_periods — pre-grant bumped it to the pre-granted period
    )
    .await;

    let expires_at = effective_at.unwrap_or_else(chrono::Utc::now) + chrono::Duration::days(30);
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
        consume_points_from_ledger(ctx, ledger_id, used_amount).await;
    }

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

    // Payment attempt snapshot — the Creem refund handler resolves the routing
    // bucket from this. It must match the schedule's bucket (both use the
    // realm's legacy test bucket via ensure_test_bucket_for_realm).
    create_payment_attempt_snapshot(ctx, realm_id, user_id, payment_id, bucket_id, amount).await;

    (subscription_id, schedule_id, ledger_id, period_number)
}

// ----------------------------------------------------------------------------
// BE-T06 §6.1 P1 scenario (5a): Creem refund.created triggers row-level
// reclaim of the subscription's chained pre-grant row (Stripe-symmetric).
// ----------------------------------------------------------------------------

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_creem_refund_reclaim_row_level(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(
        &ctx.app_state.pool,
        &realm_id,
        "creem_refund_reclaim@example.com",
    )
    .await;
    let plan_id = Uuid::now_v7();
    let event_id = generate_test_event_id();
    let refund_id = format!("refund_{}", Uuid::now_v7());
    let payment_id = format!("payment_{}", Uuid::now_v7());
    let external_sub = format!("sub_{}", Uuid::now_v7());

    ctx.with_creem_config(&realm_id, None, None, None).await;
    setup_test_plan_config(ctx, &realm_id, plan_id).await;
    create_points_wallet(ctx, user_id, &realm_id).await;

    // Future-effective pre-grant row — not yet in the available set.
    let effective_at = chrono::Utc::now() + chrono::Duration::days(5);
    let (_sub_id, schedule_id, ledger_id, period_number) = seed_creem_pregrant_for_refund_reclaim(
        ctx,
        &realm_id,
        user_id,
        &external_sub,
        &plan_id.to_string(),
        &payment_id,
        10_000,
        Some(effective_at),
        0,
    )
    .await;

    // Locator resolves to our row (proves the (schedule_id, period) bridge).
    let located = find_ledger_id_by_schedule_period(ctx, schedule_id, period_number)
        .await
        .expect("creem pre-grant row must be locatable via (schedule_id, period_number)");
    assert_eq!(located, ledger_id);
    assert_derived_balance(ctx, user_id, &realm_id, CreditType::SubscriptionCredit, 0).await;

    // Fire Creem refund.created as a subscription refund.
    let event = build_refund_created_event_with_user_and_type(
        event_id,
        refund_id.clone(),
        payment_id.clone(),
        10_000,
        10_000,
        &realm_id,
        user_id,
        "subscription",
    );
    let app = ctx.create_unified_test_router();
    let response = send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;
    assert_webhook_success(&response);

    // Row-level reclaim revoked the chained pre-grant row.
    let ledger = get_ledger_by_id(ctx, ledger_id).await;
    assert_eq!(ledger.status, CreditLedgerStatus::Revoked);
    assert_eq!(ledger.remaining_amount, 0);
    assert_eq!(
        ledger.revoked_amount, 10_000,
        "full pre-grant amount revoked by row-level reclaim"
    );
    assert_eq!(ledger.used_amount, 0);

    // Derived balance excludes the revoked row (A7).
    assert_derived_balance(ctx, user_id, &realm_id, CreditType::SubscriptionCredit, 0).await;

    // Fully-unused row ⟹ no reclaim debt record.
    let revocations = get_revocation_records(ctx, user_id).await;
    assert!(
        revocations
            .iter()
            .all(|r| r.reason != "subscription_pre_grant_reclaim"),
        "fully-unused creem refund reclaim must not produce a debt record; got {revocations:?}"
    );
}

// ----------------------------------------------------------------------------
// BE-T06 §6.1 P1 scenario (5b): Creem subscription.canceled reclaims an
// unfulfilled (future-effective) pre-grant row (Stripe-symmetric with test_43
// scenario 1). Covers the Creem arm of the cancel→reclaim symmetry.
// ----------------------------------------------------------------------------

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_creem_cancel_reclaim_unfulfilled(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(
        &ctx.app_state.pool,
        &realm_id,
        "creem_cancel_reclaim@example.com",
    )
    .await;
    let plan_id = Uuid::now_v7();
    let event_id = generate_test_event_id();
    let external_sub = format!("sub_{}", event_id);

    ctx.with_creem_config(&realm_id, None, None, None).await;
    setup_test_plan_config(ctx, &realm_id, plan_id).await;
    create_points_wallet(ctx, user_id, &realm_id).await;

    // Future-effective pre-grant row.
    let effective_at = chrono::Utc::now() + chrono::Duration::days(5);
    let (_sub_id, schedule_id, ledger_id, _period) = seed_creem_pregrant_for_refund_reclaim(
        ctx,
        &realm_id,
        user_id,
        &external_sub,
        &plan_id.to_string(),
        &format!("payment_{}", Uuid::now_v7()), // unused for cancel path
        10_000,
        Some(effective_at),
        0,
    )
    .await;

    assert_eq!(
        find_ledger_id_by_schedule_period(ctx, schedule_id, 2).await,
        Some(ledger_id),
        "pre-grant row locatable before webhook"
    );

    // Creem subscription.canceled with cancel_at_period_end=true isolates the
    // reclaim (cancel path only sets expiry; no bulk revoke).
    let event = build_subscription_canceled_event(event_id, user_id, true, &realm_id);
    let app = ctx.create_unified_test_router();
    let response = send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;
    assert_webhook_success(&response);

    let ledger = get_ledger_by_id(ctx, ledger_id).await;
    assert_eq!(
        ledger.status,
        CreditLedgerStatus::Revoked,
        "creem cancel reclaims the pre-grant row"
    );
    assert_eq!(ledger.revoked_amount, 10_000);
    assert_eq!(ledger.remaining_amount, 0);
    assert_eq!(ledger.used_amount, 0);

    assert_derived_balance(ctx, user_id, &realm_id, CreditType::SubscriptionCredit, 0).await;
}
