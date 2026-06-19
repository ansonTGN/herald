// =============================================================================
// Regression Tests: Code Review Fixes (2026-06-10)
// =============================================================================
//
// Tests that guard against regressions of bugs found during code review.
//
// 1. Expired-but-unmarked ledgers must not be consumed (expires_at filter fix)
// 2. grant_points must reject frozen/closed wallets inside the transaction
//
// =============================================================================

use crate::tests::helpers::points_helpers::*;
use crate::tests::scenarios::points::fixtures::*;
use crate::tests::schema_test_context::SchemaTestContext;
use chrono::{Duration, Utc};
use herald_core::domain::points::dtos::{ConsumePointsInput, GrantPointsInput};
use herald_core::domain::points::entities::{CreditSourceType, CreditType};
use test_context::test_context;
use uuid::Uuid;

// ============================================================================
// Test 1: Expired-but-unmarked ledgers are skipped during consumption
// ============================================================================
//
// Regression guard for: find_active_ledgers_by_expiration_for_update
// now filters by (expires_at IS NULL OR expires_at > NOW()).
//
// Previously: a ledger with expires_at in the past but status='active'
// (because the expiration cron hadn't run yet) would be consumed first
// (sorted by expires_at ASC). The user would spend already-expired credits.
//
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_expired_unmarked_ledger_skipped_during_consume(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id =
        create_test_user(&ctx.app_state.pool, &realm_id, "regression76a@example.com").await;

    create_points_wallet(ctx, user_id, &realm_id).await;

    // Create a ledger that has expires_at in the past but status is still 'active'
    // (simulates the gap between expiration time and cron job marking it)
    let expired_at = Utc::now() - Duration::hours(1);
    let expired_ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        CreditSourceType::SubscriptionInitial,
        Uuid::now_v7().to_string(),
        10000,
        Some(expired_at),
    )
    .await;

    // Verify the ledger is still 'active' (cron hasn't run)
    let ledger = get_ledger_by_id(ctx, expired_ledger_id).await;
    assert_eq!(
        ledger.remaining_amount, 10000,
        "Ledger should have full remaining_amount"
    );

    // Create a valid (non-expired) ledger
    let valid_ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::TopupCredit,
        CreditSourceType::Topup,
        Uuid::now_v7().to_string(),
        5000,
        None, // permanent — no expiration
    )
    .await;

    // Consume 5000 points
    let identity = create_test_third_party_identity(&realm_id);
    let input = ConsumePointsInput {
        user_id: user_id.to_string(),
        client_app_id: ctx._client_app_id.clone(),
        amount: 5000,
        description: Some("regression: consume should skip expired".to_string()),
    };
    let result = ctx
        .app_state
        .points_service
        .consume_points(identity, &realm_id, input)
        .await;

    assert!(
        result.is_ok(),
        "Consume should succeed using only valid ledger"
    );

    // The expired ledger must NOT be touched
    let expired_ledger = get_ledger_by_id(ctx, expired_ledger_id).await;
    assert_eq!(
        expired_ledger.used_amount, 0,
        "Expired-unmarked ledger must not be consumed"
    );
    assert_eq!(
        expired_ledger.remaining_amount, 10000,
        "Expired-unmarked ledger remaining must be untouched"
    );

    // The valid ledger must be consumed
    let valid_ledger = get_ledger_by_id(ctx, valid_ledger_id).await;
    assert_eq!(
        valid_ledger.used_amount, 5000,
        "Valid ledger should be fully consumed"
    );
    assert_eq!(
        valid_ledger.remaining_amount, 0,
        "Valid ledger remaining should be 0"
    );
}

// ============================================================================
// Test 2: Consume fails when only expired-unmarked ledgers exist
// ============================================================================
//
// If all ledgers are past expires_at (but status='active'), consumption
// should fail with insufficient balance — not consume the expired credits.
//
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_consume_fails_when_only_expired_unmarked_ledgers(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id =
        create_test_user(&ctx.app_state.pool, &realm_id, "regression76b@example.com").await;

    create_points_wallet(ctx, user_id, &realm_id).await;

    // Only ledger: expired in the past but status='active'
    let expired_at = Utc::now() - Duration::minutes(30);
    create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        CreditSourceType::SubscriptionInitial,
        Uuid::now_v7().to_string(),
        10000,
        Some(expired_at),
    )
    .await;

    // Attempt to consume — should fail because the only ledger is expired
    let identity = create_test_third_party_identity(&realm_id);
    let input = ConsumePointsInput {
        user_id: user_id.to_string(),
        client_app_id: ctx._client_app_id.clone(),
        amount: 100,
        description: Some("regression: only expired ledgers".to_string()),
    };
    let result = ctx
        .app_state
        .points_service
        .consume_points(identity, &realm_id, input)
        .await;

    assert!(
        result.is_err(),
        "Consume must fail when all ledgers are expired"
    );
}

// ============================================================================
// Test 3: grant_points rejected for frozen wallet (in-tx status check)
// ============================================================================
//
// Regression guard for: grant_points_atomic now checks wallet.status
// inside the transaction. Previously the check was only at the service
// layer (outside tx), creating a TOCTOU window.
//
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_grant_points_rejected_for_frozen_wallet(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id =
        create_test_user(&ctx.app_state.pool, &realm_id, "regression76c@example.com").await;

    let wallet_id = create_points_wallet(ctx, user_id, &realm_id).await;

    // Freeze the wallet directly in DB (bypasses service-layer checks)
    sqlx::query("UPDATE points_wallets SET status = 'frozen' WHERE id = $1")
        .bind(wallet_id)
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to freeze wallet");

    // Credit-bucket: GrantPointsInput now requires a bucket_id. Create a real
    // bucket as the grant target — the wallet-status rejection happens before
    // the ledger is written, so the bucket is never used at runtime here.
    use crate::tests::helpers::credit_bucket_helpers::{
        CreditBucketOpts, create_test_credit_bucket,
    };
    let bucket_id =
        create_test_credit_bucket(&ctx.app_state.pool, &realm_id, CreditBucketOpts::default())
            .await;

    // Attempt to grant points — service layer reads status='frozen' and should reject
    let input = GrantPointsInput {
        user_id,
        bucket_id,
        source_type: CreditSourceType::AdminGrant,
        amount: 100,
        reason: "regression test: grant to frozen wallet".to_string(),
        source_id: Uuid::now_v7().to_string(),
        validity_days: None,
    };
    let result = ctx
        .app_state
        .points_service
        .grant_points_for_sdk(&realm_id, input)
        .await;

    assert!(
        result.is_err(),
        "grant_points must be rejected for frozen wallet"
    );

    // Verify no ledger was created
    let ledger_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM points_credit_ledger WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&ctx.app_state.pool)
            .await
            .expect("Failed to count ledgers");

    assert_eq!(
        ledger_count, 0,
        "No ledger should be created for frozen wallet"
    );
}

// ============================================================================
// Test 4: grant_points rejected for closed wallet (in-tx status check)
// ============================================================================
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_grant_points_rejected_for_closed_wallet(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id =
        create_test_user(&ctx.app_state.pool, &realm_id, "regression76d@example.com").await;

    let wallet_id = create_points_wallet(ctx, user_id, &realm_id).await;

    // Close the wallet directly in DB
    sqlx::query("UPDATE points_wallets SET status = 'closed' WHERE id = $1")
        .bind(wallet_id)
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to close wallet");

    // Credit-bucket: GrantPointsInput now requires a bucket_id.
    use crate::tests::helpers::credit_bucket_helpers::{
        CreditBucketOpts, create_test_credit_bucket,
    };
    let bucket_id =
        create_test_credit_bucket(&ctx.app_state.pool, &realm_id, CreditBucketOpts::default())
            .await;

    let input = GrantPointsInput {
        user_id,
        bucket_id,
        source_type: CreditSourceType::AdminGrant,
        amount: 100,
        reason: "regression test: grant to closed wallet".to_string(),
        source_id: Uuid::now_v7().to_string(),
        validity_days: None,
    };
    let result = ctx
        .app_state
        .points_service
        .grant_points_for_sdk(&realm_id, input)
        .await;

    assert!(
        result.is_err(),
        "grant_points must be rejected for closed wallet"
    );
}
