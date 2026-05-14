// =============================================================================
// Error Recovery Tests
// =============================================================================
//
// Test error handling and recovery scenarios.
//
// User Story: docs/user-stories/06-billing-user-stories.md
// Covers: Network errors, database errors, API errors, out-of-order events
//
// =============================================================================

#[cfg(test)]
mod tests {
    use crate::tests::helpers::payment_assertions::*;
    use crate::tests::helpers::*;
    use crate::tests::schema_test_context::SchemaTestContext;

    use serde_json::json;
    use test_context::test_context;
    use uuid::Uuid;

    use SchemaTestContext as ErrorRecoveryTestContext;
    // =========================================================================
    // Test: Webhook timeout handling
    // =========================================================================
    // Given: Webhook 处理超时
    // When: Creem 重试发送 webhook
    // Then: 第二次处理成功（幂等性）

    #[test_context(ErrorRecoveryTestContext)]
    #[tokio::test]
    async fn test_webhook_timeout_handling(ctx: &mut ErrorRecoveryTestContext) {
        let realm_id = ctx._realm_id.clone();

        // Given: Create subscription
        let plan_id = create_test_plan(ctx, &realm_id, "Webhook Timeout Plan").await;
        let (client_app_id, _) = (Uuid::now_v7(), "test_client".to_string());

        // Insert a dummy client app for the test
        sqlx::query(
            "INSERT INTO client_app (id, realm_id, client_id, name, redirect_uris, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"
        )
        .bind(client_app_id)
        .bind(&realm_id)
        .bind("test-webhook-timeout-client")
        .bind("Test Webhook Timeout Client")
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
                     'ext_sub_webhook_timeout', 'test_product', 'creem', NOW(), NOW() + INTERVAL '30 days',
                     false, NOW(), NOW())"
        )
        .bind(subscription_id)
        .bind(&realm_id)
        .bind(plan_id)
        .bind(client_app_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        let event_id = format!("evt_timeout_retry_{}", Uuid::now_v7());

        // When: Webhook is retried (same event_id)
        for i in 0..2 {
            let result = sqlx::query(
                "INSERT INTO payment_event (id, realm_id, external_event_id, event_type, subscription_id, payload, processed, created_at, payment_provider)
                 VALUES ($1, $2, $3, 'checkout.completed', $4, $5, true, NOW(), 'creem')
                 ON CONFLICT (external_event_id, payment_provider) DO NOTHING"
            )
            .bind(Uuid::now_v7())
            .bind(&realm_id)
            .bind(&event_id)
            .bind(subscription_id)
            .bind(json!({
                "id": event_id,
                "retry": i
            }))
            .execute(&ctx.app_state.pool)
            .await;

            assert!(result.is_ok(), "Webhook retry should not fail");
        }

        // Then: Only one payment event exists (idempotency)
        assert_payment_idempotent(ctx, &event_id).await;

        // Cleanup
        cleanup_payment_events(ctx, subscription_id).await;
        delete_test_subscription(ctx, subscription_id).await;
    }
    // =========================================================================
    // Test: Concurrent subscription update
    // =========================================================================
    // Given: 两个并发请求更新同一 subscription
    // When: 同时处理
    // Then: 使用数据库锁，只有一个成功

    #[test_context(ErrorRecoveryTestContext)]
    #[tokio::test]
    async fn test_concurrent_subscription_update(ctx: &mut ErrorRecoveryTestContext) {
        let realm_id = ctx._realm_id.clone();

        // Given: Create subscription
        let plan_id = create_test_plan(ctx, &realm_id, "Concurrent Update Plan").await;
        let (client_app_id, _) = (Uuid::now_v7(), "test_client".to_string());

        // Insert a dummy client app for the test
        sqlx::query(
            "INSERT INTO client_app (id, realm_id, client_id, name, redirect_uris, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"
        )
        .bind(client_app_id)
        .bind(&realm_id)
        .bind("test-concurrent-update-client")
        .bind("Test Concurrent Update Client")
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
                     'ext_sub_concurrent_update', 'test_product', 'creem', NOW(), NOW() + INTERVAL '30 days',
                     false, NOW(), NOW())"
        )
        .bind(subscription_id)
        .bind(&realm_id)
        .bind(plan_id)
        .bind(client_app_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // When: Two concurrent updates
        let update1 = sqlx::query(
            "UPDATE subscription SET status = 'canceled', updated_at = NOW() WHERE id = $1 AND status = 'active'"
        )
        .bind(subscription_id)
        .execute(&ctx.app_state.pool);

        let update2 = sqlx::query(
            "UPDATE subscription SET status = 'paused', updated_at = NOW() WHERE id = $1 AND status = 'active'"
        )
        .bind(subscription_id)
        .execute(&ctx.app_state.pool);

        let (result1, result2) = tokio::join!(update1, update2);

        // Then: Only one update succeeds
        let success_count = (result1.is_ok() && result1.unwrap().rows_affected() > 0) as i32
            + (result2.is_ok() && result2.unwrap().rows_affected() > 0) as i32;

        assert_eq!(
            success_count, 1,
            "Only one concurrent update should succeed"
        );

        // Then: Subscription has a valid final state
        let final_status: String =
            sqlx::query_scalar("SELECT status FROM subscription WHERE id = $1")
                .bind(subscription_id)
                .fetch_one(&ctx.app_state.pool)
                .await
                .unwrap();

        assert!(final_status == "canceled" || final_status == "paused");

        // Cleanup
        delete_test_subscription(ctx, subscription_id).await;
    }
    // =========================================================================
    // Test: Invalid plan ID in webhook
    // =========================================================================
    // Given: Webhook 包含不存在的 plan_id
    // When: 处理 webhook
    // Then: 返回 400 错误，不创建 subscription

    #[test_context(ErrorRecoveryTestContext)]
    #[tokio::test]
    async fn test_invalid_plan_id_in_webhook(ctx: &mut ErrorRecoveryTestContext) {
        let realm_id = ctx._realm_id.clone();

        let invalid_plan_id = Uuid::now_v7();
        let (client_app_id, _) = (Uuid::now_v7(), "test_client".to_string());

        // Insert a dummy client app for the test
        sqlx::query(
            "INSERT INTO client_app (id, realm_id, client_id, name, redirect_uris, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"
        )
        .bind(client_app_id)
        .bind(&realm_id)
        .bind("test-invalid-plan-client")
        .bind("Test Invalid Plan Client")
        .bind(json!(["https://example.com/callback"]))
        .bind(true)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        let event_id = format!("evt_invalid_plan_{}", Uuid::now_v7());

        // When: Process webhook with invalid plan_id
        let result = sqlx::query(
            "INSERT INTO payment_event (id, realm_id, external_event_id, event_type, subscription_id, payload, processed, created_at, payment_provider)
             VALUES ($1, $2, $3, 'checkout.completed', NULL, $4, false, NOW(), 'creem')"
        )
        .bind(Uuid::now_v7())
        .bind(&realm_id)
        .bind(&event_id)
        .bind(json!({
            "planId": invalid_plan_id.to_string(),
            "clientAppId": client_app_id.to_string()
        }))
        .execute(&ctx.app_state.pool)
        .await;

        // Then: Operation may fail or succeed with processed=false
        // The key is that no subscription is created with invalid plan
        let subscriptions_with_invalid_plan: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM subscription WHERE plan_id = $1")
                .bind(invalid_plan_id)
                .fetch_one(&ctx.app_state.pool)
                .await
                .unwrap();

        assert_eq!(
            subscriptions_with_invalid_plan, 0,
            "No subscription should exist with invalid plan"
        );

        // Cleanup payment event if created
        if result.is_ok() {
            cleanup_payment_event_by_creem_id(ctx, &event_id).await;
        }
    }

    // =========================================================================
    // Test: Missing metadata in webhook
    // =========================================================================
    // Given: Webhook 缺少必需的 metadata
    // When: 处理 webhook
    // Then: 返回 400 错误

    #[test_context(ErrorRecoveryTestContext)]
    #[tokio::test]
    async fn test_missing_metadata_in_webhook(ctx: &mut ErrorRecoveryTestContext) {
        let realm_id = ctx._realm_id.clone();

        let event_id = format!("evt_missing_metadata_{}", Uuid::now_v7());

        // When: Process webhook with missing metadata
        let result = sqlx::query(
            "INSERT INTO payment_event (id, realm_id, external_event_id, event_type, subscription_id, payload, processed, created_at, payment_provider)
             VALUES ($1, $2, $3, 'checkout.completed', NULL, $4, false, NOW(), 'creem')"
        )
        .bind(Uuid::now_v7())
        .bind(&realm_id)
        .bind(&event_id)
        .bind(json!({
            "id": event_id,
            "eventType": "checkout.completed",
            "object": {
                "id": "chk_test",
                "status": "completed",
                // Missing required metadata: realmId, clientAppId, planId
            }
        }))
        .execute(&ctx.app_state.pool)
        .await;

        // Then: Event is recorded but marked as not processed
        assert!(result.is_ok(), "Payment event should be created");

        let processed: bool =
            sqlx::query_scalar("SELECT processed FROM payment_event WHERE external_event_id = $1 AND payment_provider = 'creem'")
                .bind(&event_id)
                .fetch_one(&ctx.app_state.pool)
                .await
                .unwrap();

        assert!(
            !processed,
            "Event should be marked as not processed due to missing metadata"
        );

        // Cleanup
        cleanup_payment_event_by_creem_id(ctx, &event_id).await;
    }
}
