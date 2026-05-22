// =============================================================================
// Points System Scenario Test 3: Balance Permission
// =============================================================================
//
// **User Story**: US-PU-01 (View My Points Balance)
// **Priority**: P0
//
// **Scenario**: User Cannot View Another User's Balance
//
// **Given**:
// - User A with authentication
// - User B with a different ID and existing points account
// - User A is not an admin
//
// **When**:
// - User A calls `GET /api/{realmId}/points/balance?user_id={userB_id}`
//
// **Then**:
// - HTTP status is 403 Forbidden
// - Error message indicates insufficient permissions
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
/// Scenario 1.3: User Cannot View Another User's Balance
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_user_cannot_view_other_user_balance(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    // ============================================================================
    // Given: User A with authentication, User B with points account
    // ============================================================================
    println!("[Step 1] Create test users");

    let email_a = "user3a@example.com";
    let password_a = "password123";
    let user_a_id =
        create_test_user_with_auth(&ctx._app_state.pool, &ctx._realm_id, email_a, password_a).await;

    let user_b_id =
        create_test_user(&ctx._app_state.pool, &ctx._realm_id, "user3b@example.com").await;
    let balance_b = 5000;

    let account_b_id = create_test_points_wallet(&ctx._app_state.pool, user_b_id, balance_b).await;

    println!(
        "[Step 1] ✓ Test data created: user_a={}, user_b={}, account_b={}",
        user_a_id, user_b_id, account_b_id
    );

    // ============================================================================
    // When: User A logs in and tries to view User B's balance
    // ============================================================================
    println!("[Step 2] User A logs in");

    // Login as User A
    let login_payload = json!({
        "clientId": ctx._client_id,
        "email": email_a,
        "password": password_a,
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

    println!("[Step 2] ✓ User A logged in: token={}", token);

    println!("[Step 3] User A tries to view User B's balance");

    let request = Request::builder()
        .method("GET")
        .uri(format!(
            "/api/points/{}/wallets/{}",
            ctx._realm_id, user_b_id
        ))
        .header("cookie", format!("X-Auth={}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();

    // ============================================================================
    // Then: Verify 403 Forbidden
    // ============================================================================
    println!("[Step 4] Verify 403 Forbidden");

    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "Should return 403 Forbidden when user tries to view another user's balance"
    );

    println!("[Step 4] ✓ 403 Forbidden returned correctly");

    println!("\n✅ Scenario 1.3 完成：普通用户无法查看其他用户的积分余额");
}
