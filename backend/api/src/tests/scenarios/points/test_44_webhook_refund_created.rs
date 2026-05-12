// =============================================================================
// Test: Refund Created Webhook
// =============================================================================
//
// Tests for refund.created webhook events.
//
// User Story: docs/user-stories/points-billing-events.md
// Covers: US-PO-06 (Refund revokes unused points proportionally)
//
// =============================================================================

use crate::tests::helpers::points_helpers::*;
use crate::tests::helpers::webhook_helpers::*;
use crate::tests::scenarios::points::fixtures::*;
use crate::tests::schema_test_context::SchemaTestContext;
use herald_core::domain::points::entities::{
    CreditLedgerStatus, CreditSourceType, CreditType, RevocationType,
};
use sqlx::Row;
use test_context::test_context;
use uuid::Uuid;

// ============================================================================
// Test 1: Topup Refund - Proportional Recovery
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-PO-06 场景 1 - 充值退款按未使用比例回收
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_refund_topup_proportional_recovery(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user1@example.com").await;
    let event_id = generate_test_event_id();
    let refund_id = format!("refund_{}", Uuid::now_v7());
    let payment_id = format!("payment_{}", Uuid::now_v7());

    create_points_account(ctx, user_id, &realm_id).await;

    // Configure Creem webhook for this realm
    ctx.with_creem_config(&realm_id, None, None, None).await;

    // Grant 10000 topup credits
    let ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::TopupCredit,
        CreditSourceType::Topup,
        payment_id.clone(),
        10000,
        None, // No expiry
    )
    .await;

    // Consume 3000, remaining 7000
    consume_points_from_ledger(ctx, ledger_id, 3000).await;

    // DEBUG: Verify ledger was created
    let all_ledgers = get_all_ledgers_for_user(ctx, user_id, &realm_id)
        .await
        .expect("Failed to query ledgers");

    println!("DEBUG: Total ledgers for user: {}", all_ledgers.len());
    for (id, credit_type, status, remaining) in &all_ledgers {
        println!(
            "  Ledger: id={}, credit_type={}, status={}, remaining={}",
            id, credit_type, status, remaining
        );
    }

    // When: Refund of 5000 (50% of original 10000)
    let event = build_refund_created_event_with_user(
        event_id,
        refund_id.clone(),
        payment_id.clone(),
        5000,  // refund amount
        10000, // original amount
        &realm_id,
        user_id, // Use the actual user_id from the test
    );

    let app = ctx.create_unified_test_router();
    let response = send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;

    // Then
    assert_webhook_success(&response);

    // Should revoke 50% of remaining: 7000 * 0.5 = 3500
    let ledger = get_ledger_by_id(ctx, ledger_id).await;
    assert_eq!(
        ledger.remaining_amount, 3500,
        "7000 - 3500 = 3500 remaining"
    );
    assert_eq!(ledger.revoked_amount, 3500, "50% of remaining revoked");
    assert_eq!(ledger.used_amount, 3000, "Used amount unchanged");

    // Verify revocation record
    let revocations = get_revocation_records(ctx, user_id).await;
    assert_eq!(revocations.len(), 1);
    assert_eq!(revocations[0].revocation_type, RevocationType::RefundRevoke);
    assert_eq!(revocations[0].revoked_amount, 3500);
    assert_eq!(revocations[0].reference_id, Some(refund_id));
}

// ============================================================================
// Test 2: Subscription Refund - Only Revoke Unused Subscription Credits
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-PO-06 场景 2 - 会员退款仅回收未使用会员积分
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_refund_subscription_only_unused(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user@example.com").await;
    let plan_id = Uuid::now_v7();
    let event_id = generate_test_event_id();
    let refund_id = format!("refund_{}", Uuid::now_v7());
    let payment_id = format!("payment_{}", Uuid::now_v7());

    create_points_account(ctx, user_id, &realm_id).await;

    // Configure Creem webhook for this realm
    ctx.with_creem_config(&realm_id, None, None, None).await;

    // Grant 5000 subscription credits
    let ledger_id = create_credit_ledger_entry_v2(
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

    // Consume 2000, remaining 3000
    consume_points_from_ledger(ctx, ledger_id, 2000).await;

    // When: Full refund (5000) - subscription type
    let event = build_refund_created_event_with_user_and_type(
        event_id,
        refund_id.clone(),
        payment_id.clone(),
        5000,
        5000,
        &realm_id,
        user_id,
        "subscription", // Subscription refund type
    );

    let app = ctx.create_unified_test_router();
    let response = send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;

    // Then
    assert_webhook_success(&response);

    // Should revoke all remaining 3000
    let ledger = get_ledger_by_id(ctx, ledger_id).await;
    assert_eq!(ledger.remaining_amount, 0, "All remaining revoked");
    assert_eq!(ledger.revoked_amount, 3000, "Only unused portion revoked");
    assert_eq!(ledger.used_amount, 2000, "Used amount unchanged");
    assert_eq!(ledger.status, CreditLedgerStatus::Revoked);

    // Verify revocation record
    let revocations = get_revocation_records(ctx, user_id).await;
    assert_eq!(revocations.len(), 1);
    assert_eq!(revocations[0].revocation_type, RevocationType::RefundRevoke);
    assert_eq!(revocations[0].revoked_amount, 3000);
}

// ============================================================================
// Test 3: Refund Event Idempotency
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-PO-06 场景 3 - 退款事件幂等性
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_refund_idempotency(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user@example.com").await;
    let event_id = generate_test_event_id();
    let refund_id = format!("refund_{}", Uuid::now_v7());
    let payment_id = format!("payment_{}", Uuid::now_v7());

    create_points_account(ctx, user_id, &realm_id).await;

    // Configure Creem webhook for this realm
    ctx.with_creem_config(&realm_id, None, None, None).await;

    create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::TopupCredit,
        CreditSourceType::Topup,
        payment_id.clone(),
        10000,
        None,
    )
    .await;

    let event = build_refund_created_event_with_user(
        event_id.clone(),
        refund_id,
        payment_id.clone(),
        5000,
        10000,
        &realm_id,
        user_id, // Use the actual user_id from the test
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

// ============================================================================
// Test 4: Used Points Not Recovered
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-PO-06 场景 5 - 已使用积分不回收
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_refund_used_points_not_recovered(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user@example.com").await;
    let event_id = generate_test_event_id();
    let refund_id = format!("refund_{}", Uuid::now_v7());
    let payment_id = format!("payment_{}", Uuid::now_v7());

    create_points_account(ctx, user_id, &realm_id).await;

    // Configure Creem webhook for this realm
    ctx.with_creem_config(&realm_id, None, None, None).await;

    // Grant 10000 topup credits
    let ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::TopupCredit,
        CreditSourceType::Topup,
        payment_id.clone(),
        10000,
        None,
    )
    .await;

    // Consume 8000, remaining 2000
    consume_points_from_ledger(ctx, ledger_id, 8000).await;

    // When: Full refund (10000)
    let event = build_refund_created_event_with_user(
        event_id,
        refund_id,
        payment_id.clone(),
        10000,
        10000,
        &realm_id,
        user_id, // Use the actual user_id from the test
    );

    let app = ctx.create_unified_test_router();
    let response = send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;

    // Then
    assert_webhook_success(&response);

    // Should only revoke remaining 2000, not the 8000 used
    let ledger = get_ledger_by_id(ctx, ledger_id).await;
    assert_eq!(ledger.remaining_amount, 0, "All remaining revoked");
    assert_eq!(ledger.revoked_amount, 2000, "Only remaining points revoked");
    assert_eq!(ledger.used_amount, 8000, "Used points not recovered");

    // Verify revocation record
    let revocations = get_revocation_records(ctx, user_id).await;
    assert_eq!(revocations.len(), 1);
    assert_eq!(revocations[0].revoked_amount, 2000);
}

/// Get all ledgers for user (debug function)
async fn get_all_ledgers_for_user(
    ctx: &SchemaTestContext,
    user_id: Uuid,
    realm_id: &str,
) -> Result<Vec<(Uuid, String, String, i64)>, Box<dyn std::error::Error>> {
    // DEBUG: Check schema
    let (current_schema, backend_pid): (Option<String>, Option<i32>) =
        sqlx::query_as("SELECT current_schema(), pg_backend_pid()")
            .fetch_one(&ctx.app_state.pool)
            .await
            .ok()
            .unwrap_or((None, None));
    println!(
        "DEBUG get_all_ledgers_for_user: schema={:?}, pid={:?}",
        current_schema, backend_pid
    );

    let rows = sqlx::query(
        "SELECT id, credit_type, status, remaining_amount FROM points_credit_ledger
         WHERE user_id = $1 AND realm_id = $2",
    )
    .bind(user_id)
    .bind(realm_id)
    .fetch_all(&ctx.app_state.pool)
    .await?;

    let result = rows
        .iter()
        .map(|row| {
            (
                row.get("id"),
                row.get("credit_type"),
                row.get("status"),
                row.get("remaining_amount"),
            )
        })
        .collect();

    Ok(result)
}
