// =============================================================================
// SDK Billing Scenarios
// =============================================================================
//
// Test SDK billing API against real API
// =============================================================================

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use herald_sdk::Client;
use herald_test_support::SchemaTestContext;
use serde_json::json;
use test_context::test_context;
use tower::ServiceExt;
use uuid::Uuid;

/// ============================================================================
/// Billing API Scenarios
/// ============================================================================
/// Test scenario: SDK get subscription - success
///
/// Verifies that the SDK correctly retrieves subscription details
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_sdk_get_subscription_success(ctx: &mut SchemaTestContext) {
    // 1. Setup: Create admin session with billing permissions
    let email = "billing_admin@example.com";
    let (_session_token, user_id) =
        herald_test_support::helpers::create_admin_session_with_user(ctx, email, 1800).await;
    herald_test_support::helpers::grant_realm_admin_role(ctx, &user_id).await;

    // 2. Create SDK client pointing to the test server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", addr);

    // 3. Start the test server in the background
    let router = ctx.create_unified_test_router();
    let handle = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    // 4. Create a test API key for SDK authentication
    let (api_key, _api_key_entity) =
        herald_test_support::helpers::create_test_api_key(ctx, "SDK Test Key", true, None).await;

    // 5. Create a test subscription
    let realm_id = ctx._realm_id.clone();
    let client_app_id = ctx._client_app_id.clone();
    let plan_id = herald_test_support::helpers::create_test_plan(ctx, &realm_id, "test-plan").await;
    let client_app_uuid = Uuid::parse_str(&client_app_id).unwrap();
    herald_test_support::helpers::create_test_subscription(
        ctx,
        &realm_id,
        client_app_uuid,
        plan_id,
        "monthly",
    )
    .await;

    // 6. Create SDK client with the test API key and get subscription
    let client = Client::new(base_url, api_key, None);

    let result = client
        .get_subscription(&ctx._realm_id, "admin-web-console")
        .await;

    // 7. Verify: Subscription should be retrieved
    if let Err(e) = &result {
        eprintln!("SDK Error: {:?}", e);
    }
    assert!(
        result.is_ok(),
        "Get subscription should succeed: {:?}",
        result.err()
    );
    let subscription = result.unwrap();
    assert_eq!(
        subscription.status, "active",
        "Subscription should be active"
    );
    assert!(
        subscription.plan.is_some(),
        "Subscription should have a plan"
    );
    assert_eq!(
        subscription.plan.as_ref().unwrap().name,
        "test-plan",
        "Plan name should match"
    );

    // 8. Cleanup
    handle.abort();
}

/// Test scenario: SDK list plans - success
///
/// Verifies that the SDK correctly lists available plans
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_sdk_list_plans_success(ctx: &mut SchemaTestContext) {
    // 1. Setup: Create admin session
    let email = "plans_admin@example.com";
    let realm_id = ctx._realm_id.clone();
    let (_session_token, user_id) =
        herald_test_support::helpers::create_admin_session_with_user(ctx, email, 1800).await;
    herald_test_support::helpers::grant_realm_admin_role(ctx, &user_id).await;

    // 2. Create test plans
    herald_test_support::helpers::create_test_plan(ctx, &realm_id, "basic-plan").await;
    herald_test_support::helpers::create_test_plan(ctx, &realm_id, "premium-plan").await;

    // 3. Create SDK client pointing to the test server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", addr);

    // 4. Start the test server in the background
    let router = ctx.create_unified_test_router();
    let handle = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    // 5. Create a test API key for SDK authentication
    let (api_key, _api_key_entity) =
        herald_test_support::helpers::create_test_api_key(ctx, "SDK Test Key", true, None).await;

    // 6. Create SDK client with the test API key and list plans
    let client = Client::new(base_url, api_key, None);

    let result = client.list_plans(&ctx._realm_id).await;

    // 7. Verify: Plans should be listed
    if let Err(e) = &result {
        eprintln!("SDK Error: {:?}", e);
    }
    assert!(
        result.is_ok(),
        "List plans should succeed: {:?}",
        result.err()
    );
    let plans = result.unwrap();
    assert!(
        plans.len() >= 2,
        "Should have at least the 2 test plans we created"
    );
    assert!(
        plans.iter().any(|p| p.name == "basic-plan"),
        "Should have basic-plan"
    );
    assert!(
        plans.iter().any(|p| p.name == "premium-plan"),
        "Should have premium-plan"
    );

    // 8. Cleanup
    handle.abort();
}

/// Test scenario: SDK list plan assignments - success
///
/// Verifies that the SDK correctly lists plan assignments for a client app
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_sdk_list_plan_assignments_success(ctx: &mut SchemaTestContext) {
    // 1. Setup: Create admin session
    let email = "assignments_admin@example.com";
    let (session_token, user_id) =
        herald_test_support::helpers::create_admin_session_with_user(ctx, email, 1800).await;
    herald_test_support::helpers::grant_realm_admin_role(ctx, &user_id).await;

    // 2. Create test plan and assign it
    let realm_id = ctx._realm_id.clone();
    let plan_id =
        herald_test_support::helpers::create_test_plan(ctx, &realm_id, "assigned-plan").await;

    // Assign plan to client app via HTTP API
    let router_for_assign = ctx.create_unified_test_router();

    // Create assignment request
    let assignment_payload = json!({
        "planId": plan_id.to_string(),
        "enabled": true
    });

    let client_app_uuid = Uuid::parse_str(&ctx._client_app_id).unwrap();
    let req = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/bill/{}/client/{}/plans",
            ctx._realm_id, client_app_uuid
        ))
        .header("content-type", "application/json")
        .header(header::COOKIE, format!("X-Auth={}", session_token))
        .body(Body::from(assignment_payload.to_string()))
        .unwrap();

    let resp = router_for_assign.oneshot(req).await.unwrap();
    let status = resp.status();
    if status != StatusCode::OK {
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let error_msg = String::from_utf8(body.to_vec()).unwrap();
        panic!(
            "Plan assignment failed with status {}: {}",
            status, error_msg
        );
    }

    // Start server for the SDK test
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", addr);

    let router = ctx.create_unified_test_router();
    let handle = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    // 3. Create a test API key for SDK authentication
    let (api_key, _api_key_entity) =
        herald_test_support::helpers::create_test_api_key(ctx, "SDK Test Key", true, None).await;

    // 4. Create SDK client with the test API key and list plan assignments
    let client = Client::new(base_url, api_key, None);

    let result = client
        .list_plan_assignments(&ctx._realm_id, &ctx._client_id)
        .await;

    // 5. Verify: Plan assignments should be listed
    if let Err(e) = &result {
        eprintln!("SDK Error: {:?}", e);
    }
    assert!(
        result.is_ok(),
        "List plan assignments should succeed: {:?}",
        result.err()
    );
    let assignments = result.unwrap();
    assert!(
        !assignments.is_empty(),
        "Should have at least one plan assignment"
    );
    assert!(
        assignments.iter().any(|a| a.plan_id == plan_id.to_string()),
        "Should have the plan we assigned"
    );

    // 6. Cleanup
    handle.abort();
}
