//! 场景测试：Shopify Webhook 处理
//!
//! 测试 Shopify 订阅计费 webhook 的处理流程，包括：
//! - subscription_contracts/create - 创建订阅和绑定，发放初始积分
//! - subscription_contracts/update - 升级/降级，版本检查
//! - subscription_billing_attempts/success - 续费成功，发放续费积分
//! - subscription_billing_attempts/failure - 标记为past_due
//! - refunds/create - 退款并撤销积分
//! - HMAC验证和幂等性
//!
//! User Stories:
//! - US-PP-011: Shopify 订阅合同创建与同步
//! - US-PP-012: Shopify 订阅续费与状态同步

#[cfg(test)]
mod tests {
    use crate::tests::helpers::shopify_helpers::*;
    use crate::tests::helpers::test_setup_helpers::*;
    use crate::tests::helpers::*;
    use crate::tests::schema_test_context::SchemaTestContext;

    use axum::http::StatusCode;
    use serde_json::json;
    use test_context::test_context;

    use uuid::Uuid;

    use SchemaTestContext as ShopifyTestContext;

    /// ============================================================================
    /// HMAC 验证测试
    /// ============================================================================
    /// User Story: US-PP-011 验收标准 5
    /// 场景：Webhook HMAC 验证失败应该返回 401
    #[test_context(ShopifyTestContext)]
    #[tokio::test]
    async fn test_scenario_shopify_hmac_verification_fails_with_invalid_signature(
        ctx: &mut ShopifyTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();

        // Setup Shopify config
        setup_shopify_config(
            ctx,
            &realm_id,
            "test-shop.myshopify.com",
            "shpat_test_token",
            "shp_test_token",
            "test_client_secret",
            "2024-01",
        )
        .await;

        // Create webhook payload
        let event_id = generate_test_event_id();
        let contract_id = generate_shopify_contract_id();
        let customer_id = generate_shopify_customer_id();
        let user_id = generate_test_user_id();
        let plan_id = generate_test_plan_id();

        let payload = build_shopify_subscription_contracts_create_event(
            event_id.clone(),
            contract_id,
            customer_id,
            user_id,
            plan_id,
            &realm_id,
            None,
        );

        // Send webhook with invalid signature
        let response = send_shopify_webhook_with_invalid_signature(
            &app,
            &realm_id,
            "subscription_contracts/create",
            payload,
        )
        .await;

        assert_shopify_webhook_unauthorized(&response);
    }

    /// User Story: US-PP-011 验收标准 5
    /// 场景：Webhook 缺少 HMAC 签名应该返回 400
    #[test_context(ShopifyTestContext)]
    #[tokio::test]
    async fn test_scenario_shopify_hmac_verification_fails_without_signature(
        ctx: &mut ShopifyTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();

        // Setup Shopify config
        setup_shopify_config(
            ctx,
            &realm_id,
            "test-shop.myshopify.com",
            "shpat_test_token",
            "shp_test_token",
            "test_client_secret",
            "2024-01",
        )
        .await;

        // Create webhook payload
        let contract_id = generate_shopify_contract_id();
        let customer_id = generate_shopify_customer_id();
        let user_id = generate_test_user_id();
        let plan_id = generate_test_plan_id();

        let payload = build_shopify_subscription_contracts_create_event(
            generate_test_event_id(),
            contract_id,
            customer_id,
            user_id,
            plan_id,
            &realm_id,
            None,
        );

        // Send webhook without signature
        let response = send_shopify_webhook_without_signature(
            &app,
            &realm_id,
            "subscription_contracts/create",
            payload,
        )
        .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    /// ============================================================================
    /// subscription_contracts/create 测试
    /// ============================================================================
    /// User Story: US-PP-011 验收标准 1, 2
    /// 场景：接收订阅合同创建事件并发放初始积分
    #[test_context(ShopifyTestContext)]
    #[tokio::test]
    async fn test_scenario_shopify_subscription_contracts_create_success(
        ctx: &mut ShopifyTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();
        let client_secret = "test_client_secret";

        // Setup Shopify config
        setup_shopify_config(
            ctx,
            &realm_id,
            "test-shop.myshopify.com",
            "shpat_test_token",
            "shp_test_token",
            client_secret,
            "2024-01",
        )
        .await;

        // Setup plan config
        let plan_id = generate_test_plan_id();
        setup_test_plan_config_with_points(ctx, &realm_id, plan_id, 1000).await;

        // Create user
        let email = format!("test-{}@example.com", Uuid::now_v7());
        let user_id = create_test_user(ctx, &email, "password123").await;

        // Create webhook payload
        let event_id = generate_test_event_id();
        let contract_id = generate_shopify_contract_id();
        let customer_id = generate_shopify_customer_id();

        let payload = build_shopify_subscription_contracts_create_event(
            event_id.clone(),
            contract_id.clone(),
            customer_id,
            user_id,
            plan_id,
            &realm_id,
            None,
        );

        // Send webhook
        let response = send_shopify_webhook_with_signature(
            &app,
            &realm_id,
            "subscription_contracts/create",
            payload,
            client_secret,
        )
        .await;

        assert_shopify_webhook_success(&response);

        // Verify payment event was recorded
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        assert!(
            payment_event_exists(ctx, &event_id).await,
            "Payment event should be recorded"
        );

        // Verify subscription was created
        let subscription_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM subscription
             WHERE external_subscription_id = $1 AND payment_provider = 'shopify')",
        )
        .bind(&contract_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap_or(false);

        assert!(
            subscription_exists,
            "Subscription should be created for contract"
        );

        // Verify Shopify binding was created
        let binding = get_shopify_binding_by_contract(ctx, &contract_id).await;
        assert!(binding.is_some(), "Shopify binding should be created");

        // Verify points were granted (check credit ledger)
        let user_points: i64 = sqlx::query_scalar(
            "SELECT COALESCE(subscription_balance, 0) FROM points_accounts
             WHERE user_id = $1 AND realm_id = $2",
        )
        .bind(user_id)
        .bind(&realm_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap_or(0);

        assert_eq!(user_points, 1000, "User should receive 1000 initial points");
    }

    /// User Story: US-PP-011 验收标准 3
    /// 场景：缺少 casUserId 时创建未归属订阅，但不发积分
    #[test_context(ShopifyTestContext)]
    #[tokio::test]
    async fn test_scenario_shopify_subscription_contracts_create_without_herald_user_identifier(
        ctx: &mut ShopifyTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();
        let client_secret = "test_client_secret";

        // Setup Shopify config
        setup_shopify_config(
            ctx,
            &realm_id,
            "test-shop.myshopify.com",
            "shpat_test_token",
            "shp_test_token",
            client_secret,
            "2024-01",
        )
        .await;

        // Setup plan config for the unclaimed subscription
        let plan_id = generate_test_plan_id();
        setup_test_plan_config_with_points(ctx, &realm_id, plan_id, 1000).await;

        // Create webhook payload WITHOUT casUserId
        let event_id = generate_test_event_id();
        let contract_id = generate_shopify_contract_id();
        let customer_id = generate_shopify_customer_id();

        let payload = json!({
            "testEventId": event_id,
            "id": contract_id,
            "adminGraphqlApiId": contract_id,
            "customerId": customer_id,
            "originOrderId": format!("gid://shopify/Order/{}", Uuid::now_v7()),
            "sellingPlanId": format!("gid://shopify/SellingPlan/{}", Uuid::now_v7()),
            "currentPeriodEnd": (chrono::Utc::now() + chrono::Duration::days(30)).to_rfc3339(),
            "status": "ACTIVE",
            "casRealmId": realm_id,
            "casPlanId": plan_id.to_string()
        });

        // Send webhook
        let response = send_shopify_webhook_with_signature(
            &app,
            &realm_id,
            "subscription_contracts/create",
            payload,
            client_secret,
        )
        .await;

        // Should still accept the webhook (202) but only record payment_event
        assert_shopify_webhook_success(&response);

        // Verify payment event was recorded
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        assert!(
            payment_event_exists(ctx, &event_id).await,
            "Payment event should be recorded even without casUserId"
        );

        // Verify unclaimed subscription was created
        let created_user_id: Option<Uuid> = sqlx::query_scalar(
            "SELECT user_id FROM subscription
             WHERE external_subscription_id = $1 AND payment_provider = 'shopify'",
        )
        .bind(&contract_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .expect("Subscription should be created without casUserId");

        assert!(
            created_user_id.is_none(),
            "Subscription should remain unclaimed when casUserId is missing"
        );

        let unclaimed_points: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM points_accounts
             WHERE realm_id = $1",
        )
        .bind(&realm_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap_or(0);

        assert_eq!(
            unclaimed_points, 0,
            "No points should be granted before the Shopify subscription is claimed"
        );
    }

    /// User Story: US-PP-011 验收标准 4
    /// 场景：事件幂等处理 - 重复事件应该被忽略
    #[test_context(ShopifyTestContext)]
    #[tokio::test]
    async fn test_scenario_shopify_subscription_contracts_create_idempotency(
        ctx: &mut ShopifyTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();
        let client_secret = "test_client_secret";

        // Setup Shopify config
        setup_shopify_config(
            ctx,
            &realm_id,
            "test-shop.myshopify.com",
            "shpat_test_token",
            "shp_test_token",
            client_secret,
            "2024-01",
        )
        .await;

        // Setup plan config
        let plan_id = generate_test_plan_id();
        setup_test_plan_config_with_points(ctx, &realm_id, plan_id, 1000).await;

        // Create user
        let email = format!("test-{}@example.com", Uuid::now_v7());
        let user_id = create_test_user(ctx, &email, "password123").await;

        // Create webhook payload
        let event_id = generate_test_event_id();
        let contract_id = generate_shopify_contract_id();
        let customer_id = generate_shopify_customer_id();

        let payload = build_shopify_subscription_contracts_create_event(
            event_id.clone(),
            contract_id.clone(),
            customer_id,
            user_id,
            plan_id,
            &realm_id,
            None,
        );

        // Send webhook first time
        let response1 = send_shopify_webhook_with_signature(
            &app,
            &realm_id,
            "subscription_contracts/create",
            payload.clone(),
            client_secret,
        )
        .await;

        assert_shopify_webhook_success(&response1);

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Send webhook second time (duplicate)
        let response2 = send_shopify_webhook_with_signature(
            &app,
            &realm_id,
            "subscription_contracts/create",
            payload,
            client_secret,
        )
        .await;

        assert_shopify_webhook_success(&response2);

        // Verify only one subscription was created
        let subscription_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM subscription
             WHERE external_subscription_id = $1 AND payment_provider = 'shopify'",
        )
        .bind(&contract_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap_or(0);

        assert_eq!(
            subscription_count, 1,
            "Only one subscription should be created"
        );

        // Verify points were only granted once
        let user_points: i64 = sqlx::query_scalar(
            "SELECT COALESCE(subscription_balance, 0) FROM points_accounts
             WHERE user_id = $1 AND realm_id = $2",
        )
        .bind(user_id)
        .bind(&realm_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap_or(0);

        assert_eq!(user_points, 1000, "Points should only be granted once");
    }
    /// User Story: US-PP-011
    /// 场景：第一次处理失败后，相同 event_id 重试仍可成功处理
    #[test_context(ShopifyTestContext)]
    #[tokio::test]
    async fn test_scenario_shopify_webhook_retry_after_processing_failure(
        ctx: &mut ShopifyTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();
        let client_secret = "test_client_secret";

        setup_shopify_config(
            ctx,
            &realm_id,
            "test-shop.myshopify.com",
            "shpat_test_token",
            "shp_test_token",
            client_secret,
            "2024-01",
        )
        .await;

        let plan_id = generate_test_plan_id();
        let email = format!("test-{}@example.com", Uuid::now_v7());
        let user_id = create_test_user(ctx, &email, "password123").await;
        let event_id = generate_test_event_id();
        let contract_id = generate_shopify_contract_id();
        let customer_id = generate_shopify_customer_id();

        let payload = build_shopify_subscription_contracts_create_event(
            event_id.clone(),
            contract_id.clone(),
            customer_id,
            user_id,
            plan_id,
            &realm_id,
            None,
        );

        let response1 = send_shopify_webhook_with_signature(
            &app,
            &realm_id,
            "subscription_contracts/create",
            payload.clone(),
            client_secret,
        )
        .await;

        assert_ne!(
            response1.status(),
            StatusCode::ACCEPTED,
            "Missing plan config should fail the first attempt"
        );

        setup_test_plan_config_with_points(ctx, &realm_id, plan_id, 1000).await;

        let response2 = send_shopify_webhook_with_signature(
            &app,
            &realm_id,
            "subscription_contracts/create",
            payload,
            client_secret,
        )
        .await;

        assert_shopify_webhook_success(&response2);

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let subscription_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM subscription
             WHERE external_subscription_id = $1 AND payment_provider = 'shopify'",
        )
        .bind(&contract_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap_or(0);

        assert_eq!(
            subscription_count, 1,
            "Retry should create the subscription"
        );
        assert!(
            payment_event_exists(ctx, &event_id).await,
            "Successful retry should mark the event as handled"
        );
    }

    /// ============================================================================
    /// subscription_contracts/update 测试
    /// ============================================================================
    /// User Story: US-PP-012 验收标准 3
    /// 场景：处理订阅计划升级
    #[test_context(ShopifyTestContext)]
    #[tokio::test]
    async fn test_scenario_shopify_subscription_contracts_update_upgrade(
        ctx: &mut ShopifyTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();
        let client_secret = "test_client_secret";

        // Setup Shopify config
        setup_shopify_config(
            ctx,
            &realm_id,
            "test-shop.myshopify.com",
            "shpat_test_token",
            "shp_test_token",
            client_secret,
            "2024-01",
        )
        .await;

        // Setup plan configs (old plan: 1000 points, new plan: 2000 points)
        let old_plan_id = generate_test_plan_id();
        let new_plan_id = generate_test_plan_id();
        setup_test_plan_config_with_points(ctx, &realm_id, old_plan_id, 1000).await;
        setup_test_plan_config_with_points(ctx, &realm_id, new_plan_id, 2000).await;

        // Create user
        let email = format!("test-{}@example.com", Uuid::now_v7());
        let user_id = create_test_user(ctx, &email, "password123").await;

        // Create initial subscription and binding
        let contract_id = generate_shopify_contract_id();
        let customer_id = generate_shopify_customer_id();

        // First, create subscription via subscription_contracts/create
        let create_event_id = generate_test_event_id();
        let create_payload = build_shopify_subscription_contracts_create_event(
            create_event_id,
            contract_id.clone(),
            customer_id.clone(),
            user_id,
            old_plan_id,
            &realm_id,
            None,
        );

        let response = send_shopify_webhook_with_signature(
            &app,
            &realm_id,
            "subscription_contracts/create",
            create_payload,
            client_secret,
        )
        .await;

        assert_shopify_webhook_success(&response);
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Get subscription ID
        let subscription_id: Uuid =
            sqlx::query_scalar("SELECT id FROM subscription WHERE external_subscription_id = $1")
                .bind(&contract_id)
                .fetch_one(&ctx.app_state.pool)
                .await
                .expect("Subscription not found");

        // Get initial points
        let initial_points: i64 = sqlx::query_scalar(
            "SELECT COALESCE(subscription_balance, 0) FROM points_accounts
             WHERE user_id = $1 AND realm_id = $2",
        )
        .bind(user_id)
        .bind(&realm_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap_or(0);

        // Send subscription_contracts/update webhook (upgrade)
        let update_event_id = generate_test_event_id();
        let update_payload = build_shopify_subscription_contracts_update_event(
            update_event_id,
            contract_id.clone(),
            customer_id,
            user_id,
            old_plan_id,
            new_plan_id,
            &realm_id,
            Some(2),
        );

        let response = send_shopify_webhook_with_signature(
            &app,
            &realm_id,
            "subscription_contracts/update",
            update_payload,
            client_secret,
        )
        .await;

        assert_shopify_webhook_success(&response);
        let mut updated_plan_id = old_plan_id;
        let mut final_points = initial_points;

        for _ in 0..30 {
            updated_plan_id = sqlx::query_scalar("SELECT plan_id FROM subscription WHERE id = $1")
                .bind(subscription_id)
                .fetch_one(&ctx.app_state.pool)
                .await
                .expect("Subscription not found");

            final_points = sqlx::query_scalar(
                "SELECT COALESCE(subscription_balance, 0) FROM points_accounts
                 WHERE user_id = $1 AND realm_id = $2",
            )
            .bind(user_id)
            .bind(&realm_id)
            .fetch_one(&ctx.app_state.pool)
            .await
            .unwrap_or(0);

            if updated_plan_id == new_plan_id && final_points == initial_points + 1000 {
                break;
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        assert_eq!(updated_plan_id, new_plan_id, "Plan should be updated");

        assert_eq!(
            final_points,
            initial_points + 1000,
            "Upgrade difference points should be granted"
        );
    }

    /// User Story: US-PP-012 验收标准 7
    /// 场景：低 revision 事件不覆盖高 revision
    #[test_context(ShopifyTestContext)]
    #[tokio::test]
    async fn test_scenario_shopify_subscription_contracts_update_out_of_order_rejected(
        ctx: &mut ShopifyTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();
        let client_secret = "test_client_secret";

        // Setup Shopify config
        setup_shopify_config(
            ctx,
            &realm_id,
            "test-shop.myshopify.com",
            "shpat_test_token",
            "shp_test_token",
            client_secret,
            "2024-01",
        )
        .await;

        // Setup plan config
        let plan_id = generate_test_plan_id();
        setup_test_plan_config_with_points(ctx, &realm_id, plan_id, 1000).await;

        // Create user
        let email = format!("test-{}@example.com", Uuid::now_v7());
        let user_id = create_test_user(ctx, &email, "password123").await;

        // Create initial subscription
        let contract_id = generate_shopify_contract_id();
        let customer_id = generate_shopify_customer_id();

        let create_event_id = generate_test_event_id();
        let create_payload = build_shopify_subscription_contracts_create_event(
            create_event_id,
            contract_id.clone(),
            customer_id.clone(),
            user_id,
            plan_id,
            &realm_id,
            None,
        );

        let response = send_shopify_webhook_with_signature(
            &app,
            &realm_id,
            "subscription_contracts/create",
            create_payload,
            client_secret,
        )
        .await;

        assert_shopify_webhook_success(&response);
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Get initial revision ID
        let (_binding_id, _subscription_id, initial_revision) =
            get_shopify_binding_by_contract(ctx, &contract_id)
                .await
                .unwrap();

        // Send update webhook with same or lower revision (should be rejected)
        let update_event_id = generate_test_event_id();
        let update_payload = build_shopify_subscription_contracts_update_event(
            update_event_id,
            contract_id.clone(),
            customer_id,
            user_id,
            plan_id, // Same plan (no actual change)
            plan_id,
            &realm_id,
            Some(initial_revision),
        );

        let response = send_shopify_webhook_with_signature(
            &app,
            &realm_id,
            "subscription_contracts/update",
            update_payload,
            client_secret,
        )
        .await;

        // Should still return 202 (webhook accepted) but event not processed
        assert_shopify_webhook_success(&response);

        // Verify revision ID hasn't changed
        let (_binding_id, _subscription_id, final_revision) =
            get_shopify_binding_by_contract(ctx, &contract_id)
                .await
                .unwrap();

        assert_eq!(
            final_revision, initial_revision,
            "Revision ID should not change for out-of-order event"
        );
    }

    /// ============================================================================
    /// subscription_billing_attempts/success 测试
    /// ============================================================================
    /// User Story: US-PP-012 验收标准 1
    /// 场景：处理续费成功事件并发放续费积分
    #[test_context(ShopifyTestContext)]
    #[tokio::test]
    async fn test_scenario_shopify_billing_attempt_success_renewal_points_granted(
        ctx: &mut ShopifyTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();
        let client_secret = "test_client_secret";

        // Setup Shopify config
        setup_shopify_config(
            ctx,
            &realm_id,
            "test-shop.myshopify.com",
            "shpat_test_token",
            "shp_test_token",
            client_secret,
            "2024-01",
        )
        .await;

        // Setup plan config
        let plan_id = generate_test_plan_id();
        setup_test_plan_config_with_points(ctx, &realm_id, plan_id, 1000).await;

        // Create user
        let email = format!("test-{}@example.com", Uuid::now_v7());
        let user_id = create_test_user(ctx, &email, "password123").await;

        // Create initial subscription
        let contract_id = generate_shopify_contract_id();
        let customer_id = generate_shopify_customer_id();

        let create_event_id = generate_test_event_id();
        let create_payload = build_shopify_subscription_contracts_create_event(
            create_event_id,
            contract_id.clone(),
            customer_id.clone(),
            user_id,
            plan_id,
            &realm_id,
            None,
        );

        let response = send_shopify_webhook_with_signature(
            &app,
            &realm_id,
            "subscription_contracts/create",
            create_payload,
            client_secret,
        )
        .await;

        assert_shopify_webhook_success(&response);
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Get initial points
        let initial_points: i64 = sqlx::query_scalar(
            "SELECT COALESCE(subscription_balance, 0) FROM points_accounts
             WHERE user_id = $1 AND realm_id = $2",
        )
        .bind(user_id)
        .bind(&realm_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap_or(0);

        // Send billing attempt success webhook
        let billing_event_id = generate_test_event_id();
        let billing_attempt_id = generate_shopify_billing_attempt_id();
        let order_id = generate_shopify_order_id();

        let billing_payload = build_shopify_billing_attempt_success_event(
            billing_event_id,
            billing_attempt_id.clone(),
            contract_id.clone(),
            order_id.clone(),
            &realm_id,
        );

        let response = send_shopify_webhook_with_signature(
            &app,
            &realm_id,
            "subscription_billing_attempts/success",
            billing_payload,
            client_secret,
        )
        .await;

        assert_shopify_webhook_success(&response);
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Verify renewal points were granted
        let final_points: i64 = sqlx::query_scalar(
            "SELECT COALESCE(subscription_balance, 0) FROM points_accounts
             WHERE user_id = $1 AND realm_id = $2",
        )
        .bind(user_id)
        .bind(&realm_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap_or(0);

        assert_eq!(
            final_points,
            initial_points + 1000,
            "Renewal points should be granted"
        );

        // Verify Shopify binding was updated with billing attempt and order IDs
        let (_binding_id, _subscription_id, _revision) =
            get_shopify_binding_by_contract(ctx, &contract_id)
                .await
                .unwrap();

        // Check if order_id was updated (we need to query this)
        let binding_order_id: Option<String> = sqlx::query_scalar(
            "SELECT last_order_id FROM shopify_subscription_binding WHERE contract_id = $1",
        )
        .bind(&contract_id)
        .fetch_optional(&ctx.app_state.pool)
        .await
        .ok()
        .flatten();

        assert_eq!(
            binding_order_id,
            Some(order_id),
            "Order ID should be updated in binding"
        );
    }

    /// ============================================================================
    /// subscription_billing_attempts/failure 测试
    /// ============================================================================
    /// User Story: US-PP-012 验收标准 2
    /// 场景：处理续费失败事件并标记订阅为 past_due
    #[test_context(ShopifyTestContext)]
    #[tokio::test]
    async fn test_scenario_shopify_billing_attempt_failure_marks_past_due(
        ctx: &mut ShopifyTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();
        let client_secret = "test_client_secret";

        // Setup Shopify config
        setup_shopify_config(
            ctx,
            &realm_id,
            "test-shop.myshopify.com",
            "shpat_test_token",
            "shp_test_token",
            client_secret,
            "2024-01",
        )
        .await;

        // Setup plan config
        let plan_id = generate_test_plan_id();
        setup_test_plan_config_with_points(ctx, &realm_id, plan_id, 1000).await;

        // Create user
        let email = format!("test-{}@example.com", Uuid::now_v7());
        let user_id = create_test_user(ctx, &email, "password123").await;

        // Create initial subscription
        let contract_id = generate_shopify_contract_id();
        let customer_id = generate_shopify_customer_id();

        let create_event_id = generate_test_event_id();
        let create_payload = build_shopify_subscription_contracts_create_event(
            create_event_id,
            contract_id.clone(),
            customer_id.clone(),
            user_id,
            plan_id,
            &realm_id,
            None,
        );

        let response = send_shopify_webhook_with_signature(
            &app,
            &realm_id,
            "subscription_contracts/create",
            create_payload,
            client_secret,
        )
        .await;

        assert_shopify_webhook_success(&response);
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Get subscription ID
        let subscription_id: Uuid =
            sqlx::query_scalar("SELECT id FROM subscription WHERE external_subscription_id = $1")
                .bind(&contract_id)
                .fetch_one(&ctx.app_state.pool)
                .await
                .expect("Subscription not found");

        // Verify initial status is active
        let initial_status = get_subscription_status(ctx, subscription_id).await.unwrap();
        assert_eq!(initial_status, "active", "Initial status should be active");

        // Send billing attempt failure webhook
        let billing_event_id = generate_test_event_id();
        let billing_attempt_id = generate_shopify_billing_attempt_id();

        let billing_payload = build_shopify_billing_attempt_failure_event(
            billing_event_id,
            billing_attempt_id,
            contract_id.clone(),
            "card_declined".to_string(),
            "Your card was declined".to_string(),
        );

        let response = send_shopify_webhook_with_signature(
            &app,
            &realm_id,
            "subscription_billing_attempts/failure",
            billing_payload,
            client_secret,
        )
        .await;

        assert_shopify_webhook_success(&response);
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Verify subscription status was updated to past_due
        let final_status = get_subscription_status(ctx, subscription_id).await.unwrap();
        assert_eq!(
            final_status, "past_due",
            "Status should be past_due after billing failure"
        );

        // Verify points were NOT revoked
        let user_points: i64 = sqlx::query_scalar(
            "SELECT COALESCE(subscription_balance, 0) FROM points_accounts
             WHERE user_id = $1 AND realm_id = $2",
        )
        .bind(user_id)
        .bind(&realm_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap_or(0);

        assert_eq!(
            user_points, 1000,
            "Points should NOT be revoked on billing failure"
        );
    }

    /// ============================================================================
    /// refunds/create 测试
    /// ============================================================================
    /// User Story: US-PP-012 验收标准 5
    /// 场景：处理退款事件并撤销积分
    #[test_context(ShopifyTestContext)]
    #[tokio::test]
    async fn test_scenario_shopify_refunds_create_revokes_points(ctx: &mut ShopifyTestContext) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();
        let client_secret = "test_client_secret";

        // Setup Shopify config
        setup_shopify_config(
            ctx,
            &realm_id,
            "test-shop.myshopify.com",
            "shpat_test_token",
            "shp_test_token",
            client_secret,
            "2024-01",
        )
        .await;

        // Setup plan config
        let plan_id = generate_test_plan_id();
        setup_test_plan_config_with_points(ctx, &realm_id, plan_id, 1000).await;

        // Create user
        let email = format!("test-{}@example.com", Uuid::now_v7());
        let user_id = create_test_user(ctx, &email, "password123").await;

        // Create initial subscription
        let contract_id = generate_shopify_contract_id();
        let customer_id = generate_shopify_customer_id();
        let order_id = generate_shopify_order_id();

        let create_event_id = generate_test_event_id();
        let create_payload = build_shopify_subscription_contracts_create_event(
            create_event_id,
            contract_id.clone(),
            customer_id.clone(),
            user_id,
            plan_id,
            &realm_id,
            None,
        );

        let response = send_shopify_webhook_with_signature(
            &app,
            &realm_id,
            "subscription_contracts/create",
            create_payload,
            client_secret,
        )
        .await;

        assert_shopify_webhook_success(&response);
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Get subscription ID
        let _subscription_id: Uuid =
            sqlx::query_scalar("SELECT id FROM subscription WHERE external_subscription_id = $1")
                .bind(&contract_id)
                .fetch_one(&ctx.app_state.pool)
                .await
                .expect("Subscription not found");

        // Update Shopify binding with order_id (simulating successful payment)
        sqlx::query(
            "UPDATE shopify_subscription_binding
             SET last_order_id = $1
             WHERE contract_id = $2",
        )
        .bind(&order_id)
        .bind(&contract_id)
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to update order_id");

        // Verify initial points
        let initial_points: i64 = sqlx::query_scalar(
            "SELECT COALESCE(subscription_balance, 0) FROM points_accounts
             WHERE user_id = $1 AND realm_id = $2",
        )
        .bind(user_id)
        .bind(&realm_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap_or(0);

        assert_eq!(initial_points, 1000, "User should have initial points");

        // Send refund webhook
        let refund_event_id = generate_test_event_id();
        let refund_id = format!("gid://shopify/Refund/{}", Uuid::now_v7());

        let refund_payload = build_shopify_refunds_create_event(
            refund_event_id,
            refund_id.clone(),
            order_id.clone(),
            2500, // refund amount in cents
            "USD",
            Some("Customer request"),
        );

        let response = send_shopify_webhook_with_signature(
            &app,
            &realm_id,
            "refunds/create",
            refund_payload,
            client_secret,
        )
        .await;

        assert_shopify_webhook_success(&response);
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Verify points were revoked
        let final_points: i64 = sqlx::query_scalar(
            "SELECT COALESCE(subscription_balance, 0) FROM points_accounts
             WHERE user_id = $1 AND realm_id = $2",
        )
        .bind(user_id)
        .bind(&realm_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap_or(0);

        assert!(
            final_points < initial_points,
            "Points should be revoked after refund"
        );
    }
}
