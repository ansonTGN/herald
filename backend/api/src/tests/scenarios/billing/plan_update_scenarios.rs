// =============================================================================
// Plan Update Scenarios
// =============================================================================
//
// **User Story**: US-BI-002 (Edit Subscription Plan)
// **Priority**: P0
//
// **Scenario**: Realm Admin updates subscription plan details
//
// **Given**:
// - A realm admin user with proper permissions
// - An existing subscription plan
//
// **When**:
// - The admin updates plan name, price, or description
//
// **Then**:
// - Plan is updated successfully in database
// - API returns updated plan data
// - Changes are persisted correctly
//
// =============================================================================

use crate::tests::helpers::billing_helpers::*;
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use serde_json::json;
use test_context::test_context;
use tower::ServiceExt;

/// ============================================================================
/// Scenario 1: Update Plan Name and Description
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_update_plan_name_and_description(ctx: &mut TestContext) {
    // ============================================================================
    // Given: A realm admin with an existing plan
    // ============================================================================
    println!("[Step 1] Create test data");

    let app = ctx.create_unified_test_router();
    let admin_token = setup_billing_admin_session(ctx, "plan-update-admin@test.com").await;
    let realm_id = ctx._realm_id.clone();

    let plan_id = create_test_plan_with_attrs(ctx, &realm_id, "Basic Plan", "monthly", 2999).await;

    println!("[Step 1] ✓ Plan created: {}", plan_id);

    // ============================================================================
    // When: Admin updates plan name and description
    // ============================================================================
    println!("[Step 2] Update plan name and description");

    let update_request = json!({
        "name": "Pro Plan",
        "title": "Pro Plan",
        "description": "Enhanced features for professional users"
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/bill/{}/plans/{}", ctx._realm_id, plan_id))
                .header("Content-Type", "application/json")
                .header(header::COOKIE, format!("X-Auth={}", admin_token))
                .body(Body::from(update_request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    println!("[Step 2] ✓ Plan update request sent");

    // ============================================================================
    // Then: Verify plan is updated in database
    // ============================================================================
    println!("[Step 3] Verify plan updates");

    let (name, title, description): (String, String, String) =
        sqlx::query_as("SELECT name, title, description FROM plan WHERE id = $1")
            .bind(plan_id)
            .fetch_one(&ctx._app_state.pool)
            .await
            .expect("Failed to fetch plan");

    assert_eq!(name, "Pro Plan", "Plan name should be updated");
    assert_eq!(title, "Pro Plan", "Plan title should be updated");
    assert_eq!(
        description, "Enhanced features for professional users",
        "Description should be updated"
    );

    println!(
        "[Step 3] ✓ Plan verified: name={}, title={}, description={}",
        name, title, description
    );
    println!("\n✅ Scenario 1 完成：套餐名称和描述更新成功");
}

/// ============================================================================
/// Scenario 2: Update Plan Price
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_update_plan_price(ctx: &mut TestContext) {
    // ============================================================================
    // Given: A realm admin with an existing plan
    // ============================================================================
    println!("[Step 1] Create test data");

    let app = ctx.create_unified_test_router();
    let admin_token = setup_billing_admin_session(ctx, "price-update-admin@test.com").await;
    let realm_id = ctx._realm_id.clone();

    let plan_id =
        create_test_plan_with_attrs(ctx, &realm_id, "Starter Plan", "monthly", 1999).await;

    println!("[Step 1] ✓ Plan created: {}", plan_id);

    // ============================================================================
    // When: Admin updates plan price
    // ============================================================================
    println!("[Step 2] Update plan price");

    let update_request = json!({
        "price": 2499,
        "currency": "USD"
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/bill/{}/plans/{}", ctx._realm_id, plan_id))
                .header("Content-Type", "application/json")
                .header(header::COOKIE, format!("X-Auth={}", admin_token))
                .body(Body::from(update_request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    println!("[Step 2] ✓ Plan price update request sent");

    // ============================================================================
    // Then: Verify price is updated in database
    // ============================================================================
    println!("[Step 3] Verify price update");

    let (price, currency): (i32, String) =
        sqlx::query_as("SELECT price, currency FROM plan WHERE id = $1")
            .bind(plan_id)
            .fetch_one(&ctx._app_state.pool)
            .await
            .expect("Failed to fetch plan");

    assert_eq!(price, 2499, "Plan price should be updated");
    assert_eq!(currency, "USD", "Currency should remain USD");

    println!("[Step 3] ✓ Price verified: {} cents ({})", price, currency);
    println!("\n✅ Scenario 2 完成：套餐价格更新成功");
}

/// ============================================================================
/// Scenario 3: Partial Update (Only Description)
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_partial_plan_update(ctx: &mut TestContext) {
    // ============================================================================
    // Given: A realm admin with an existing plan
    // ============================================================================
    println!("[Step 1] Create test data");

    let app = ctx.create_unified_test_router();
    let admin_token = setup_billing_admin_session(ctx, "partial-update-admin@test.com").await;
    let realm_id = ctx._realm_id.clone();

    let plan_id =
        create_test_plan_with_attrs(ctx, &realm_id, "Enterprise Plan", "yearly", 9999).await;

    println!("[Step 1] ✓ Plan created: {}", plan_id);

    // Get initial values
    let (initial_name, initial_price): (String, i32) =
        sqlx::query_as("SELECT name, price FROM plan WHERE id = $1")
            .bind(plan_id)
            .fetch_one(&ctx._app_state.pool)
            .await
            .expect("Failed to fetch plan");

    // ============================================================================
    // When: Admin updates only description
    // ============================================================================
    println!("[Step 2] Update only description");

    let update_request = json!({
        "description": "Enterprise-grade features with priority support"
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/bill/{}/plans/{}", ctx._realm_id, plan_id))
                .header("Content-Type", "application/json")
                .header(header::COOKIE, format!("X-Auth={}", admin_token))
                .body(Body::from(update_request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    println!("[Step 2] ✓ Partial update request sent");

    // ============================================================================
    // Then: Verify only description changed, other fields unchanged
    // ============================================================================
    println!("[Step 3] Verify partial update");

    let (name, price, description): (String, i32, String) =
        sqlx::query_as("SELECT name, price, description FROM plan WHERE id = $1")
            .bind(plan_id)
            .fetch_one(&ctx._app_state.pool)
            .await
            .expect("Failed to fetch plan");

    assert_eq!(name, initial_name, "Name should remain unchanged");
    assert_eq!(price, initial_price, "Price should remain unchanged");
    assert_eq!(
        description, "Enterprise-grade features with priority support",
        "Description should be updated"
    );

    println!(
        "[Step 3] ✓ Partial update verified: name={}, price={}, description={}",
        name, price, description
    );
    println!("\n✅ Scenario 3 完成：套餐部分字段更新成功");
}
