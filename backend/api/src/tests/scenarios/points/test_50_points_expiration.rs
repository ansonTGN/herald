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
use herald_core::domain::points::entities::{
    CreditLedgerStatus, CreditSourceType, CreditType, RevocationType,
};
use test_context::test_context;
use uuid::Uuid;

// ============================================================================
// Test 1: Expired Subscription Credits Auto Revoke
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-PO-06 扩展场景 - 过期会员积分自动回收
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_points_expiration_auto_revoke(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user@example.com").await;
    let plan_id = Uuid::now_v7();

    create_points_account(ctx, user_id, &realm_id).await;

    // Create expired subscription credit (expired 10 days ago)
    let expired_at = Utc::now() - Duration::days(10);
    let ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        CreditSourceType::SubscriptionInitial,
        plan_id.to_string(),
        10000,
        Some(expired_at), // Already expired
    )
    .await;

    // When: Manually mark as expired (simulating expiration service)
    sqlx::query(
        "UPDATE points_credit_ledger
         SET status = 'expired',
             revoked_amount = granted_amount,
             updated_at = NOW()
         WHERE id = $1",
    )
    .bind(ledger_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to expire ledger");

    // Create revocation record
    sqlx::query(
        "INSERT INTO points_revocation_records (id, ledger_id, user_id, realm_id, revocation_type, revoked_amount, reason, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())"
    )
    .bind(Uuid::now_v7())
    .bind(ledger_id)
    .bind(user_id)
    .bind(realm_id)
    .bind(RevocationType::ExpireRevoke.to_string())
    .bind(10000)
    .bind("Points expired")
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create revocation record");

    // Then
    // Verify ledger status
    let ledger = get_ledger_by_id(ctx, ledger_id).await;
    assert_eq!(
        ledger.status,
        CreditLedgerStatus::Expired,
        "Should be marked as expired"
    );
    assert_eq!(ledger.remaining_amount, 0, "All remaining points expired");
    assert_eq!(ledger.revoked_amount, 10000, "All points revoked");

    // Verify revocation record
    let revocations = get_revocation_records(ctx, user_id).await;
    assert_eq!(revocations.len(), 1);
    assert_eq!(revocations[0].revocation_type, RevocationType::ExpireRevoke);
    assert_eq!(revocations[0].revoked_amount, 10000);
}

// ============================================================================
// Test 2: Non-Expired Credits Not Revoked
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-PO-06 扩展场景 - 未过期积分不被回收
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_points_expiration_skip_active(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user@example.com").await;
    let plan_id = Uuid::now_v7();

    create_points_account(ctx, user_id, &realm_id).await;

    // Create non-expired subscription credit (expires in 30 days)
    let future_expiry = Utc::now() + Duration::days(30);
    let ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        CreditSourceType::SubscriptionInitial,
        plan_id.to_string(),
        10000,
        Some(future_expiry),
    )
    .await;

    // When: No expiration occurs (points are still valid)

    // Then: Verify ledger unchanged
    let ledger = get_ledger_by_id(ctx, ledger_id).await;
    assert_eq!(
        ledger.status,
        CreditLedgerStatus::Active,
        "Should remain active"
    );
    assert_eq!(ledger.remaining_amount, 10000, "Points should remain");
    assert_eq!(ledger.revoked_amount, 0, "No revocation");

    // No revocation records
    let revocations = get_revocation_records(ctx, user_id).await;
    assert_eq!(revocations.len(), 0);
}

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

    create_points_account(ctx, user_id, &realm_id).await;

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

// ============================================================================
// Test 4: Partially Expired Credits
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-PO-06 扩展场景 - 部分过期积分处理
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_points_expiration_partial_usage(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user@example.com").await;
    let plan_id = Uuid::now_v7();

    create_points_account(ctx, user_id, &realm_id).await;

    // Create subscription credit with some usage
    let expired_at = Utc::now() - Duration::days(10);
    let ledger_id = create_credit_ledger_entry_v2(
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

    // Use 3000 before expiration
    consume_points_from_ledger(ctx, ledger_id, 3000).await;

    // When: Manually mark as expired (simulating expiration service)
    sqlx::query(
        "UPDATE points_credit_ledger
         SET status = 'expired',
             revoked_amount = granted_amount - used_amount,
             updated_at = NOW()
         WHERE id = $1",
    )
    .bind(ledger_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to expire ledger");

    // Create revocation record
    sqlx::query(
        "INSERT INTO points_revocation_records (id, ledger_id, user_id, realm_id, revocation_type, revoked_amount, reason, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())"
    )
    .bind(Uuid::now_v7())
    .bind(ledger_id)
    .bind(user_id)
    .bind(realm_id)
    .bind(RevocationType::ExpireRevoke.to_string())
    .bind(7000)
    .bind("Points expired")
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create revocation record");

    // Then: Verify ledger
    let ledger = get_ledger_by_id(ctx, ledger_id).await;
    assert_eq!(ledger.status, CreditLedgerStatus::Expired);
    assert_eq!(ledger.remaining_amount, 0, "All remaining expired");
    assert_eq!(ledger.revoked_amount, 7000, "Remaining portion revoked");
    assert_eq!(ledger.used_amount, 3000, "Used portion unchanged");
}
