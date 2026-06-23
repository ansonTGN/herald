// =============================================================================
// point-time BE-T02: effective_at Semantics (P0)
// =============================================================================
//
// Encodes design `.ai/design/point-time.md` §6.1 P0 scenarios 3-5:
//
//   3. "未到生效时间不可见/不可消费": a future-effective active row is excluded
//      from the consumption selection predicate AND from the derived available
//      balance — it does not leak into any "available / remaining"口径.
//   4. "零延迟可用 (核心)": a pre-written future-effective row becomes available
//      the moment the clock reaches `effective_at` purely via the predicate —
//      NO worker / state-flipping job is invoked. This is the central
//      correctness claim of point-time (availability is a predicate, not a
//      state machine).
//   5. "立即可用语义": `effective_at = NULL` ⟺ immediately available — zero
//      regression for every existing row (which is the production default).
//
// All balance assertions in this file use the BE-T01 derived-predicate helpers
// (`assert_derived_balance` / `get_derived_balance_by_credit_type`), mirroring
// production `compute_available_balance` verbatim. They NEVER read
// `points_wallets.total_balance` — BE-D11 physically removed that column and
// the derived SUM is the only available-balance authority under point-time.
//
// Ledger rows are seeded with `create_credit_ledger_entry_with_effective_at`
// which intentionally does NOT touch `points_wallets` Stored columns — this
// keeps the derived balance the sole source of truth for these assertions.
//
// =============================================================================

use crate::tests::helpers::points_helpers::{
    assert_derived_balance, count_future_effective_active_rows,
    create_credit_ledger_entry_with_effective_at, create_test_third_party_identity,
    get_derived_total_balance, get_ledger_by_id,
};
use crate::tests::scenarios::points::fixtures::*;
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use chrono::{Duration, Utc};
use herald_core::domain::common::entities::app_errors::CoreError;
use herald_core::domain::points::dtos::ConsumePointsInput;
use herald_core::domain::points::entities::{CreditSourceType, CreditType};
use test_context::test_context;
use uuid::Uuid;

// ----------------------------------------------------------------------------
// Scenario §6.1 #3: future-effective row is invisible & unconsumable
// ----------------------------------------------------------------------------

// User Story: US-PU-001 / US-PU-004 / US-PU-005 (future-period credits must
// not be visible or consumable before their effective time).
//
// Covers design §6.1 P0 "未到生效时间不可见/不可消费" + §6.3 risk
// "消费可用性谓词增 effective_at：影响 consume/refund/cancel/expire 全场景".
//
// Why this test exists: the consumption selection predicate and the derived
// balance predicate share the same `effective_at <= NOW()` gate (BE-D04). A
// future-effective active row must therefore be excluded from BOTH the
// "consumable ledger set" and the "available balance"口径 — otherwise the
// invariant "balance you see == balance you can spend" breaks and future
// periods silently leak into the current period.
#[test_context(TestContext)]
#[tokio::test]
async fn test_future_effective_not_visible_not_consumable(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "be-t02-future@exam.com").await;

    // Seed TWO subscription_credit rows on the same (user, realm, bucket):
    //   * immediately-available  (effective_at = NULL)  -> 2000 in derived balance
    //   * future-effective       (effective_at = NOW+1d) -> excluded from derived
    let _immediate_id = create_credit_ledger_entry_with_effective_at(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        CreditSourceType::SubscriptionInitial,
        format!("be-t02-future-imm-{}", Uuid::now_v7()),
        2000,
        None,
        None,
    )
    .await;

    let future_effective = Utc::now() + Duration::days(1);
    let future_id = create_credit_ledger_entry_with_effective_at(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        CreditSourceType::SubscriptionInitial,
        format!("be-t02-future-next-{}", Uuid::now_v7()),
        3000,
        None,
        Some(future_effective),
    )
    .await;

    // (a) Invisibility: derived available balance counts ONLY the immediate row.
    assert_derived_balance(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        2000,
    )
    .await;
    assert_eq!(
        count_future_effective_active_rows(ctx, user_id, &realm_id).await,
        1,
        "exactly one future-effective active row should be present"
    );

    // (b) Unconsumable: try to consume more than the immediately-available 2000.
    // The future-effective 3000 MUST NOT be selected — the request must fail
    // with Insufficient points (not silently draw down the future row).
    let identity = create_test_third_party_identity(&realm_id);
    let input = ConsumePointsInput {
        user_id: user_id.to_string(),
        client_app_id: ctx._client_app_id.clone(),
        amount: 2500,
        description: Some("be-t02 future-effective exclusion".to_string()),
    };
    let result = ctx
        .app_state
        .points_service
        .consume_points(identity, &realm_id, input)
        .await;

    match result {
        Err(CoreError::BadRequest(msg))
            if msg.contains("Insufficient points balance") || msg.contains("Insufficient") =>
        {
            // expected: 2500 requested > 2000 available (future 3000 excluded)
        }
        other => panic!(
            "consume of 2500 with only 2000 available (3000 future-effective) must fail with \
             Insufficient points; the future row must NOT be spent. got: {:?}",
            other
        ),
    }

    // (c) The future-effective row is untouched by the failed consume attempt.
    let future = get_ledger_by_id(ctx, future_id).await;
    assert_eq!(
        future.used_amount, 0,
        "future-effective row must not be consumed when effective_at > NOW"
    );
    assert_eq!(
        future.remaining_amount, 3000,
        "future-effective row remaining must be unchanged"
    );
    assert_eq!(
        future.status.as_str(),
        "active",
        "future-effective row stays active (only the predicate excludes it; no state flip)"
    );
}

// ----------------------------------------------------------------------------
// Scenario §6.1 #4: zero-delay availability — advance clock ONLY, no worker
// ----------------------------------------------------------------------------

// User Story: US-PU-009 (use this period's credits on time, unaffected by
// distribution / webhook / scheduler latency).
//
// Covers design §6.1 P0 "零延迟可用" + §6.3 risk "派生余额替代 Stored 列读取".
//
// Why this test exists: this is THE core point-time correctness claim. A
// pre-written future-effective ledger row must enter the available set purely
// because the predicate `effective_at <= NOW()` flips true when the clock
// advances — NO background worker, NO pending→active state machine, NO
// state-flipping job is involved. We simulate "the clock reached
// effective_at" by directly UPDATE-ing the row's `effective_at` to a past
// timestamp (the testing.md virtual-clock idiom: this project has no
// injectable `now`, so SQL UPDATE on `effective_at` is the canonical way to
// advance the clock without running any job). If availability required a job,
// this test would fail — which is precisely the regression we are locking
// out.
#[test_context(TestContext)]
#[tokio::test]
async fn test_zero_delay_available_advance_clock_only(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id =
        create_test_user(&ctx.app_state.pool, &realm_id, "be-t02-zerodelay@exam.com").await;

    // Seed a future-effective subscription_credit row (effective tomorrow).
    let future_effective = Utc::now() + Duration::days(1);
    let ledger_id = create_credit_ledger_entry_with_effective_at(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        CreditSourceType::SubscriptionInitial,
        format!("be-t02-zerodelay-{}", Uuid::now_v7()),
        5000,
        None,
        Some(future_effective),
    )
    .await;

    // Pre-advance assertion: row is future-effective → not in derived balance.
    assert_derived_balance(ctx, user_id, &realm_id, CreditType::SubscriptionCredit, 0).await;
    assert_eq!(
        count_future_effective_active_rows(ctx, user_id, &realm_id).await,
        1,
        "row is future-effective before clock advance"
    );

    // === THE core gesture: simulate "the clock reached effective_at" by
    // directly UPDATE-ing `effective_at` to 1 second in the past. We do NOT
    // call any worker / scheduler / job / state-machine. If point-time's
    // availability were state-driven (pending→active flip job), this single
    // UPDATE would have NO effect on the derived balance and the next
    // assertion would fail. ===
    sqlx::query(
        "UPDATE points_credit_ledger
         SET effective_at = NOW() - INTERVAL '1 second', updated_at = NOW()
         WHERE id = $1",
    )
    .bind(ledger_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to advance effective_at via SQL UPDATE (virtual-clock idiom)");

    // Post-advance assertion (a): derived balance now INCLUDES the row,
    // immediately, with no worker invocation.
    assert_derived_balance(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        5000,
    )
    .await;
    assert_eq!(
        count_future_effective_active_rows(ctx, user_id, &realm_id).await,
        0,
        "row is no longer future-effective after clock advance"
    );

    // Post-advance assertion (b): the now-available row is immediately
    // consumable — proving availability == consumable amount (the same
    // predicate gates both).
    let identity = create_test_third_party_identity(&realm_id);
    let input = ConsumePointsInput {
        user_id: user_id.to_string(),
        client_app_id: ctx._client_app_id.clone(),
        amount: 5000,
        description: Some("be-t02 zero-delay consume after clock advance".to_string()),
    };
    let result = ctx
        .app_state
        .points_service
        .consume_points(identity, &realm_id, input)
        .await
        .expect("consume after clock advance must succeed — row is now available");
    assert_eq!(
        result.len(),
        1,
        "single-bucket consume produces one per-bucket transaction"
    );

    // Row fully consumed — availability predicate flipped true → row was
    // selected → ledger used_amount updated.
    let ledger = get_ledger_by_id(ctx, ledger_id).await;
    assert_eq!(ledger.used_amount, 5000);
    assert_eq!(ledger.remaining_amount, 0);
}

// ----------------------------------------------------------------------------
// Scenario §6.1 #5: immediate availability when effective_at IS NULL
// ----------------------------------------------------------------------------

// User Story: US-PU-001 (view my balance) — zero regression for every existing
// ledger row, which by construction has `effective_at = NULL` (BE-D01
// additive migration, no backfill).
//
// Covers design §6.1 P0 "立即可用语义" + §6.3 risk "派生余额替代 Stored 列读
// 取：effective_at=NULL 时派生 SUM 与原 Stored 口径逐分桶一致".
//
// Why this test exists: the production default for `effective_at` is NULL
// (column added nullable, no backfill, A6). Every existing row must remain
// immediately available — `effective_at IS NULL OR effective_at <= NOW()`
// must short-circuit true on NULL. If a future change accidentally made
// NULL mean "pending" (e.g. tightening the predicate to require a non-NULL
// value), this test fails immediately.
#[test_context(TestContext)]
#[tokio::test]
async fn test_immediate_available_when_effective_at_null(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id =
        create_test_user(&ctx.app_state.pool, &realm_id, "be-t02-immediate@exam.com").await;

    // Three rows across two credit types, ALL with effective_at = NULL.
    let _sub = create_credit_ledger_entry_with_effective_at(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        CreditSourceType::SubscriptionInitial,
        format!("be-t02-imm-sub-{}", Uuid::now_v7()),
        4000,
        None,
        None, // ← NULL: immediately available
    )
    .await;
    let _topup = create_credit_ledger_entry_with_effective_at(
        ctx,
        user_id,
        &realm_id,
        CreditType::TopupCredit,
        CreditSourceType::Topup,
        format!("be-t02-imm-topup-{}", Uuid::now_v7()),
        1500,
        None,
        None,
    )
    .await;
    let _reg = create_credit_ledger_entry_with_effective_at(
        ctx,
        user_id,
        &realm_id,
        CreditType::RegistrationCredit,
        CreditSourceType::Registration,
        format!("be-t02-imm-reg-{}", Uuid::now_v7()),
        500,
        None,
        None,
    )
    .await;

    // NULL effective_at → every row is in the available set, immediately.
    assert_derived_balance(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        4000,
    )
    .await;
    assert_derived_balance(ctx, user_id, &realm_id, CreditType::TopupCredit, 1500).await;
    assert_derived_balance(ctx, user_id, &realm_id, CreditType::RegistrationCredit, 500).await;

    // Total derived balance = sum of all immediately-available rows.
    assert_eq!(
        get_derived_total_balance(ctx, user_id, &realm_id).await,
        4000 + 1500 + 500,
        "total derived available balance must equal sum of all effective_at=NULL rows"
    );

    // And all 4000 subscription credits are immediately consumable.
    let identity = create_test_third_party_identity(&realm_id);
    let input = ConsumePointsInput {
        user_id: user_id.to_string(),
        client_app_id: ctx._client_app_id.clone(),
        amount: 4000,
        description: Some("be-t02 immediate-availability consume".to_string()),
    };
    let result = ctx
        .app_state
        .points_service
        .consume_points(identity, &realm_id, input)
        .await
        .expect("consume must succeed — effective_at=NULL rows are immediately available");

    // After consume, subscription pool drops to 0; other pools untouched.
    assert_derived_balance(ctx, user_id, &realm_id, CreditType::SubscriptionCredit, 0).await;
    assert_derived_balance(ctx, user_id, &realm_id, CreditType::TopupCredit, 1500).await;
    assert_derived_balance(ctx, user_id, &realm_id, CreditType::RegistrationCredit, 500).await;
    // sanity: result had exactly one per-bucket transaction
    assert_eq!(result.len(), 1);
}
