// =============================================================================
// Points System Scenario Test 4: Admin View Balance
// =============================================================================
//
// **User Story**: US-PO-02 (View All User Points Accounts)
// **Priority**: P1
//
// **Scenario**: Admin Can View Any User's Balance
//
// **Given**:
// - An admin user with RealmAdmin identity
// - A regular user User A with points account balance 5000
//
// **When**:
// - The admin calls `GET /api/{realmId}/points/balance?user_id={userA_id}`
//
// **Then**:
// - The response returns balance: 5000
// - HTTP status is 200 OK
//
// =============================================================================

use crate::tests::scenarios::points::fixtures::{
    create_test_admin, create_test_points_account, create_test_user as create_points_user,
};
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use serde_json::json;
use test_context::test_context;
use tower::ServiceExt;

/// ============================================================================
/// Scenario 1.4: Admin Can View Any User's Balance
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_admin_can_view_any_user_balance(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    // ============================================================================
    // Given: An admin user and a regular user with points account
    // ============================================================================
    println!("[Step 1] Create admin and regular users");

    let admin_email = "admin4@example.com";
    let admin_password = "admin123";
    let admin_id = create_test_admin(&ctx._app_state.pool, &ctx._realm_id, admin_email).await;

    let user_id =
        create_points_user(&ctx._app_state.pool, &ctx._realm_id, "user4@example.com").await;
    let balance = 5000;

    let account_id = create_test_points_account(&ctx._app_state.pool, user_id, balance).await;

    println!(
        "[Step 1] ✓ Test data created: admin={}, user={}, account={}",
        admin_id, user_id, account_id
    );

    // ============================================================================
    // When: The admin logs in and views User A's balance
    // ============================================================================
    println!("[Step 2] Admin logs in");

    // Login as admin
    let login_payload = json!({
        "clientId": ctx._client_id,
        "email": admin_email,
        "password": admin_password,
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

    println!("[Step 2] ✓ Admin logged in: token={}", token);

    println!("[Step 3] Admin views user's balance");

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
        "Admin should be able to view user's balance"
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
        body["userId"].as_str(),
        Some(user_id.to_string().as_str()),
        "User ID should match"
    );

    println!(
        "[Step 4] ✓ Balance verified: balance={}, user_id={}",
        balance, user_id
    );

    println!("\n✅ Scenario 1.4 完成：管理员可以查看任意用户的积分余额");
}
