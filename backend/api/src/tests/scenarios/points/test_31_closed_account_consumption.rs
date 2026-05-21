// =============================================================================
// Points System Scenario Test 31: Closed Account Consumption
// =============================================================================
//
// **User Story**: Account state management
// **Priority**: P2
//
// **Scenario**: Closed Account Cannot Be Consumed
//
// **Given**:
// - A valid API Key
// - A user with closed points account
//
// **When**:
// - The third party calls `POST /api/{realmId}/points/consume`
//
// **Then**:
// - HTTP status is 403 Forbidden or 400 Bad Request
// - Error indicates account is closed
// - No transaction is created
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

#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_closed_account_cannot_consume(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    println!("[Step 1] Create user with closed account");

    let user_id =
        create_test_user(&ctx._app_state.pool, &ctx._realm_id, "user31@example.com").await;
    let balance = 1000;

    let wallet_id = create_test_points_wallet(&ctx._app_state.pool, user_id, balance).await;

    // Set account status to closed
    sqlx::query("UPDATE points_wallets SET status = 'closed' WHERE id = $1")
        .bind(wallet_id)
        .execute(&ctx._app_state.pool)
        .await
        .expect("Failed to set account status to closed");

    let client_app_id = create_test_client_app(&ctx._app_state.pool, &ctx._realm_id).await;
    let api_key = create_test_api_key(&ctx._app_state.pool, &ctx._realm_id, client_app_id).await;

    println!("[Step 1] ✓ Created user with closed account");

    println!("[Step 2] Attempt to consume from closed account");

    let consume_payload = json!({
        "userId": user_id.to_string(),
        "clientAppId": client_app_id.to_string(),
        "amount": 100,
        "description": "Attempt to consume from closed account"
    });

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/ext/points/{}/consume", ctx._realm_id))
        .header("content-type", "application/json")
        .header("X-API-Key", api_key)
        .body(Body::from(consume_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::FORBIDDEN || response.status() == StatusCode::BAD_REQUEST,
        "Should return 403 Forbidden or 400 Bad Request for closed account"
    );

    println!("[Step 3] Verify no transaction was created");

    let transaction_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM points_transactions WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&ctx._app_state.pool)
            .await
            .expect("Failed to count transactions");

    assert_eq!(
        transaction_count, 0,
        "No transaction should be created for closed account"
    );

    println!("\n✅ Scenario 6.2 完成：关闭账户无法消耗积分");
}
