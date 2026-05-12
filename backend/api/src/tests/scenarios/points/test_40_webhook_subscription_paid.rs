// =============================================================================
// Test: Subscription Paid Webhook
// =============================================================================
//
// Tests for subscription.paid webhook events (initial subscription and renewals).
//
// User Story: docs/user-stories/points-billing-events.md
// Covers: US-PO-06 (Subscription grants and renewals)
//
// =============================================================================

use crate::tests::helpers::points_helpers::*;
use crate::tests::helpers::webhook_helpers::*;
use crate::tests::scenarios::points::fixtures::*;
use crate::tests::schema_test_context::SchemaTestContext;
use herald_core::domain::points::entities::{CreditLedgerStatus, CreditSourceType, CreditType};
use test_context::test_context;
use uuid::Uuid;

// ============================================================================
// Test 1: Initial Subscription Grant
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-PO-06 场景 - Initial subscription grants points
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_subscription_paid_initial_grant(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone(); // Clone to avoid borrow issues
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user1@example.com").await;
    let plan_id = Uuid::now_v7();
    let event_id = generate_test_event_id();

    // Configure Creem webhook for this realm
    ctx.with_creem_config(&realm_id, None, None, None).await;

    // Setup plan config for the test
    setup_test_plan_config(ctx, &realm_id, plan_id).await;

    // Create points account for user
    create_points_account(ctx, user_id, &realm_id).await;

    // Build subscription.paid event (is_renewal = false)
    let event = build_subscription_paid_event(
        event_id, user_id, plan_id, false, // initial subscription
        &realm_id,
    );

    // When
    let app = ctx.create_unified_test_router();
    let response = send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;

    // Then
    assert_webhook_success(&response);

    // Verify subscription_credit ledger was created
    let ledgers =
        get_user_ledgers_by_credit_type(ctx, user_id, CreditType::SubscriptionCredit).await;
    assert_eq!(
        ledgers.len(),
        1,
        "Should create one subscription credit ledger"
    );

    let ledger = &ledgers[0];
    assert_eq!(ledger.credit_type, CreditType::SubscriptionCredit);
    assert_eq!(ledger.source_type, CreditSourceType::SubscriptionInitial);
    assert_eq!(ledger.granted_amount, 1000); // Amount from setup_test_plan_config
    assert_eq!(ledger.remaining_amount, 1000);
    assert_eq!(ledger.status, CreditLedgerStatus::Active);
    assert!(
        ledger.expires_at.is_some(),
        "Subscription credits should have expiry"
    );

    // Verify transaction record
    assert_transaction_exists_by_type(
        ctx,
        user_id,
        herald_core::domain::points::entities::TransactionType::SubscriptionGrant,
        1000,
    )
    .await;
}

// ============================================================================
// Test 2: Subscription Renewal
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-PO-06 场景 - Subscription renewal grants points
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_subscription_paid_renewal_grant(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone(); // Clone to avoid borrow issues
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user2@example.com").await;
    let plan_id = Uuid::now_v7();
    let event_id = generate_test_event_id();

    // Configure Creem webhook for this realm
    ctx.with_creem_config(&realm_id, None, None, None).await;

    // Setup plan config for the test
    setup_test_plan_config(ctx, &realm_id, plan_id).await;

    create_points_account(ctx, user_id, &realm_id).await;

    // Build subscription.paid event (is_renewal = true)
    let event = build_subscription_paid_event(
        event_id, user_id, plan_id, true, // renewal
        &realm_id,
    );

    // When
    let app = ctx.create_unified_test_router();
    let response = send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;

    // Then
    assert_webhook_success(&response);

    // Verify subscription_credit ledger was created with renewal source type
    let ledgers =
        get_user_ledgers_by_credit_type(ctx, user_id, CreditType::SubscriptionCredit).await;
    assert_eq!(ledgers.len(), 1);

    let ledger = &ledgers[0];
    assert_eq!(ledger.credit_type, CreditType::SubscriptionCredit);
    assert_eq!(ledger.source_type, CreditSourceType::SubscriptionRenewal);
    assert_eq!(ledger.granted_amount, 1000);
    assert_eq!(ledger.remaining_amount, 1000);
    assert_eq!(ledger.status, CreditLedgerStatus::Active);

    // Verify transaction record
    assert_transaction_exists_by_type(
        ctx,
        user_id,
        herald_core::domain::points::entities::TransactionType::SubscriptionRenewal,
        1000,
    )
    .await;
}

// ============================================================================
// Test 3: Subscription Paid Idempotency
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-PO-06 场景 - Subscription paid event idempotency
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_subscription_paid_idempotency(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone(); // Clone to avoid borrow issues
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user3@example.com").await;
    let plan_id = Uuid::now_v7();
    let event_id = generate_test_event_id();

    // Configure Creem webhook for this realm
    ctx.with_creem_config(&realm_id, None, None, None).await;

    // Setup plan config for the test
    setup_test_plan_config(ctx, &realm_id, plan_id).await;

    create_points_account(ctx, user_id, &realm_id).await;

    let event = build_subscription_paid_event(event_id.clone(), user_id, plan_id, false, &realm_id);

    let app = ctx.create_unified_test_router();

    // When: First processing
    let response1 =
        send_webhook_with_signature(&app, &realm_id, event.clone(), "test_webhook_secret").await;
    assert_webhook_success(&response1);

    // When: Second processing (same event_id)
    let response2 =
        send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;
    assert_webhook_success(&response2);

    // Then: Should only create one ledger
    let ledgers =
        get_user_ledgers_by_credit_type(ctx, user_id, CreditType::SubscriptionCredit).await;
    assert_eq!(ledgers.len(), 1, "Should not duplicate ledger on retry");

    // Note: Idempotency uses Redis, not database. The key exists in Redis cache.
    // We verify idempotency worked by checking only one ledger was created.
}
