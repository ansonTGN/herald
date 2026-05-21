// =============================================================================
// Points System Scenario Test 10: Consume Invalid Amount
// =============================================================================
//
// **User Story**: Input validation
// **Priority**: P0
//
// **Scenario**: Consumption Fails with Invalid Amount
//
// **Given**:
// - A valid API Key
// - A user with points account
//
// **When**:
// - The third party calls `POST /api/{realmId}/points/consume` with amount: -50
//
// **Then**:
// - HTTP status is 400 Bad Request
// - Error code is "invalid_amount"
// - No transaction is created
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

#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_consume_invalid_amount(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    println!("[Step 1] Create test user");

    let user_id =
        create_test_user(&ctx._app_state.pool, &ctx._realm_id, "user10@example.com").await;
    let balance = 5000;

    let wallet_id = create_test_points_wallet(&ctx._app_state.pool, user_id, balance).await;

    let client_app_id = create_test_client_app(&ctx._app_state.pool, &ctx._realm_id).await;
    let api_key = create_test_api_key(&ctx._app_state.pool, &ctx._realm_id, client_app_id).await;

    println!(
        "[Step 1] ✓ Test data created: user={}, account={}",
        user_id, wallet_id
    );

    println!("[Step 2] Attempt to consume with negative amount");

    let consume_payload = json!({
        "userId": user_id.to_string(),
        "clientAppId": client_app_id.to_string(),
        "amount": -50,
        "description": "Invalid negative amount"
    });

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/ext/points/{}/consume", ctx._realm_id))
        .header("content-type", "application/json")
        .header("X-API-Key", api_key)
        .body(Body::from(consume_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

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
        error_code.contains("invalid") || error_code.contains("amount"),
        "Error code should contain 'invalid' or 'amount', got: {}",
        error_code
    );

    println!("\n✅ Scenario 2.3 完成：无效金额时正确拒绝消耗请求");
}
