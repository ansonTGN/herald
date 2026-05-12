// =============================================================================
// Points System Scenario Test 21: Admin View User Transactions
// =============================================================================
//
// **User Story**: US-PO-03 (View User Points Transaction History)
// **Priority**: P1
//
// **Scenario**: Admin Views Transactions for Specific User
//
// **Given**:
// - An admin user
// - User A with 5 transactions
// - User B with 3 transactions
//
// **When**:
// - The admin calls `GET /api/{realmId}/points/transactions?user_id={userA_id}`
//
// **Then**:
// - Response contains only User A's transactions
// - Response total is 5
// - User B's transactions are not included
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
async fn test_scenario_admin_view_specific_user_transactions(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    println!("[Step 1] Create admin and two users with transactions");

    let admin_email = "admin21@example.com";
    let admin_password = "admin123";
    create_test_admin(&ctx._app_state.pool, &ctx._realm_id, admin_email).await;

    let user_a_id =
        create_test_user(&ctx._app_state.pool, &ctx._realm_id, "user21a@example.com").await;
    let account_a_id = create_test_points_account(&ctx._app_state.pool, user_a_id, 5000).await;

    // User A: 5 transactions
    for i in 1..=5 {
        create_test_transaction(
            &ctx._app_state.pool,
            account_a_id,
            user_a_id,
            "consume",
            -100 * i,
            5000 - 100 * i,
            Some(&format!("User A txn {}", i)),
            None,
        )
        .await;
    }

    let user_b_id =
        create_test_user(&ctx._app_state.pool, &ctx._realm_id, "user21b@example.com").await;
    let account_b_id = create_test_points_account(&ctx._app_state.pool, user_b_id, 3000).await;

    // User B: 3 transactions
    for i in 1..=3 {
        create_test_transaction(
            &ctx._app_state.pool,
            account_b_id,
            user_b_id,
            "consume",
            -100 * i,
            3000 - 100 * i,
            Some(&format!("User B txn {}", i)),
            None,
        )
        .await;
    }

    println!("[Step 1] ✓ Created admin and two users with transactions");

    println!("[Step 2] Admin logs in");

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

    let set_cookie = login_response
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|v| v.to_str().ok())
        .expect("Should return Set-Cookie header");

    let token = crate::tests::extract_set_cookie_token(set_cookie, "X-Auth")
        .expect("Should extract X-Auth token");

    println!("[Step 2] ✓ Admin logged in");

    println!("[Step 3] Admin filters by userId");

    let request = Request::builder()
        .method("GET")
        .uri(format!(
            "/api/points/{}/transactions?userId={}",
            ctx._realm_id, user_a_id
        ))
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
    assert_eq!(
        transactions.len(),
        5,
        "Should return only User A's 5 transactions"
    );

    for txn in transactions {
        let txn_user_id = txn["userId"]
            .as_str()
            .expect("Transaction should have userId");
        assert_eq!(
            txn_user_id,
            user_a_id.to_string(),
            "All transactions should belong to User A"
        );
    }

    println!("\n✅ Scenario 4.4 完成：管理员成功查看特定用户的交易历史");
}
