// =============================================================================
// Points System Scenario Test 8: Consume Success
// =============================================================================
//
// **User Story**: US-PU-01 (balance update), SDK consumption
// **Priority**: P0
//
// **Scenario**: Third Party Consumes Points Successfully
//
// **Given**:
// - A valid API Key for a client app
// - A user with points account balance 5000
// - The API Key belongs to realm matching user's realm
//
// **When**:
// - The third party calls `POST /api/ext/points/{realmId}/consume` with:
//   - userId: valid user UUID
//   - clientAppId: valid client app UUID
//   - amount: 100
//   - description: "AI API call"
//
// **Then**:
// - Response contains transactionId
// - Response contains amount: -100
// - Response contains balanceAfter: 4900
// - HTTP status is 200 OK
// - Account balance is updated to 4900
// - Total consumed is increased by 100
// - A transaction record is created
//
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

/// ============================================================================
/// Scenario 2.1: Third Party Consumes Points Successfully
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_consume_points_success(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    // ============================================================================
    // Given: A valid API Key, user with points account
    // ============================================================================
    println!("[Step 1] Create test user and API Key");

    let user_id = create_test_user(&ctx._app_state.pool, &ctx._realm_id, "user8@example.com").await;
    let initial_balance = 5000;

    let wallet_id = create_test_points_wallet(&ctx._app_state.pool, user_id, initial_balance).await;

    let client_app_id = create_test_client_app(&ctx._app_state.pool, &ctx._realm_id).await;
    let api_key = create_test_api_key(&ctx._app_state.pool, &ctx._realm_id, client_app_id).await;

    let consume_amount = 100;
    let description = "AI API call";

    println!(
        "[Step 1] ✓ Test data created: user={}, account={}, client_app={}, api_key={}",
        user_id, wallet_id, client_app_id, api_key
    );

    // ============================================================================
    // When: Third party consumes points
    // ============================================================================
    println!("[Step 2] Third party consumes points");

    let consume_payload = json!({
        "userId": user_id.to_string(),
        "clientAppId": client_app_id.to_string(),
        "amount": consume_amount,
        "description": description.to_string()
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
    // Then: Verify consumption response and database state
    // ============================================================================
    println!("[Step 3] Verify consumption response");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Consumption should succeed with 200 OK"
    );

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");
    let body: serde_json::Value =
        serde_json::from_slice(&body_bytes).expect("Failed to parse JSON");

    assert!(
        body["transactions"].is_array(),
        "Response should contain a transactions array (multi-bucket consume, design §4.2.2)"
    );
    let transactions = body["transactions"].as_array().unwrap();
    assert_eq!(
        transactions.len(),
        1,
        "Single-pool consume produces exactly one per-bucket transaction"
    );
    let txn = &transactions[0];
    assert!(
        txn["transactionId"].is_string(),
        "Per-bucket transaction should contain transactionId"
    );
    assert_eq!(
        body["amount"].as_i64(),
        Some(consume_amount),
        "Response amount should be the total consumed (100)"
    );
    assert_eq!(
        txn["amount"].as_i64(),
        Some(consume_amount),
        "Per-bucket transaction amount should be 100 (deduction magnitude)"
    );
    assert_eq!(
        txn["balanceAfter"].as_i64(),
        Some(initial_balance - consume_amount),
        "Per-bucket transaction balanceAfter should be 4900"
    );

    let expected_balance_after = initial_balance - consume_amount;
    println!(
        "[Step 3] ✓ Response verified: transactionId={}, amount={}, balanceAfter={}",
        txn["transactionId"], txn["amount"], txn["balanceAfter"]
    );

    // Verify database state
    println!("[Step 4] Verify database state");

    let (new_balance, total_consumed): (i64, i64) =
        sqlx::query_as("SELECT total_balance, total_consumed FROM points_wallets WHERE id = $1")
            .bind(wallet_id)
            .fetch_one(&ctx._app_state.pool)
            .await
            .expect("Failed to fetch account");

    assert_eq!(
        new_balance, expected_balance_after,
        "Account balance should be updated to 4900"
    );
    assert_eq!(
        total_consumed, consume_amount,
        "Total consumed should be increased by 100"
    );

    println!(
        "[Step 4] ✓ Database verified: balance={}, total_consumed={}",
        new_balance, total_consumed
    );

    // Verify transaction was created
    println!("[Step 5] Verify transaction record");

    let (txn_type, txn_amount, txn_balance_after, txn_description): (
        String,
        i64,
        i64,
        Option<String>,
    ) = sqlx::query_as(
        "SELECT type, amount, balance_after, description
             FROM points_transactions
             WHERE user_id = $1 AND type = 'consume'
             ORDER BY created_at DESC
             LIMIT 1",
    )
    .bind(user_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to fetch transaction");

    assert_eq!(txn_type, "consume", "Transaction type should be consume");
    assert_eq!(
        txn_amount, -consume_amount,
        "Transaction amount should be -100"
    );
    assert_eq!(
        txn_balance_after, expected_balance_after,
        "Transaction balance_after should be 4900"
    );
    // Verify transaction description
    assert!(
        txn_description.is_some(),
        "Transaction description should exist"
    );
    assert_eq!(
        txn_description.unwrap(),
        description,
        "Transaction description should match"
    );

    println!(
        "[Step 5] ✓ Transaction verified: type={}, amount={}",
        txn_type, txn_amount
    );

    println!("\n✅ Scenario 2.1 完成：第三方成功消耗积分");
}
