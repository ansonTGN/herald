// =============================================================================
// Points System Scenario Test 24: Update Entitlement Mapping Points Policy
// =============================================================================
//
// **User Story**: US-PO-01 (Configure Points Plans)
// **Priority**: P0
//
// **Scenario**: Admin Updates Entitlement Mapping Points Policy
//
// **Given**:
// - An admin user
// - An existing entitlement mapping (created via sync or DB)
//
// **When**:
// - The admin calls `PATCH /api/bill/{realmId}/entitlement-mappings/{mappingId}` with:
//   - pointsPerPeriod: 1000
//   - validityDays: 30 (one-time expiration policy)
//   - grantOnSubscribe: true
//
// **Then**:
// - Response contains all submitted values
// - HTTP status is 200 OK
//
// =============================================================================

use crate::tests::helpers::create_admin_session_with_user;
use crate::tests::helpers::test_setup_helpers::record_test_user_consent;
use crate::tests::scenarios::points::fixtures::*;
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use test_context::test_context;
use tower::ServiceExt;

#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_admin_update_entitlement_points_policy(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    println!("[Step 1] Create admin and entitlement mapping");

    let admin_email = "admin24@example.com";
    let admin_user_id = create_test_admin(&ctx._app_state.pool, &ctx._realm_id, admin_email).await;
    record_test_user_consent(&ctx._app_state.pool, admin_user_id, &ctx._realm_id).await;

    let mapping_id =
        create_test_entitlement_mapping(&ctx._app_state.pool, &ctx._realm_id, "pro-monthly", 2999)
            .await;

    println!("[Step 1] ✓ Created admin and entitlement mapping");

    println!("[Step 2] Create admin session (FirstParty token)");

    let (token, _) = create_admin_session_with_user(ctx, admin_email, 1800).await;

    println!("[Step 2] ✓ Admin session created");

    println!("[Step 3] Admin updates entitlement mapping points policy");

    // Distribution-rules model: the points policy is configured as a fixed
    // `subscription_initial` distribution rule owned by the mapping (the
    // new-model equivalent of the removed `pointsPerPeriod` / `validityDays` /
    // `grantOnSubscribe` fields). The rule is validated against the mapping's
    // billing_type, so set `billing_type='recurring'` (grant_on_subscribe=true
    // maps to `subscription_initial`) before PATCHing. Mirror the canonical
    // `entitlement_mapping_crud_scenarios::test_update_entitlement_mapping_set_points_policy`.
    let bucket_id = crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm(
        &ctx._app_state.pool,
        &ctx._realm_id,
    )
    .await;
    sqlx::query(
        "UPDATE provider_entitlement_mappings
         SET billing_type = 'recurring', billing_period = 'monthly'
         WHERE id = $1",
    )
    .bind(mapping_id)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to set billing_type for points-policy PATCH");

    let points_amount = 1000;
    let validity_days = 30;

    let update_payload = json!({
        "pointRules": [{
            "bucketId": bucket_id,
            "triggerSources": ["subscription_initial"],
            "grantMode": "fixed",
            "pointsAmount": points_amount,
            "validityDays": validity_days,
            "enabled": true,
            "displayOrder": 0
        }]
    });

    let request = Request::builder()
        .method("PATCH")
        .uri(format!(
            "/api/bill/{}/entitlement-mappings/{}",
            ctx._realm_id, mapping_id
        ))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(update_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");
    let body: serde_json::Value =
        serde_json::from_slice(&body_bytes).expect("Failed to parse JSON");

    let rules = body["pointRules"]
        .as_array()
        .expect("pointRules must be an array in the new-model response");
    assert_eq!(rules.len(), 1, "the upserted rule must be returned");
    assert_eq!(
        rules[0]["grantMode"], "fixed",
        "grantMode should match the fixed points policy"
    );
    assert_eq!(
        rules[0]["pointsAmount"].as_i64(),
        Some(points_amount),
        "Points amount should match"
    );
    assert_eq!(
        rules[0]["validityDays"].as_i64(),
        Some(validity_days),
        "Validity days should match"
    );
    assert_eq!(
        rules[0]["triggerSources"][0], "subscription_initial",
        "trigger should reflect grant-on-subscribe (subscription_initial)"
    );

    println!("\n✅ Scenario 24 完成：管理员成功更新积分套餐配置");
}
