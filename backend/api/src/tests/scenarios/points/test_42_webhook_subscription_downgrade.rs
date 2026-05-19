// =============================================================================
// Test: Subscription Downgrade Webhook
// =============================================================================
//
// Tests for subscription.update webhook events (downgrades).
//
// User Story: docs/user-stories/points-billing-events.md
// Covers: US-BI-010 (Subscription downgrade takes effect next period)
//
// =============================================================================

use crate::tests::helpers::points_helpers::*;
use crate::tests::helpers::webhook_helpers::*;
use crate::tests::scenarios::points::fixtures::*;
use crate::tests::schema_test_context::SchemaTestContext;
use chrono::{Duration, Utc};
use herald_core::domain::points::entities::{CreditLedgerStatus, CreditSourceType, CreditType};
use test_context::test_context;
use uuid::Uuid;

// ============================================================================
// Test 1: Downgrade Takes Effect Next Period (No Immediate Revoke)
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-BI-010 场景 1 - 降级下周期生效，不回收当前积分
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_subscription_downgrade_no_immediate_revoke(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user1@example.com").await;
    let premium_plan_id = Uuid::now_v7();
    let basic_plan_id = Uuid::now_v7();
    let event_id = generate_test_event_id();
    let period_end = Utc::now() + Duration::days(30);

    // Configure Creem webhook for this realm
    ctx.with_creem_config(&realm_id, None, None, None).await;

    // Setup plan configs for the test
    setup_test_plan_config_with_points(ctx, &realm_id, premium_plan_id, 10000).await;
    setup_test_plan_config_with_points(ctx, &realm_id, basic_plan_id, 5000).await;

    create_points_account(ctx, user_id, &realm_id).await;

    // User currently has Premium Plan (10000 points)
    create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        CreditSourceType::SubscriptionInitial,
        premium_plan_id.to_string(),
        10000,
        Some(period_end),
    )
    .await;

    // When: User downgrades to Basic Plan (5000 points)
    let event = build_subscription_updated_event(
        event_id,
        user_id,
        premium_plan_id,
        basic_plan_id,
        &realm_id,
    );

    let app = ctx.create_unified_test_router();
    let response = send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;

    // Then
    assert_webhook_success(&response);

    // Current period ledger should remain unchanged
    let ledgers =
        get_user_ledgers_by_credit_type(ctx, user_id, CreditType::SubscriptionCredit).await;
    assert_eq!(
        ledgers.len(),
        1,
        "Should not create new ledger for downgrade"
    );

    let ledger = &ledgers[0];
    assert_eq!(
        ledger.remaining_amount, 10000,
        "Points should remain unchanged"
    );
    assert_eq!(
        ledger.status,
        CreditLedgerStatus::Active,
        "Status should remain active"
    );
    assert_eq!(ledger.revoked_amount, 0, "No points should be revoked");

    // No revocation records should be created
    let revocations = get_revocation_records(ctx, user_id).await;
    assert_eq!(revocations.len(), 0, "No revocation records for downgrade");
}

// ============================================================================
// Test 2: Subscription Downgrade Event Idempotency
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-BI-010 场景 - subscription.downgraded 幂等性，相同 event_id 不重复处理降级
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_subscription_downgrade_idempotency(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user2@example.com").await;
    let premium_plan_id = Uuid::now_v7();
    let basic_plan_id = Uuid::now_v7();
    let event_id = generate_test_event_id();
    let period_end = Utc::now() + Duration::days(30);

    // Configure Creem webhook for this realm
    ctx.with_creem_config(&realm_id, None, None, None).await;

    // Setup plan configs for the test
    setup_test_plan_config_with_points(ctx, &realm_id, premium_plan_id, 10000).await;
    setup_test_plan_config_with_points(ctx, &realm_id, basic_plan_id, 5000).await;

    create_points_account(ctx, user_id, &realm_id).await;

    // User currently has Premium Plan (10000 points)
    create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        CreditSourceType::SubscriptionInitial,
        premium_plan_id.to_string(),
        10000,
        Some(period_end),
    )
    .await;

    // Build downgrade event with a shared event_id
    let event = build_subscription_updated_event(
        event_id.clone(),
        user_id,
        premium_plan_id,
        basic_plan_id,
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

    // Then: Should still have exactly 1 ledger (downgrade does not create new ledger)
    let ledgers =
        get_user_ledgers_by_credit_type(ctx, user_id, CreditType::SubscriptionCredit).await;
    assert_eq!(
        ledgers.len(),
        1,
        "Should still have exactly one ledger after idempotent downgrade"
    );

    // Points should remain unchanged
    let ledger = &ledgers[0];
    assert_eq!(
        ledger.remaining_amount, 10000,
        "Points should remain unchanged"
    );
    assert_eq!(ledger.revoked_amount, 0, "No points should be revoked");

    // No revocation records should be created
    let revocations = get_revocation_records(ctx, user_id).await;
    assert_eq!(
        revocations.len(),
        0,
        "No revocation records for idempotent downgrade"
    );
}
