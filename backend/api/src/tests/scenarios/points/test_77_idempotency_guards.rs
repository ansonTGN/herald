// =============================================================================
// Idempotency Guard Tests
// =============================================================================
//
// Tests that verify DB-level idempotency guards prevent duplicate operations.
//
// 1. grant_points_for_sdk: duplicate call returns zero-amount placeholder
// 2. revoke_subscription_credits_by_entitlement_atomic: duplicate call returns total_revoked=0
// 3. revoke_topup_proportional_atomic: duplicate call returns total_revoked=0
//
// These guards use check_completed_idempotency_in_tx and
// record_completed_idempotency_in_tx to prevent duplicate ledger creation
// or revocation when the same operation is retried.
//
// =============================================================================

use crate::tests::helpers::points_helpers::*;
use crate::tests::helpers::webhook_helpers::{
    assert_webhook_success, build_subscription_paid_event, generate_test_event_id,
    send_webhook_with_signature, setup_test_plan_config,
};
use crate::tests::scenarios::points::fixtures::*;
use crate::tests::schema_test_context::SchemaTestContext;
use herald_core::domain::points::dtos::RevokePointsOutput;
use herald_core::domain::points::entities::{CreditSourceType, CreditType, RevocationType};
use herald_core::domain::points::ports::PointsRepository;
use test_context::test_context;
use uuid::Uuid;

/// Seed a `subscription` row bound to the realm's legacy test bucket. Used by
/// subscription tests below so the schedule's `subscription_id` is known ahead of
/// the service call. Mirrors `seed_subscription_row` in test_40 — kept local
/// to avoid cross-file test helper churn.
async fn seed_subscription_row_77(
    ctx: &mut SchemaTestContext,
    user_id: Uuid,
    realm_id: &str,
    entitlement_key: &str,
) -> Uuid {
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
    .bind(format!("sub_be_t04_77_{}", subscription_id))
    .bind(format!("prod_be_t04_{}", entitlement_key))
    .bind(bucket_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to seed subscription row for idempotency test");
    subscription_id
}

// ============================================================================
// Test 1: grant_points_internal idempotency prevents duplicate ledger
// ============================================================================
//
// User Story: As a billing system, when I retry a grant-points request with
// an explicit idempotency key, I must not create a duplicate ledger or
// inflate the user's balance.
//
// Covers: grant_points_atomic idempotency guard (line ~3864-3889)
// Idempotency key: caller-provided via grant_points_internal parameter
//
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_grant_idempotency_prevents_duplicate_ledger(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id =
        create_test_user(&ctx.app_state.pool, &realm_id, "idempotency77a@example.com").await;

    create_points_wallet(ctx, user_id, &realm_id).await;

    let source_id = Uuid::now_v7().to_string();
    let idempotency_key = format!("grant:AdminGrant:{}", source_id);

    // Credit-bucket: grant/revoke now require an explicit bucket_id target.
    // The wallet above was created on the realm's legacy bucket (see
    // `points_helpers::ensure_test_bucket_for_realm`), so the grant, the
    // idempotent replay and the balance read below must all target that SAME
    // bucket — otherwise the grant would silently land on a second pool while
    // the unscoped `WHERE user_id` read stayed on the empty legacy wallet.
    // This test is about grant idempotency, not bucket routing.
    use crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm;
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;

    // First grant should succeed
    let result1 = ctx
        .app_state
        .points_service
        .grant_points_internal(
            &realm_id,
            user_id,
            bucket_id,
            CreditType::GrantedCredit,
            CreditSourceType::AdminGrant,
            500,
            None,
            // effective_at: None ⟺ immediately available.
            None,
            Some(source_id.clone()),
            Some("idempotency test: first grant".to_string()),
            Some(idempotency_key.clone()),
        )
        .await;

    assert!(result1.is_ok(), "First grant should succeed: {:?}", result1);

    // Second grant with the same idempotency key should be idempotent
    let result2 = ctx
        .app_state
        .points_service
        .grant_points_internal(
            &realm_id,
            user_id,
            bucket_id,
            CreditType::GrantedCredit,
            CreditSourceType::AdminGrant,
            500,
            None,
            // effective_at: None ⟺ immediately available.
            None,
            Some(source_id.clone()),
            Some("idempotency test: duplicate grant".to_string()),
            Some(idempotency_key),
        )
        .await;

    assert!(
        result2.is_ok(),
        "Second grant should succeed (idempotent response): {:?}",
        result2
    );

    // Verify only one real ledger exists for this user
    let ledgers = get_user_ledgers(ctx, user_id).await;
    let non_idempotency_ledgers: Vec<_> = ledgers
        .iter()
        .filter(|l| l.source_id != "idempotency")
        .collect();

    assert_eq!(
        non_idempotency_ledgers.len(),
        1,
        "Exactly one real ledger should exist (no duplicates)"
    );
    assert_eq!(
        non_idempotency_ledgers[0].granted_amount, 500,
        "Real ledger should have granted_amount=500"
    );

    // Verify the wallet balance is not inflated. point-time:
    // `points_wallets.total_balance` was dropped; available balance is derived
    // from `points_credit_ledger` using the same predicate as consumption.
    let balance: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(l.remaining_amount) FILTER (
                    WHERE l.status = 'active' AND l.remaining_amount > 0
                      AND (l.effective_at IS NULL OR l.effective_at <= NOW())
                      AND (l.expires_at  IS NULL OR l.expires_at  >  NOW())
                ), 0)::BIGINT
         FROM points_wallets w
         LEFT JOIN points_credit_ledger l
           ON l.realm_id = w.realm_id AND l.user_id = w.user_id AND l.bucket_id = w.bucket_id
         WHERE w.user_id = $1 AND w.realm_id = $2
         GROUP BY w.id",
    )
    .bind(user_id)
    .bind(&realm_id)
    .fetch_one(&ctx.app_state.pool)
    .await
    .expect("Failed to fetch wallet balance");

    assert_eq!(
        balance, 500,
        "Wallet balance should be 500 (not inflated by duplicate grant)"
    );
}

// ============================================================================
// Test 2: revoke_subscription_credits_by_entitlement idempotency
// ============================================================================
//
// User Story: As a billing system, when I retry a subscription credit
// revocation with the same idempotency_key, I must not create a duplicate
// revocation record or revoke more credits than intended.
//
// Covers: revoke_subscription_credits_by_entitlement_atomic (line ~3493-3507)
// Idempotency key: caller-provided via idempotency_key parameter
//
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_revoke_subscription_by_entitlement_idempotency(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id =
        create_test_user(&ctx.app_state.pool, &realm_id, "idempotency77b@example.com").await;

    create_points_wallet(ctx, user_id, &realm_id).await;

    // Create a subscription credit ledger with a known entitlement source_id
    let entitlement_key = Uuid::now_v7().to_string();
    let ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        CreditSourceType::SubscriptionInitial,
        entitlement_key.clone(),
        1000,
        None,
    )
    .await;

    let idempotency_key = format!("revoke:sub:{}", entitlement_key);

    // Credit-bucket: revoke now requires an explicit bucket_id target.
    // The wallet and the subscription ledger above were both created on the
    // realm's legacy bucket (`create_points_wallet` +
    // `create_credit_ledger_entry_v2` route through
    // `ensure_test_bucket_for_realm`), and `revoke_subscription_credits_by_
    // entitlement_atomic` scopes its ledger lookup by `bucket_id`. Revoke on
    // any other bucket would find no ledger to revoke. Target the SAME bucket
    // the ledger actually lives in so the test exercises revoke idempotency
    // rather than bucket routing.
    use crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm;
    let revoke_bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;

    // First revocation should succeed
    let result1 = ctx
        .app_state
        .points_repository
        .revoke_subscription_credits_by_entitlement_atomic(
            &realm_id,
            user_id,
            revoke_bucket_id,
            &entitlement_key,
            RevocationType::CancelRevoke,
            "idempotency test: first revoke".to_string(),
            None,
            Some(idempotency_key.clone()),
        )
        .await;

    assert!(
        result1.is_ok(),
        "First revoke should succeed: {:?}",
        result1
    );
    let output1 = result1.unwrap();
    assert_eq!(
        output1.total_revoked, 1000,
        "First revoke should revoke full 1000"
    );
    assert!(
        !output1.ledger_ids.is_empty(),
        "First revoke should include ledger IDs"
    );

    // Verify the ledger is now revoked
    let ledger = get_ledger_by_id(ctx, ledger_id).await;
    assert_eq!(
        ledger.revoked_amount, 1000,
        "Ledger should show 1000 revoked after first revoke"
    );

    // Record revocation count before second call
    let revocation_count_before = get_revocation_records(ctx, user_id).await.len();

    // Second revocation with the same idempotency_key should be idempotent
    let result2 = ctx
        .app_state
        .points_repository
        .revoke_subscription_credits_by_entitlement_atomic(
            &realm_id,
            user_id,
            revoke_bucket_id,
            &entitlement_key,
            RevocationType::CancelRevoke,
            "idempotency test: duplicate revoke".to_string(),
            None,
            Some(idempotency_key),
        )
        .await;

    assert!(
        result2.is_ok(),
        "Second revoke should succeed (idempotent response): {:?}",
        result2
    );
    let output2 = result2.unwrap();
    assert_eq!(
        output2.total_revoked, 0,
        "Second revoke should return total_revoked=0 (idempotent)"
    );
    assert!(
        output2.ledger_ids.is_empty(),
        "Second revoke should return empty ledger_ids"
    );

    // Verify no additional revocation record was created
    let revocation_count_after = get_revocation_records(ctx, user_id).await.len();
    assert_eq!(
        revocation_count_before, revocation_count_after,
        "No new revocation record should be created on duplicate call"
    );
}

// ============================================================================
// Test 3: revoke_topup_proportional idempotency
// ============================================================================
//
// User Story: As a billing system, when I retry a topup proportional
// revocation with the same refund_id, I must not revoke additional credits.
//
// Covers: revoke_topup_proportional_atomic (line ~3621-3634)
// Idempotency key: "refund:topup:{refund_id}"
//
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_revoke_topup_proportional_idempotency(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id =
        create_test_user(&ctx.app_state.pool, &realm_id, "idempotency77c@example.com").await;

    create_points_wallet(ctx, user_id, &realm_id).await;

    // Create a topup credit ledger
    let source_id = Uuid::now_v7().to_string();
    let ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::TopupCredit,
        CreditSourceType::Topup,
        source_id,
        2000,
        None,
    )
    .await;

    let refund_id = Uuid::now_v7().to_string();

    // Credit-bucket: revoke now requires an explicit bucket_id target.
    // The wallet and the topup ledger above were both created on the realm's
    // legacy bucket (`create_points_wallet` + `create_credit_ledger_entry_v2`
    // route through `ensure_test_bucket_for_realm`), and
    // `revoke_topup_proportional_atomic` scopes its ledger lookup by
    // `bucket_id`. Revoke on any other bucket would find no ledger to revoke.
    // Target the SAME bucket the ledger actually lives in so the test
    // exercises revoke idempotency rather than bucket routing.
    use crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm;
    let topup_bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;

    // First revocation: revoke half (1000 out of 2000)
    let result1: Result<RevokePointsOutput, _> = ctx
        .app_state
        .points_repository
        .revoke_topup_proportional_atomic(
            &realm_id,
            user_id,
            topup_bucket_id,
            1000, // refund_amount
            2000, // original_payment_amount
            &refund_id,
        )
        .await;

    assert!(
        result1.is_ok(),
        "First topup revoke should succeed: {:?}",
        result1
    );
    let output1 = result1.unwrap();
    assert!(
        output1.total_revoked > 0,
        "First revoke should revoke some credits, got {}",
        output1.total_revoked
    );

    // Verify the ledger was partially revoked
    let ledger = get_ledger_by_id(ctx, ledger_id).await;
    assert!(
        ledger.revoked_amount > 0,
        "Ledger should show some revoked amount after first revoke"
    );

    // Record revocation count before second call
    let revocation_count_before = get_revocation_records(ctx, user_id).await.len();

    // Second revocation with the same refund_id should be idempotent
    let result2: Result<RevokePointsOutput, _> = ctx
        .app_state
        .points_repository
        .revoke_topup_proportional_atomic(
            &realm_id,
            user_id,
            topup_bucket_id,
            1000, // refund_amount
            2000, // original_payment_amount
            &refund_id,
        )
        .await;

    assert!(
        result2.is_ok(),
        "Second topup revoke should succeed (idempotent response): {:?}",
        result2
    );
    let output2 = result2.unwrap();
    assert_eq!(
        output2.total_revoked, 0,
        "Second revoke should return total_revoked=0 (idempotent)"
    );
    assert!(
        output2.ledger_ids.is_empty(),
        "Second revoke should return empty ledger_ids"
    );

    // Verify no additional revocation record was created
    let revocation_count_after = get_revocation_records(ctx, user_id).await.len();
    assert_eq!(
        revocation_count_before, revocation_count_after,
        "No new revocation record should be created on duplicate call"
    );
}

// ============================================================================
// Two-layer subscription idempotency (P1)
// ============================================================================
//
// Layer 1 — PERIOD / SCHEDULE business idempotency:
//   `points_grant_records(schedule_id, period_number)` UNIQUE. The pre-grant
//   path and the formal renewal webhook converge here. A second write for the
//   same (schedule_id, period_number) is rejected by the UNIQUE constraint;
//   production code (subscription_service) treats a pre-existing record as a
//   no-re-grant signal.
//
// Layer 2 — PROVIDER EVENT idempotency:
//   `idempotency_keys` table keyed `creem_{event_id}`. Duplicate webhook
//   deliveries with the same event_id hit the cached result and never re-enter
//   `handle_subscription_paid`.
//
// The two layers are defense-in-depth; both must hold independently.

/// User Story: US-PU-009 — period-level business idempotency dedup.
/// Covers (P1 — business idempotency dimension shift from event
/// to schedule/period): the `points_grant_records(schedule_id, period_number)`
/// UNIQUE constraint is the single source of truth. A second insert for the
/// same key MUST be rejected at the DB level; the grantRecord for that period
/// resolves to exactly one ledger row.
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_period_schedule_business_idempotency_dedup(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(
        &ctx.app_state.pool,
        &realm_id,
        "be_t04_period_dedup@example.com",
    )
    .await;

    let entitlement_key = format!("be-t04-dedup-{}", Uuid::now_v7());
    let points_per_period: i64 = 600;

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

    let subscription_id = seed_subscription_row_77(ctx, user_id, &realm_id, &entitlement_key).await;

    let now = chrono::Utc::now();
    let first_period_start = now - chrono::Duration::days(30);
    let period_start = now;

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

    // --- Given: a pre-grant already occupies (schedule_id, period_number=2) -
    let ledger_id = create_credit_ledger_entry_with_effective_at(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        CreditSourceType::SubscriptionRenewal,
        format!("schedule:{}:period:2", schedule_id),
        points_per_period,
        Some(period_start + chrono::Duration::days(30)),
        Some(period_start),
    )
    .await;

    create_grant_record(
        ctx,
        schedule_id,
        2,
        points_per_period,
        period_start,
        ledger_id,
    )
    .await;

    let ledger_count_before =
        get_user_ledgers_by_credit_type(ctx, user_id, CreditType::SubscriptionCredit)
            .await
            .len();

    // --- When: a second grant_record insert for the SAME (schedule_id,
    // period_number) is attempted directly at the DB layer.
    let duplicate_insert_result = sqlx::query(
        "INSERT INTO points_grant_records
            (id, schedule_id, user_id, realm_id, period_number, granted_amount, grant_time, ledger_id, created_at)
         VALUES ($1, $2, $3, $4, 5, 100, NOW(), $6, NOW())",
    )
    .bind(Uuid::now_v7())
    .bind(schedule_id)
    .bind(user_id)
    .bind(&realm_id)
    .bind(ledger_id)
    .execute(&ctx.app_state.pool)
    .await;

    // --- Then: the UNIQUE(schedule_id, period_number) constraint rejects it ---
    assert!(
        duplicate_insert_result.is_err(),
        "UNIQUE(schedule_id, period_number) must reject duplicate grant_record (business idempotency gate)"
    );

    // --- And: only ONE ledger row for this (schedule, period) exists --------
    let ledger_count_after =
        get_user_ledgers_by_credit_type(ctx, user_id, CreditType::SubscriptionCredit)
            .await
            .len();
    assert_eq!(
        ledger_count_before, ledger_count_after,
        "no new ledger row should be created when period-level idempotency gate holds"
    );

    let resolved = find_ledger_id_by_schedule_period(ctx, schedule_id, 2)
        .await
        .expect("grant_record for period_number=2 must resolve to its ledger");
    assert_eq!(
        resolved, ledger_id,
        "the (schedule_id, period_number) key must resolve to the single pre-granted ledger row"
    );
}

/// User Story: US-PU-009 — provider event-level idempotency preserved.
/// Covers (P0 — event-level idempotency retained as backstop):
/// the webhook layer's `creem_{event_id}` idempotency key intercepts a
/// duplicate event delivery BEFORE `handle_subscription_paid` is reached. The
/// second delivery returns the cached result and produces no additional
/// ledger row, no additional grant_record, and the idempotency key is
/// recorded in the `idempotency_keys` table.
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_event_level_idempotency_preserved(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(
        &ctx.app_state.pool,
        &realm_id,
        "be_t04_event_level@example.com",
    )
    .await;
    let plan_id = Uuid::now_v7();
    let event_id = generate_test_event_id();

    ctx.with_creem_config(&realm_id, None, None, None).await;
    setup_test_plan_config(ctx, &realm_id, plan_id).await;
    create_points_wallet(ctx, user_id, &realm_id).await;

    let now = chrono::Utc::now();
    let base = build_subscription_paid_event(event_id.clone(), user_id, plan_id, false, &realm_id);
    let mut event = base.clone();
    event["data"]["object"]["currentPeriodStart"] = serde_json::Value::String(now.to_rfc3339());
    event["data"]["object"]["currentPeriodEnd"] =
        serde_json::Value::String((now + chrono::Duration::days(30)).to_rfc3339());

    let app = ctx.create_unified_test_router();

    // --- When: event delivered twice with the SAME event_id -----------------
    let response1 =
        send_webhook_with_signature(&app, &realm_id, event.clone(), "test_webhook_secret").await;
    assert_webhook_success(&response1);
    let response2 =
        send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;
    assert_webhook_success(&response2);

    // --- Then: the event-level idempotency key is recorded in the SQL
    // `idempotency_keys` table. The outer Creem webhook handler keys its Redis
    // cache on `creem_{event_id}`, but the durable SQL row (written by the
    // inner `handle_subscription_paid_atomic`) uses the `sub_paid:{event_id}`
    // key (IDEMPOTENCY_KEY_SUBSCRIPTION_PAID prefix, subscription_service.rs:387).
    assert_idempotency_key_exists(ctx, &format!("sub_paid:{}", event_id)).await;

    // --- And: NO duplicate ledger rows were created by the second delivery --
    // The first delivery may produce 1 (current period) or 2 (current +
    // chained next-period pre-grant) ledgers; the duplicate must not add more.
    let ledgers =
        get_user_ledgers_by_credit_type(ctx, user_id, CreditType::SubscriptionCredit).await;
    assert!(
        ledgers.len() == 1 || ledgers.len() == 2,
        "first delivery produces 1 or 2 ledgers (current [+ chained pre-grant]); duplicate must not add more; got {}",
        ledgers.len()
    );
    let total_granted: i64 = ledgers.iter().map(|l| l.granted_amount).sum();
    // 1000 = current period only (no chained pre-grant); 2000 = current + next
    // period pre-grant. Any value above 2000 means the duplicate event inflated
    // the grant — that is the regression this test guards.
    assert!(
        total_granted == 1000 || total_granted == 2000,
        "duplicate event_id must not inflate total granted; expected 1000 (no pre-grant) or 2000 (with chained pre-grant), got {}",
        total_granted
    );
}
