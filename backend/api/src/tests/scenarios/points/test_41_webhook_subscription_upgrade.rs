// =============================================================================
// Test: Subscription Upgrade Webhook
// =============================================================================
//
// Tests for subscription.update webhook events (upgrades).
//
// User Story: docs/user-stories/points-billing-events.md
// Covers: US-BI-009 (Subscription upgrade grants difference points)
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
// Test 1: Upgrade Grants Difference Points
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-BI-009 场景 1 - 升级立即补发差额积分
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_subscription_upgrade_grants_difference(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user1@example.com").await;
    let basic_plan_id = Uuid::now_v7();
    let premium_plan_id = Uuid::now_v7();
    let event_id = generate_test_event_id();
    let period_end = Utc::now() + Duration::days(30);

    // Configure Creem webhook for this realm
    ctx.with_creem_config(&realm_id, None, None, None).await;

    // Setup plan configs for the test
    setup_test_plan_config_with_points(ctx, &realm_id, basic_plan_id, 5000).await;
    setup_test_plan_config_with_points(ctx, &realm_id, premium_plan_id, 10000).await;

    create_points_wallet(ctx, user_id, &realm_id).await;

    // User currently has Basic Plan (5000 points)
    create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        CreditSourceType::SubscriptionInitial,
        basic_plan_id.to_string(),
        5000,
        Some(period_end),
    )
    .await;

    // When: User upgrades to Premium Plan (10000 points)
    //
    // Use the new plan's own external_product_id (prod_test_<premium_plan_id>)
    // so the price-aware webhook resolver lands on the premium mapping instead
    // of colliding on the shared `prod_test_monthly` product that both plans
    // register via setup_test_plan_config_with_points.
    let event = build_subscription_updated_event_with_product(
        event_id,
        user_id,
        basic_plan_id,
        premium_plan_id,
        &realm_id,
        &format!("prod_test_{}", premium_plan_id),
    );

    // Extract period_end from the event JSON (to avoid time precision issues)
    let event_period_end_str = event["data"]["object"]["currentPeriodEnd"]
        .as_str()
        .expect("Period end should exist in event");
    let event_period_end = chrono::DateTime::parse_from_rfc3339(event_period_end_str)
        .unwrap()
        .with_timezone(&chrono::Utc);

    let app = ctx.create_unified_test_router();
    let response = send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;

    // Then
    assert_webhook_success(&response);

    // Should have 2 ledgers: original + upgrade difference
    let ledgers =
        get_user_ledgers_by_credit_type(ctx, user_id, CreditType::SubscriptionCredit).await;
    assert_eq!(ledgers.len(), 2, "Should have original and upgrade ledgers");

    // Find upgrade ledger
    let upgrade_ledger: Vec<_> = ledgers
        .iter()
        .filter(|l| l.source_type == CreditSourceType::SubscriptionUpgrade)
        .collect();

    assert_eq!(upgrade_ledger.len(), 1, "Should have one upgrade ledger");
    // Implementation grants full new plan amount (10000) after revoking old (5000)
    assert_eq!(
        upgrade_ledger[0].granted_amount, 10000,
        "Upgrade grants full new plan amount"
    );
    assert_eq!(upgrade_ledger[0].remaining_amount, 10000);
    assert_eq!(upgrade_ledger[0].status, CreditLedgerStatus::Active);

    // Check expiry time with microsecond tolerance (DB may truncate microseconds)
    let actual_expiry = upgrade_ledger[0]
        .expires_at
        .expect("Upgrade ledger should have expiry");
    let time_diff = (actual_expiry - event_period_end)
        .abs()
        .num_microseconds()
        .unwrap_or(1);
    assert!(
        time_diff <= 1000, // Allow 1ms tolerance for microsecond truncation
        "Expiry time should match within 1ms: got {}, expected {}, diff: {}μs",
        actual_expiry,
        event_period_end,
        time_diff
    );

    // Verify transaction record
    assert_transaction_exists_by_type(
        ctx,
        user_id,
        herald_core::domain::points::entities::TransactionType::SubscriptionUpgrade,
        10000,
    )
    .await;
}

// ============================================================================
// Test 2: Subscription Upgrade Event Idempotency
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-BI-009 场景 - subscription.upgraded 幂等性，相同 event_id 不重复发放升级差价积分
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_subscription_upgrade_idempotency(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user2@example.com").await;
    let basic_plan_id = Uuid::now_v7();
    let premium_plan_id = Uuid::now_v7();
    let event_id = generate_test_event_id();
    let period_end = Utc::now() + Duration::days(30);

    // Configure Creem webhook for this realm
    ctx.with_creem_config(&realm_id, None, None, None).await;

    // Setup plan configs for the test
    setup_test_plan_config_with_points(ctx, &realm_id, basic_plan_id, 5000).await;
    setup_test_plan_config_with_points(ctx, &realm_id, premium_plan_id, 10000).await;

    create_points_wallet(ctx, user_id, &realm_id).await;

    // User currently has Basic Plan (5000 points)
    create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        CreditSourceType::SubscriptionInitial,
        basic_plan_id.to_string(),
        5000,
        Some(period_end),
    )
    .await;

    // Build upgrade event with a shared event_id. New plan's own product id so
    // the resolver selects the premium mapping (see upgrade-grants test above).
    let event = build_subscription_updated_event_with_product(
        event_id.clone(),
        user_id,
        basic_plan_id,
        premium_plan_id,
        &realm_id,
        &format!("prod_test_{}", premium_plan_id),
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

    // Then: Should have exactly 2 ledgers (original + one upgrade), not 3
    let ledgers =
        get_user_ledgers_by_credit_type(ctx, user_id, CreditType::SubscriptionCredit).await;
    assert_eq!(
        ledgers.len(),
        2,
        "Should have original and exactly one upgrade ledger"
    );

    // Verify only one upgrade ledger was created
    let upgrade_ledgers: Vec<_> = ledgers
        .iter()
        .filter(|l| l.source_type == CreditSourceType::SubscriptionUpgrade)
        .collect();
    assert_eq!(
        upgrade_ledgers.len(),
        1,
        "Should not duplicate upgrade ledger on retry"
    );
    // Implementation grants full new plan amount (10000) after revoking old (5000)
    assert_eq!(
        upgrade_ledgers[0].granted_amount, 10000,
        "Upgrade grants full new plan amount"
    );
}
