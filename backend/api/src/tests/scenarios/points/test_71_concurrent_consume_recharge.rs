// =============================================================================
// Test: Concurrent Consume + Recharge
// =============================================================================
//
// Tests that concurrent consume and recharge operations maintain data
// consistency and accounting invariants.
//
// User Story: docs/user-stories/points-billing-events.md
// Covers: Concurrency safety for consume + recharge race conditions
//
// =============================================================================

use crate::tests::helpers::points_helpers::*;
use crate::tests::scenarios::points::fixtures::*;
use crate::tests::schema_test_context::SchemaTestContext;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use herald_core::domain::points::entities::{CreditSourceType, CreditType, RechargeType};
use test_context::test_context;
use tower::ServiceExt;
use uuid::Uuid;

// User Story: docs/user-stories/points-billing-events.md
// Covers: Concurrency safety - consume(3000) + recharge(4000) on balance 5000
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_concurrent_consume_recharge_final_balance(ctx: &mut SchemaTestContext) {
    // Given: User has topup balance 5000
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user71@example.com").await;

    create_points_account(ctx, user_id, &realm_id).await;

    let payment_id = format!("payment_{}", Uuid::now_v7());

    let _ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::TopupCredit,
        CreditSourceType::Topup,
        payment_id.clone(),
        5000,
        None,
    )
    .await;

    // Prepare consume: 3000 via SDK API
    let client_app_id = create_test_client_app(&ctx.app_state.pool, &realm_id).await;
    let api_key = create_test_api_key(&ctx.app_state.pool, &realm_id, client_app_id).await;

    let consume_payload = serde_json::json!({
        "userId": user_id.to_string(),
        "clientAppId": client_app_id.to_string(),
        "amount": 3000,
        "description": "Concurrent consume"
    });

    let app = ctx.create_unified_test_router();

    let consume_request = Request::builder()
        .method("POST")
        .uri(format!("/api/ext/points/{}/consume", realm_id))
        .header("content-type", "application/json")
        .header("X-API-Key", &api_key)
        .body(Body::from(consume_payload.to_string()))
        .unwrap();

    // Prepare recharge: 4000 via service call
    let recharge_fut = ctx.app_state.points_service.recharge_points_internal(
        &realm_id,
        user_id,
        4000,
        RechargeType::Subscribe,
        Some(format!("recharge_ref_{}", Uuid::now_v7())),
        None, // expires_at: no expiration for test
    );

    // When: Fire consume and recharge concurrently
    let consume_fut = app.clone().oneshot(consume_request);

    let (consume_result, recharge_result) = tokio::join!(consume_fut, recharge_fut);

    let consume_response = consume_result.expect("consume response should be returned");
    let consume_status = consume_response.status();

    let recharge_is_ok = recharge_result.is_ok();

    // Both operations should succeed
    assert_eq!(
        consume_status,
        StatusCode::OK,
        "consume should succeed, got {}",
        consume_status
    );
    assert!(
        recharge_is_ok,
        "recharge should succeed, got {:?}",
        recharge_result.as_ref().err()
    );

    // 1. Balances must be non-negative
    let (total_balance, topup_balance, subscription_balance) =
        assert_balances_non_negative(ctx, user_id, &realm_id).await;

    // 2. Exactly 1 consume transaction and 1 recharge transaction
    let consume_tx_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM points_transactions WHERE user_id = $1 AND type = 'consume'",
    )
    .bind(user_id)
    .fetch_one(&ctx.app_state.pool)
    .await
    .expect("Failed to count consume transactions");

    assert_eq!(
        consume_tx_count, 1,
        "should have exactly 1 consume transaction, got {}",
        consume_tx_count
    );

    let recharge_tx_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM points_transactions WHERE user_id = $1 AND type = 'subscription_grant'",
    )
    .bind(user_id)
    .fetch_one(&ctx.app_state.pool)
    .await
    .expect("Failed to count recharge transactions");

    assert_eq!(
        recharge_tx_count, 1,
        "should have exactly 1 subscription_grant transaction, got {}",
        recharge_tx_count
    );

    // 3. Ledger accounting invariant
    assert_ledger_invariants(ctx, user_id).await;

    // 4. Account balance must match ledger source of truth
    assert_account_matches_ledger_sums(
        ctx,
        user_id,
        &realm_id,
        topup_balance,
        subscription_balance,
    )
    .await;

    eprintln!(
        "Result: total_balance={}, topup={}, subscription={}",
        total_balance, topup_balance, subscription_balance
    );
}
