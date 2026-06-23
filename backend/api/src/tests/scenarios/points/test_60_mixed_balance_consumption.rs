// =============================================================================
// Test: Mixed Balance Consumption
// =============================================================================
//
// Tests for consuming points when user has both subscription and topup credits.
// Implements FIFO (First-In-First-Out) priority: subscription credits first.
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
use herald_core::domain::points::entities::{CreditSourceType, CreditType};
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

    // Grant 5000 subscription credits
    let sub_ledger_id = create_credit_ledger_entry_v2(
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

    // Grant 3000 topup credits
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
    let transaction = &result.unwrap()[0];
    assert_eq!(transaction.amount, -6000);

    // Verify subscription credits fully consumed (FIFO priority)
    let sub_ledger = get_ledger_by_id(ctx, sub_ledger_id).await;
    assert_eq!(
        sub_ledger.remaining_amount, 0,
        "Subscription credits fully consumed"
    );
    assert_eq!(
        sub_ledger.used_amount, 5000,
        "All 5000 subscription credits used"
    );

    // Verify topup credits partially consumed (1000 remaining)
    let topup_ledger = get_ledger_by_id(ctx, topup_ledger_id).await;
    assert_eq!(
        topup_ledger.remaining_amount, 2000,
        "2000 topup credits remaining"
    );
    assert_eq!(topup_ledger.used_amount, 1000, "1000 topup credits used");

    // Verify consumption allocations
    let allocations = get_consumption_allocations(ctx, user_id).await;
    assert_eq!(allocations.len(), 2, "Should have 2 allocation records");

    // First allocation: 5000 from subscription
    let sub_alloc = allocations
        .iter()
        .find(|a| a.ledger_id == sub_ledger_id)
        .unwrap();
    assert_eq!(sub_alloc.allocated_amount, 5000);

    // Second allocation: 1000 from topup
    let topup_alloc = allocations
        .iter()
        .find(|a| a.ledger_id == topup_ledger_id)
        .unwrap();
    assert_eq!(topup_alloc.allocated_amount, 1000);
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

    // Grant 5000 subscription credits
    let sub_ledger_id = create_credit_ledger_entry_v2(
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

    // Grant 3000 topup credits
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
            // Error message should contain details about insufficient balance
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

    // Verify balances unchanged
    let sub_ledger = get_ledger_by_id(ctx, sub_ledger_id).await;
    assert_eq!(
        sub_ledger.remaining_amount, 5000,
        "Subscription credits unchanged"
    );
    assert_eq!(sub_ledger.used_amount, 0, "No consumption occurred");

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

    // Grant 3000 subscription credits
    let sub_ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        CreditSourceType::SubscriptionInitial,
        plan_id.to_string(),
        3000,
        None,
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

    // Verify subscription credits fully consumed
    let sub_ledger = get_ledger_by_id(ctx, sub_ledger_id).await;
    assert_eq!(sub_ledger.remaining_amount, 0);
    assert_eq!(sub_ledger.used_amount, 3000);

    // Verify first topup fully consumed
    let topup1 = get_ledger_by_id(ctx, topup1_id).await;
    assert_eq!(topup1.remaining_amount, 0);
    assert_eq!(topup1.used_amount, 2000);

    // Verify second topup partially consumed (2000 used)
    let topup2 = get_ledger_by_id(ctx, topup2_id).await;
    assert_eq!(topup2.remaining_amount, 1000);
    assert_eq!(topup2.used_amount, 2000);

    // Verify 3 allocation records
    let allocations = get_consumption_allocations(ctx, user_id).await;
    assert_eq!(allocations.len(), 3);
}

// ============================================================================
// Test: derived available balance == consumable amount (point-time §6.1 P0)
// ============================================================================
//
// User Story: US-PU-001 (view my balance) / US-PU-004 (consume credits).
//
// Covers design §6.1 P0 "派生余额 = 可消费额" + §6.3 risk "派生余额替代
// Stored 列读取" / "消费可用性谓词增 effective_at 影响全场景".
//
// Why this test exists: the derived SUM (`compute_available_balance`) and the
// consumption selection predicate share the SAME filter
//   status='active' AND remaining_amount>0
//     AND (effective_at IS NULL OR effective_at <= NOW())
//     AND (expires_at  IS NULL OR expires_at  >  NOW())
// (design §5.1, BE-D04). "The balance you see" MUST equal "the balance you
// can spend". We construct a mixed wallet with four ledger rows —
// immediately-available, future-effective, expired, fully-consumed — then
// assert each row contributes 0 to the derived balance except the
// immediately-available one, and that a consume operation draws ONLY from
// the immediately-available row, leaving the other three untouched. If the
// two predicates ever diverged, this test would fail.
//
// Uses `create_credit_ledger_entry_with_effective_at` (not the v2 helper)
// because the v2 helper still UPDATEs the BE-D11-removed wallet Stored
// columns — we want the derived SUM to be the ONLY authority in this
// assertion, unmasked by any Stored write.
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_derived_equals_consumable(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id =
        create_test_user(&ctx.app_state.pool, &realm_id, "be-t02-derived-eq@exam.com").await;

    // (a) Immediately-available subscription_credit — 5000 in derived balance.
    let imm_id = create_credit_ledger_entry_with_effective_at(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        CreditSourceType::SubscriptionInitial,
        format!("be-t02-de-imm-{}", Uuid::now_v7()),
        5000,
        None,
        None, // effective_at NULL → available now
    )
    .await;

    // (b) Future-effective subscription_credit — excluded by effective_at gate.
    let future_id = create_credit_ledger_entry_with_effective_at(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        CreditSourceType::SubscriptionRenewal,
        format!("be-t02-de-future-{}", Uuid::now_v7()),
        3000,
        None,
        Some(Utc::now() + Duration::days(1)),
    )
    .await;

    // (c) Already-expired topup_credit — excluded by expires_at gate.
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

    // (d) Fully-consumed granted_credit — excluded by remaining_amount > 0 gate.
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
            "UPDATE points_credit_ledger
             SET used_amount = granted_amount, updated_at = NOW()
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
        "(a) immediately-available subscription_credit contributes 5000; (b) future-effective is excluded"
    );
    assert_eq!(
        get_derived_balance_by_credit_type(ctx, user_id, &realm_id, CreditType::TopupCredit).await,
        0,
        "(c) expired topup_credit contributes 0"
    );
    assert_eq!(
        get_derived_balance_by_credit_type(ctx, user_id, &realm_id, CreditType::GrantedCredit)
            .await,
        0,
        "(d) fully-used granted_credit contributes 0 (remaining_amount=0)"
    );
    assert_eq!(
        get_derived_total_balance(ctx, user_id, &realm_id).await,
        5000,
        "total derived available balance = 5000 (only (a))"
    );

    // === Consume exactly the available 5000; only row (a) may be drawn down. ===
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
        result.len(),
        1,
        "single-bucket consume produces exactly one per-bucket transaction"
    );

    // (a) fully consumed.
    let imm = get_ledger_by_id(ctx, imm_id).await;
    assert_eq!(imm.used_amount, 5000);
    assert_eq!(imm.remaining_amount, 0);

    // (b) future-effective untouched.
    let future = get_ledger_by_id(ctx, future_id).await;
    assert_eq!(
        future.used_amount, 0,
        "future-effective row must not be consumed"
    );
    assert_eq!(future.remaining_amount, 3000);

    // (c) expired untouched.
    let expired = get_ledger_by_id(ctx, expired_id).await;
    assert_eq!(expired.used_amount, 0, "expired row must not be consumed");
    assert_eq!(expired.remaining_amount, 2000);

    // (d) fully-used untouched (was already 0 remaining).
    let used_up = get_ledger_by_id(ctx, used_up_id).await;
    assert_eq!(used_up.remaining_amount, 0);

    // Post-consume derived balances: everything is 0.
    assert_eq!(
        get_derived_balance_by_credit_type(ctx, user_id, &realm_id, CreditType::SubscriptionCredit)
            .await,
        0
    );
    assert_eq!(
        get_derived_total_balance(ctx, user_id, &realm_id).await,
        0,
        "after consuming the only available row, derived total is 0"
    );
}
