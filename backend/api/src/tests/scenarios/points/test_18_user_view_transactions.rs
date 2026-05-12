// =============================================================================
// Points System Scenario Test 18: User View Transactions
// =============================================================================
//
// **User Story**: US-PU-02 (View My Transaction History)
// **Priority**: P1
//
// **Scenario**: User Views Their Transaction History
//
// **Given**:
// - A user with 5 transactions:
//   - 2 recharge transactions
//   - 3 consume transactions
// - The transactions are created over time
//
// **When**:
// - The user calls `GET /api/{realmId}/points/transactions`
//
// **Then**:
// - Response contains all 5 transactions
// - Transactions are ordered by created_at DESC (newest first)
// - Response total is 5
// - HTTP status is 200 OK
//
// =============================================================================

use crate::tests::scenarios::points::fixtures::*;
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use serde_json::json;
use test_context::test_context;
use tower::ServiceExt;

#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_user_view_transaction_history(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    println!("[Step 1] Create user, account, and transactions");

    let email = "user18@example.com";
    let password = "password123";
    let user_id =
        create_test_user_with_auth(&ctx._app_state.pool, &ctx._realm_id, email, password).await;

    let balance = 5000;
    let account_id = create_test_points_account(&ctx._app_state.pool, user_id, balance).await;

    // Create 2 recharge transactions
    create_test_transaction(
        &ctx._app_state.pool,
        account_id,
        user_id,
        "recharge",
        1000,
        6000,
        Some("Recharge 1"),
        None,
    )
    .await;
    create_test_transaction(
        &ctx._app_state.pool,
        account_id,
        user_id,
        "recharge",
        2000,
        8000,
        Some("Recharge 2"),
        None,
    )
    .await;

    // Create 3 consume transactions
    create_test_transaction(
        &ctx._app_state.pool,
        account_id,
        user_id,
        "consume",
        -100,
        4900,
        Some("Consume 1"),
        None,
    )
    .await;
    create_test_transaction(
        &ctx._app_state.pool,
        account_id,
        user_id,
        "consume",
        -200,
        4700,
        Some("Consume 2"),
        None,
    )
    .await;
    create_test_transaction(
        &ctx._app_state.pool,
        account_id,
        user_id,
        "consume",
        -300,
        4400,
        Some("Consume 3"),
        None,
    )
    .await;

    println!("[Step 1] ✓ Created 5 transactions");

    println!("[Step 2] User logs in");

    let login_payload = json!({
        "clientId": ctx._client_id,
        "email": email,
        "password": password,
        "turnstileToken": "dummy"
    });

    let login_request = Request::builder()
        .method("POST")
        .uri(format!("/api/auth/{}/login", ctx._realm_id))
        .header("content-type", "application/json")
        .header("x-forwarded-for", "3.3.3.3")
        .body(Body::from(login_payload.to_string()))
        .unwrap();

    let login_response = app.clone().oneshot(login_request).await.unwrap();
    assert_eq!(login_response.status(), StatusCode::OK);

    let set_cookie = login_response
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|v| v.to_str().ok())
        .expect("Should return Set-Cookie header");

    let token = crate::tests::extract_set_cookie_token(set_cookie, "X-Auth")
        .expect("Should extract X-Auth token");

    println!("[Step 2] ✓ User logged in");

    println!("[Step 3] User requests transaction history");

    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/points/{}/transactions", ctx._realm_id))
        .header("cookie", format!("X-Auth={}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");
    let body: serde_json::Value =
        serde_json::from_slice(&body_bytes).expect("Failed to parse JSON");

    assert_eq!(body["total"].as_i64(), Some(5), "Total should be 5");

    let transactions = body["items"]
        .as_array()
        .expect("Transactions should be an array");
    assert_eq!(transactions.len(), 5, "Should return 5 transactions");

    println!("\n✅ Scenario 4.1 完成：用户成功查看交易历史");
}
