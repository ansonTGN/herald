// =============================================================================
// Points System Scenario Test 2: View Balance
// =============================================================================
//
// **User Story**: US-PU-01 (View My Points Balance)
// **Priority**: P0
//
// **Scenario**: User Views Their Own Balance
//
// **Given**:
// - A user with authentication
// - An existing points account with balance 5000
// - Total recharged: 10000
// - Total consumed: 5000
//
// **When**:
// - The user calls `GET /api/{realmId}/points/balance`
//
// **Then**:
// - The response returns balance: 5000
// - The response returns total_recharged: 10000
// - The response returns total_consumed: 5000
// - The response returns currency: "points"
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

/// ============================================================================
/// Scenario 1.2: User Views Their Own Balance
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_user_view_own_balance(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    // ============================================================================
    // Given: A user with authentication and points account
    // ============================================================================
    println!("[Step 1] Create test user and points account");

    let email = "user2@example.com";
    let password = "password123";
    let user_id =
        create_test_user_with_auth(&ctx._app_state.pool, &ctx._realm_id, email, password).await;

    let balance = 5000;
    let total_recharged = 10000;
    let total_consumed = 5000;

    let account_id = create_test_points_account(&ctx._app_state.pool, user_id, balance).await;

    // Update total_recharged and total_consumed
    sqlx::query(
        "UPDATE points_accounts SET total_recharged = $1, total_consumed = $2 WHERE id = $3",
    )
    .bind(total_recharged)
    .bind(total_consumed)
    .bind(account_id)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to update account totals");

    println!(
        "[Step 1] ✓ Test data created: user={}, account={}, balance={}, recharged={}, consumed={}",
        user_id, account_id, balance, total_recharged, total_consumed
    );

    // ============================================================================
    // When: The user logs in and views their balance
    // ============================================================================
    println!("[Step 2] User logs in");

    // Login to get session token
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

    // Extract session token
    let set_cookie = login_response
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|v| v.to_str().ok())
        .expect("Should return Set-Cookie header");

    let token = crate::tests::extract_set_cookie_token(set_cookie, "X-Auth")
        .expect("Should extract X-Auth token");

    println!("[Step 2] ✓ User logged in: token={}", token);

    println!("[Step 3] User requests balance");

    let request = Request::builder()
        .method("GET")
        .uri(format!(
            "/api/points/{}/accounts/{}",
            ctx._realm_id, user_id
        ))
        .header("cookie", format!("X-Auth={}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();

    // ============================================================================
    // Then: Verify balance response
    // ============================================================================
    println!("[Step 4] Verify balance response");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Get balance should return 200 OK"
    );

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");
    let body: serde_json::Value =
        serde_json::from_slice(&body_bytes).expect("Failed to parse JSON");

    assert_eq!(
        body["balance"].as_i64(),
        Some(balance),
        "Balance should be 5000"
    );
    assert_eq!(
        body["totalRecharged"].as_i64(),
        Some(total_recharged),
        "Total recharged should be 10000"
    );
    assert_eq!(
        body["totalConsumed"].as_i64(),
        Some(total_consumed),
        "Total consumed should be 5000"
    );
    assert_eq!(
        body["currency"].as_str(),
        Some("points"),
        "Currency should be 'points'"
    );
    assert_eq!(
        body["userId"].as_str(),
        Some(user_id.to_string().as_str()),
        "User ID should match"
    );

    println!(
        "[Step 4] ✓ Balance verified: balance={}, recharged={}, consumed={}",
        balance, total_recharged, total_consumed
    );

    println!("\n✅ Scenario 1.2 完成：用户成功查看自己的积分余额");
}
