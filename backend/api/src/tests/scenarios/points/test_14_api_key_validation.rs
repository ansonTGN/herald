// =============================================================================
// Points System Scenario Test 14: API Key Validation
// =============================================================================
//
// **User Story**: API Key validation
// **Priority**: P0
//
// **Scenario**: Consumption Requires Valid API Key
//
// **Given**:
// - An invalid or expired API Key
// - A user with points account
//
// **When**:
// - The third party calls `POST /api/{realmId}/points/consume` with invalid API Key
//
// **Then**:
// - HTTP status is 401 Unauthorized
// - No points are deducted
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
async fn test_scenario_api_key_validation(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    println!("[Step 1] Create test user");

    let user_id =
        create_test_user(&ctx._app_state.pool, &ctx._realm_id, "user14@example.com").await;
    let balance = 5000;

    let account_id = create_test_points_account(&ctx._app_state.pool, user_id, balance).await;

    let client_app_id = create_test_client_app(&ctx._app_state.pool, &ctx._realm_id).await;
    let invalid_api_key = "invalid-api-key-12345";

    println!(
        "[Step 1] ✓ Test data created: user={}, account={}, balance={}",
        user_id, account_id, balance
    );

    println!("[Step 2] Attempt to consume with invalid API Key");

    let consume_payload = json!({
        "userId": user_id.to_string(),
        "clientAppId": client_app_id.to_string(),
        "amount": 100,
        "description": "Invalid API Key attempt"
    });

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/ext/points/{}/consume", ctx._realm_id))
        .header("content-type", "application/json")
        .header("X-API-Key", invalid_api_key)
        .body(Body::from(consume_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    println!("[Step 3] Verify no points were deducted");

    let (current_balance, total_consumed): (i64, i64) =
        sqlx::query_as("SELECT total_balance, total_consumed FROM points_accounts WHERE id = $1")
            .bind(account_id)
            .fetch_one(&ctx._app_state.pool)
            .await
            .expect("Failed to fetch account");

    assert_eq!(current_balance, balance, "Balance should remain unchanged");
    assert_eq!(total_consumed, 0, "Total consumed should remain 0");

    println!("\n✅ Scenario 2.7 完成：无效 API Key 正确被拒绝，未扣除积分");
}
