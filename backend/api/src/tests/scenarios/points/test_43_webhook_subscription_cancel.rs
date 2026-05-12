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

    create_points_account(ctx, user_id, &realm_id).await;

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

    create_points_account(ctx, user_id, &realm_id).await;

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
// Test 3: Immediate Cancel with Partial Usage
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-BI-011 场景 3 - 立即取消已使用会员积分不回收
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_subscription_cancel_partial_usage(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user3@example.com").await;
    let plan_id = Uuid::now_v7();
    let event_id = generate_test_event_id();
    let period_end = Utc::now() + Duration::days(30);

    // Configure Creem webhook for this realm
    ctx.with_creem_config(&realm_id, None, None, None).await;

    // Setup plan config for the test
    setup_test_plan_config(ctx, &realm_id, plan_id).await;

    create_points_account(ctx, user_id, &realm_id).await;

    let ledger_id = create_credit_ledger_entry_v2(
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

    // Simulate partial usage: 5000 consumed, 5000 remaining
    consume_points_from_ledger(ctx, ledger_id, 5000).await;

    // When: Cancel immediately
    let event = build_subscription_canceled_event(
        event_id, user_id, false, // immediate cancel
        &realm_id,
    );

    let app = ctx.create_unified_test_router();
    let response = send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;

    // Then
    assert_webhook_success(&response);

    // Only remaining points should be revoked
    let ledgers =
        get_user_ledgers_by_credit_type(ctx, user_id, CreditType::SubscriptionCredit).await;
    assert_eq!(ledgers.len(), 1);

    let ledger = &ledgers[0];
    assert_eq!(ledger.remaining_amount, 0, "All remaining points revoked");
    assert_eq!(ledger.revoked_amount, 5000, "Only unused portion revoked");
    assert_eq!(ledger.used_amount, 5000, "Used amount unchanged");
    assert_eq!(ledger.status, CreditLedgerStatus::Revoked);

    // Verify revocation record
    let revocations = get_revocation_records(ctx, user_id).await;
    assert_eq!(revocations.len(), 1);
    assert_eq!(revocations[0].revocation_type, RevocationType::CancelRevoke);
    assert_eq!(revocations[0].revoked_amount, 5000);
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

    create_points_account(ctx, user_id, &realm_id).await;

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
