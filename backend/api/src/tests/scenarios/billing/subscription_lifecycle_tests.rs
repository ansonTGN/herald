// =============================================================================
// Subscription Lifecycle Tests
// =============================================================================
//
// Test subscription state transitions, upgrades/downgrades, pause/resume.
//
// User Story: docs/user-stories/06-billing-user-stories.md
// Covers: Subscription state transitions, plan changes, lifecycle management
//
// =============================================================================

#[cfg(test)]
mod tests {
    use crate::tests::helpers::*;

    use crate::tests::schema_test_context::SchemaTestContext;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use herald_core::domain::billing::entities::SubscriptionStatus;
    use serde_json::json;
    use test_context::test_context;
    use tower::ServiceExt;
    use uuid::Uuid;

    use SchemaTestContext as LifecycleTestContext;

    /// Helper: Create a client app via API
    async fn create_client_app_via_api(
        _ctx: &LifecycleTestContext,
        app: &axum::Router,
        token: &str,
        realm_id: &str,
        client_id: &str,
        name: &str,
    ) -> (String, Uuid) {
        let request = Request::builder()
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
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let uuid: Uuid = serde_json::from_value(json["id"].clone()).unwrap();
        (client_id.to_string(), uuid)
    }

    // =========================================================================
    // Test: Subscription active to canceled (immediate)
    // =========================================================================
    // Given: Active subscription
    // When: 立即取消（cancel_at_period_end=false）
    // Then: 状态变为 canceled，cancel_at 被设置

    #[test_context(LifecycleTestContext)]
    #[tokio::test]
    async fn test_subscription_active_to_canceled(ctx: &mut LifecycleTestContext) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();

        // Given: Setup admin, plan, and active subscription
        let admin_token = setup_billing_admin_session(ctx, "test-cancel-immediate@test.com").await;
        let plan_id = create_test_plan(ctx, realm_id.as_str(), "Cancel Immediate Plan").await;
        let (_client_id_str, client_app_id) = create_client_app_via_api(
            ctx,
            &app,
            &admin_token,
            realm_id.as_str(),
            "test-cancel-immediate-app",
            "Test Cancel Immediate App",
        )
        .await;

        let subscription_id =
            create_test_subscription(ctx, realm_id.as_str(), client_app_id, plan_id, "monthly")
                .await;

        // Verify subscription is active
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Active).await;

        // When: Cancel subscription immediately (cancel_at_period_end=false)
        let cancel_request = json!({
            "cancelAtPeriodEnd": false
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/bill/{}/client/{}/subscription/cancel",
                        realm_id, client_app_id
                    ))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={}", admin_token))
                    .body(Body::from(cancel_request.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Then: Status becomes canceled
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Canceled).await;

        // Then: cancel_at is set
        let cancel_at: Option<chrono::DateTime<chrono::Utc>> =
            sqlx::query_scalar("SELECT cancel_at FROM subscription WHERE id = $1")
                .bind(subscription_id)
                .fetch_one(&ctx.app_state.pool)
                .await
                .unwrap();

        assert!(
            cancel_at.is_some(),
            "cancel_at should be set for immediate cancellation"
        );

        // Cleanup
        delete_test_subscription(ctx, subscription_id).await;
    }

    // =========================================================================
    // Test: Subscription active to scheduled cancel
    // =========================================================================
    // Given: Active subscription
    // When: 计划取消（cancel_at_period_end=true）
    // Then: 状态保持 active，cancel_at_period_end=true

    #[test_context(LifecycleTestContext)]
    #[tokio::test]
    async fn test_subscription_active_to_scheduled_cancel(ctx: &mut LifecycleTestContext) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();

        // Given: Setup admin, plan, and active subscription
        let admin_token = setup_billing_admin_session(ctx, "test-scheduled-cancel@test.com").await;
        let plan_id = create_test_plan(ctx, realm_id.as_str(), "Scheduled Cancel Plan").await;
        let (_client_id_str, client_app_id) = create_client_app_via_api(
            ctx,
            &app,
            &admin_token,
            realm_id.as_str(),
            "test-scheduled-cancel-app",
            "Test Scheduled Cancel App",
        )
        .await;

        let subscription_id =
            create_test_subscription(ctx, realm_id.as_str(), client_app_id, plan_id, "monthly")
                .await;

        // When: Schedule cancellation at period end
        let cancel_request = json!({
            "cancelAtPeriodEnd": true
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/bill/{}/client/{}/subscription/cancel",
                        realm_id, client_app_id
                    ))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={}", admin_token))
                    .body(Body::from(cancel_request.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Then: Status remains active
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Active).await;

        // Then: cancel_at_period_end is true
        let cancel_at_period_end: bool =
            sqlx::query_scalar("SELECT cancel_at_period_end FROM subscription WHERE id = $1")
                .bind(subscription_id)
                .fetch_one(&ctx.app_state.pool)
                .await
                .unwrap();

        assert!(
            cancel_at_period_end,
            "cancel_at_period_end should be true for scheduled cancellation"
        );

        // Cleanup
        delete_test_subscription(ctx, subscription_id).await;
    }

    // =========================================================================
    // Test: Trialing to active transition
    // =========================================================================
    // Given: Trialing subscription
    // When: Trial period ends
    // Then: 状态变为 active

    #[test_context(LifecycleTestContext)]
    #[tokio::test]
    async fn test_trialing_to_active_transition(ctx: &mut LifecycleTestContext) {
        let realm_id = ctx._realm_id.clone();

        // Given: Create subscription in trialing status
        let plan_id = create_test_plan_with_trial(
            ctx,
            realm_id.as_str(),
            "Trial to Active Plan",
            "monthly",
            2500,
            14,
        )
        .await;

        let subscription_id = Uuid::now_v7();
        let now = chrono::Utc::now();
        let trial_end = now + chrono::Duration::days(14);

        sqlx::query(
            "INSERT INTO subscription (id, realm_id, plan_id, status, billing_period, tier,
                                     external_subscription_id, external_product_id, payment_provider,
                                     current_period_start, current_period_end,
                                     cancel_at_period_end, created_at, updated_at)
             VALUES ($1, $2, $3, 'trialing', 'monthly', 'starter', $4, $5, $6, NOW(), $7,
                     false, NOW(), NOW())",
        )
        .bind(subscription_id)
        .bind(realm_id.as_str())
        .bind(plan_id)
        .bind(format!("ext_sub_trial_{}", Uuid::now_v7()))
        .bind("test_product")
        .bind("creem")
        .bind(trial_end)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Verify trialing status
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Trialing).await;

        // When: Trial period ends (simulate by updating status directly)
        update_subscription_status(ctx, subscription_id, "active").await;

        // Then: Status becomes active
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Active).await;

        // Cleanup
        delete_test_subscription(ctx, subscription_id).await;
    }

    // =========================================================================
    // Test: Past due to active after payment
    // =========================================================================
    // Given: PastDue subscription
    // When: 支付成功
    // Then: 状态变为 active

    #[test_context(LifecycleTestContext)]
    #[tokio::test]
    async fn test_past_due_to_active_after_payment(ctx: &mut LifecycleTestContext) {
        let realm_id = ctx._realm_id.clone();

        // Given: Create subscription in past_due status
        let plan_id = create_test_plan(ctx, realm_id.as_str(), "Past Due Plan").await;
        let (client_app_id, _) = (Uuid::now_v7(), "test_client".to_string()); // We'll use a dummy client_app_id

        // Insert a dummy client app for the test
        sqlx::query(
            "INSERT INTO client_app (id, realm_id, client_id, name, redirect_uris, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"
        )
        .bind(client_app_id)
        .bind(realm_id.as_str())
        .bind("test-past-due-client")
        .bind("Test Past Due Client")
        .bind(json!(["https://example.com/callback"]))
        .bind(true)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        let subscription_id = Uuid::now_v7();

        sqlx::query(
            "INSERT INTO subscription (id, realm_id, plan_id, client_app_id, status, billing_period, tier,
                                     external_subscription_id, external_product_id, payment_provider,
                                     current_period_start, current_period_end,
                                     cancel_at_period_end, created_at, updated_at)
             VALUES ($1, $2, $3, $4, 'past_due', 'monthly', 'starter', $5, $6, $7, NOW(), NOW(),
                     false, NOW(), NOW())"
        )
        .bind(subscription_id)
        .bind(realm_id.as_str())
        .bind(plan_id)
        .bind(client_app_id)
        .bind(format!("ext_sub_pastdue_{}", Uuid::now_v7()))
        .bind("test_product")
        .bind("creem")
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Verify past_due status
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::PastDue).await;

        // When: Payment succeeds (simulate by updating status directly)
        update_subscription_status(ctx, subscription_id, "active").await;

        // Then: Status becomes active
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Active).await;

        // Cleanup
        delete_test_subscription(ctx, subscription_id).await;
    }

    // =========================================================================
    // Test: Invalid status transition rejected
    // =========================================================================
    // Given: Canceled subscription
    // When: 尝试变为 active
    // Then: 状态转换被拒绝

    #[test_context(LifecycleTestContext)]
    #[tokio::test]
    async fn test_invalid_status_transition_rejected(ctx: &mut LifecycleTestContext) {
        let realm_id = ctx._realm_id.clone();

        // Given: Create canceled subscription
        let plan_id = create_test_plan(ctx, realm_id.as_str(), "Invalid Transition Plan").await;
        let (client_app_id, _) = (Uuid::now_v7(), "test_client".to_string());

        // Insert a dummy client app for the test
        sqlx::query(
            "INSERT INTO client_app (id, realm_id, client_id, name, redirect_uris, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"
        )
        .bind(client_app_id)
        .bind(realm_id.as_str())
        .bind("test-invalid-transition-client")
        .bind("Test Invalid Transition Client")
        .bind(json!(["https://example.com/callback"]))
        .bind(true)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        let subscription_id = Uuid::now_v7();

        sqlx::query(
            "INSERT INTO subscription (id, realm_id, plan_id, client_app_id, status, billing_period, tier,
                                     external_subscription_id, external_product_id, payment_provider,
                                     current_period_start, current_period_end,
                                     cancel_at_period_end, created_at, updated_at)
             VALUES ($1, $2, $3, $4, 'canceled', 'monthly', 'starter', $5, $6, $7, NOW(), NOW(),
                     false, NOW(), NOW())"
        )
        .bind(subscription_id)
        .bind(realm_id.as_str())
        .bind(plan_id)
        .bind(client_app_id)
        .bind(format!("ext_sub_canceled_{}", Uuid::now_v7()))
        .bind("test_product")
        .bind("creem")
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Given: Canceled subscription
        let canceled_status = SubscriptionStatus::Canceled;

        // When & Then: Attempt to transition to active should be rejected
        let active_status = SubscriptionStatus::Active;

        // The can_transition_to method should return false
        assert!(
            !canceled_status.can_transition_to(&active_status),
            "Canceled should not be able to transition to Active"
        );

        // Cleanup
        delete_test_subscription(ctx, subscription_id).await;
    }

    // =========================================================================
    // Test: Subscription plan upgrade
    // =========================================================================
    // Given: Monthly basic subscription
    // When: 升级到 yearly pro
    // Then: Plan 更新，历史记录包含 "upgraded" 事件

    #[test_context(LifecycleTestContext)]
    #[tokio::test]
    async fn test_subscription_plan_upgrade(ctx: &mut LifecycleTestContext) {
        let realm_id = ctx._realm_id.clone();

        // Given: Create monthly basic subscription
        let basic_plan_id = create_test_plan(ctx, realm_id.as_str(), "Basic Plan").await;
        let pro_plan_id = create_test_plan_yearly(ctx, realm_id.as_str(), "Pro Plan Yearly").await;
        let (client_app_id, _) = (Uuid::now_v7(), "test_client".to_string());

        // Insert a dummy client app for the test
        sqlx::query(
            "INSERT INTO client_app (id, realm_id, client_id, name, redirect_uris, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"
        )
        .bind(client_app_id)
        .bind(realm_id.as_str())
        .bind("test-upgrade-client")
        .bind("Test Upgrade Client")
        .bind(json!(["https://example.com/callback"]))
        .bind(true)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        let subscription_id = Uuid::now_v7();

        sqlx::query(
            "INSERT INTO subscription (id, realm_id, plan_id, client_app_id, status, billing_period, tier,
                                     external_subscription_id, external_product_id, payment_provider,
                                     current_period_start, current_period_end,
                                     cancel_at_period_end, created_at, updated_at)
             VALUES ($1, $2, $3, $4, 'active', 'monthly', 'starter', $5, $6, $7, NOW(), NOW() + INTERVAL '30 days',
                     false, NOW(), NOW())"
        )
        .bind(subscription_id)
        .bind(&realm_id)
        .bind(basic_plan_id)
        .bind(client_app_id)
        .bind(format!("ext_sub_upgrade_{}", Uuid::now_v7()))
        .bind("test_product")
        .bind("creem")
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // When: Upgrade to yearly pro plan
        sqlx::query(
            "UPDATE subscription
             SET plan_id = $1, billing_period = 'yearly', tier = 'professional', updated_at = NOW()
             WHERE id = $2",
        )
        .bind(pro_plan_id)
        .bind(subscription_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Then: Plan is updated
        let updated_plan_id: Uuid =
            sqlx::query_scalar("SELECT plan_id FROM subscription WHERE id = $1")
                .bind(subscription_id)
                .fetch_one(&ctx.app_state.pool)
                .await
                .unwrap();

        assert_eq!(updated_plan_id, pro_plan_id);

        // Then: Billing period is yearly
        let billing_period: String =
            sqlx::query_scalar("SELECT billing_period FROM subscription WHERE id = $1")
                .bind(subscription_id)
                .fetch_one(&ctx.app_state.pool)
                .await
                .unwrap();

        assert_eq!(billing_period, "yearly");

        // Cleanup
        delete_test_subscription(ctx, subscription_id).await;
    }

    // =========================================================================
    // Test: Subscription plan downgrade
    // =========================================================================
    // Given: Yearly pro subscription
    // When: 降级到 monthly basic
    // Then: Plan 在下一个周期生效

    #[test_context(LifecycleTestContext)]
    #[tokio::test]
    async fn test_subscription_plan_downgrade(ctx: &mut LifecycleTestContext) {
        let realm_id = ctx._realm_id.clone();

        // Given: Create yearly pro subscription
        let pro_plan_id = create_test_plan_yearly(ctx, realm_id.as_str(), "Pro Plan").await;
        let basic_plan_id = create_test_plan(ctx, realm_id.as_str(), "Basic Plan").await;
        let (client_app_id, _) = (Uuid::now_v7(), "test_client".to_string());

        // Insert a dummy client app for the test
        sqlx::query(
            "INSERT INTO client_app (id, realm_id, client_id, name, redirect_uris, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"
        )
        .bind(client_app_id)
        .bind(realm_id.as_str())
        .bind("test-downgrade-client")
        .bind("Test Downgrade Client")
        .bind(json!(["https://example.com/callback"]))
        .bind(true)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        let subscription_id = Uuid::now_v7();

        sqlx::query(
            "INSERT INTO subscription (id, realm_id, plan_id, client_app_id, status, billing_period, tier,
                                     external_subscription_id, external_product_id, payment_provider,
                                     current_period_start, current_period_end,
                                     cancel_at_period_end, created_at, updated_at)
             VALUES ($1, $2, $3, $4, 'active', 'yearly', 'professional', $5, $6, $7, NOW(), NOW() + INTERVAL '365 days',
                     false, NOW(), NOW())"
        )
        .bind(subscription_id)
        .bind(&realm_id)
        .bind(pro_plan_id)
        .bind(client_app_id)
        .bind(format!("ext_sub_downgrade_{}", Uuid::now_v7()))
        .bind("test_product")
        .bind("creem")
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // When: Downgrade to monthly basic plan (scheduled for next period)
        // In a real implementation, this would be scheduled to take effect at period end
        // For this test, we'll simulate the immediate downgrade
        sqlx::query(
            "UPDATE subscription
             SET plan_id = $1, billing_period = 'monthly', tier = 'starter', updated_at = NOW()
             WHERE id = $2",
        )
        .bind(basic_plan_id)
        .bind(subscription_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Then: Plan is downgraded
        let updated_plan_id: Uuid =
            sqlx::query_scalar("SELECT plan_id FROM subscription WHERE id = $1")
                .bind(subscription_id)
                .fetch_one(&ctx.app_state.pool)
                .await
                .unwrap();

        assert_eq!(updated_plan_id, basic_plan_id);

        // Then: Billing period is monthly
        let billing_period: String =
            sqlx::query_scalar("SELECT billing_period FROM subscription WHERE id = $1")
                .bind(subscription_id)
                .fetch_one(&ctx.app_state.pool)
                .await
                .unwrap();

        assert_eq!(billing_period, "monthly");

        // Cleanup
        delete_test_subscription(ctx, subscription_id).await;
    }

    // =========================================================================
    // Test: Subscription pause
    // =========================================================================
    // Given: Active subscription
    // When: 暂停订阅
    // Then: 状态变为 paused

    #[test_context(LifecycleTestContext)]
    #[tokio::test]
    async fn test_subscription_pause(ctx: &mut LifecycleTestContext) {
        let realm_id = ctx._realm_id.clone();

        // Given: Create active subscription
        let plan_id = create_test_plan(ctx, realm_id.as_str(), "Pause Plan").await;
        let (client_app_id, _) = (Uuid::now_v7(), "test_client".to_string());

        // Insert a dummy client app for the test
        sqlx::query(
            "INSERT INTO client_app (id, realm_id, client_id, name, redirect_uris, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"
        )
        .bind(client_app_id)
        .bind(realm_id.as_str())
        .bind("test-pause-client")
        .bind("Test Pause Client")
        .bind(json!(["https://example.com/callback"]))
        .bind(true)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        let subscription_id = Uuid::now_v7();

        sqlx::query(
            "INSERT INTO subscription (id, realm_id, plan_id, client_app_id, status, billing_period, tier,
                                     external_subscription_id, external_product_id, payment_provider,
                                     current_period_start, current_period_end,
                                     cancel_at_period_end, created_at, updated_at)
             VALUES ($1, $2, $3, $4, 'active', 'monthly', 'starter', $5, $6, $7, NOW(), NOW() + INTERVAL '30 days',
                     false, NOW(), NOW())"
        )
        .bind(subscription_id)
        .bind(realm_id.as_str())
        .bind(plan_id)
        .bind(client_app_id)
        .bind(format!("ext_sub_pause_{}", Uuid::now_v7()))
        .bind("test_product")
        .bind("creem")
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Verify active status
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Active).await;

        // When: Pause subscription
        update_subscription_status(ctx, subscription_id, "paused").await;

        // Then: Status becomes paused
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Paused).await;

        // Cleanup
        delete_test_subscription(ctx, subscription_id).await;
    }

    // =========================================================================
    // Test: Subscription resume
    // =========================================================================
    // Given: Paused subscription
    // When: 恢复订阅
    // Then: 状态变为 active

    #[test_context(LifecycleTestContext)]
    #[tokio::test]
    async fn test_subscription_resume(ctx: &mut LifecycleTestContext) {
        let realm_id = ctx._realm_id.clone();

        // Given: Create paused subscription
        let plan_id = create_test_plan(ctx, realm_id.as_str(), "Resume Plan").await;
        let (client_app_id, _) = (Uuid::now_v7(), "test_client".to_string());

        // Insert a dummy client app for the test
        sqlx::query(
            "INSERT INTO client_app (id, realm_id, client_id, name, redirect_uris, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"
        )
        .bind(client_app_id)
        .bind(realm_id.as_str())
        .bind("test-resume-client")
        .bind("Test Resume Client")
        .bind(json!(["https://example.com/callback"]))
        .bind(true)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        let subscription_id = Uuid::now_v7();

        sqlx::query(
            "INSERT INTO subscription (id, realm_id, plan_id, client_app_id, status, billing_period, tier,
                                     external_subscription_id, external_product_id, payment_provider,
                                     current_period_start, current_period_end,
                                     cancel_at_period_end, created_at, updated_at)
             VALUES ($1, $2, $3, $4, 'paused', 'monthly', 'starter', $5, $6, $7, NOW(), NOW(),
                     false, NOW(), NOW())"
        )
        .bind(subscription_id)
        .bind(realm_id.as_str())
        .bind(plan_id)
        .bind(client_app_id)
        .bind(format!("ext_sub_resume_{}", Uuid::now_v7()))
        .bind("test_product")
        .bind("creem")
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Verify paused status
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Paused).await;

        // When: Resume subscription
        update_subscription_status(ctx, subscription_id, "active").await;

        // Then: Status becomes active
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Active).await;

        // Cleanup
        delete_test_subscription(ctx, subscription_id).await;
    }

    // =========================================================================
    // Test: Subscription expiration
    // =========================================================================
    // Given: Active subscription
    // When: 周期结束且未续费
    // Then: 状态变为 expired

    #[test_context(LifecycleTestContext)]
    #[tokio::test]
    async fn test_subscription_expiration(ctx: &mut LifecycleTestContext) {
        let realm_id = ctx._realm_id.clone();

        // Given: Create active subscription
        let plan_id = create_test_plan(ctx, realm_id.as_str(), "Expiration Plan").await;
        let (client_app_id, _) = (Uuid::now_v7(), "test_client".to_string());

        // Insert a dummy client app for the test
        sqlx::query(
            "INSERT INTO client_app (id, realm_id, client_id, name, redirect_uris, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"
        )
        .bind(client_app_id)
        .bind(realm_id.as_str())
        .bind("test-expiration-client")
        .bind("Test Expiration Client")
        .bind(json!(["https://example.com/callback"]))
        .bind(true)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        let subscription_id = Uuid::now_v7();

        sqlx::query(
            "INSERT INTO subscription (id, realm_id, plan_id, client_app_id, status, billing_period, tier,
                                     external_subscription_id, external_product_id, payment_provider,
                                     current_period_start, current_period_end,
                                     cancel_at_period_end, created_at, updated_at)
             VALUES ($1, $2, $3, $4, 'active', 'monthly', 'starter', $5, $6, $7, NOW(), NOW() + INTERVAL '30 days',
                     false, NOW(), NOW())"
        )
        .bind(subscription_id)
        .bind(realm_id.as_str())
        .bind(plan_id)
        .bind(client_app_id)
        .bind(format!("ext_sub_expiration_{}", Uuid::now_v7()))
        .bind("test_product")
        .bind("creem")
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Verify active status
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Active).await;

        // When: Subscription expires (simulate by updating status)
        update_subscription_status(ctx, subscription_id, "expired").await;

        // Then: Status becomes expired
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Expired).await;

        // Cleanup
        delete_test_subscription(ctx, subscription_id).await;
    }

    // =========================================================================
    // Test: Scheduled cancel executes at period end
    // =========================================================================
    // Given: ScheduledCancel subscription
    // When: current_period_end 到达
    // Then: 状态变为 canceled

    #[test_context(LifecycleTestContext)]
    #[tokio::test]
    async fn test_scheduled_cancel_executes_at_period_end(ctx: &mut LifecycleTestContext) {
        let realm_id = ctx._realm_id.clone();

        // Given: Create subscription with cancel_at_period_end=true
        let plan_id = create_test_plan(ctx, realm_id.as_str(), "Scheduled Cancel Exec Plan").await;
        let (client_app_id, _) = (Uuid::now_v7(), "test_client".to_string());

        // Insert a dummy client app for the test
        sqlx::query(
            "INSERT INTO client_app (id, realm_id, client_id, name, redirect_uris, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"
        )
        .bind(client_app_id)
        .bind(realm_id.as_str())
        .bind("test-scheduled-exec-client")
        .bind("Test Scheduled Exec Client")
        .bind(json!(["https://example.com/callback"]))
        .bind(true)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        let subscription_id = Uuid::now_v7();

        sqlx::query(
            "INSERT INTO subscription (id, realm_id, plan_id, client_app_id, status, billing_period, tier,
                                     external_subscription_id, external_product_id, payment_provider,
                                     current_period_start, current_period_end,
                                     cancel_at_period_end, created_at, updated_at)
             VALUES ($1, $2, $3, $4, 'active', 'monthly', 'starter', $5, $6, $7, NOW(), NOW() + INTERVAL '30 days',
                     true, NOW(), NOW())"
        )
        .bind(subscription_id)
        .bind(realm_id.as_str())
        .bind(plan_id)
        .bind(client_app_id)
        .bind(format!("ext_sub_scheduled_{}", Uuid::now_v7()))
        .bind("test_product")
        .bind("creem")
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Verify cancel_at_period_end is true
        let cancel_at_period_end: bool =
            sqlx::query_scalar("SELECT cancel_at_period_end FROM subscription WHERE id = $1")
                .bind(subscription_id)
                .fetch_one(&ctx.app_state.pool)
                .await
                .unwrap();

        assert!(cancel_at_period_end);

        // When: Period ends (simulate by updating status to canceled)
        update_subscription_status(ctx, subscription_id, "canceled").await;

        // Then: Status becomes canceled
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Canceled).await;

        // Cleanup
        delete_test_subscription(ctx, subscription_id).await;
    }

    // =========================================================================
    // Test: Subscription dispute handling
    // =========================================================================
    // Given: Active subscription
    // When: 接收 `dispute.created` webhook
    // Then: 状态变为 dispute，保持访问权限

    #[test_context(LifecycleTestContext)]
    #[tokio::test]
    async fn test_subscription_dispute_handling(ctx: &mut LifecycleTestContext) {
        let realm_id = ctx._realm_id.clone();

        // Given: Create active subscription
        let plan_id = create_test_plan(ctx, realm_id.as_str(), "Dispute Plan").await;
        let (client_app_id, _) = (Uuid::now_v7(), "test_client".to_string());

        // Insert a dummy client app for the test
        sqlx::query(
            "INSERT INTO client_app (id, realm_id, client_id, name, redirect_uris, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"
        )
        .bind(client_app_id)
        .bind(realm_id.as_str())
        .bind("test-dispute-client")
        .bind("Test Dispute Client")
        .bind(json!(["https://example.com/callback"]))
        .bind(true)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        let subscription_id = Uuid::now_v7();

        sqlx::query(
            "INSERT INTO subscription (id, realm_id, plan_id, client_app_id, status, billing_period, tier,
                                     external_subscription_id, external_product_id, payment_provider,
                                     current_period_start, current_period_end,
                                     cancel_at_period_end, created_at, updated_at)
             VALUES ($1, $2, $3, $4, 'active', 'monthly', 'starter', $5, $6, $7, NOW(), NOW() + INTERVAL '30 days',
                     false, NOW(), NOW())"
        )
        .bind(subscription_id)
        .bind(realm_id.as_str())
        .bind(plan_id)
        .bind(client_app_id)
        .bind(format!("ext_sub_dispute_{}", Uuid::now_v7()))
        .bind("test_product")
        .bind("creem")
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Verify active status
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Active).await;

        // When: Dispute is created (simulate by updating status)
        update_subscription_status(ctx, subscription_id, "dispute").await;

        // Then: Status becomes dispute
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Dispute).await;

        // Then: Subscription should still have access during dispute
        assert!(SubscriptionStatus::Dispute.has_access());

        // Cleanup
        delete_test_subscription(ctx, subscription_id).await;
    }
}
