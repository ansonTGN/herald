// =============================================================================
// Points System Scenario Test 5: Admin List Accounts
// =============================================================================
//
// **User Story**: US-PO-02 (View All User Points Accounts)
// **Priority**: P1
//
// **Scenario**: Admin Views All User Accounts with Pagination
//
// **Given**:
// - An admin user with RealmAdmin identity
// - 25 users with points accounts in the realm
//
// **When**:
// - The admin calls `GET /api/{realmId}/points/accounts?page=1&page_size=20`
//
// **Then**:
// - Response contains 20 accounts
// - Response total is 25
// - Response page is 1
// - Response page_size is 20
// - HTTP status is 200 OK
//
// =============================================================================

use crate::tests::helpers::test_setup_helpers::assert_response_pagination;
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
/// Scenario 1.5: Admin Views All User Accounts with Pagination
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_admin_list_accounts_pagination(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    // ============================================================================
    // Given: An admin user and 25 users with points accounts
    // ============================================================================
    println!("[Step 1] Create admin and 25 regular users");

    let admin_email = "admin5@example.com";
    let admin_password = "admin123";
    create_test_admin(&ctx._app_state.pool, &ctx._realm_id, admin_email).await;

    let total_users = 25;
    for i in 0..total_users {
        let email = format!("user5_{}@example.com", i);
        let user_id = create_test_user(&ctx._app_state.pool, &ctx._realm_id, &email).await;
        let balance = (i + 1) * 100;
        create_test_points_account(&ctx._app_state.pool, user_id, balance).await;
    }

    println!(
        "[Step 1] ✓ Created {} users with points accounts",
        total_users
    );

    // ============================================================================
    // When: The admin logs in and lists accounts
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

    println!("[Step 3] Admin lists accounts with pagination");

    let request = Request::builder()
        .method("GET")
        .uri(format!(
            "/api/points/{}/accounts?page=1&page_size=20",
            ctx._realm_id
        ))
        .header("cookie", format!("X-Auth={}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();

    // ============================================================================
    // Then: Verify pagination response
    // ============================================================================
    println!("[Step 4] Verify pagination response");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Admin should be able to list accounts"
    );

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");
    let body: serde_json::Value =
        serde_json::from_slice(&body_bytes).expect("Failed to parse JSON");

    assert_response_pagination(&body, 1, 20, total_users);

    let accounts = body["items"].as_array().expect("Items should be an array");

    assert_eq!(accounts.len(), 20, "Should return 20 accounts on page 1");

    println!(
        "[Step 4] ✓ Pagination verified: page={}, pageSize={}, total={}, accounts_on_page={}",
        body["page"],
        body["pageSize"],
        body["total"],
        accounts.len()
    );

    println!("\n✅ Scenario 1.5 完成：管理员成功列出所有用户积分账户（分页）");
}
