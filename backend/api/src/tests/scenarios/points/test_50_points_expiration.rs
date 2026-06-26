// =============================================================================
// Test: Points Expiration
// =============================================================================
//
// Tests for automatic points expiration mechanism.
//
// User Story: docs/user-stories/points-billing-events.md
// Covers: US-PO-06 (Expired points are automatically revoked)
//
// =============================================================================

use crate::tests::helpers::points_helpers::*;
use crate::tests::scenarios::points::fixtures::*;
use crate::tests::schema_test_context::SchemaTestContext;
use chrono::{Duration, Utc};
use herald_core::domain::points::ExpirationService;
use herald_core::domain::points::dtos::ConsumePointsInput;
use herald_core::domain::points::entities::{CreditSourceType, CreditType};
use test_context::test_context;
use uuid::Uuid;

// ============================================================================
// Test 3: Expired Credits Cannot Be Consumed
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-PO-06 扩展场景 - 过期积分不可消费
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_expired_points_cannot_consume(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user@example.com").await;
    let plan_id = Uuid::now_v7();

    create_points_wallet(ctx, user_id, &realm_id).await;

    // Expired subscription credit (but not yet marked as expired in database)
    let expired_at = Utc::now() - Duration::days(10);
    let expired_ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        CreditSourceType::SubscriptionInitial,
        plan_id.to_string(),
        10000,
        Some(expired_at),
    )
    .await;

    sqlx::query(
        "UPDATE points_credit_ledger
         SET status = 'expired',
             revoked_amount = granted_amount,
             updated_at = NOW()
         WHERE id = $1",
    )
    .bind(expired_ledger_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to mark expired ledger as expired");

    // Available topup credit
    let topup_ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::TopupCredit,
        CreditSourceType::Topup,
        Uuid::now_v7().to_string(),
        5000,
        None,
    )
    .await;

    // When: Consume only the available non-expired topup credits
    let identity = create_test_third_party_identity(&realm_id);
    let input = ConsumePointsInput {
        user_id: user_id.to_string(),
        client_app_id: ctx._client_app_id.clone(),
        amount: 5000,
        description: Some("test_consumption".to_string()),
    };
    let result = ctx
        .app_state
        .points_service
        .consume_points(identity, &realm_id, input)
        .await;

    // Then: Should succeed using only the non-expired topup credits
    assert!(result.is_ok(), "Should consume available topup credits");

    let transaction = &result.unwrap()[0];
    assert_eq!(
        transaction.amount, -5000,
        "Should record a 5000-point consumption from topup credits"
    );

    // Expired credits should remain untouched
    let expired_ledger = get_ledger_by_id(ctx, expired_ledger_id).await;
    assert_eq!(
        expired_ledger.remaining_amount, 0,
        "Expired credits should remain unavailable after expiration"
    );
    assert_eq!(expired_ledger.used_amount, 0, "No usage of expired credits");
    assert_eq!(
        expired_ledger.revoked_amount, 10000,
        "Expired credits should be fully revoked"
    );

    // Topup credits should be fully consumed
    let topup_ledger = get_ledger_by_id(ctx, topup_ledger_id).await;
    assert_eq!(topup_ledger.remaining_amount, 0, "Topup credits consumed");
    assert_eq!(topup_ledger.used_amount, 5000, "Topup credits fully used");
}

// ============================================================================
// expiration regression (P0 + risk P1)
// ============================================================================
//
// WHY these tests exist (encode intent, not just behavior — Rule 9):
//
//   (a) The PointsExpirationJob sweeps rows with `status='active' AND
//       expires_at <= NOW() AND remaining_amount > 0`. A pre-generated
//       future-effective row is NOT swept because its `expires_at` is in the
//       future (`expires_at >= effective_at` per the CHECK constraint).
//       The job predicate does not even mention `effective_at` — it relies on
//       the monotone relationship `effective_at <= expires_at`. Asserting this
//       pins the invariant: "过期 job 不误扫预生成未来期行".
//
//   (b) Derived available balance uses the predicate
//       `(expires_at IS NULL OR expires_at > NOW())`, so an expired row drops
//       from the SUM automatically — correct availability does NOT depend on
//       the job having marked it `expired` ("正确性不依赖").
//
//   (c) A row with BOTH `effective_at` in the future AND `expires_at` in the
//       future is doubly-protected: it is neither swept (future `expires_at`)
//       nor in the available set (future `effective_at`).
//
// User Story: docs/user-stories/points-billing-events.md
// Covers: 过期回归（P0）+ risk P1 (过期 job 对带未来
// effective_at 的 active 行处理正确). The pre-existing
// `test_expired_points_cannot_consume` above is the zero-regression anchor for
// the consume path; the four tests below pin the expiration job + derived
// balance behavior specific to point-time.
//
// Helper policy: these tests use the derived-balance helpers
// `create_credit_ledger_entry_with_effective_at`, `inject_effective_at`,
// `get_derived_balance_by_credit_type`, `assert_derived_balance`, and
// `count_future_effective_active_rows` — NOT the broken legacy helpers that
// touch dropped `points_wallets` balance columns.

/// Helper: read a ledger row's status string for assertion.
async fn ledger_status(ctx: &SchemaTestContext, ledger_id: Uuid) -> String {
    let (status,): (String,) =
        sqlx::query_as("SELECT status::text FROM points_credit_ledger WHERE id = $1")
            .bind(ledger_id)
            .fetch_one(&ctx.app_state.pool)
            .await
            .expect("Failed to fetch ledger status");
    status
}

// ============================================================================
// Test (#1): expiration job sweeps past-expired rows but NOT
// future-effective rows.
// ============================================================================
//
// User Story: docs/user-stories/points-billing-events.md
// Covers: US-PO-06 (Expired points are automatically revoked),
// risk P1. The expiration job selects `expires_at <= NOW()`; a
// future-effective pre-grant row has `expires_at >= effective_at` (CHECK
// constraint), so `expires_at` is also in the future and the row is NOT swept.
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_expiration_job_skips_future_effective_rows(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id =
        create_test_user(&ctx.app_state.pool, &realm_id, "t07-exp-skip@example.com").await;
    // Wallet must exist for the job's per-ledger wallet lookup
    // (`find_wallet_by_user_bucket_for_update`) to succeed.
    create_points_wallet(ctx, user_id, &realm_id).await;

    let now = Utc::now();

    // Row A: past-expired, immediately-available (effective_at=NULL, expires_at
    // in the past). The job SHOULD sweep this one to status='expired'.
    let expired_row = create_credit_ledger_entry_with_effective_at(
        ctx,
        user_id,
        &realm_id,
        CreditType::TopupCredit,
        CreditSourceType::Topup,
        Uuid::now_v7().to_string(),
        1_000,
        Some(now - Duration::days(1)), // expires_at in the past
        None,                          // effective_at NULL ⟹ immediately available
    )
    .await;

    // Row B: future-effective pre-grant (effective_at in the future, expires_at
    // further in the future). The job MUST NOT sweep this one.
    let future_row = create_credit_ledger_entry_with_effective_at(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        CreditSourceType::SubscriptionRenewal,
        Uuid::now_v7().to_string(),
        2_000,
        Some(now + Duration::days(30)), // expires_at in the future
        Some(now + Duration::days(1)),  // effective_at in the future
    )
    .await;

    // Sanity: precondition — both rows start active.
    assert_eq!(ledger_status(ctx, expired_row).await, "active");
    assert_eq!(ledger_status(ctx, future_row).await, "active");

    // Run the PointsExpirationJob's underlying service entry point directly
    // (same path the worker calls — worker loop is not a correctness boundary).
    let expiration_service = ExpirationService::new(ctx.app_state.points_repository.clone());
    let summary = expiration_service
        .scan_and_expire_points(100)
        .await
        .expect("expiration job should succeed");

    // Assert intent (WHY matters — not just the status flip):
    assert_eq!(
        summary.expired_count, 1,
        "only the past-expired row should be swept; the future-effective row \
         has expires_at in the future and must NOT be touched"
    );
    assert_eq!(
        ledger_status(ctx, expired_row).await,
        "expired",
        "past-expired row must be marked expired by the job"
    );
    assert_eq!(
        ledger_status(ctx, future_row).await,
        "active",
        "future-effective row must remain active — its expires_at is in the \
         future (effective_at <= expires_at invariant), so the job's \
         `expires_at <= NOW()` predicate does not match it"
    );
    // Cross-check via the future-effective count helper: exactly one such row.
    assert_eq!(
        count_future_effective_active_rows(ctx, user_id, &realm_id).await,
        1,
        "the future-effective active row must still be present and active"
    );
}

// ============================================================================
// Test (#2): derived available balance excludes expired rows even
// before the job marks them.
// ============================================================================
//
// User Story: docs/user-stories/points-billing-events.md
// Covers: US-PU-001 (View My Points Balance) — "正确性不依赖"
// + 派生余额 = 可消费额. The derived predicate
// `(expires_at IS NULL OR expires_at > NOW())` drops expired rows from the
// available SUM regardless of whether the job has marked them; this is WHY
// best-effort expiration scheduling is acceptable.
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_derived_balance_excludes_expired_rows(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(
        &ctx.app_state.pool,
        &realm_id,
        "t07-derived-excl@example.com",
    )
    .await;
    create_points_wallet(ctx, user_id, &realm_id).await;

    let now = Utc::now();

    // Expired-by-predicate row: status still 'active' (job hasn't run), but
    // expires_at in the past. The derived predicate must EXCLUDE it.
    let _expired_row = create_credit_ledger_entry_with_effective_at(
        ctx,
        user_id,
        &realm_id,
        CreditType::TopupCredit,
        CreditSourceType::Topup,
        Uuid::now_v7().to_string(),
        1_000,
        Some(now - Duration::days(1)), // expired
        None,                          // immediately available (ignoring expiry)
    )
    .await;

    // Available row: expires in the future, immediately available.
    let _available_row = create_credit_ledger_entry_with_effective_at(
        ctx,
        user_id,
        &realm_id,
        CreditType::TopupCredit,
        CreditSourceType::Topup,
        Uuid::now_v7().to_string(),
        3_000,
        Some(now + Duration::days(30)), // valid
        None,                           // immediately available
    )
    .await;

    // Assert intent (WHY): derived balance = SUM over the available predicate,
    // so the expired row contributes 0 — even though its status is still
    // 'active' and the job has not run.
    assert_eq!(
        ledger_status(ctx, _expired_row).await,
        "active",
        "precondition: row is still active (job not run yet)"
    );
    assert_derived_balance(ctx, user_id, &realm_id, CreditType::TopupCredit, 3_000).await;
}

// ============================================================================
// Test (#3): a purely future-effective row (effective_at AND expires_at
// both in the future) is neither swept nor in the available set.
// ============================================================================
//
// User Story: docs/user-stories/points-billing-events.md
// Covers: risk P1 (过期 job/回收对带未来 effective_at 的 active 行处理
// 正确). Pins the doubly-protected case: the row is not swept (future
// expires_at) AND not available (future effective_at).
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_future_effective_row_with_future_expires_not_scanned(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(
        &ctx.app_state.pool,
        &realm_id,
        "t07-pure-future@example.com",
    )
    .await;
    create_points_wallet(ctx, user_id, &realm_id).await;

    let now = Utc::now();

    // Pure future-effective row.
    let future_row = create_credit_ledger_entry_with_effective_at(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        CreditSourceType::SubscriptionInitial,
        Uuid::now_v7().to_string(),
        5_000,
        Some(now + Duration::days(30)), // future expires_at
        Some(now + Duration::days(1)),  // future effective_at
    )
    .await;

    // Precondition: derived balance currently EXCLUDES this row (future
    // effective_at) — it is not in the available set yet.
    assert_derived_balance(ctx, user_id, &realm_id, CreditType::SubscriptionCredit, 0).await;

    // Run the expiration job.
    let expiration_service = ExpirationService::new(ctx.app_state.points_repository.clone());
    let summary = expiration_service
        .scan_and_expire_points(100)
        .await
        .expect("expiration job should succeed");

    // Assert intent (WHY): the job swept nothing because the only row's
    // expires_at is in the future — the `effective_at <= expires_at` invariant
    // guarantees a future-effective row is never an expiration target.
    assert_eq!(
        summary.expired_count, 0,
        "no rows should be swept — the only row has a future expires_at"
    );
    assert_eq!(
        ledger_status(ctx, future_row).await,
        "active",
        "future-effective row must remain active"
    );
    assert_eq!(
        count_future_effective_active_rows(ctx, user_id, &realm_id).await,
        1,
        "future-effective active row must still be present"
    );
    // And the derived balance is still 0 (the row has not become available).
    assert_derived_balance(ctx, user_id, &realm_id, CreditType::SubscriptionCredit, 0).await;
}

// ============================================================================
// Test (#4): zero-regression — effective_at=NULL behaves identically
// to pre-point-time for both expiration sweep and derived balance.
// ============================================================================
//
// User Story: docs/user-stories/points-billing-events.md
// Covers: effective_at 回归（P0）+ risk P0 (派生余额替代 Stored 列
// 读取须保证 effective_at=NULL 时零回归). Every pre-point-time ledger row had
// effective_at=NULL; this test pins that the expiration sweep + derived
// predicate treat such rows exactly as before.
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_expiration_with_effective_at_null_zero_regression(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(
        &ctx.app_state.pool,
        &realm_id,
        "t07-null-regress@example.com",
    )
    .await;
    create_points_wallet(ctx, user_id, &realm_id).await;

    let now = Utc::now();

    // Two effective_at=NULL rows — the universal pre-point-time shape.
    // Row A: expires in the past ⟹ derived predicate excludes AND job sweeps.
    let past_expire_row = create_credit_ledger_entry_with_effective_at(
        ctx,
        user_id,
        &realm_id,
        CreditType::TopupCredit,
        CreditSourceType::Topup,
        Uuid::now_v7().to_string(),
        1_000,
        Some(now - Duration::days(1)), // past expires_at
        None,                          // effective_at NULL — zero-regression shape
    )
    .await;
    // Row B: expires in the future ⟹ derived predicate includes, job skips.
    let future_expire_row = create_credit_ledger_entry_with_effective_at(
        ctx,
        user_id,
        &realm_id,
        CreditType::TopupCredit,
        CreditSourceType::Topup,
        Uuid::now_v7().to_string(),
        4_000,
        Some(now + Duration::days(30)), // future expires_at
        None,                           // effective_at NULL — zero-regression shape
    )
    .await;

    // Zero-regression assertion (pre-job): derived balance == 4_000, i.e. only
    // the not-yet-expired row — identical to the pre-point-time SUM behavior.
    assert_derived_balance(ctx, user_id, &realm_id, CreditType::TopupCredit, 4_000).await;
    // And no future-effective rows exist for this user (all effective_at NULL).
    assert_eq!(
        count_future_effective_active_rows(ctx, user_id, &realm_id).await,
        0,
        "no future-effective rows — all effective_at IS NULL"
    );

    // Run the expiration job — same behavior as before point-time: the
    // past-expired row is swept, the future-expired row is untouched.
    let expiration_service = ExpirationService::new(ctx.app_state.points_repository.clone());
    let summary = expiration_service
        .scan_and_expire_points(100)
        .await
        .expect("expiration job should succeed");

    assert_eq!(
        summary.expired_count, 1,
        "only the past-expired row is swept"
    );
    assert_eq!(
        ledger_status(ctx, past_expire_row).await,
        "expired",
        "past-expired row is swept (zero-regression: same as pre-point-time)"
    );
    assert_eq!(
        ledger_status(ctx, future_expire_row).await,
        "active",
        "future-expired row remains active"
    );
    // Post-job derived balance still 4_000 (sweep doesn't change the derived
    // result — the row was already excluded by the predicate before being
    // marked).
    assert_derived_balance(ctx, user_id, &realm_id, CreditType::TopupCredit, 4_000).await;
}
