// =============================================================================
// Points System Scenario Test 9: Consume Insufficient Points
// =============================================================================
//
// **User Story**: US-PU-01 (insufficient points error)
// **Priority**: P0
//
// **Scenario**: Consumption Fails with Insufficient Points
//
// **Given**:
// - A valid API Key
// - A user with points account balance 50
//
// **When**:
// - The third party calls `POST /api/{realmId}/points/consume` with amount: 100
//
// **Then**:
// - HTTP status is 400 Bad Request
// - Error code is "insufficient_points"
// - Balance remains unchanged at 50
//
// =============================================================================

use crate::tests::helpers::test_setup_helpers::assert_response_error;
use crate::tests::scenarios::points::fixtures::*;
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use test_context::test_context;
use tower::ServiceExt;

/// ============================================================================
/// Scenario 2.2: Consumption Fails with Insufficient Points
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_consume_insufficient_points(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    // ============================================================================
    // Given: A valid API Key, user with low balance
    // ============================================================================
    println!("[Step 1] Create test user with low balance");

    let user_id = create_test_user(&ctx._app_state.pool, &ctx._realm_id, "user9@example.com").await;
    let balance = 50;

    let account_id = create_test_points_account(&ctx._app_state.pool, user_id, balance).await;

    let client_app_id = create_test_client_app(&ctx._app_state.pool, &ctx._realm_id).await;
    let api_key = create_test_api_key(&ctx._app_state.pool, &ctx._realm_id, client_app_id).await;

    println!(
        "[Step 1] ✓ Test data created: user={}, account={}, balance={}",
        user_id, account_id, balance
    );

    // ============================================================================
    // When: Third party tries to consume more than balance
    // ============================================================================
    println!(
        "[Step 2] Third party attempts to consume {} points",
        balance * 2
    );

    let consume_amount = balance * 2; // Try to consume 100 when balance is 50

    let consume_payload = json!({
        "userId": user_id.to_string(),
        "clientAppId": client_app_id.to_string(),
        "amount": consume_amount,
        "description": "Attempt to consume more than balance"
    });

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/ext/points/{}/consume", ctx._realm_id))
        .header("content-type", "application/json")
        .header("X-API-Key", api_key)
        .body(Body::from(consume_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();

    // ============================================================================
    // Then: Verify 400 Bad Request with insufficient_points error
    // ============================================================================
    println!("[Step 3] Verify 400 Bad Request");

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Should return 400 Bad Request for insufficient points"
    );

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");
    let body: serde_json::Value =
        serde_json::from_slice(&body_bytes).expect("Failed to parse JSON");

    assert_response_error(&body, None);

    let error_code = body
        .get("error")
        .and_then(|v| v.as_str())
        .or_else(|| body.get("message").and_then(|v| v.as_str()))
        .unwrap_or("");

    assert!(
        error_code.contains("insufficient") || error_code.contains("balance"),
        "Error code should be 'insufficient_points' or contain 'balance', got: {}",
        error_code
    );

    println!(
        "[Step 3] ✓ 400 Bad Request returned with error code: {}",
        error_code
    );

    // Verify balance remains unchanged
    println!("[Step 4] Verify balance unchanged");

    let (current_balance, total_consumed): (i64, i64) =
        sqlx::query_as("SELECT total_balance, total_consumed FROM points_accounts WHERE id = $1")
            .bind(account_id)
            .fetch_one(&ctx._app_state.pool)
            .await
            .expect("Failed to fetch account");

    assert_eq!(
        current_balance, balance,
        "Balance should remain unchanged at 50"
    );
    assert_eq!(total_consumed, 0, "Total consumed should remain 0");

    println!(
        "[Step 4] ✓ Balance verified: balance={}, total_consumed={}",
        current_balance, total_consumed
    );

    // Verify no transaction was created
    println!("[Step 5] Verify no transaction was created");

    let transaction_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM points_transactions WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&ctx._app_state.pool)
            .await
            .expect("Failed to count transactions");

    assert_eq!(
        transaction_count, 0,
        "No transaction should be created for failed consumption"
    );

    println!(
        "[Step 5] ✓ No transaction created: transaction_count={}",
        transaction_count
    );

    println!("\n✅ Scenario 2.2 完成：积分不足时正确拒绝消耗请求");
}
