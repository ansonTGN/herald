// =============================================================================
// Idempotency Guard Tests
// =============================================================================
//
// Tests that verify DB-level idempotency guards prevent duplicate operations.
//
// 1. grant_points_for_sdk: duplicate call returns zero-amount placeholder
// 2. revoke_subscription_credits_by_entitlement_atomic: duplicate call returns total_revoked=0
// 3. revoke_topup_proportional_atomic: duplicate call returns total_revoked=0
//
// These guards use check_completed_idempotency_in_tx and
// record_completed_idempotency_in_tx to prevent duplicate ledger creation
// or revocation when the same operation is retried.
//
// =============================================================================

use crate::tests::helpers::points_helpers::*;
use crate::tests::scenarios::points::fixtures::*;
use crate::tests::schema_test_context::SchemaTestContext;
use herald_core::domain::points::dtos::RevokePointsOutput;
use herald_core::domain::points::entities::{CreditSourceType, CreditType, RevocationType};
use herald_core::domain::points::ports::PointsRepository;
use test_context::test_context;
use uuid::Uuid;

// ============================================================================
// Test 1: grant_points_internal idempotency prevents duplicate ledger
// ============================================================================
//
// User Story: As a billing system, when I retry a grant-points request with
// an explicit idempotency key, I must not create a duplicate ledger or
// inflate the user's balance.
//
// Covers: grant_points_atomic idempotency guard (line ~3864-3889)
// Idempotency key: caller-provided via grant_points_internal parameter
//
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_grant_idempotency_prevents_duplicate_ledger(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id =
        create_test_user(&ctx.app_state.pool, &realm_id, "idempotency77a@example.com").await;

    create_points_wallet(ctx, user_id, &realm_id).await;

    let source_id = Uuid::now_v7().to_string();
    let idempotency_key = format!("grant:AdminGrant:{}", source_id);

    // First grant should succeed
    let result1 = ctx
        .app_state
        .points_service
        .grant_points_internal(
            &realm_id,
            user_id,
            CreditType::GrantedCredit,
            CreditSourceType::AdminGrant,
            500,
            None,
            Some(source_id.clone()),
            Some("idempotency test: first grant".to_string()),
            Some(idempotency_key.clone()),
        )
        .await;

    assert!(result1.is_ok(), "First grant should succeed: {:?}", result1);

    // Second grant with the same idempotency key should be idempotent
    let result2 = ctx
        .app_state
        .points_service
        .grant_points_internal(
            &realm_id,
            user_id,
            CreditType::GrantedCredit,
            CreditSourceType::AdminGrant,
            500,
            None,
            Some(source_id.clone()),
            Some("idempotency test: duplicate grant".to_string()),
            Some(idempotency_key),
        )
        .await;

    assert!(
        result2.is_ok(),
        "Second grant should succeed (idempotent response): {:?}",
        result2
    );

    // Verify only one real ledger exists for this user
    let ledgers = get_user_ledgers(ctx, user_id).await;
    let non_idempotency_ledgers: Vec<_> = ledgers
        .iter()
        .filter(|l| l.source_id != "idempotency")
        .collect();

    assert_eq!(
        non_idempotency_ledgers.len(),
        1,
        "Exactly one real ledger should exist (no duplicates)"
    );
    assert_eq!(
        non_idempotency_ledgers[0].granted_amount, 500,
        "Real ledger should have granted_amount=500"
    );

    // Verify the wallet balance is not inflated
    let balance: i64 = sqlx::query_scalar(
        "SELECT total_balance FROM points_wallets WHERE user_id = $1 AND realm_id = $2",
    )
    .bind(user_id)
    .bind(&realm_id)
    .fetch_one(&ctx.app_state.pool)
    .await
    .expect("Failed to fetch wallet balance");

    assert_eq!(
        balance, 500,
        "Wallet balance should be 500 (not inflated by duplicate grant)"
    );
}

// ============================================================================
// Test 2: revoke_subscription_credits_by_entitlement idempotency
// ============================================================================
//
// User Story: As a billing system, when I retry a subscription credit
// revocation with the same idempotency_key, I must not create a duplicate
// revocation record or revoke more credits than intended.
//
// Covers: revoke_subscription_credits_by_entitlement_atomic (line ~3493-3507)
// Idempotency key: caller-provided via idempotency_key parameter
//
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_revoke_subscription_by_entitlement_idempotency(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id =
        create_test_user(&ctx.app_state.pool, &realm_id, "idempotency77b@example.com").await;

    create_points_wallet(ctx, user_id, &realm_id).await;

    // Create a subscription credit ledger with a known entitlement source_id
    let entitlement_key = Uuid::now_v7().to_string();
    let ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        CreditSourceType::SubscriptionInitial,
        entitlement_key.clone(),
        1000,
        None,
    )
    .await;

    let idempotency_key = format!("revoke:sub:{}", entitlement_key);

    // First revocation should succeed
    let result1 = ctx
        .app_state
        .points_repository
        .revoke_subscription_credits_by_entitlement_atomic(
            &realm_id,
            user_id,
            &entitlement_key,
            RevocationType::CancelRevoke,
            "idempotency test: first revoke".to_string(),
            None,
            Some(idempotency_key.clone()),
        )
        .await;

    assert!(
        result1.is_ok(),
        "First revoke should succeed: {:?}",
        result1
    );
    let output1 = result1.unwrap();
    assert_eq!(
        output1.total_revoked, 1000,
        "First revoke should revoke full 1000"
    );
    assert!(
        !output1.ledger_ids.is_empty(),
        "First revoke should include ledger IDs"
    );

    // Verify the ledger is now revoked
    let ledger = get_ledger_by_id(ctx, ledger_id).await;
    assert_eq!(
        ledger.revoked_amount, 1000,
        "Ledger should show 1000 revoked after first revoke"
    );

    // Record revocation count before second call
    let revocation_count_before = get_revocation_records(ctx, user_id).await.len();

    // Second revocation with the same idempotency_key should be idempotent
    let result2 = ctx
        .app_state
        .points_repository
        .revoke_subscription_credits_by_entitlement_atomic(
            &realm_id,
            user_id,
            &entitlement_key,
            RevocationType::CancelRevoke,
            "idempotency test: duplicate revoke".to_string(),
            None,
            Some(idempotency_key),
        )
        .await;

    assert!(
        result2.is_ok(),
        "Second revoke should succeed (idempotent response): {:?}",
        result2
    );
    let output2 = result2.unwrap();
    assert_eq!(
        output2.total_revoked, 0,
        "Second revoke should return total_revoked=0 (idempotent)"
    );
    assert!(
        output2.ledger_ids.is_empty(),
        "Second revoke should return empty ledger_ids"
    );

    // Verify no additional revocation record was created
    let revocation_count_after = get_revocation_records(ctx, user_id).await.len();
    assert_eq!(
        revocation_count_before, revocation_count_after,
        "No new revocation record should be created on duplicate call"
    );
}

// ============================================================================
// Test 3: revoke_topup_proportional idempotency
// ============================================================================
//
// User Story: As a billing system, when I retry a topup proportional
// revocation with the same refund_id, I must not revoke additional credits.
//
// Covers: revoke_topup_proportional_atomic (line ~3621-3634)
// Idempotency key: "refund:topup:{refund_id}"
//
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_revoke_topup_proportional_idempotency(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id =
        create_test_user(&ctx.app_state.pool, &realm_id, "idempotency77c@example.com").await;

    create_points_wallet(ctx, user_id, &realm_id).await;

    // Create a topup credit ledger
    let source_id = Uuid::now_v7().to_string();
    let ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::TopupCredit,
        CreditSourceType::Topup,
        source_id,
        2000,
        None,
    )
    .await;

    let refund_id = Uuid::now_v7().to_string();

    // First revocation: revoke half (1000 out of 2000)
    let result1: Result<RevokePointsOutput, _> = ctx
        .app_state
        .points_repository
        .revoke_topup_proportional_atomic(
            &realm_id, user_id, 1000, // refund_amount
            2000, // original_payment_amount
            &refund_id,
        )
        .await;

    assert!(
        result1.is_ok(),
        "First topup revoke should succeed: {:?}",
        result1
    );
    let output1 = result1.unwrap();
    assert!(
        output1.total_revoked > 0,
        "First revoke should revoke some credits, got {}",
        output1.total_revoked
    );

    // Verify the ledger was partially revoked
    let ledger = get_ledger_by_id(ctx, ledger_id).await;
    assert!(
        ledger.revoked_amount > 0,
        "Ledger should show some revoked amount after first revoke"
    );

    // Record revocation count before second call
    let revocation_count_before = get_revocation_records(ctx, user_id).await.len();

    // Second revocation with the same refund_id should be idempotent
    let result2: Result<RevokePointsOutput, _> = ctx
        .app_state
        .points_repository
        .revoke_topup_proportional_atomic(
            &realm_id, user_id, 1000, // refund_amount
            2000, // original_payment_amount
            &refund_id,
        )
        .await;

    assert!(
        result2.is_ok(),
        "Second topup revoke should succeed (idempotent response): {:?}",
        result2
    );
    let output2 = result2.unwrap();
    assert_eq!(
        output2.total_revoked, 0,
        "Second revoke should return total_revoked=0 (idempotent)"
    );
    assert!(
        output2.ledger_ids.is_empty(),
        "Second revoke should return empty ledger_ids"
    );

    // Verify no additional revocation record was created
    let revocation_count_after = get_revocation_records(ctx, user_id).await.len();
    assert_eq!(
        revocation_count_before, revocation_count_after,
        "No new revocation record should be created on duplicate call"
    );
}
