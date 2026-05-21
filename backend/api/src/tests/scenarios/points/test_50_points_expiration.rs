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

    let transaction = result.unwrap();
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
