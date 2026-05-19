// =============================================================================
// Test: Mixed Credit Type Concurrent Consumption
// =============================================================================
//
// Tests that when a user has both topup and subscription credits, concurrent
// consume requests result in exactly one success and one failure when the
// second request cannot be satisfied.
//
// User Story: docs/user-stories/points-billing-events.md
// Covers: Concurrency safety for mixed credit type consumption
//
// =============================================================================

use crate::tests::helpers::points_helpers::*;
use crate::tests::scenarios::points::fixtures::*;
use crate::tests::schema_test_context::SchemaTestContext;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use herald_core::domain::points::entities::{CreditSourceType, CreditType};
use test_context::test_context;
use tower::ServiceExt;
use uuid::Uuid;

// User Story: docs/user-stories/points-billing-events.md
// Covers: Mixed topup(6000) + subscription(4000) with 2 concurrent consume(6000)
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_mixed_credit_concurrent_consume_one_succeeds(ctx: &mut SchemaTestContext) {
    // Given: User has topup 6000 + subscription 4000 (total 10000)
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user72@example.com").await;
    let plan_id = Uuid::now_v7();

    create_points_account(ctx, user_id, &realm_id).await;

    let topup_ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::TopupCredit,
        CreditSourceType::Topup,
        format!("topup_{}", Uuid::now_v7()),
        6000,
        None,
    )
    .await;

    let sub_ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        CreditSourceType::SubscriptionInitial,
        plan_id.to_string(),
        4000,
        None,
    )
    .await;

    // Prepare 2 consume requests of 6000 each
    // First consume(6000): uses subscription 4000 + topup 2000, leaves topup 4000
    // Second consume(6000): needs 6000 but only 4000 topup remains -> fail
    let client_app_id = create_test_client_app(&ctx.app_state.pool, &realm_id).await;
    let api_key = create_test_api_key(&ctx.app_state.pool, &realm_id, client_app_id).await;

    let consume_amount: i64 = 6000;
    let make_consume_payload = || {
        serde_json::json!({
            "userId": user_id.to_string(),
            "clientAppId": client_app_id.to_string(),
            "amount": consume_amount,
            "description": "Mixed credit concurrent consume"
        })
    };

    let make_consume_request = |payload: serde_json::Value| {
        Request::builder()
            .method("POST")
            .uri(format!("/api/ext/points/{}/consume", realm_id))
            .header("content-type", "application/json")
            .header("X-API-Key", &api_key)
            .body(Body::from(payload.to_string()))
            .unwrap()
    };

    let app = ctx.create_unified_test_router();

    let req1 = make_consume_request(make_consume_payload());
    let req2 = make_consume_request(make_consume_payload());

    // When: Fire both consume requests concurrently
    let (res1, res2) = tokio::join!(app.clone().oneshot(req1), app.clone().oneshot(req2));

    let res1 = res1.expect("consume 1 response");
    let res2 = res2.expect("consume 2 response");

    let statuses = [res1.status(), res2.status()];
    let success_count = statuses.iter().filter(|&&s| s == StatusCode::OK).count();
    let failure_count = statuses
        .iter()
        .filter(|&&s| s == StatusCode::CONFLICT || s == StatusCode::BAD_REQUEST)
        .count();

    // Then: Exactly 1 success and 1 failure
    assert_eq!(
        success_count, 1,
        "exactly 1 consume should succeed, got {:?}",
        statuses
    );
    assert_eq!(
        failure_count, 1,
        "exactly 1 consume should fail (insufficient balance), got {:?}",
        statuses
    );

    // 1. Balances must be non-negative
    let (total_balance, topup_balance, subscription_balance) =
        assert_balances_non_negative(ctx, user_id, &realm_id).await;

    // 2. Exactly 1 consume transaction
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

    // 3. Ledger accounting invariant
    assert_ledger_invariants(ctx, user_id).await;

    // 4. Account balance = SUM(ledger.remaining_amount)
    assert_account_matches_ledger_sums(
        ctx,
        user_id,
        &realm_id,
        topup_balance,
        subscription_balance,
    )
    .await;

    // 5. Verify per-credit-type ledger consistency
    let topup_ledger = get_ledger_by_id(ctx, topup_ledger_id).await;
    assert!(
        topup_ledger.remaining_amount >= 0,
        "topup ledger remaining must be >= 0"
    );

    let sub_ledger = get_ledger_by_id(ctx, sub_ledger_id).await;
    assert!(
        sub_ledger.remaining_amount >= 0,
        "subscription ledger remaining must be >= 0"
    );

    eprintln!(
        "Result: total={}, topup={}, sub={}, consume_txns={}, statuses={:?}",
        total_balance, topup_balance, subscription_balance, consume_tx_count, statuses
    );
}
