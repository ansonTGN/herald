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
    CreditSourceType, CreditType, QuotaEntitlementStatus, QuotaSourceType, RevocationType,
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

    create_points_wallet(ctx, user_id, &realm_id).await;

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

    // Seed the payment_attempts snapshot the Creem refund webhook resolves the
    // routing bucket from. Without it the handler fails loud with
    // "no payment_attempt for payment_id".
    let bucket_id = get_wallet_bucket_id(ctx, &realm_id, user_id).await;
    create_payment_attempt_snapshot(ctx, &realm_id, user_id, &payment_id, bucket_id, 10000).await;

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
// Test 2: Subscription Refund - Revoke Active Quota Entitlement
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-PO-06 场景 2 - 会员退款回收该订阅的窗口配额授权
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_refund_subscription_only_unused(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user@example.com").await;
    let plan_id = Uuid::now_v7();
    let subscription_id = Uuid::now_v7();
    let external_subscription_id = format!("sub_{}", subscription_id);
    let event_id = generate_test_event_id();
    let refund_id = format!("refund_{}", Uuid::now_v7());
    let payment_id = format!("payment_{}", Uuid::now_v7());

    ctx.with_creem_config(&realm_id, None, None, None).await;
    setup_test_plan_config(ctx, &realm_id, plan_id).await;
    create_points_wallet(ctx, user_id, &realm_id).await;

    let bucket_id = get_wallet_bucket_id(ctx, &realm_id, user_id).await;

    // Pre-insert the originating subscription row.
    sqlx::query(
        "INSERT INTO subscription
            (id, realm_id, user_id, status, entitlement_key,
             external_subscription_id, external_product_id, payment_provider,
             current_period_start, current_period_end, cancel_at_period_end,
             bucket_id, created_at, updated_at, billing_type)
         VALUES ($1, $2, $3, 'active', $4, $5, $6, 'creem',
                 NOW() - INTERVAL '1 day', NOW() + INTERVAL '30 days', false,
                 $7, NOW(), NOW(), 'recurring')",
    )
    .bind(subscription_id)
    .bind(&realm_id)
    .bind(user_id)
    .bind(plan_id.to_string())
    .bind(&external_subscription_id)
    .bind(format!("prod_test_{}", plan_id))
    .bind(bucket_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("seed subscription");

    // Seed an active SubscriptionCredit quota entitlement keyed by subscription_id.
    let effective_from = chrono::Utc::now() - chrono::Duration::days(1);
    let effective_until = Some(chrono::Utc::now() + chrono::Duration::days(30));
    grant_quota_entitlement_for_test(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
        QuotaSourceType::SubscriptionInitial,
        &subscription_id.to_string(),
        &[(2_592_000, 5000, "period")],
        effective_from,
        effective_until,
    )
    .await;

    // Consume 2000 of the quota; 3000 nominally remains, but a full refund
    // revokes the entire active entitlement under the window-quota model.
    seed_quota_consume_for_test(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
        2000,
        chrono::Utc::now(),
    )
    .await;

    // Seed the payment_attempts snapshot the Creem refund webhook resolves the
    // routing bucket from.
    create_payment_attempt_snapshot(ctx, &realm_id, user_id, &payment_id, bucket_id, 5000).await;

    // When: Full subscription refund
    let mut event = build_refund_created_event_with_user_and_type(
        event_id,
        refund_id.clone(),
        payment_id.clone(),
        5000,
        5000,
        &realm_id,
        user_id,
        "subscription",
    );
    event["data"]["object"]["subscriptionId"] =
        serde_json::Value::String(external_subscription_id.clone());

    let app = ctx.create_unified_test_router();
    let response = send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;

    // Then
    assert_webhook_success(&response);

    // The active entitlement for this subscription is revoked.
    let entitlements =
        get_user_quota_entitlements(ctx, user_id, CreditType::SubscriptionCredit).await;
    assert_eq!(entitlements.len(), 1);
    assert_eq!(entitlements[0].source_id, subscription_id.to_string());
    assert_eq!(
        entitlements[0].status,
        QuotaEntitlementStatus::Revoked,
        "subscription refund must revoke the entitlement keyed by subscription source_id"
    );

    // Derived balance drops to zero because no active entitlement remains.
    assert_derived_balance(ctx, user_id, &realm_id, CreditType::SubscriptionCredit, 0).await;
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

// ============================================================================
// Test 3: Refund Created Event Idempotency
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-PO-06 场景 - refund.created 幂等性，相同 event_id 不重复回收积分
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_refund_created_idempotency(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user3@example.com").await;
    let event_id = generate_test_event_id();
    let refund_id = format!("refund_{}", Uuid::now_v7());
    let payment_id = format!("payment_{}", Uuid::now_v7());

    create_points_wallet(ctx, user_id, &realm_id).await;

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

    // Consume 3000, remaining 7000
    consume_points_from_ledger(ctx, ledger_id, 3000).await;

    // Seed the payment_attempts snapshot the Creem refund webhook resolves the
    // routing bucket from.
    let bucket_id = get_wallet_bucket_id(ctx, &realm_id, user_id).await;
    create_payment_attempt_snapshot(ctx, &realm_id, user_id, &payment_id, bucket_id, 10000).await;

    // Build refund event with a shared event_id
    let event = build_refund_created_event_with_user(
        event_id.clone(),
        refund_id.clone(),
        payment_id.clone(),
        5000,  // refund amount
        10000, // original amount
        &realm_id,
        user_id,
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
    assert_eq!(revocations[0].revocation_type, RevocationType::RefundRevoke);
    assert_eq!(
        revocations[0].revoked_amount, 3500,
        "50% of 7000 remaining = 3500"
    );

    // Verify ledger state is correct (not double-revoked)
    let ledger = get_ledger_by_id(ctx, ledger_id).await;
    assert_eq!(
        ledger.revoked_amount, 3500,
        "Should revoke exactly 3500, not double"
    );
    assert_eq!(ledger.remaining_amount, 3500);
}

// Covers retry after outer webhook bookkeeping fails: a different webhook event id
// carrying the same refund id must not revoke topup credits again.
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_refund_topup_same_refund_id_different_event_id_is_idempotent(
    ctx: &mut SchemaTestContext,
) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user4@example.com").await;
    let refund_id = format!("refund_{}", Uuid::now_v7());
    let payment_id = format!("payment_{}", Uuid::now_v7());

    create_points_wallet(ctx, user_id, &realm_id).await;
    ctx.with_creem_config(&realm_id, None, None, None).await;

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
    consume_points_from_ledger(ctx, ledger_id, 3000).await;

    let bucket_id = get_wallet_bucket_id(ctx, &realm_id, user_id).await;
    create_payment_attempt_snapshot(ctx, &realm_id, user_id, &payment_id, bucket_id, 10000).await;

    let app = ctx.create_unified_test_router();
    for _ in 0..2 {
        let event = build_refund_created_event_with_user(
            generate_test_event_id(),
            refund_id.clone(),
            payment_id.clone(),
            5000,
            10000,
            &realm_id,
            user_id,
        );
        let response =
            send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;
        assert_webhook_success(&response);
    }

    let revocations = get_revocation_records(ctx, user_id).await;
    assert_eq!(revocations.len(), 1, "same refund id must not revoke twice");
    assert_eq!(revocations[0].revoked_amount, 3500);

    let ledger = get_ledger_by_id(ctx, ledger_id).await;
    assert_eq!(ledger.revoked_amount, 3500);
    assert_eq!(ledger.remaining_amount, 3500);
}

// Covers retry after outer webhook bookkeeping fails: subscription refund revoke
// must use the refund id as business idempotency, not only the webhook event id.
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_refund_subscription_same_refund_id_different_event_id_is_idempotent(
    ctx: &mut SchemaTestContext,
) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user5@example.com").await;
    let plan_id = Uuid::now_v7();
    let subscription_id = Uuid::now_v7();
    let external_subscription_id = format!("sub_{}", subscription_id);
    let refund_id = format!("refund_{}", Uuid::now_v7());
    let payment_id = format!("payment_{}", Uuid::now_v7());

    ctx.with_creem_config(&realm_id, None, None, None).await;
    setup_test_plan_config(ctx, &realm_id, plan_id).await;
    create_points_wallet(ctx, user_id, &realm_id).await;

    let bucket_id = get_wallet_bucket_id(ctx, &realm_id, user_id).await;

    // Pre-insert the originating subscription row.
    sqlx::query(
        "INSERT INTO subscription
            (id, realm_id, user_id, status, entitlement_key,
             external_subscription_id, external_product_id, payment_provider,
             current_period_start, current_period_end, cancel_at_period_end,
             bucket_id, created_at, updated_at, billing_type)
         VALUES ($1, $2, $3, 'active', $4, $5, $6, 'creem',
                 NOW() - INTERVAL '1 day', NOW() + INTERVAL '30 days', false,
                 $7, NOW(), NOW(), 'recurring')",
    )
    .bind(subscription_id)
    .bind(&realm_id)
    .bind(user_id)
    .bind(plan_id.to_string())
    .bind(&external_subscription_id)
    .bind(format!("prod_test_{}", plan_id))
    .bind(bucket_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("seed subscription");

    // Seed an active SubscriptionCredit quota entitlement keyed by subscription_id.
    let effective_from = chrono::Utc::now() - chrono::Duration::days(1);
    let effective_until = Some(chrono::Utc::now() + chrono::Duration::days(30));
    grant_quota_entitlement_for_test(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
        QuotaSourceType::SubscriptionInitial,
        &subscription_id.to_string(),
        &[(2_592_000, 5000, "period")],
        effective_from,
        effective_until,
    )
    .await;

    // Seed the payment_attempts snapshot the Creem refund webhook resolves the
    // routing bucket from.
    create_payment_attempt_snapshot(ctx, &realm_id, user_id, &payment_id, bucket_id, 5000).await;

    let app = ctx.create_unified_test_router();
    for _ in 0..2 {
        let mut event = build_refund_created_event_with_user_and_type(
            generate_test_event_id(),
            refund_id.clone(),
            payment_id.clone(),
            5000,
            5000,
            &realm_id,
            user_id,
            "subscription",
        );
        event["data"]["object"]["subscriptionId"] =
            serde_json::Value::String(external_subscription_id.clone());

        let response =
            send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;
        assert_webhook_success(&response);
    }

    // Exactly one entitlement row exists and it is revoked.
    assert_eq!(
        count_subscription_quota_entitlements(ctx, user_id).await,
        1,
        "same refund id must not produce duplicate entitlement side effects"
    );
    let entitlements =
        get_user_quota_entitlements(ctx, user_id, CreditType::SubscriptionCredit).await;
    assert_eq!(entitlements.len(), 1);
    assert_eq!(entitlements[0].source_id, subscription_id.to_string());
    assert_eq!(
        entitlements[0].status,
        QuotaEntitlementStatus::Revoked,
        "subscription refund must stay revoked on replay"
    );
}
