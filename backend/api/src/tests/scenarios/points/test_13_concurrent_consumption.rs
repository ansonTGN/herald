// =============================================================================
// Points System Scenario Test 13: Concurrent Consumption
// =============================================================================

use crate::tests::scenarios::points::fixtures::*;
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use test_context::test_context;
use tower::ServiceExt;

#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_concurrent_consumption_prevents_overspending(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    let user_id =
        create_test_user(&ctx._app_state.pool, &ctx._realm_id, "user13@example.com").await;
    let balance = 100;
    let wallet_id = create_test_points_wallet(&ctx._app_state.pool, user_id, balance).await;

    let client_app_id = create_test_client_app(&ctx._app_state.pool, &ctx._realm_id).await;
    let api_key = create_test_api_key(&ctx._app_state.pool, &ctx._realm_id, client_app_id).await;

    let consume_payload = json!({
        "userId": user_id.to_string(),
        "clientAppId": client_app_id.to_string(),
        "amount": 80,
        "description": "Concurrent consumption"
    });

    let request1 = Request::builder()
        .method("POST")
        .uri(format!("/api/ext/points/{}/consume", ctx._realm_id))
        .header("content-type", "application/json")
        .header("X-API-Key", api_key.clone())
        .body(Body::from(consume_payload.to_string()))
        .unwrap();

    let request2 = Request::builder()
        .method("POST")
        .uri(format!("/api/ext/points/{}/consume", ctx._realm_id))
        .header("content-type", "application/json")
        .header("X-API-Key", api_key)
        .body(Body::from(consume_payload.to_string()))
        .unwrap();

    let (response1, response2) =
        tokio::join!(app.clone().oneshot(request1), app.clone().oneshot(request2));

    let response1 = response1.expect("first response should be returned");
    let response2 = response2.expect("second response should be returned");
    let statuses = [response1.status(), response2.status()];

    let success_count = statuses.iter().filter(|&&s| s == StatusCode::OK).count();
    let failure_count = statuses
        .iter()
        .filter(|&&s| s == StatusCode::CONFLICT || s == StatusCode::BAD_REQUEST)
        .count();

    assert_eq!(
        success_count, 1,
        "exactly one concurrent request should succeed: {:?}",
        statuses
    );
    assert_eq!(
        failure_count, 1,
        "exactly one concurrent request should fail: {:?}",
        statuses
    );

    let (final_balance,): (i64,) =
        sqlx::query_as("SELECT total_balance FROM points_wallets WHERE id = $1")
            .bind(wallet_id)
            .fetch_one(&ctx._app_state.pool)
            .await
            .expect("Failed to fetch account");
    assert_eq!(
        final_balance, 20,
        "final balance should reflect a single successful consumption"
    );

    let (consume_tx_count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM points_transactions WHERE user_id = $1 AND type = 'consume'",
    )
    .bind(user_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to count consume transactions");
    assert_eq!(
        consume_tx_count, 1,
        "should persist exactly one consume transaction"
    );

    let (allocation_count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM points_consumption_allocations WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&ctx._app_state.pool)
            .await
            .expect("Failed to count allocations");
    assert_eq!(
        allocation_count, 1,
        "should persist allocations only for the successful request"
    );

    let (negative_balance_count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM points_wallets WHERE id = $1 AND total_balance < 0")
            .bind(wallet_id)
            .fetch_one(&ctx._app_state.pool)
            .await
            .expect("Failed to verify final balance");
    assert_eq!(
        negative_balance_count, 0,
        "account balance must never go negative"
    );
}
