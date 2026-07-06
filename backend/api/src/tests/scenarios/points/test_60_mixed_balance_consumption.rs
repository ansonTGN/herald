// =============================================================================
// Test: Mixed Balance Consumption
// =============================================================================
//
// Tests for consuming points when user has both subscription and topup credits.
// Subscription credit lives in `points_quota_entitlements` (window-quota model);
// topup credit lives in `points_credit_ledger` (pool model). Consumption follows
// the window-first + overflow-to-pool strategy.
//
// User Story: docs/user-stories/points-billing-events.md
// Covers: US-BI-013 (Mixed balance consumption with FIFO priority)
//
// =============================================================================

use crate::tests::helpers::points_helpers::*;
use crate::tests::scenarios::points::fixtures::*;
use crate::tests::schema_test_context::SchemaTestContext;
use chrono::{Duration, Utc};
use herald_core::domain::common::entities::app_errors::CoreError;
use herald_core::domain::points::dtos::ConsumePointsInput;
use herald_core::domain::points::entities::{CreditSourceType, CreditType, QuotaSourceType};
use test_context::test_context;
use uuid::Uuid;

// ============================================================================
// Test 1: Mixed Balance FIFO Consumption
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-BI-013 场景 1 - 混合余额消费优先级 (FIFO)
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_mixed_balance_fifo_consumption(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user@example.com").await;
    let plan_id = Uuid::now_v7();

    create_points_wallet(ctx, user_id, &realm_id).await;
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;

    // Grant 5000 subscription credits via a window-quota entitlement.
    grant_quota_entitlement_for_test(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
        QuotaSourceType::SubscriptionInitial,
        &plan_id.to_string(),
        &[(86_400, 5_000, "day")],
        Utc::now() - Duration::hours(1),
        Some(Utc::now() + Duration::days(30)),
    )
    .await;

    // Grant 3000 topup credits as a ledger pool row.
    let topup_ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::TopupCredit,
        CreditSourceType::Topup,
        Uuid::now_v7().to_string(),
        3000,
        None,
    )
    .await;

    // When: Consume 6000 points
    let identity = create_test_third_party_identity(&realm_id);
    let input = ConsumePointsInput {
        user_id: user_id.to_string(),
        client_app_id: ctx._client_app_id.clone(),
        amount: 6000,
        description: Some("test_consumption".to_string()),
    };
    let result = ctx
        .app_state
        .points_service
        .consume_points(identity, &realm_id, input)
        .await;

    // Then: Should succeed
    assert!(result.is_ok(), "Should successfully consume 6000 points");
    let transactions = result.unwrap();
    let total_consumed: i64 = transactions.iter().map(|t| -t.amount).sum();
    assert_eq!(total_consumed, 6000, "Total consumed across all buckets");

    // Verify subscription window is exhausted.
    assert_derived_balance(ctx, user_id, &realm_id, CreditType::SubscriptionCredit, 0).await;

    // A subscription-credit consume transaction of 5000 was recorded.
    let sub_consume_amount: Option<i64> = sqlx::query_scalar(
        "SELECT amount FROM points_transactions \
         WHERE user_id = $1 AND realm_id = $2 \
           AND credit_type = 'subscription_credit' \
           AND type = 'consume' \
         LIMIT 1",
    )
    .bind(user_id)
    .bind(&realm_id)
    .fetch_optional(&ctx.app_state.pool)
    .await
    .expect("Failed to query subscription consume transaction");
    assert_eq!(
        sub_consume_amount,
        Some(-5000),
        "Subscription window consume transaction amount is -5000"
    );

    // Verify topup credits partially consumed (1000 used, 2000 remaining).
    let topup_ledger = get_ledger_by_id(ctx, topup_ledger_id).await;
    assert_eq!(
        topup_ledger.remaining_amount, 2000,
        "2000 topup credits remaining"
    );
    assert_eq!(topup_ledger.used_amount, 1000, "1000 topup credits used");

    // Verify consumption allocations. Window consumption does not write
    // allocations, so at minimum the topup ledger allocation must exist.
    let allocations = get_consumption_allocations(ctx, user_id).await;
    let topup_allocated: i64 = allocations
        .iter()
        .filter(|a| a.ledger_id == topup_ledger_id)
        .map(|a| a.allocated_amount)
        .sum();
    assert_eq!(
        topup_allocated, 1000,
        "Topup ledger allocation should be 1000"
    );

    // If the implementation also records an allocation row for the window
    // side, the two allocations must sum to the full consume amount.
    if allocations.len() == 2 {
        let total_allocated: i64 = allocations.iter().map(|a| a.allocated_amount).sum();
        assert_eq!(
            total_allocated, 6000,
            "Window + topup allocations should sum to 6000"
        );
    } else {
        assert_eq!(
            allocations.len(),
            1,
            "Only the topup ledger should have an allocation when window rows do not"
        );
    }
}

// ============================================================================
// Test 3: Mixed Balance Insufficient Funds
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-BI-013 场景 3 - 混合余额不足场景
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_mixed_balance_insufficient_funds(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user@example.com").await;
    let plan_id = Uuid::now_v7();

    create_points_wallet(ctx, user_id, &realm_id).await;
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;

    // Grant 5000 subscription credits via window-quota entitlement.
    grant_quota_entitlement_for_test(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
        QuotaSourceType::SubscriptionInitial,
        &plan_id.to_string(),
        &[(86_400, 5_000, "day")],
        Utc::now() - Duration::hours(1),
        Some(Utc::now() + Duration::days(30)),
    )
    .await;

    // Grant 3000 topup credits as a ledger pool row.
    let topup_ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::TopupCredit,
        CreditSourceType::Topup,
        Uuid::now_v7().to_string(),
        3000,
        None,
    )
    .await;

    // When: Try to consume 10000 points (exceeds total balance of 8000)
    let identity = create_test_third_party_identity(&realm_id);
    let input = ConsumePointsInput {
        user_id: user_id.to_string(),
        client_app_id: ctx._client_app_id.clone(),
        amount: 10000,
        description: Some("test_consumption".to_string()),
    };
    let result = ctx
        .app_state
        .points_service
        .consume_points(identity, &realm_id, input)
        .await;

    // Then: Should fail with insufficient balance error
    assert!(result.is_err(), "Should fail when exceeding total balance");

    let error = result.unwrap_err();
    match error {
        CoreError::BadRequest(msg) if msg.contains("Insufficient points balance") => {
            assert!(
                msg.contains("10000"),
                "Error should mention requested amount 10000"
            );
            assert!(
                msg.contains("8000"),
                "Error should mention available amount 8000"
            );
        }
        _ => panic!(
            "Expected BadRequest with insufficient points message, got: {:?}",
            error
        ),
    }

    // Verify subscription window unchanged.
    assert_derived_balance(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        5000,
    )
    .await;

    // Verify topup ledger unchanged.
    let topup_ledger = get_ledger_by_id(ctx, topup_ledger_id).await;
    assert_eq!(
        topup_ledger.remaining_amount, 3000,
        "Topup credits unchanged"
    );
    assert_eq!(topup_ledger.used_amount, 0, "No consumption occurred");

    // No allocations should be created
    let allocations = get_consumption_allocations(ctx, user_id).await;
    assert_eq!(
        allocations.len(),
        0,
        "No allocations for failed consumption"
    );
}

// ============================================================================
// Test 4: Multiple Topup Ledgers FIFO Priority
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-BI-013 场景 - 多个 topup ledger 按 FIFO 消费
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_mixed_balance_multiple_topup_ledgers(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user@example.com").await;
    let plan_id = Uuid::now_v7();

    create_points_wallet(ctx, user_id, &realm_id).await;
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;

    // Grant 3000 subscription credits via window-quota entitlement.
    grant_quota_entitlement_for_test(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
        QuotaSourceType::SubscriptionInitial,
        &plan_id.to_string(),
        &[(86_400, 3_000, "day")],
        Utc::now() - Duration::hours(1),
        Some(Utc::now() + Duration::days(30)),
    )
    .await;

    // Grant first topup: 2000
    let topup1_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::TopupCredit,
        CreditSourceType::Topup,
        Uuid::now_v7().to_string(),
        2000,
        None,
    )
    .await;

    // Grant second topup: 3000
    let topup2_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::TopupCredit,
        CreditSourceType::Topup,
        Uuid::now_v7().to_string(),
        3000,
        None,
    )
    .await;

    // When: Consume 7000 points
    let identity = create_test_third_party_identity(&realm_id);
    let input = ConsumePointsInput {
        user_id: user_id.to_string(),
        client_app_id: ctx._client_app_id.clone(),
        amount: 7000,
        description: Some("test_consumption".to_string()),
    };
    let result = ctx
        .app_state
        .points_service
        .consume_points(identity, &realm_id, input)
        .await;

    // Then: Should succeed
    assert!(result.is_ok());

    // Verify subscription window is exhausted.
    assert_derived_balance(ctx, user_id, &realm_id, CreditType::SubscriptionCredit, 0).await;

    // Verify first topup fully consumed
    let topup1 = get_ledger_by_id(ctx, topup1_id).await;
    assert_eq!(topup1.remaining_amount, 0);
    assert_eq!(topup1.used_amount, 2000);

    // Verify second topup partially consumed (2000 used, 1000 remaining)
    let topup2 = get_ledger_by_id(ctx, topup2_id).await;
    assert_eq!(topup2.remaining_amount, 1000);
    assert_eq!(topup2.used_amount, 2000);

    // Verify consumption allocations reflect FIFO topup order.
    let allocations = get_consumption_allocations(ctx, user_id).await;
    let topup1_allocated: i64 = allocations
        .iter()
        .filter(|a| a.ledger_id == topup1_id)
        .map(|a| a.allocated_amount)
        .sum();
    let topup2_allocated: i64 = allocations
        .iter()
        .filter(|a| a.ledger_id == topup2_id)
        .map(|a| a.allocated_amount)
        .sum();
    assert_eq!(topup1_allocated, 2000);
    assert_eq!(topup2_allocated, 2000);
}

// ============================================================================
// Test: derived available balance == consumable amount (point-time P0)
// ============================================================================
//
// User Story: US-PU-001 (view my balance) / US-PU-004 (consume credits).
//
// Why this test exists: the derived available balance and the consumption
// selection predicate must agree. Under the window-quota model subscription
// credit is no longer a ledger row, so we seed it as a quota entitlement.
// Topup/granted rows still live in `points_credit_ledger` and are governed by
// the pool predicate (status='active', remaining_amount>0, not expired).
//
// Invariant: the balance you see must equal the balance you can spend.
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_derived_equals_consumable(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id =
        create_test_user(&ctx.app_state.pool, &realm_id, "be-t02-derived-eq@exam.com").await;

    create_points_wallet(ctx, user_id, &realm_id).await;
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;

    // (a) Immediately-available subscription_credit — 5000 in derived balance.
    grant_quota_entitlement_for_test(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
        QuotaSourceType::SubscriptionInitial,
        "be-t02-de-sub",
        &[(86_400, 5_000, "day")],
        Utc::now() - Duration::hours(1),
        Some(Utc::now() + Duration::days(30)),
    )
    .await;

    // (b) Expired topup_credit — excluded by expires_at gate.
    let expired_id = create_credit_ledger_entry_with_effective_at(
        ctx,
        user_id,
        &realm_id,
        CreditType::TopupCredit,
        CreditSourceType::Topup,
        format!("be-t02-de-expired-{}", Uuid::now_v7()),
        2000,
        Some(Utc::now() - Duration::days(1)), // expires_at < NOW → excluded
        None,
    )
    .await;

    // (c) Fully-consumed granted_credit — excluded by remaining_amount > 0 gate.
    //     granted_amount = used_amount → remaining_amount = 0.
    let used_up_id = {
        let id = create_credit_ledger_entry_with_effective_at(
            ctx,
            user_id,
            &realm_id,
            CreditType::GrantedCredit,
            CreditSourceType::AdminGrant,
            format!("be-t02-de-usedup-{}", Uuid::now_v7()),
            1000,
            None,
            None,
        )
        .await;
        sqlx::query(
            "UPDATE points_credit_ledger \
             SET used_amount = granted_amount, updated_at = NOW() \
             WHERE id = $1",
        )
        .bind(id)
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to mark granted_credit row as fully used");
        id
    };

    // === Per-pool derived balance assertions: only (a) is in the available set. ===
    assert_eq!(
        get_derived_balance_by_credit_type(ctx, user_id, &realm_id, CreditType::SubscriptionCredit)
            .await,
        5000,
        "(a) immediately-available subscription_credit contributes 5000"
    );
    assert_eq!(
        get_derived_balance_by_credit_type(ctx, user_id, &realm_id, CreditType::TopupCredit).await,
        0,
        "(b) expired topup_credit contributes 0"
    );
    assert_eq!(
        get_derived_balance_by_credit_type(ctx, user_id, &realm_id, CreditType::GrantedCredit)
            .await,
        0,
        "(c) fully-used granted_credit contributes 0 (remaining_amount=0)"
    );
    assert_eq!(
        get_derived_total_balance(ctx, user_id, &realm_id).await,
        5000,
        "total derived available balance = 5000 (only (a))"
    );

    // === Consume exactly the available 5000; only the subscription window may be drawn down. ===
    let identity = create_test_third_party_identity(&realm_id);
    let input = ConsumePointsInput {
        user_id: user_id.to_string(),
        client_app_id: ctx._client_app_id.clone(),
        amount: 5000,
        description: Some("be-t02 derived-equals-consumable consume".to_string()),
    };
    let result = ctx
        .app_state
        .points_service
        .consume_points(identity, &realm_id, input)
        .await
        .expect("consume must succeed — 5000 available matches the derived balance");

    assert_eq!(
        result.iter().map(|t| -t.amount).sum::<i64>(),
        5000,
        "total consumed equals the derived available balance"
    );

    // (a) fully consumed.
    assert_derived_balance(ctx, user_id, &realm_id, CreditType::SubscriptionCredit, 0).await;

    // (b) expired untouched.
    let expired = get_ledger_by_id(ctx, expired_id).await;
    assert_eq!(expired.used_amount, 0, "expired row must not be consumed");
    assert_eq!(expired.remaining_amount, 2000);

    // (c) fully-used untouched (was already 0 remaining).
    let used_up = get_ledger_by_id(ctx, used_up_id).await;
    assert_eq!(used_up.remaining_amount, 0);
    assert_eq!(used_up.used_amount, 1000);

    // Post-consume derived balances: everything is 0.
    assert_eq!(
        get_derived_balance_by_credit_type(ctx, user_id, &realm_id, CreditType::SubscriptionCredit)
            .await,
        0
    );
    assert_eq!(
        get_derived_total_balance(ctx, user_id, &realm_id).await,
        0,
        "after consuming the only available balance, derived total is 0"
    );
}
