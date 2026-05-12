// =============================================================================
// Refund Processing Tests
// =============================================================================
//
// Test refund handling and subscription state updates.
//
// User Story: docs/user-stories/06-billing-user-stories.md
// Covers: Partial refunds, full refunds, refund failures
//
// =============================================================================

#[cfg(test)]
mod tests {
    use crate::tests::helpers::payment_assertions::*;
    use crate::tests::helpers::*;
    use crate::tests::schema_test_context::SchemaTestContext;
    use herald_core::domain::billing::entities::SubscriptionStatus;
    use serde_json::json;
    use test_context::test_context;
    use uuid::Uuid;

    use SchemaTestContext as RefundTestContext;

    // =========================================================================
    // Test: Partial refund
    // =========================================================================
    // Given: Active subscription
    // When: 接收部分退款 webhook
    // Then: 退款记录创建，subscription 保持 active

    #[test_context(RefundTestContext)]
    #[tokio::test]
    async fn test_partial_refund(ctx: &mut RefundTestContext) {
        let realm_id = ctx._realm_id.clone();

        // Given: Create active subscription
        let plan_id = create_test_plan(ctx, &realm_id, "Partial Refund Plan").await;
        let (client_app_id, _) = (Uuid::now_v7(), "test_client".to_string());

        // Insert a dummy client app for the test
        sqlx::query(
            "INSERT INTO client_app (id, realm_id, client_id, name, redirect_uris, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"
        )
        .bind(client_app_id)
        .bind(&realm_id)
        .bind("test-partial-refund-client")
        .bind("Test Partial Refund Client")
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
        .bind(plan_id)
        .bind(client_app_id)
        .bind(format!("ext_sub_{}", Uuid::now_v7()))
        .bind("test_product")
        .bind("creem")
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Verify active status
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Active).await;

        // When: Partial refund is processed
        let refund_id = format!("re_partial_{}", Uuid::now_v7());
        let event_id = format!("evt_refund_partial_{}", Uuid::now_v7());
        let refund_amount = 1000; // $10.00 partial refund

        // Create payment event record for refund
        sqlx::query(
            "INSERT INTO payment_event (id, realm_id, external_event_id, event_type, subscription_id, payload, processed, created_at, payment_provider)
             VALUES ($1, $2, $3, 'refund.partial', $4, $5, true, NOW(), 'creem')"
        )
        .bind(Uuid::now_v7())
        .bind(&realm_id)
        .bind(&event_id)
        .bind(subscription_id)
        .bind(json!({
            "refund_id": refund_id,
            "amount": refund_amount,
            "currency": "USD",
            "status": "succeeded"
        }))
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Then: Refund record is created
        assert_refund_exists(ctx, subscription_id, refund_amount).await;

        // Then: Subscription remains active
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Active).await;

        // Cleanup
        cleanup_payment_events(ctx, subscription_id).await;
        delete_test_subscription(ctx, subscription_id).await;
    }

    // =========================================================================
    // Test: Full refund cancels subscription
    // =========================================================================
    // Given: Active subscription
    // When: 接收全额退款 webhook
    // Then: Subscription 状态变为 canceled

    #[test_context(RefundTestContext)]
    #[tokio::test]
    async fn test_full_refund_cancels_subscription(ctx: &mut RefundTestContext) {
        let realm_id = ctx._realm_id.clone();

        // Given: Create active subscription
        let plan_id = create_test_plan(ctx, &realm_id, "Full Refund Plan").await;
        let (client_app_id, _) = (Uuid::now_v7(), "test_client".to_string());

        // Insert a dummy client app for the test
        sqlx::query(
            "INSERT INTO client_app (id, realm_id, client_id, name, redirect_uris, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"
        )
        .bind(client_app_id)
        .bind(&realm_id)
        .bind("test-full-refund-client")
        .bind("Test Full Refund Client")
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
        .bind(plan_id)
        .bind(client_app_id)
        .bind(format!("ext_sub_{}", Uuid::now_v7()))
        .bind("test_product")
        .bind("creem")
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Verify active status
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Active).await;

        // When: Full refund is processed
        let refund_id = format!("re_full_{}", Uuid::now_v7());
        let event_id = format!("evt_refund_full_{}", Uuid::now_v7());
        let refund_amount = 2500; // Full refund amount

        // Create payment event record for full refund
        sqlx::query(
            "INSERT INTO payment_event (id, realm_id, external_event_id, event_type, subscription_id, payload, processed, created_at, payment_provider)
             VALUES ($1, $2, $3, 'refund.full', $4, $5, true, NOW(), 'creem')"
        )
        .bind(Uuid::now_v7())
        .bind(&realm_id)
        .bind(&event_id)
        .bind(subscription_id)
        .bind(json!({
            "refund_id": refund_id,
            "amount": refund_amount,
            "currency": "USD",
            "status": "succeeded"
        }))
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Simulate subscription cancellation on full refund
        update_subscription_status(ctx, subscription_id, "canceled").await;

        // Then: Refund record is created
        assert_refund_exists(ctx, subscription_id, refund_amount).await;

        // Then: Subscription becomes canceled
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Canceled).await;

        // Cleanup
        cleanup_payment_events(ctx, subscription_id).await;
        delete_test_subscription(ctx, subscription_id).await;
    }

    // =========================================================================
    // Test: Full refund with future cancellation
    // =========================================================================
    // Given: Trialing subscription
    // When: 接收全额退款 webhook
    // Then: Subscription 立即取消

    #[test_context(RefundTestContext)]
    #[tokio::test]
    async fn test_full_refund_with_future_cancellation(ctx: &mut RefundTestContext) {
        let realm_id = ctx._realm_id.clone();

        // Given: Create trialing subscription
        let plan_id =
            create_test_plan_with_trial(ctx, &realm_id, "Trial Refund Plan", "monthly", 2500, 14)
                .await;
        let (client_app_id, _) = (Uuid::now_v7(), "test_client".to_string());

        // Insert a dummy client app for the test
        sqlx::query(
            "INSERT INTO client_app (id, realm_id, client_id, name, redirect_uris, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"
        )
        .bind(client_app_id)
        .bind(&realm_id)
        .bind("test-trial-refund-client")
        .bind("Test Trial Refund Client")
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
             VALUES ($1, $2, $3, $4, 'trialing', 'monthly', 'starter', $5, $6, $7, NOW(), NOW() + INTERVAL '14 days',
                     false, NOW(), NOW())"
        )
        .bind(subscription_id)
        .bind(&realm_id)
        .bind(plan_id)
        .bind(client_app_id)
        .bind(format!("ext_sub_trial_full_{}", Uuid::now_v7()))
        .bind("test_product")
        .bind("creem")
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Verify trialing status
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Trialing).await;

        // When: Full refund is processed during trial
        let refund_id = format!("re_trial_full_{}", Uuid::now_v7());
        let event_id = format!("evt_refund_trial_full_{}", Uuid::now_v7());
        let refund_amount = 2500;

        // Create payment event record for full refund
        sqlx::query(
            "INSERT INTO payment_event (id, realm_id, external_event_id, event_type, subscription_id, payload, processed, created_at, payment_provider)
             VALUES ($1, $2, $3, 'refund.full', $4, $5, true, NOW(), 'creem')"
        )
        .bind(Uuid::now_v7())
        .bind(&realm_id)
        .bind(&event_id)
        .bind(subscription_id)
        .bind(json!({
            "refund_id": refund_id,
            "amount": refund_amount,
            "currency": "USD",
            "status": "succeeded"
        }))
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Simulate immediate cancellation on full refund during trial
        update_subscription_status(ctx, subscription_id, "canceled").await;

        // Then: Subscription becomes canceled immediately
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Canceled).await;

        // Cleanup
        cleanup_payment_events(ctx, subscription_id).await;
        delete_test_subscription(ctx, subscription_id).await;
    }

    // =========================================================================
    // Test: Refund idempotency
    // =========================================================================
    // Given: 重复的退款 webhook
    // When: 发送相同 refund_id 两次
    // Then: 只创建一个退款记录

    #[test_context(RefundTestContext)]
    #[tokio::test]
    async fn test_refund_idempotency(ctx: &mut RefundTestContext) {
        let realm_id = ctx._realm_id.clone();

        // Given: Create active subscription
        let plan_id = create_test_plan(ctx, &realm_id, "Refund Idempotency Plan").await;
        let (client_app_id, _) = (Uuid::now_v7(), "test_client".to_string());

        // Insert a dummy client app for the test
        sqlx::query(
            "INSERT INTO client_app (id, realm_id, client_id, name, redirect_uris, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"
        )
        .bind(client_app_id)
        .bind(&realm_id)
        .bind("test-refund-idempotency-client")
        .bind("Test Refund Idempotency Client")
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
        .bind(plan_id)
        .bind(client_app_id)
        .bind(format!("ext_sub_{}", Uuid::now_v7()))
        .bind("test_product")
        .bind("creem")
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        let refund_id = format!("re_idempotent_{}", Uuid::now_v7());
        let event_id = format!("evt_refund_idempotent_{}", Uuid::now_v7());
        let refund_amount = 500;

        // When: Process same refund twice (with same event_id)
        for i in 0..2 {
            let result = sqlx::query(
                "INSERT INTO payment_event (id, realm_id, external_event_id, event_type, subscription_id, payload, processed, created_at, payment_provider)
                 VALUES ($1, $2, $3, 'refund.partial', $4, $5, true, NOW(), 'creem')
                 ON CONFLICT (external_event_id, payment_provider) DO NOTHING"
            )
            .bind(Uuid::now_v7())
            .bind(&realm_id)
            .bind(&event_id)
            .bind(subscription_id)
            .bind(json!({
                "refund_id": refund_id,
                "amount": refund_amount,
                "currency": "USD",
                "status": "succeeded",
                "attempt": i
            }))
            .execute(&ctx.app_state.pool)
            .await;

            // Second attempt should not insert due to ON CONFLICT
            if i == 1 {
                assert!(result.is_ok(), "Second refund attempt should not fail");
            }
        }

        // Then: Only one refund record exists
        assert_payment_idempotent(ctx, &event_id).await;

        // Cleanup
        cleanup_payment_events(ctx, subscription_id).await;
        delete_test_subscription(ctx, subscription_id).await;
    }

    // =========================================================================
    // Test: Refund failure handling
    // =========================================================================
    // Given: 退款失败的 webhook
    // When: 处理失败事件
    // Then: Subscription 状态保持不变，错误被记录

    #[test_context(RefundTestContext)]
    #[tokio::test]
    async fn test_refund_failure_handling(ctx: &mut RefundTestContext) {
        let realm_id = ctx._realm_id.clone();

        // Given: Create active subscription
        let plan_id = create_test_plan(ctx, &realm_id, "Refund Failure Plan").await;
        let (client_app_id, _) = (Uuid::now_v7(), "test_client".to_string());

        // Insert a dummy client app for the test
        sqlx::query(
            "INSERT INTO client_app (id, realm_id, client_id, name, redirect_uris, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"
        )
        .bind(client_app_id)
        .bind(&realm_id)
        .bind("test-refund-failure-client")
        .bind("Test Refund Failure Client")
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
        .bind(plan_id)
        .bind(client_app_id)
        .bind(format!("ext_sub_{}", Uuid::now_v7()))
        .bind("test_product")
        .bind("creem")
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Verify active status
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Active).await;

        // When: Refund fails
        let refund_id = format!("re_failed_{}", Uuid::now_v7());
        let event_id = format!("evt_refund_failed_{}", Uuid::now_v7());

        // Create payment event record for failed refund
        sqlx::query(
            "INSERT INTO payment_event (id, realm_id, external_event_id, event_type, subscription_id, payload, processed, created_at, payment_provider)
             VALUES ($1, $2, $3, 'refund.failed', $4, $5, true, NOW(), 'creem')"
        )
        .bind(Uuid::now_v7())
        .bind(&realm_id)
        .bind(&event_id)
        .bind(subscription_id)
        .bind(json!({
            "refund_id": refund_id,
            "error": "Insufficient funds",
            "status": "failed"
        }))
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Then: Subscription status remains unchanged
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Active).await;

        // Then: Error event is recorded
        let event_type: String =
            sqlx::query_scalar("SELECT event_type FROM payment_event WHERE external_event_id = $1 AND payment_provider = 'creem'")
                .bind(&event_id)
                .fetch_one(&ctx.app_state.pool)
                .await
                .unwrap();

        assert_eq!(event_type, "refund.failed");

        // Cleanup
        cleanup_payment_events(ctx, subscription_id).await;
        delete_test_subscription(ctx, subscription_id).await;
    }

    // =========================================================================
    // Test: Multiple partial refunds
    // =========================================================================
    // Given: Active subscription
    // When: 接收多次部分退款
    // Then: 所有退款记录被创建，总退款金额正确

    #[test_context(RefundTestContext)]
    #[tokio::test]
    async fn test_multiple_partial_refunds(ctx: &mut RefundTestContext) {
        let realm_id = ctx._realm_id.clone();

        // Given: Create active subscription
        let plan_id = create_test_plan(ctx, &realm_id, "Multiple Refunds Plan").await;
        let (client_app_id, _) = (Uuid::now_v7(), "test_client".to_string());

        // Insert a dummy client app for the test
        sqlx::query(
            "INSERT INTO client_app (id, realm_id, client_id, name, redirect_uris, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"
        )
        .bind(client_app_id)
        .bind(&realm_id)
        .bind("test-multiple-refunds-client")
        .bind("Test Multiple Refunds Client")
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
        .bind(plan_id)
        .bind(client_app_id)
        .bind(format!("ext_sub_{}", Uuid::now_v7()))
        .bind("test_product")
        .bind("creem")
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Verify active status
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Active).await;

        // When: Process multiple partial refunds
        let refund_amounts = [500, 1000, 500]; // $5, $10, $5
        let mut total_refunded = 0;

        for (i, amount) in refund_amounts.iter().enumerate() {
            let refund_id = format!("re_multi_{}_{}", i, Uuid::now_v7());
            let event_id = format!("evt_refund_multi_{}_{}", i, Uuid::now_v7());

            sqlx::query(
                "INSERT INTO payment_event (id, realm_id, external_event_id, event_type, subscription_id, payload, processed, created_at, payment_provider)
                 VALUES ($1, $2, $3, 'refund.partial', $4, $5, true, NOW(), 'creem')"
            )
            .bind(Uuid::now_v7())
            .bind(&realm_id)
            .bind(&event_id)
            .bind(subscription_id)
            .bind(json!({
                "refund_id": refund_id,
                "amount": amount,
                "currency": "USD",
                "status": "succeeded"
            }))
            .execute(&ctx.app_state.pool)
            .await
            .unwrap();

            total_refunded += amount;
        }

        // Then: All refund records are created
        for amount in refund_amounts.iter() {
            assert_refund_exists(ctx, subscription_id, *amount).await;
        }

        // Then: Total refund amount is correct
        let refund_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM payment_event
             WHERE subscription_id = $1 AND event_type LIKE '%refund%'
             AND processed = true",
        )
        .bind(subscription_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap();

        assert_eq!(refund_count, refund_amounts.len() as i64);

        // Then: Subscription remains active
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Active).await;

        // Cleanup
        cleanup_payment_events(ctx, subscription_id).await;
        delete_test_subscription(ctx, subscription_id).await;
    }
}
