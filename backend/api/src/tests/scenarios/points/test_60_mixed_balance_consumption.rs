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

    create_points_account(ctx, user_id, &realm_id).await;

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
    let transaction = result.unwrap();
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

    create_points_account(ctx, user_id, &realm_id).await;

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

    create_points_account(ctx, user_id, &realm_id).await;

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
