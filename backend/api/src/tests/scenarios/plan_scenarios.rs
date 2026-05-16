/// 场景测试：Plan 管理生命周期
///
/// 测试 Realm Admin 创建、查询、更新、删除 Plan 的完整流程
#[cfg(test)]
mod tests {
    use crate::tests::helpers::*;
    use crate::tests::schema_test_context::SchemaTestContext;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
        http::StatusCode,
    };
    use serde_json::json;
    use test_context::test_context;
    use tower::ServiceExt;
    use uuid::Uuid;

    use SchemaTestContext as PlanTestContext;

    // All billing-specific helpers are now in billing_helpers module

    /// Helper function to create a plan via API
    async fn create_plan_via_api(
        ctx: &PlanTestContext,
        admin_token: &str,
        realm_id: &str,
        name: &str,
    ) -> (StatusCode, serde_json::Value) {
        // Ensure default product exists for this realm
        let product_id: Uuid = sqlx::query_scalar(
            "INSERT INTO products (id, realm_id, code, title, description, enabled, created_at, updated_at)
             VALUES ($1, $2, 'default', 'Default Product', 'Default test product', true, NOW(), NOW())
             ON CONFLICT (realm_id, code) DO UPDATE SET updated_at = products.updated_at
             RETURNING id"
        )
            .bind(Uuid::now_v7())
            .bind(realm_id)
            .fetch_one(&ctx.app_state.pool)
            .await
            .expect("Failed to ensure default product");

        let app = ctx.create_unified_test_router();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/bill/{}/plans", realm_id))
                    .header("content-type", "application/json")
                    .header("cookie", format!("X-Auth={}", admin_token))
                    .body(Body::from(
                        json!({
                            "name": name.to_lowercase().replace(" ", "_"),
                            "title": format!("{} Title", name),
                            "description": format!("{} description", name),
                            "type": "monthly",
                            "price": 2500,
                            "currency": "USD",
                            "paymentProvider": "creem",
                            "productId": product_id,
                            "externalProductId": format!("prod_{}_monthly", name.to_lowercase().replace(" ", "_")),
                            "externalPriceId": format!("price_{}_monthly_usd", name.to_lowercase().replace(" ", "_")),
                            "checkoutUrl": format!("https://app.example.com/billing/checkout?plan_id={}", name.to_lowercase().replace(" ", "_")),
                            "trialDays": 14,
                            "sortOrder": 1
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json_value: serde_json::Value =
            serde_json::from_slice(&body).unwrap_or(serde_json::json!({}));

        // Debug: print response details
        if status != StatusCode::OK {
            eprintln!("Response status: {}", status);
            eprintln!("Response body bytes: {:?}", String::from_utf8_lossy(&body));
        }

        (status, json_value)
    }

    /// Helper to create a client app via API
    async fn create_client_app_via_api(
        _ctx: &PlanTestContext,
        app: &axum::Router,
        token: &str,
        realm_id: &str,
        client_id: &str,
        name: &str,
    ) -> (String, Uuid) {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/client/{}", realm_id))
                    .header("content-type", "application/json")
                    .header("cookie", format!("X-Auth={}", token))
                    .body(Body::from(
                        json!({
                            "clientId": client_id,
                            "name": name,
                            "redirectUris": ["https://example.com/callback"],
                            "enabled": true
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json_value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let uuid: Uuid = serde_json::from_value(json_value["id"].clone()).unwrap();
        (client_id.to_string(), uuid)
    }

    /// 场景测试：Realm Admin 创建和管理自定义 Plans
    #[test_context(PlanTestContext)]
    #[tokio::test]
    async fn test_scenario_realm_admin_manages_plans(ctx: &mut PlanTestContext) {
        let app = ctx.create_unified_test_router();

        let admin_token = setup_billing_admin_session(ctx, "test-plan-admin@test.com").await;
        let realm_id = ctx._realm_id.clone();

        // Ensure default product exists for this realm
        let product_id: Uuid = sqlx::query_scalar(
            "INSERT INTO products (id, realm_id, code, title, description, enabled, created_at, updated_at)
             VALUES ($1, $2, 'default', 'Default Product', 'Default test product', true, NOW(), NOW())
             ON CONFLICT (realm_id, code) DO UPDATE SET updated_at = products.updated_at
             RETURNING id"
        )
            .bind(Uuid::now_v7())
            .bind(&realm_id)
            .fetch_one(&ctx.app_state.pool)
            .await
            .expect("Failed to ensure default product");

        // Step 1: Create Pro Plan
        let pro_plan_request = Request::builder()
            .method("POST")
            .uri(format!("/api/bill/{}/plans", realm_id))
            .header("content-type", "application/json")
            .header("cookie", format!("X-Auth={}", admin_token))
            .body(Body::from(
                json!({
                    "name": "pro",
                    "title": "Pro Plan",
                    "description": "Professional features for growing teams",
                    "type": "monthly",
                    "price": 2500,
                    "currency": "USD",
                    "paymentProvider": "creem",
                    "productId": product_id,
                    "externalProductId": "prod_pro_monthly",
                    "externalPriceId": "price_pro_monthly_usd",
                    "checkoutUrl": "https://app.example.com/billing/checkout?plan_id=pro",
                    "trialDays": 14,
                    "sortOrder": 1
                })
                .to_string(),
            ))
            .unwrap();

        let pro_plan_response = app.clone().oneshot(pro_plan_request).await.unwrap();
        assert_eq!(pro_plan_response.status(), StatusCode::OK);

        let pro_plan_body = to_bytes(pro_plan_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let pro_plan: serde_json::Value = serde_json::from_slice(&pro_plan_body).unwrap();
        let pro_plan_id: Uuid = serde_json::from_value(pro_plan["id"].clone()).unwrap();
        assert_eq!(pro_plan["name"], "pro");
        assert_eq!(pro_plan["title"], "Pro Plan");

        // Step 2: Create Enterprise Plan
        let enterprise_plan_request = Request::builder()
            .method("POST")
            .uri(format!("/api/bill/{}/plans", realm_id))
            .header("content-type", "application/json")
            .header("cookie", format!("X-Auth={}", admin_token))
            .body(Body::from(
                json!({
                    "name": "enterprise",
                    "title": "Enterprise Plan",
                    "description": "Advanced features for large organizations",
                    "type": "monthly",
                    "price": 5000,
                    "currency": "USD",
                    "paymentProvider": "creem",
                    "productId": product_id,
                    "externalProductId": "prod_enterprise_monthly",
                    "externalPriceId": "price_enterprise_monthly_usd",
                    "checkoutUrl": "https://app.example.com/billing/checkout?plan_id=enterprise",
                    "sortOrder": 2
                })
                .to_string(),
            ))
            .unwrap();

        let enterprise_plan_response = app.clone().oneshot(enterprise_plan_request).await.unwrap();
        assert_eq!(enterprise_plan_response.status(), StatusCode::OK);

        let enterprise_plan_body = to_bytes(enterprise_plan_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let enterprise_plan: serde_json::Value =
            serde_json::from_slice(&enterprise_plan_body).unwrap();
        assert_eq!(enterprise_plan["name"], "enterprise");
        assert_eq!(enterprise_plan["title"], "Enterprise Plan");

        // Step 3: List all Plans
        let list_request = Request::builder()
            .method("GET")
            .uri(format!("/api/bill/{}/plans", realm_id))
            .header("cookie", format!("X-Auth={}", admin_token))
            .body(Body::empty())
            .unwrap();

        let list_response = app.clone().oneshot(list_request).await.unwrap();
        assert_eq!(list_response.status(), StatusCode::OK);

        let list_body = to_bytes(list_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let plans_list: serde_json::Value = serde_json::from_slice(&list_body).unwrap();
        assert_eq!(plans_list["plans"].as_array().unwrap().len(), 2);

        // Step 4: Update Plan
        let update_request = Request::builder()
            .method("PATCH")
            .uri(format!("/api/bill/{}/plans/{}", realm_id, pro_plan_id))
            .header("content-type", "application/json")
            .header("cookie", format!("X-Auth={}", admin_token))
            .body(Body::from(
                json!({
                    "description": "Updated Pro Plan description",
                    "active": true
                })
                .to_string(),
            ))
            .unwrap();

        let update_response = app.clone().oneshot(update_request).await.unwrap();
        assert_eq!(update_response.status(), StatusCode::OK);

        // Step 5: Delete Plan (no active subscriptions)
        let delete_request = Request::builder()
            .method("DELETE")
            .uri(format!("/api/bill/{}/plans/{}", realm_id, pro_plan_id))
            .header("cookie", format!("X-Auth={}", admin_token))
            .body(Body::empty())
            .unwrap();

        let delete_response = app.clone().oneshot(delete_request).await.unwrap();
        assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);
    }

    /// 场景测试：Plan 分配到 Client App
    #[test_context(PlanTestContext)]
    #[tokio::test]
    async fn test_scenario_assign_plan_to_client_app(ctx: &mut PlanTestContext) {
        let app = ctx.create_unified_test_router();

        let admin_token = setup_billing_admin_session(ctx, "test-assign-plan@test.com").await;
        let realm_id = ctx._realm_id.clone();

        // Step 1: Create Plan
        let (status, plan_json) =
            create_plan_via_api(ctx, &admin_token, &realm_id, "Assignable Plan").await;
        assert_eq!(status, StatusCode::OK);
        let plan_id: Uuid = serde_json::from_value(plan_json["id"].clone()).unwrap();

        // Step 2: Create Client App
        let (_client_id_str, client_app_id) = create_client_app_via_api(
            ctx,
            &app,
            &admin_token,
            &realm_id,
            "test-app",
            "Test Application",
        )
        .await;

        // Step 3: Assign Plan to Client App
        let assign_request = Request::builder()
            .method("POST")
            .uri(format!(
                "/api/bill/{}/client/{}/plans",
                realm_id, client_app_id
            ))
            .header("content-type", "application/json")
            .header("cookie", format!("X-Auth={}", admin_token))
            .body(Body::from(json!({"planId": plan_id}).to_string()))
            .unwrap();

        let assign_response = app.clone().oneshot(assign_request).await.unwrap();
        assert_eq!(assign_response.status(), StatusCode::OK);

        let assign_body = to_bytes(assign_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let assign_json: serde_json::Value = serde_json::from_slice(&assign_body).unwrap();
        let assignment_id: Uuid = serde_json::from_value(assign_json["id"].clone()).unwrap();
        assert_eq!(assign_json["planId"], serde_json::json!(plan_id));

        // Step 4: List Client App Plans
        let list_request = Request::builder()
            .method("GET")
            .uri(format!(
                "/api/bill/{}/client/{}/plans",
                realm_id, client_app_id
            ))
            .header("cookie", format!("X-Auth={}", admin_token))
            .body(Body::empty())
            .unwrap();

        let list_response = app.clone().oneshot(list_request).await.unwrap();
        assert_eq!(list_response.status(), StatusCode::OK);

        let list_body = to_bytes(list_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let assignments_json: serde_json::Value = serde_json::from_slice(&list_body).unwrap();
        assert_eq!(assignments_json["assignments"].as_array().unwrap().len(), 1);

        // Step 5: Toggle Plan Assignment (disable)
        let toggle_request = Request::builder()
            .method("PATCH")
            .uri(format!(
                "/api/bill/{}/client/{}/plans/{}",
                realm_id, client_app_id, assignment_id
            ))
            .header("content-type", "application/json")
            .header("cookie", format!("X-Auth={}", admin_token))
            .body(Body::from(json!({"enabled": false}).to_string()))
            .unwrap();

        let toggle_response = app.clone().oneshot(toggle_request).await.unwrap();
        assert_eq!(toggle_response.status(), StatusCode::OK);

        let toggle_body = to_bytes(toggle_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let toggle_json: serde_json::Value = serde_json::from_slice(&toggle_body).unwrap();
        assert_eq!(toggle_json["enabled"], false);

        // Step 6: Delete Plan Assignment
        let remove_request = Request::builder()
            .method("DELETE")
            .uri(format!(
                "/api/bill/{}/client/{}/plans/{}",
                realm_id, client_app_id, assignment_id
            ))
            .header("cookie", format!("X-Auth={}", admin_token))
            .body(Body::empty())
            .unwrap();

        let remove_response = app.clone().oneshot(remove_request).await.unwrap();
        assert_eq!(remove_response.status(), StatusCode::NO_CONTENT);
    }

    /// 场景测试：Plan 删除验证 (有活跃订阅时不能删除)
    #[test_context(PlanTestContext)]
    #[tokio::test]
    async fn test_scenario_cannot_delete_plan_with_active_subscriptions(ctx: &mut PlanTestContext) {
        let app = ctx.create_unified_test_router();

        let admin_token = setup_billing_admin_session(ctx, "test-delete-plan@test.com").await;
        let realm_id = ctx._realm_id.clone();

        // Step 1: Create Plan
        let (status, plan_json) =
            create_plan_via_api(ctx, &admin_token, &realm_id, "Deletable Plan").await;
        assert_eq!(status, StatusCode::OK);
        let plan_id: Uuid = serde_json::from_value(plan_json["id"].clone()).unwrap();

        // Step 2: Create Client App and subscription
        let (_client_id_str, client_app_id) = create_client_app_via_api(
            ctx,
            &app,
            &admin_token,
            &realm_id,
            "test-subscription-app",
            "Test Subscription App",
        )
        .await;

        // Create subscription using helper function
        let subscription_id =
            create_test_subscription(ctx, &realm_id, client_app_id, plan_id, "monthly").await;

        // Step 3: Attempt to delete Plan (should fail)
        let delete_request = Request::builder()
            .method("DELETE")
            .uri(format!("/api/bill/{}/plans/{}", realm_id, plan_id))
            .header("cookie", format!("X-Auth={}", admin_token))
            .body(Body::empty())
            .unwrap();

        let delete_response = app.clone().oneshot(delete_request).await.unwrap();
        assert_eq!(delete_response.status(), StatusCode::BAD_REQUEST);

        // Step 4: Cancel subscription
        let cancel_request = Request::builder()
            .method("POST")
            .uri(format!(
                "/api/bill/{}/client/{}/subscription/cancel",
                realm_id, client_app_id
            ))
            .header("content-type", "application/json")
            .header("cookie", format!("X-Auth={}", admin_token))
            .body(Body::from(json!({"cancelAtPeriodEnd": false}).to_string()))
            .unwrap();

        let cancel_response = app.clone().oneshot(cancel_request).await.unwrap();
        assert_eq!(cancel_response.status(), StatusCode::OK);

        // Step 5: Now delete Plan should succeed
        let delete_request2 = Request::builder()
            .method("DELETE")
            .uri(format!("/api/bill/{}/plans/{}", realm_id, plan_id))
            .header("cookie", format!("X-Auth={}", admin_token))
            .body(Body::empty())
            .unwrap();

        let delete_response2 = app.clone().oneshot(delete_request2).await.unwrap();
        assert_eq!(delete_response2.status(), StatusCode::NO_CONTENT);

        // Cleanup
        delete_test_subscription(ctx, subscription_id).await;
    }

    /// 场景测试：Plan 名称重复验证
    #[test_context(PlanTestContext)]
    #[tokio::test]
    async fn test_scenario_duplicate_plan_name_fails(ctx: &mut PlanTestContext) {
        let _app = ctx.create_unified_test_router();

        let admin_token = setup_billing_admin_session(ctx, "test-duplicate-plan@test.com").await;
        let realm_id = ctx._realm_id.clone();

        // Step 1: Create first Plan
        let (status1, json1) =
            create_plan_via_api(ctx, &admin_token, &realm_id, "Duplicate Plan").await;
        if status1 != StatusCode::OK {
            eprintln!("Failed to create plan: status={}, body={}", status1, json1);
        }
        assert_eq!(status1, StatusCode::OK);

        // Step 2: Attempt to create Plan with same name (should fail)
        let (status2, _json2) =
            create_plan_via_api(ctx, &admin_token, &realm_id, "Duplicate Plan").await;
        assert_eq!(status2, StatusCode::BAD_REQUEST);
    }
}
