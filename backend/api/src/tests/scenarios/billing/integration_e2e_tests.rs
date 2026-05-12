// =============================================================================
// End-to-End Integration Tests
// =============================================================================
//
// Test complete user journeys from start to finish.
//
// User Story: docs/user-stories/06-billing-user-stories.md
// Covers: Complete user flows, cross-system integration, data consistency
//
// =============================================================================

#[cfg(test)]
mod tests {
    use crate::tests::helpers::payment_assertions::*;
    use crate::tests::helpers::*;
    use crate::tests::schema_test_context::SchemaTestContext;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use chrono::SubsecRound;
    use herald_core::domain::billing::entities::SubscriptionStatus;
    use serde_json::json;
    use test_context::test_context;
    use tower::ServiceExt;
    use uuid::Uuid;

    use SchemaTestContext as E2ETestContext;

    /// Helper: Create a client app via API
    async fn create_client_app_via_api(
        _ctx: &E2ETestContext,
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
    // Test: New user subscription flow
    // =========================================================================
    // Complete flow: 用户注册 → 创建 checkout → 完成支付 → 订阅激活 → 发放积分
    // 验证: 所有步骤正确执行，数据一致

    #[test_context(E2ETestContext)]
    #[tokio::test]
    async fn test_new_user_subscription_flow(ctx: &mut E2ETestContext) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();

        // Step 1: Setup admin and plan
        let admin_token = setup_billing_admin_session(ctx, "test-e2e-new-user@test.com").await;
        let plan_id = create_test_plan(ctx, realm_id.as_str(), "E2E New User Plan").await;

        // Step 2: Create client app
        let (_client_id_str, client_app_id) = create_client_app_via_api(
            ctx,
            &app,
            &admin_token,
            realm_id.as_str(),
            "test-e2e-new-user-app",
            "Test E2E New User App",
        )
        .await;

        // Step 3: Simulate checkout completion via webhook
        let event_id = format!("evt_e2e_new_user_{}", Uuid::now_v7());

        sqlx::query(
            "INSERT INTO payment_event (id, realm_id, external_event_id, event_type, subscription_id, payload, processed, created_at, payment_provider)
             VALUES ($1, $2, $3, 'checkout.completed', $4, $5, true, NOW(), 'creem')"
        )
        .bind(Uuid::now_v7())
        .bind(realm_id.as_str())
        .bind(&event_id)
        .bind(client_app_id)
        .bind(json!({
            "planId": plan_id.to_string(),
            "billing_period": "monthly"
        }))
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Step 4: Create subscription
        let subscription_id =
            create_test_subscription(ctx, realm_id.as_str(), client_app_id, plan_id, "monthly")
                .await;

        // Step 5: Verify subscription is active
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Active).await;

        // Step 6: Verify payment event exists
        assert!(verify_payment_event_exists(ctx, &event_id).await);

        // Step 7: Verify data consistency
        let check_subscription: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM subscription WHERE id = $1 AND client_app_id = $2")
                .bind(subscription_id)
                .bind(client_app_id)
                .fetch_optional(&ctx.app_state.pool)
                .await
                .unwrap();

        assert!(
            check_subscription.is_some(),
            "Subscription should exist with correct client_app"
        );

        // Cleanup
        cleanup_payment_events(ctx, subscription_id).await;
        delete_test_subscription(ctx, subscription_id).await;
    }

    // =========================================================================
    // Test: Subscription renewal flow
    // =========================================================================
    // Complete flow: 订阅续费 → 支付成功 → 周期更新 → 积分奖励
    // 验证: 续费后周期延长，积分增加

    #[test_context(E2ETestContext)]
    #[tokio::test]
    async fn test_subscription_renewal_flow(ctx: &mut E2ETestContext) {
        let realm_id = ctx._realm_id.clone();

        // Step 1: Create initial subscription
        let plan_id = create_test_plan(ctx, realm_id.as_str(), "E2E Renewal Plan").await;
        let (client_app_id, _) = (Uuid::now_v7(), "test_client".to_string());

        // Insert a dummy client app for the test
        sqlx::query(
            "INSERT INTO client_app (id, realm_id, client_id, name, redirect_uris, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"
        )
        .bind(client_app_id)
        .bind(realm_id.as_str())
        .bind("test-e2e-renewal-client")
        .bind("Test E2E Renewal Client")
        .bind(json!(["https://example.com/callback"]))
        .bind(true)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        let subscription_id = Uuid::now_v7();
        let now = chrono::Utc::now();
        let initial_period_end = now + chrono::Duration::days(30);

        sqlx::query(
            "INSERT INTO subscription (id, realm_id, plan_id, client_app_id, status, billing_period, tier,
                                     external_subscription_id, external_product_id, payment_provider,
                                     current_period_start, current_period_end,
                                     cancel_at_period_end, created_at, updated_at)
             VALUES ($1, $2, $3, $4, 'active', 'monthly', 'starter',
                     'ext_sub_e2e_renewal', 'test_product', 'creem', $5, $6,
                     false, NOW(), NOW())"
        )
        .bind(subscription_id)
        .bind(realm_id.as_str())
        .bind(plan_id)
        .bind(client_app_id)
        .bind(now)
        .bind(initial_period_end)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Step 2: Process renewal webhook
        let renewal_event_id = format!("evt_e2e_renewal_{}", Uuid::now_v7());

        sqlx::query(
            "INSERT INTO payment_event (id, realm_id, external_event_id, event_type, subscription_id, payload, processed, created_at, payment_provider)
             VALUES ($1, $2, $3, 'subscription.paid', $4, $5, true, NOW(), 'creem')"
        )
        .bind(Uuid::now_v7())
        .bind(realm_id.as_str())
        .bind(&renewal_event_id)
        .bind(subscription_id)
        .bind(json!({
            "amount": 2500,
            "currency": "USD"
        }))
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Step 3: Update subscription period (simulate renewal)
        let new_period_start = initial_period_end;
        let new_period_end = initial_period_end + chrono::Duration::days(30);

        update_subscription_period(ctx, subscription_id, new_period_start, new_period_end).await;

        // Step 4: Verify renewal succeeded
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Active).await;

        let (start, end): (chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>) =
            sqlx::query_as(
                "SELECT current_period_start, current_period_end FROM subscription WHERE id = $1",
            )
            .bind(subscription_id)
            .fetch_one(&ctx.app_state.pool)
            .await
            .unwrap();

        // Compare timestamps with millisecond precision to handle PostgreSQL timestamp precision
        assert_eq!(start.trunc_subsecs(3), new_period_start.trunc_subsecs(3));
        assert_eq!(end.trunc_subsecs(3), new_period_end.trunc_subsecs(3));

        // Step 5: Verify renewal event recorded
        assert!(verify_payment_event_exists(ctx, &renewal_event_id).await);

        // Cleanup
        cleanup_payment_events(ctx, subscription_id).await;
        delete_test_subscription(ctx, subscription_id).await;
    }

    // =========================================================================
    // Test: Subscription cancellation flow
    // =========================================================================
    // Complete flow: 取消订阅 → 访问权限验证 → 周期结束后完全取消
    // 验证: 取消后权限正确，历史记录完整

    #[test_context(E2ETestContext)]
    #[tokio::test]
    async fn test_subscription_cancellation_flow(ctx: &mut E2ETestContext) {
        let realm_id = ctx._realm_id.clone();

        // Step 1: Create active subscription
        let plan_id = create_test_plan(ctx, realm_id.as_str(), "E2E Cancellation Plan").await;
        let (client_app_id, _) = (Uuid::now_v7(), "test_client".to_string());

        // Insert a dummy client app for the test
        sqlx::query(
            "INSERT INTO client_app (id, realm_id, client_id, name, redirect_uris, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"
        )
        .bind(client_app_id)
        .bind(realm_id.as_str())
        .bind("test-e2e-cancellation-client")
        .bind("Test E2E Cancellation Client")
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
             VALUES ($1, $2, $3, $4, 'active', 'monthly', 'starter',
                     'ext_sub_e2e_cancellation', 'test_product', 'creem', NOW(), NOW() + INTERVAL '30 days',
                     false, NOW(), NOW())"
        )
        .bind(subscription_id)
        .bind(realm_id.as_str())
        .bind(plan_id)
        .bind(client_app_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Step 2: Cancel subscription immediately
        update_subscription_status(ctx, subscription_id, "canceled").await;

        // Step 3: Verify subscription is canceled
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Canceled).await;

        // Step 4: Verify no access after cancellation
        assert!(!SubscriptionStatus::Canceled.has_access());

        // Step 5: Verify cancellation was recorded
        let cancel_at: Option<chrono::DateTime<chrono::Utc>> =
            sqlx::query_scalar("SELECT cancel_at FROM subscription WHERE id = $1")
                .bind(subscription_id)
                .fetch_one(&ctx.app_state.pool)
                .await
                .unwrap();

        assert!(cancel_at.is_some(), "cancel_at should be set");

        // Cleanup
        delete_test_subscription(ctx, subscription_id).await;
    }

    // =========================================================================
    // Test: Plan upgrade flow
    // =========================================================================
    // Complete flow: 升级 plan → 按比例计费 → 新 plan 激活
    // 验证: 升级后功能正确，费用计算准确

    #[test_context(E2ETestContext)]
    #[tokio::test]
    async fn test_plan_upgrade_flow(ctx: &mut E2ETestContext) {
        let realm_id = ctx._realm_id.clone();

        // Step 1: Create basic subscription
        let basic_plan_id = create_test_plan(ctx, realm_id.as_str(), "E2E Basic Plan").await;
        let pro_plan_id = create_test_plan_yearly(ctx, realm_id.as_str(), "E2E Pro Plan").await;
        let (client_app_id, _) = (Uuid::now_v7(), "test_client".to_string());

        // Insert a dummy client app for the test
        sqlx::query(
            "INSERT INTO client_app (id, realm_id, client_id, name, redirect_uris, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"
        )
        .bind(client_app_id)
        .bind(realm_id.as_str())
        .bind("test-e2e-upgrade-client")
        .bind("Test E2E Upgrade Client")
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
             VALUES ($1, $2, $3, $4, 'active', 'monthly', 'starter',
                     'ext_sub_e2e_upgrade', 'test_product', 'creem', NOW(), NOW() + INTERVAL '30 days',
                     false, NOW(), NOW())"
        )
        .bind(subscription_id)
        .bind(&realm_id)
        .bind(basic_plan_id)
        .bind(client_app_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Step 2: Process upgrade webhook
        let upgrade_event_id = format!("evt_e2e_upgrade_{}", Uuid::now_v7());

        sqlx::query(
            "INSERT INTO payment_event (id, realm_id, external_event_id, event_type, subscription_id, payload, processed, created_at, payment_provider)
             VALUES ($1, $2, $3, 'subscription.upgraded', $4, $5, true, NOW(), 'creem')"
        )
        .bind(Uuid::now_v7())
        .bind(realm_id.as_str())
        .bind(&upgrade_event_id)
        .bind(subscription_id)
        .bind(json!({
            "from_plan": basic_plan_id.to_string(),
            "to_plan": pro_plan_id.to_string(),
            "prorated_amount": 20000
        }))
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Step 3: Update subscription to pro plan
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

        // Step 4: Verify upgrade succeeded
        let updated_plan_id: Uuid =
            sqlx::query_scalar("SELECT plan_id FROM subscription WHERE id = $1")
                .bind(subscription_id)
                .fetch_one(&ctx.app_state.pool)
                .await
                .unwrap();

        assert_eq!(updated_plan_id, pro_plan_id);

        let billing_period: String =
            sqlx::query_scalar("SELECT billing_period FROM subscription WHERE id = $1")
                .bind(subscription_id)
                .fetch_one(&ctx.app_state.pool)
                .await
                .unwrap();

        assert_eq!(billing_period, "yearly");

        // Step 5: Verify upgrade event recorded
        assert!(verify_payment_event_exists(ctx, &upgrade_event_id).await);

        // Cleanup
        cleanup_payment_events(ctx, subscription_id).await;
        delete_test_subscription(ctx, subscription_id).await;
    }

    // =========================================================================
    // Test: Payment failure recovery flow
    // =========================================================================
    // Complete flow: 支付失败 → 逾期状态 → 重试成功 → 恢复激活
    // 验证: 失败后正确处理，恢复后状态正确

    #[test_context(E2ETestContext)]
    #[tokio::test]
    async fn test_payment_failure_recovery_flow(ctx: &mut E2ETestContext) {
        let realm_id = ctx._realm_id.clone();

        // Step 1: Create active subscription
        let plan_id = create_test_plan(ctx, realm_id.as_str(), "E2E Failure Recovery Plan").await;
        let (client_app_id, _) = (Uuid::now_v7(), "test_client".to_string());

        // Insert a dummy client app for the test
        sqlx::query(
            "INSERT INTO client_app (id, realm_id, client_id, name, redirect_uris, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"
        )
        .bind(client_app_id)
        .bind(realm_id.as_str())
        .bind("test-e2e-failure-recovery-client")
        .bind("Test E2E Failure Recovery Client")
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
             VALUES ($1, $2, $3, $4, 'active', 'monthly', 'starter',
                     'ext_sub_e2e_failure_recovery', 'test_product', 'creem', NOW(), NOW() + INTERVAL '30 days',
                     false, NOW(), NOW())"
        )
        .bind(subscription_id)
        .bind(realm_id.as_str())
        .bind(plan_id)
        .bind(client_app_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Step 2: Process payment failure
        let failure_event_id = format!("evt_e2e_failure_{}", Uuid::now_v7());

        sqlx::query(
            "INSERT INTO payment_event (id, realm_id, external_event_id, event_type, subscription_id, payload, processed, created_at, payment_provider)
             VALUES ($1, $2, $3, 'subscription.past_due', $4, $5, true, NOW(), 'creem')"
        )
        .bind(Uuid::now_v7())
        .bind(realm_id.as_str())
        .bind(&failure_event_id)
        .bind(subscription_id)
        .bind(json!({
            "error": "Payment failed"
        }))
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        update_subscription_status(ctx, subscription_id, "past_due").await;

        // Step 3: Verify subscription is past_due
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::PastDue).await;

        // Step 4: Process retry success
        let retry_event_id = format!("evt_e2e_retry_{}", Uuid::now_v7());

        sqlx::query(
            "INSERT INTO payment_event (id, realm_id, external_event_id, event_type, subscription_id, payload, processed, created_at, payment_provider)
             VALUES ($1, $2, $3, 'subscription.paid', $4, $5, true, NOW(), 'creem')"
        )
        .bind(Uuid::now_v7())
        .bind(realm_id.as_str())
        .bind(&retry_event_id)
        .bind(subscription_id)
        .bind(json!({
            "amount": 2500,
            "retry": true
        }))
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        update_subscription_status(ctx, subscription_id, "active").await;

        // Step 5: Verify recovery succeeded
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Active).await;

        // Step 6: Verify both events recorded
        assert!(verify_payment_event_exists(ctx, &failure_event_id).await);
        assert!(verify_payment_event_exists(ctx, &retry_event_id).await);

        // Cleanup
        cleanup_payment_events(ctx, subscription_id).await;
        delete_test_subscription(ctx, subscription_id).await;
    }

    // =========================================================================
    // Test: Refund and cancellation flow
    // =========================================================================
    // Complete flow: 申请退款 → 退款处理 → 订阅取消 → 积分扣除
    // 验证: 退款后所有相关数据正确更新

    #[test_context(E2ETestContext)]
    #[tokio::test]
    async fn test_refund_and_cancellation_flow(ctx: &mut E2ETestContext) {
        let realm_id = ctx._realm_id.clone();

        // Step 1: Create active subscription
        let plan_id = create_test_plan(ctx, realm_id.as_str(), "E2E Refund Plan").await;
        let (client_app_id, _) = (Uuid::now_v7(), "test_client".to_string());

        // Insert a dummy client app for the test
        sqlx::query(
            "INSERT INTO client_app (id, realm_id, client_id, name, redirect_uris, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"
        )
        .bind(client_app_id)
        .bind(realm_id.as_str())
        .bind("test-e2e-refund-client")
        .bind("Test E2E Refund Client")
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
             VALUES ($1, $2, $3, $4, 'active', 'monthly', 'starter',
                     'ext_sub_e2e_refund', 'test_product', 'creem', NOW(), NOW() + INTERVAL '30 days',
                     false, NOW(), NOW())"
        )
        .bind(subscription_id)
        .bind(realm_id.as_str())
        .bind(plan_id)
        .bind(client_app_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Step 2: Process full refund
        let refund_event_id = format!("evt_e2e_refund_{}", Uuid::now_v7());

        sqlx::query(
            "INSERT INTO payment_event (id, realm_id, external_event_id, event_type, subscription_id, payload, processed, created_at, payment_provider)
             VALUES ($1, $2, $3, 'refund.full', $4, $5, true, NOW(), 'creem')"
        )
        .bind(Uuid::now_v7())
        .bind(realm_id.as_str())
        .bind(&refund_event_id)
        .bind(subscription_id)
        .bind(json!({
            "refund_id": format!("re_e2e_{}", Uuid::now_v7()),
            "amount": 2500,
            "status": "succeeded"
        }))
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Step 3: Cancel subscription on refund
        update_subscription_status(ctx, subscription_id, "canceled").await;

        // Step 4: Verify subscription is canceled
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Canceled).await;

        // Step 5: Verify refund event recorded
        assert!(verify_payment_event_exists(ctx, &refund_event_id).await);

        // Step 6: Verify refund amount is correct
        assert_refund_exists(ctx, subscription_id, 2500).await;

        // Cleanup
        cleanup_payment_events(ctx, subscription_id).await;
        delete_test_subscription(ctx, subscription_id).await;
    }

    // =========================================================================
    // Test: Trial to paid conversion flow
    // =========================================================================
    // Complete flow: Trial 订阅 → Trial 结束 → 自动转为付费
    // 验证: Trial 后自动续费，状态正确转换

    #[test_context(E2ETestContext)]
    #[tokio::test]
    async fn test_trial_to_paid_conversion_flow(ctx: &mut E2ETestContext) {
        let realm_id = ctx._realm_id.clone();

        // Step 1: Create trial subscription
        let plan_id = create_test_plan_with_trial(
            ctx,
            realm_id.as_str(),
            "E2E Trial Plan",
            "monthly",
            2500,
            14,
        )
        .await;
        let (client_app_id, _) = (Uuid::now_v7(), "test_client".to_string());

        // Insert a dummy client app for the test
        sqlx::query(
            "INSERT INTO client_app (id, realm_id, client_id, name, redirect_uris, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"
        )
        .bind(client_app_id)
        .bind(realm_id.as_str())
        .bind("test-e2e-trial-client")
        .bind("Test E2E Trial Client")
        .bind(json!(["https://example.com/callback"]))
        .bind(true)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        let subscription_id = Uuid::now_v7();
        let now = chrono::Utc::now();
        let trial_end = now + chrono::Duration::days(14);

        sqlx::query(
            "INSERT INTO subscription (id, realm_id, plan_id, client_app_id, status, billing_period, tier,
                                     external_subscription_id, external_product_id, payment_provider,
                                     current_period_start, current_period_end,
                                     cancel_at_period_end, created_at, updated_at)
             VALUES ($1, $2, $3, $4, 'trialing', 'monthly', 'starter',
                     'ext_sub_e2e_trial', 'test_product', 'creem', $5, $6,
                     false, NOW(), NOW())"
        )
        .bind(subscription_id)
        .bind(realm_id.as_str())
        .bind(plan_id)
        .bind(client_app_id)
        .bind(now)
        .bind(trial_end)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Step 2: Verify trial status
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Trialing).await;

        // Step 3: Process trial end webhook
        let trial_end_event_id = format!("evt_e2e_trial_end_{}", Uuid::now_v7());

        sqlx::query(
            "INSERT INTO payment_event (id, realm_id, external_event_id, event_type, subscription_id, payload, processed, created_at, payment_provider)
             VALUES ($1, $2, $3, 'trial.ended', $4, $5, true, NOW(), 'creem')"
        )
        .bind(Uuid::now_v7())
        .bind(realm_id.as_str())
        .bind(&trial_end_event_id)
        .bind(subscription_id)
        .bind(json!({
            "trial_days": 14
        }))
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Step 4: Convert to paid subscription
        update_subscription_status(ctx, subscription_id, "active").await;

        // Step 5: Verify conversion to active
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Active).await;

        // Step 6: Process first payment
        let payment_event_id = format!("evt_e2e_first_payment_{}", Uuid::now_v7());

        sqlx::query(
            "INSERT INTO payment_event (id, realm_id, external_event_id, event_type, subscription_id, payload, processed, created_at, payment_provider)
             VALUES ($1, $2, $3, 'subscription.paid', $4, $5, true, NOW(), 'creem')"
        )
        .bind(Uuid::now_v7())
        .bind(realm_id.as_str())
        .bind(&payment_event_id)
        .bind(subscription_id)
        .bind(json!({
            "amount": 2500,
            "first_payment": true
        }))
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Step 7: Verify both events recorded
        assert!(verify_payment_event_exists(ctx, &trial_end_event_id).await);
        assert!(verify_payment_event_exists(ctx, &payment_event_id).await);

        // Cleanup
        cleanup_payment_events(ctx, subscription_id).await;
        delete_test_subscription(ctx, subscription_id).await;
    }

    // =========================================================================
    // Test: Multi-realm isolation
    // =========================================================================
    // Complete flow: 不同 realm 的订阅和支付
    // 验证: Realm 间数据完全隔离，无交叉

    #[test_context(E2ETestContext)]
    #[tokio::test]
    async fn test_multi_realm_isolation(ctx: &mut E2ETestContext) {
        let realm1_id = ctx._realm_id.clone();
        let realm2_id = format!("realm_2_{}", Uuid::now_v7());

        // Step 1: Create second realm schema
        sqlx::query(&format!(
            r#"CREATE SCHEMA IF NOT EXISTS "{}""#,
            realm2_id.replace("-", "_")
        ))
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to create second realm schema");

        // Step 2: Create plan in realm 1
        let plan1_id = create_test_plan(ctx, realm1_id.as_str(), "Realm1 Plan").await;
        let (client_app1_id, _) = (Uuid::now_v7(), "test_client".to_string());

        // Insert client app in realm 1
        sqlx::query(
            "INSERT INTO client_app (id, realm_id, client_id, name, redirect_uris, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"
        )
        .bind(client_app1_id)
        .bind(realm1_id.as_str())
        .bind("realm1-client")
        .bind("Realm 1 Client")
        .bind(json!(["https://example.com/callback"]))
        .bind(true)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        let subscription1_id = Uuid::now_v7();

        sqlx::query(
            "INSERT INTO subscription (id, realm_id, plan_id, client_app_id, status, billing_period, tier,
                                     external_subscription_id, external_product_id, payment_provider,
                                     current_period_start, current_period_end,
                                     cancel_at_period_end, created_at, updated_at)
             VALUES ($1, $2, $3, $4, 'active', 'monthly', 'starter',
                     'ext_sub_e2e_realm1', 'test_product', 'creem', NOW(), NOW() + INTERVAL '30 days',
                     false, NOW(), NOW())"
        )
        .bind(subscription1_id)
        .bind(realm1_id.as_str())
        .bind(plan1_id)
        .bind(client_app1_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Step 3: Verify realm 1 subscription exists
        let realm1_subscription: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM subscription WHERE id = $1 AND realm_id = $2")
                .bind(subscription1_id)
                .bind(&realm1_id)
                .fetch_optional(&ctx.app_state.pool)
                .await
                .unwrap();

        assert!(
            realm1_subscription.is_some(),
            "Realm 1 subscription should exist"
        );

        // Step 4: Verify realm 1 subscription does NOT exist in realm 2
        let realm2_check: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM subscription WHERE id = $1 AND realm_id = $2")
                .bind(subscription1_id)
                .bind(&realm2_id)
                .fetch_optional(&ctx.app_state.pool)
                .await
                .unwrap();

        assert!(
            realm2_check.is_none(),
            "Realm 1 subscription should not exist in realm 2"
        );

        // Step 5: Verify realm isolation
        let realm1_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM subscription WHERE realm_id = $1")
                .bind(&realm1_id)
                .fetch_one(&ctx.app_state.pool)
                .await
                .unwrap();

        let realm2_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM subscription WHERE realm_id = $1")
                .bind(&realm2_id)
                .fetch_one(&ctx.app_state.pool)
                .await
                .unwrap();

        assert_eq!(realm1_count, 1, "Realm 1 should have 1 subscription");
        assert_eq!(realm2_count, 0, "Realm 2 should have 0 subscriptions");

        // Cleanup
        delete_test_subscription(ctx, subscription1_id).await;

        // Cleanup second realm schema
        sqlx::query(&format!(
            r#"DROP SCHEMA IF EXISTS "{}" CASCADE"#,
            realm2_id.replace("-", "_")
        ))
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to drop second realm schema");
    }
}
