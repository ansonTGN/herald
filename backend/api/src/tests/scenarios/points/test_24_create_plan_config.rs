// =============================================================================
// Points System Scenario Test 24: Create Plan Config
// =============================================================================
//
// **User Story**: US-PO-01 (Configure Points Plans)
// **Priority**: P0
//
// **Scenario**: Admin Creates Plan Configuration
//
// **Given**:
// - An admin user
// - A valid plan_id
//
// **When**:
// - The admin calls `POST /api/points/{realmId}/plan-configs` with:
//   - plan_id: valid UUID
//   - points_on_subscribe: 1000
//   - points_on_renewal: 1000
//   - renewal_enabled: true
//   - renewal_period_type: "monthly"
//   - max_accumulation: 10000
//
// **Then**:
// - Response contains config_id
// - Response contains all submitted values
// - Response active is true
// - HTTP status is 201 Created
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
async fn test_scenario_admin_create_plan_config(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    println!("[Step 1] Create admin and billing plan");

    let admin_email = "admin24@example.com";
    let admin_password = "admin123";
    create_test_admin(&ctx._app_state.pool, &ctx._realm_id, admin_email).await;

    let plan_id = create_test_plan(&ctx._app_state.pool, &ctx._realm_id, "pro-monthly", 2999).await;

    println!("[Step 1] ✓ Created admin and plan");

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

    println!("[Step 3] Admin creates plan config");

    // New grant period system design (flexible发放周期)
    let grant_period_type = "monthly"; // once / daily / weekly / monthly
    let points_per_period = 1000; // 每次发放积分数
    let validity_days = 30; // 有效期天数（0=永久有效）
    let grant_on_subscribe = true; // 订阅时是否立即发放
    let max_periods = Some(12i64); // 最大发放期数（null=无限期）

    let create_payload = json!({
        "planId": plan_id.to_string(),
        "grantPeriodType": grant_period_type,
        "pointsPerPeriod": points_per_period,
        "validityDays": validity_days,
        "grantOnSubscribe": grant_on_subscribe,
        "maxPeriods": max_periods
    });

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/points/{}/plan-configs", ctx._realm_id))
        .header("content-type", "application/json")
        .header("cookie", format!("X-Auth={}", token))
        .body(Body::from(create_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");
    let body: serde_json::Value =
        serde_json::from_slice(&body_bytes).expect("Failed to parse JSON");

    assert!(
        body["configId"].is_string(),
        "Response should contain configId"
    );
    assert_eq!(
        body["planId"].as_str(),
        Some(plan_id.to_string().as_str()),
        "Plan ID should match"
    );
    assert_eq!(
        body["grantPeriodType"].as_str(),
        Some(grant_period_type),
        "Grant period type should match"
    );
    assert_eq!(
        body["pointsPerPeriod"].as_i64(),
        Some(points_per_period),
        "Points per period should match"
    );
    assert_eq!(
        body["validityDays"].as_i64(),
        Some(validity_days),
        "Validity days should match"
    );
    assert_eq!(
        body["grantOnSubscribe"].as_bool(),
        Some(grant_on_subscribe),
        "Grant on subscribe should match"
    );
    assert_eq!(
        body["maxPeriods"].as_i64(),
        max_periods,
        "Max periods should match"
    );
    assert_eq!(
        body["active"].as_bool(),
        Some(true),
        "Active should be true"
    );

    println!("\n✅ Scenario 5.1 完成：管理员成功创建积分套餐配置");
}
