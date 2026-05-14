// =============================================================================
// Billing Permission Check Tests
// =============================================================================
//
// Test: Verify that billing APIs require proper permissions
//
// User Story: docs/user-stories/06-billing-user-stories.md
// Covers: US-BI-001, US-BI-002, US-BI-003, US-BI-004, US-BI-005
//
// =============================================================================

#[cfg(test)]
mod tests {
    use crate::tests::helpers::*;
    use crate::tests::schema_test_context::SchemaTestContext;
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use serde_json::json;
    use test_context::test_context;
    use tower::ServiceExt;
    use uuid::Uuid;

    use SchemaTestContext as BillingTestContext;

    // User Story: docs/user-stories/06-billing-user-stories.md
    // Covers: US-BI-001 (创建订阅套餐), US-BI-002 (编辑订阅套餐),
    //          US-BI-003 (删除订阅套餐), US-BI-004 (分配套餐到 Client App),
    //          US-BI-005 (查看订阅列表)

    /// Test: 创建套餐需要 billing.manage 权限
    #[test_context(BillingTestContext)]
    #[tokio::test]
    async fn test_scenario_create_plan_requires_billing_manage_permission(
        ctx: &mut BillingTestContext,
    ) {
        let app = ctx.create_unified_test_router();

        // Step 1: 创建用户（无 billing.manage 权限）
        let (token, user_id) =
            create_admin_session_with_user(ctx, "test-user-no-perm@test.com", 1800).await;

        // 不授予任何权限，用户只有基本认证

        // Ensure default product exists (plan table requires product_id FK)
        let realm_id_for_product = ctx._realm_id.clone();
        let product_id = ensure_default_product(ctx, &realm_id_for_product).await;

        // Step 2: 尝试创建套餐（应该失败）
        let payload = json!({
            "name": "test-plan",
            "title": "Test Plan",
            "description": "Test Description",
            "type": "monthly",
            "price": 1000,
            "currency": "USD",
            "paymentProvider": "creem",
            "externalProductId": "prod_test",
            "externalPriceId": "price_test_monthly_usd",
            "checkoutUrl": Some("https://example.com".to_string()),
            "trialDays": 0,
            "sortOrder": 0,
            "productId": product_id
        });

        let realm_id = ctx._realm_id.clone();
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/bill/{}/plans", realm_id))
                    .header("cookie", format!("X-Auth={}", token))
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Step 3: 验证返回 403 Forbidden
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let error_text = String::from_utf8_lossy(&body).to_lowercase();

        // 验证错误消息包含权限相关的信息
        assert!(
            error_text.contains("permission") || error_text.contains("forbidden"),
            "Expected permission error, got body: {}",
            error_text
        );

        // Cleanup
        let user_uuid = uuid::Uuid::parse_str(&user_id).expect("user_id should be a valid UUID");
        sqlx::query("DELETE FROM account WHERE id = $1::uuid")
            .bind(user_uuid)
            .execute(&ctx.app_state.pool)
            .await
            .unwrap();
    }
    /// Test: 查看套餐需要 billing.view 权限
    #[test_context(BillingTestContext)]
    #[tokio::test]
    async fn test_scenario_view_plan_requires_billing_view_permission(
        ctx: &mut BillingTestContext,
    ) {
        let app = ctx.create_unified_test_router();

        // Step 1: 创建用户（无 billing.view 权限）
        let (token, user_id) =
            create_admin_session_with_user(ctx, "test-user-no-perm-4@test.com", 1800).await;

        // Step 2: 尝试查看套餐列表（应该失败）
        let realm_id = ctx._realm_id.clone();
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/bill/{}/plans", realm_id))
                    .header("cookie", format!("X-Auth={}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Step 3: 验证返回 403 Forbidden
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        // Cleanup
        let user_uuid = uuid::Uuid::parse_str(&user_id).expect("user_id should be a valid UUID");
        sqlx::query("DELETE FROM account WHERE id = $1::uuid")
            .bind(user_uuid)
            .execute(&ctx.app_state.pool)
            .await
            .unwrap();
    }

    /// Test: 全局订阅历史需要 realm admin 权限
    #[test_context(BillingTestContext)]
    #[tokio::test]
    async fn test_scenario_global_subscription_history_requires_realm_admin(
        ctx: &mut BillingTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let (token, user_id) =
            create_admin_session_with_user(ctx, "test-history-no-admin@test.com", 1800).await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/bill/{}/subscriptions/history", ctx._realm_id))
                    .header("cookie", format!("X-Auth={}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let user_uuid = uuid::Uuid::parse_str(&user_id).expect("user_id should be a valid UUID");
        sqlx::query("DELETE FROM account WHERE id = $1::uuid")
            .bind(user_uuid)
            .execute(&ctx.app_state.pool)
            .await
            .unwrap();
    }

    /// Test: 单订阅历史按本地 subscription UUID 查询，不存在时返回 404
    #[test_context(BillingTestContext)]
    #[tokio::test]
    async fn test_scenario_get_subscription_history_returns_404_for_unknown_subscription_id(
        ctx: &mut BillingTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let (token, user_id) =
            create_admin_session_with_user(ctx, "test-history-admin@test.com", 1800).await;
        grant_realm_admin_role(ctx, &user_id).await;

        let missing_subscription_id = Uuid::now_v7();
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!(
                        "/api/bill/{}/subscriptions/{}/history",
                        ctx._realm_id, missing_subscription_id
                    ))
                    .header("cookie", format!("X-Auth={}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let user_uuid = uuid::Uuid::parse_str(&user_id).expect("user_id should be a valid UUID");
        sqlx::query("DELETE FROM account WHERE id = $1::uuid")
            .bind(user_uuid)
            .execute(&ctx.app_state.pool)
            .await
            .unwrap();
    }

    /// Test: 历史列表过滤条件会真正生效
    #[test_context(BillingTestContext)]
    #[tokio::test]
    async fn test_scenario_list_subscription_history_applies_supported_filters(
        ctx: &mut BillingTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let (token, user_id) =
            create_admin_session_with_user(ctx, "test-history-filters@test.com", 1800).await;
        grant_realm_admin_role(ctx, &user_id).await;

        let realm_id = ctx._realm_id.clone();
        let plan_a = create_test_plan(ctx, &realm_id, "history-plan-a").await;
        let plan_b = create_test_plan(ctx, &realm_id, "history-plan-b").await;
        let client_app_a = Uuid::now_v7();
        let client_app_b = Uuid::now_v7();

        for client_app_id in [client_app_a, client_app_b] {
            sqlx::query(
                "INSERT INTO client_app (id, client_id, realm_id, name, redirect_uris, created_at, updated_at)
                 VALUES ($1, $2, $3, 'History Test App', '[]', NOW(), NOW())",
            )
            .bind(client_app_id)
            .bind(format!("history-client-{client_app_id}"))
            .bind(&realm_id)
            .execute(&ctx.app_state.pool)
            .await
            .unwrap();
        }

        let subscription_a =
            create_test_subscription(ctx, &realm_id, client_app_a, plan_a, "monthly").await;
        let subscription_b =
            create_test_subscription(ctx, &realm_id, client_app_b, plan_b, "monthly").await;

        sqlx::query("UPDATE subscription SET status = 'canceled' WHERE id = $1")
            .bind(subscription_b)
            .execute(&ctx.app_state.pool)
            .await
            .unwrap();

        for (event_id, subscription_id, event_type) in [
            ("hist_event_a", subscription_a, "created"),
            ("hist_event_b", subscription_b, "canceled"),
        ] {
            sqlx::query(
                "INSERT INTO subscription_history
                 (id, subscription_id, event_type, timestamp, actor, realm_id, created_at)
                 VALUES ($1, $2, $3, NOW(), 'test', $4, NOW())",
            )
            .bind(event_id)
            .bind(subscription_id)
            .bind(event_type)
            .bind(&realm_id)
            .execute(&ctx.app_state.pool)
            .await
            .unwrap();
        }

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!(
                        "/api/bill/{}/subscriptions/history?userId={}&planId={}&subscriptionStatus=active",
                        realm_id, client_app_a, plan_a
                    ))
                    .header("cookie", format!("X-Auth={}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let events = json["events"]
            .as_array()
            .expect("events should be an array");
        assert_eq!(events.len(), 1);
        assert_eq!(
            json["events"][0]["subscription"]["id"],
            subscription_a.to_string()
        );

        sqlx::query("DELETE FROM subscription_history WHERE realm_id = $1")
            .bind(&realm_id)
            .execute(&ctx.app_state.pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM subscription WHERE id = $1 OR id = $2")
            .bind(subscription_a)
            .bind(subscription_b)
            .execute(&ctx.app_state.pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM client_app WHERE id = $1 OR id = $2")
            .bind(client_app_a)
            .bind(client_app_b)
            .execute(&ctx.app_state.pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM plan WHERE id = $1 OR id = $2")
            .bind(plan_a)
            .bind(plan_b)
            .execute(&ctx.app_state.pool)
            .await
            .unwrap();
        let user_uuid = uuid::Uuid::parse_str(&user_id).expect("user_id should be a valid UUID");
        sqlx::query("DELETE FROM account WHERE id = $1::uuid")
            .bind(user_uuid)
            .execute(&ctx.app_state.pool)
            .await
            .unwrap();
    }
    /// Test: 拥有 billing.manage 权限的用户可以创建套餐
    #[test_context(BillingTestContext)]
    #[tokio::test]
    async fn test_scenario_user_with_billing_manage_can_create_plan(ctx: &mut BillingTestContext) {
        let app = ctx.create_unified_test_router();

        // Step 1: 创建用户并授予 billing.manage 权限
        let (token, user_id) =
            create_admin_session_with_user(ctx, "test-user-with-perm@test.com", 1800).await;

        // 授予 billing.manage 权限
        grant_billing_permission(ctx, &user_id, "billing.manage").await;

        // Ensure default product exists (plan table requires product_id FK)
        let realm_id_for_product = ctx._realm_id.clone();
        let product_id = ensure_default_product(ctx, &realm_id_for_product).await;

        // Step 2: 尝试创建套餐（应该成功）
        let realm_id = ctx._realm_id.clone();
        let payload = json!({
            "name": "test-plan-with-perm",
            "title": "Test Plan With Permission",
            "description": "Test Description",
            "type": "monthly",
            "price": 1000,
            "currency": "USD",
            "paymentProvider": "creem",
            "externalProductId": "prod_test_perm",
            "externalPriceId": "price_test_monthly_usd",
            "checkoutUrl": Some("https://example.com".to_string()),
            "trialDays": 0,
            "sortOrder": 0,
            "productId": product_id
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/bill/{}/plans", realm_id))
                    .header("cookie", format!("X-Auth={}", token))
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Step 3: 验证返回 200 OK（或 201 Created）
        let status = response.status();
        assert!(
            status == StatusCode::OK || status == StatusCode::CREATED,
            "Expected 200 or 201, got {}",
            status
        );

        // Cleanup
        sqlx::query("DELETE FROM plan WHERE realm_id = $1 AND name = 'test-plan-with-perm'")
            .bind(&realm_id)
            .execute(&ctx.app_state.pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM account WHERE id = $1::uuid")
            .bind(&user_id)
            .execute(&ctx.app_state.pool)
            .await
            .unwrap();
    }

    /// Test: 拥有 billing.view 权限的用户可以查看套餐
    #[test_context(BillingTestContext)]
    #[tokio::test]
    async fn test_scenario_user_with_billing_view_can_list_plans(ctx: &mut BillingTestContext) {
        let app = ctx.create_unified_test_router();

        // Step 1: 创建用户并授予 billing.view 权限
        let (token, user_id) =
            create_admin_session_with_user(ctx, "test-user-with-view@test.com", 1800).await;

        // 授予 billing.view 权限
        grant_billing_permission(ctx, &user_id, "billing.view").await;

        // Step 2: 创建一个测试套餐（通过 SQL 插入）
        let realm_id = ctx._realm_id.clone();
        let plan_id = create_test_plan(ctx, &realm_id, "test-view-plan").await;

        // Step 3: 尝试查看套餐列表（应该成功）
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/bill/{}/plans", realm_id))
                    .header("cookie", format!("X-Auth={}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Step 4: 验证返回 200 OK
        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let plans: serde_json::Value = serde_json::from_slice(&body).unwrap();

        // 验证套餐列表包含我们创建的套餐
        assert!(!plans["plans"].as_array().unwrap().is_empty());

        // Cleanup
        sqlx::query("DELETE FROM plan WHERE id = $1")
            .bind(plan_id)
            .execute(&ctx.app_state.pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM account WHERE id = $1::uuid")
            .bind(&user_id)
            .execute(&ctx.app_state.pool)
            .await
            .unwrap();
    }

    /// Helper function to grant billing permission to a user
    async fn grant_billing_permission(ctx: &BillingTestContext, user_id: &str, permission: &str) {
        use herald_core::domain::authorization::PermissionService;

        // Parse resource and action from permission (e.g., "billing.manage" -> resource="billing", action="manage")
        let parts: Vec<&str> = permission.split('.').collect();
        let (resource, action) = (parts[0], parts[1]);

        // Create role with billing permission
        let role_uuid = uuid::Uuid::now_v7();
        let _role_id = role_uuid.to_string(); // Prefix with underscore to indicate intentional non-use

        sqlx::query(
            "INSERT INTO roles (id, name, description, realm_id, client_id, is_builtin)
             VALUES ($1, 'billing-test-role', 'Test role with billing permission', $2, $3, false)",
        )
        .bind(role_uuid)
        .bind(&ctx._realm_id)
        .bind(&ctx._client_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Add permission policy to role
        let policy_id = uuid::Uuid::now_v7();
        sqlx::query(
            "INSERT INTO role_policies (id, role_id, realm_id, resource, action)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(policy_id)
        .bind(role_uuid)
        .bind(&ctx._realm_id)
        .bind(resource)
        .bind(action)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Add user to role
        let user_role_id = uuid::Uuid::now_v7();
        let user_uuid = uuid::Uuid::parse_str(user_id).expect("Failed to parse user_id as UUID");
        sqlx::query(
            "INSERT INTO user_roles (id, user_id, role_id, realm_id, client_id)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(user_role_id)
        .bind(user_uuid)
        .bind(role_uuid)
        .bind(&ctx._realm_id)
        .bind(&ctx._client_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Invalidate cache
        let _ = ctx
            .app_state
            .permission_checker
            .invalidate_realm_cache(&ctx._realm_id)
            .await;
    }
}
