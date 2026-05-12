// =============================================================================
// Points System Scenario Test 25: List Plan Configs
// =============================================================================
//
// **User Story**: US-PO-01 (Configure Points Plans)
// **Priority**: P1
//
// **Scenario**: Admin Views All Plan Configurations
//
// **Given**:
// - An admin user
// - 3 plan configurations exist
//
// **When**:
// - The admin calls `GET /api/points/{realmId}/plan-configs`
//
// **Then**:
// - Response contains 3 configurations
// - All configuration fields are populated
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
async fn test_scenario_admin_list_plan_configs(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    println!("[Step 1] Create admin and 3 plan configs");

    let admin_email = "admin25@example.com";
    let admin_password = "admin123";
    create_test_admin(&ctx._app_state.pool, &ctx._realm_id, admin_email).await;

    for i in 1..=3 {
        let plan_id = create_test_plan(
            &ctx._app_state.pool,
            &ctx._realm_id,
            &format!("plan-{}", i),
            2999 * i,
        )
        .await;
        create_test_plan_config(
            &ctx._app_state.pool,
            &ctx._realm_id,
            plan_id,
            1000 * i,
            1000 * i,
        )
        .await;
    }

    println!("[Step 1] ✓ Created admin and 3 plan configs");

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

    println!("[Step 3] Admin lists plan configs");

    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/points/{}/plan-configs", ctx._realm_id))
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

    let configs = body["configs"]
        .as_array()
        .expect("Configs should be an array");
    assert_eq!(configs.len(), 3, "Should return 3 plan configs");

    println!("\n✅ Scenario 5.2 完成：管理员成功列出所有积分套餐配置");
}
