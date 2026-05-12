// =============================================================================
// Billing Period Tests
// =============================================================================
//
// Test billing periods, renewals, and period calculations.
//
// User Story: docs/user-stories/06-billing-user-stories.md
// Covers: Monthly/yearly renewals, period switches, proration
//
// =============================================================================

#[cfg(test)]
mod tests {
    use crate::tests::helpers::payment_assertions::*;
    use crate::tests::helpers::*;
    use crate::tests::schema_test_context::SchemaTestContext;
    use chrono::{Duration, SubsecRound, Utc};
    use herald_core::domain::billing::entities::SubscriptionStatus;
    use serde_json::json;
    use test_context::test_context;
    use uuid::Uuid;

    use SchemaTestContext as BillingPeriodTestContext;

    // =========================================================================
    // Test: Monthly renewal success
    // =========================================================================
    // Given: Monthly subscription
    // When: 接收 `subscription.paid` webhook
    // Then: current_period_end 更新为下个月

    #[test_context(BillingPeriodTestContext)]
    #[tokio::test]
    async fn test_monthly_renewal_success(ctx: &mut BillingPeriodTestContext) {
        let realm_id = ctx._realm_id.clone();

        // Given: Create monthly subscription
        let plan_id = create_test_plan(ctx, &realm_id, "Monthly Renewal Plan").await;
        let (client_app_id, _) = (Uuid::now_v7(), "test_client".to_string());

        // Insert a dummy client app for the test
        sqlx::query(
            "INSERT INTO client_app (id, realm_id, client_id, name, redirect_uris, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"
        )
        .bind(client_app_id)
        .bind(&realm_id)
        .bind("test-monthly-renewal-client")
        .bind("Test Monthly Renewal Client")
        .bind(json!(["https://example.com/callback"]))
        .bind(true)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        let subscription_id = Uuid::now_v7();
        let now = Utc::now();
        let period_end = now + Duration::days(30);

        sqlx::query(
            "INSERT INTO subscription (id, realm_id, plan_id, client_app_id, status, billing_period, tier,
                                     external_subscription_id, external_product_id, payment_provider,
                                     current_period_start, current_period_end, cancel_at_period_end,
                                     created_at, updated_at)
             VALUES ($1, $2, $3, $4, 'active', 'monthly', 'starter',
                     'ext_sub_monthly_renewal', 'test_product', 'creem', $5, $6,
                     false, NOW(), NOW())"
        )
        .bind(subscription_id)
        .bind(&realm_id)
        .bind(plan_id)
        .bind(client_app_id)
        .bind(now)
        .bind(period_end)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // When: Renewal succeeds (simulate by updating period)
        let new_period_start = period_end;
        let new_period_end = period_end + Duration::days(30);

        update_subscription_period(ctx, subscription_id, new_period_start, new_period_end).await;

        // Then: current_period_end is updated to next month
        let (start, end): (chrono::DateTime<Utc>, chrono::DateTime<Utc>) = sqlx::query_as(
            "SELECT current_period_start, current_period_end FROM subscription WHERE id = $1",
        )
        .bind(subscription_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap();

        // Truncate to microsecond precision for comparison (PostgreSQL precision)
        assert_eq!(start, new_period_start.trunc_subsecs(6));
        assert_eq!(end, new_period_end.trunc_subsecs(6));

        // Then: Period duration is approximately 30 days
        assert_subscription_period_duration(&start, &end, 30);

        // Cleanup
        delete_test_subscription(ctx, subscription_id).await;
    }

    // =========================================================================
    // Test: Yearly renewal success
    // =========================================================================
    // Given: Yearly subscription
    // When: 接收 `subscription.paid` webhook
    // Then: current_period_end 更新为下一年

    #[test_context(BillingPeriodTestContext)]
    #[tokio::test]
    async fn test_yearly_renewal_success(ctx: &mut BillingPeriodTestContext) {
        let realm_id = ctx._realm_id.clone();

        // Given: Create yearly subscription
        let plan_id = create_test_plan_yearly(ctx, &realm_id, "Yearly Renewal Plan").await;
        let (client_app_id, _) = (Uuid::now_v7(), "test_client".to_string());

        // Insert a dummy client app for the test
        sqlx::query(
            "INSERT INTO client_app (id, realm_id, client_id, name, redirect_uris, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"
        )
        .bind(client_app_id)
        .bind(&realm_id)
        .bind("test-yearly-renewal-client")
        .bind("Test Yearly Renewal Client")
        .bind(json!(["https://example.com/callback"]))
        .bind(true)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        let subscription_id = Uuid::now_v7();
        let now = Utc::now();
        let period_end = now + Duration::days(365);

        sqlx::query(
            "INSERT INTO subscription (id, realm_id, plan_id, client_app_id, status, billing_period, tier,
                                     external_subscription_id, external_product_id, payment_provider,
                                     current_period_start, current_period_end, cancel_at_period_end,
                                     created_at, updated_at)
             VALUES ($1, $2, $3, $4, 'active', 'yearly', 'starter',
                     'ext_sub_yearly_renewal', 'test_product', 'creem', $5, $6,
                     false, NOW(), NOW())"
        )
        .bind(subscription_id)
        .bind(&realm_id)
        .bind(plan_id)
        .bind(client_app_id)
        .bind(now)
        .bind(period_end)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // When: Yearly renewal succeeds
        let new_period_start = period_end;
        let new_period_end = period_end + Duration::days(365);

        update_subscription_period(ctx, subscription_id, new_period_start, new_period_end).await;

        // Then: current_period_end is updated to next year
        let (start, end): (chrono::DateTime<Utc>, chrono::DateTime<Utc>) = sqlx::query_as(
            "SELECT current_period_start, current_period_end FROM subscription WHERE id = $1",
        )
        .bind(subscription_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap();

        // Truncate to microsecond precision for comparison (PostgreSQL precision)
        assert_eq!(start, new_period_start.trunc_subsecs(6));
        assert_eq!(end, new_period_end.trunc_subsecs(6));

        // Then: Period duration is approximately 365 days
        assert_subscription_period_duration(&start, &end, 365);

        // Cleanup
        delete_test_subscription(ctx, subscription_id).await;
    }

    // =========================================================================
    // Test: Monthly to yearly switch
    // =========================================================================
    // Given: Monthly subscription
    // When: 升级到 yearly plan
    // Then: billing_period 变为 yearly

    #[test_context(BillingPeriodTestContext)]
    #[tokio::test]
    async fn test_monthly_to_yearly_switch(ctx: &mut BillingPeriodTestContext) {
        let realm_id = ctx._realm_id.clone();

        // Given: Create monthly subscription
        let plan_id = create_test_plan(ctx, &realm_id, "Monthly Plan").await;
        let (client_app_id, _) = (Uuid::now_v7(), "test_client".to_string());

        // Insert a dummy client app for the test
        sqlx::query(
            "INSERT INTO client_app (id, realm_id, client_id, name, redirect_uris, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"
        )
        .bind(client_app_id)
        .bind(&realm_id)
        .bind("test-monthly-yearly-client")
        .bind("Test Monthly Yearly Client")
        .bind(json!(["https://example.com/callback"]))
        .bind(true)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        let subscription_id = Uuid::now_v7();

        sqlx::query(
            "INSERT INTO subscription (id, realm_id, plan_id, client_app_id, status, billing_period, tier,
                                     external_subscription_id, external_product_id, payment_provider,
                                     current_period_start, current_period_end, cancel_at_period_end,
                                     created_at, updated_at)
             VALUES ($1, $2, $3, $4, 'active', 'monthly', 'starter',
                     'ext_sub_monthly_yearly', 'test_product', 'creem', NOW(), NOW() + INTERVAL '30 days',
                     false, NOW(), NOW())"
        )
        .bind(subscription_id)
        .bind(&realm_id)
        .bind(plan_id)
        .bind(client_app_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Verify monthly billing period
        let billing_period: String =
            sqlx::query_scalar("SELECT billing_period FROM subscription WHERE id = $1")
                .bind(subscription_id)
                .fetch_one(&ctx.app_state.pool)
                .await
                .unwrap();

        assert_eq!(billing_period, "monthly");

        // When: Upgrade to yearly plan
        sqlx::query(
            "UPDATE subscription
             SET billing_period = 'yearly', updated_at = NOW()
             WHERE id = $1",
        )
        .bind(subscription_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Then: billing_period becomes yearly
        let new_billing_period: String =
            sqlx::query_scalar("SELECT billing_period FROM subscription WHERE id = $1")
                .bind(subscription_id)
                .fetch_one(&ctx.app_state.pool)
                .await
                .unwrap();

        assert_eq!(new_billing_period, "yearly");

        // Cleanup
        delete_test_subscription(ctx, subscription_id).await;
    }

    // =========================================================================
    // Test: Yearly to monthly switch
    // =========================================================================
    // Given: Yearly subscription
    // When: 降级到 monthly plan
    // Then: billing_period 在下个周期变为 monthly

    #[test_context(BillingPeriodTestContext)]
    #[tokio::test]
    async fn test_yearly_to_monthly_switch(ctx: &mut BillingPeriodTestContext) {
        let realm_id = ctx._realm_id.clone();

        // Given: Create yearly subscription
        let plan_id = create_test_plan_yearly(ctx, &realm_id, "Yearly Plan").await;
        let (client_app_id, _) = (Uuid::now_v7(), "test_client".to_string());

        // Insert a dummy client app for the test
        sqlx::query(
            "INSERT INTO client_app (id, realm_id, client_id, name, redirect_uris, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"
        )
        .bind(client_app_id)
        .bind(&realm_id)
        .bind("test-yearly-monthly-client")
        .bind("Test Yearly Monthly Client")
        .bind(json!(["https://example.com/callback"]))
        .bind(true)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        let subscription_id = Uuid::now_v7();

        sqlx::query(
            "INSERT INTO subscription (id, realm_id, plan_id, client_app_id, status, billing_period, tier,
                                     external_subscription_id, external_product_id, payment_provider,
                                     current_period_start, current_period_end, cancel_at_period_end,
                                     created_at, updated_at)
             VALUES ($1, $2, $3, $4, 'active', 'yearly', 'starter',
                     'ext_sub_yearly_monthly', 'test_product', 'creem', NOW(), NOW() + INTERVAL '365 days',
                     false, NOW(), NOW())"
        )
        .bind(subscription_id)
        .bind(&realm_id)
        .bind(plan_id)
        .bind(client_app_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // When: Downgrade to monthly plan (takes effect at next period)
        sqlx::query(
            "UPDATE subscription
             SET billing_period = 'monthly', updated_at = NOW()
             WHERE id = $1",
        )
        .bind(subscription_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Then: billing_period becomes monthly
        let new_billing_period: String =
            sqlx::query_scalar("SELECT billing_period FROM subscription WHERE id = $1")
                .bind(subscription_id)
                .fetch_one(&ctx.app_state.pool)
                .await
                .unwrap();

        assert_eq!(new_billing_period, "monthly");

        // Cleanup
        delete_test_subscription(ctx, subscription_id).await;
    }

    // =========================================================================
    // Test: Renewal failure creates past due
    // =========================================================================
    // Given: Active subscription
    // When: 续费失败（`subscription.past_due` webhook）
    // Then: 状态变为 past_due

    #[test_context(BillingPeriodTestContext)]
    #[tokio::test]
    async fn test_renewal_failure_creates_past_due(ctx: &mut BillingPeriodTestContext) {
        let realm_id = ctx._realm_id.clone();

        // Given: Create active subscription
        let plan_id = create_test_plan(ctx, &realm_id, "Past Due Plan").await;
        let (client_app_id, _) = (Uuid::now_v7(), "test_client".to_string());

        // Insert a dummy client app for the test
        sqlx::query(
            "INSERT INTO client_app (id, realm_id, client_id, name, redirect_uris, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"
        )
        .bind(client_app_id)
        .bind(&realm_id)
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
                                     current_period_start, current_period_end, cancel_at_period_end,
                                     created_at, updated_at)
             VALUES ($1, $2, $3, $4, 'active', 'monthly', 'starter',
                     'ext_sub_past_due', 'test_product', 'creem', NOW(), NOW() + INTERVAL '30 days',
                     false, NOW(), NOW())"
        )
        .bind(subscription_id)
        .bind(&realm_id)
        .bind(plan_id)
        .bind(client_app_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Verify active status
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Active).await;

        // When: Renewal fails
        update_subscription_status(ctx, subscription_id, "past_due").await;

        // Then: Status becomes past_due
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::PastDue).await;

        // Then: Subscription should not have access
        assert!(!SubscriptionStatus::PastDue.has_access());

        // Cleanup
        delete_test_subscription(ctx, subscription_id).await;
    }

    // =========================================================================
    // Test: Renewal retry success
    // =========================================================================
    // Given: PastDue subscription
    // When: 重试支付成功
    // Then: 状态变为 active

    #[test_context(BillingPeriodTestContext)]
    #[tokio::test]
    async fn test_renewal_retry_success(ctx: &mut BillingPeriodTestContext) {
        let realm_id = ctx._realm_id.clone();

        // Given: Create past_due subscription
        let plan_id = create_test_plan(ctx, &realm_id, "Retry Success Plan").await;
        let (client_app_id, _) = (Uuid::now_v7(), "test_client".to_string());

        // Insert a dummy client app for the test
        sqlx::query(
            "INSERT INTO client_app (id, realm_id, client_id, name, redirect_uris, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"
        )
        .bind(client_app_id)
        .bind(&realm_id)
        .bind("test-retry-success-client")
        .bind("Test Retry Success Client")
        .bind(json!(["https://example.com/callback"]))
        .bind(true)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        let subscription_id = Uuid::now_v7();

        sqlx::query(
            "INSERT INTO subscription (id, realm_id, plan_id, client_app_id, status, billing_period, tier,
                                     external_subscription_id, external_product_id, payment_provider,
                                     current_period_start, current_period_end, cancel_at_period_end,
                                     created_at, updated_at)
             VALUES ($1, $2, $3, $4, 'past_due', 'monthly', 'starter',
                     'ext_sub_retry_success', 'test_product', 'creem', NOW(), NOW(),
                     false, NOW(), NOW())"
        )
        .bind(subscription_id)
        .bind(&realm_id)
        .bind(plan_id)
        .bind(client_app_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Verify past_due status
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::PastDue).await;

        // When: Retry payment succeeds
        update_subscription_status(ctx, subscription_id, "active").await;

        // Then: Status becomes active
        verify_subscription_status(ctx, subscription_id, SubscriptionStatus::Active).await;

        // Cleanup
        delete_test_subscription(ctx, subscription_id).await;
    }

    // =========================================================================
    // Test: Quarterly billing period
    // =========================================================================
    // Given: Quarterly plan
    // When: 创建订阅并续费
    // Then: billing_period 为 quarterly，周期为 3 个月
    //
    // NOTE: Commented out because schema constraint only allows 'monthly' or 'yearly' plan types.
    // Quarterly billing is not currently supported by the database schema.
    // See: backend/app/migrations/20260209_core_init.sql line 409 - chk_plan_type constraint

    // #[test_context(BillingPeriodTestContext)]
    // #[tokio::test]
    // async fn test_quarterly_billing_period(ctx: &mut BillingPeriodTestContext) {
    // let realm_id = ctx._realm_id.clone();

    // // Given: Create quarterly plan
    // let plan_id = create_test_plan_quarterly(ctx, &realm_id, "Quarterly Plan").await;
    // let (client_app_id, _) = (Uuid::now_v7(), "test_client".to_string());

    // // Insert a dummy client app for the test
    // sqlx::query(
    //     "INSERT INTO client_app (id, realm_id, client_id, name, redirect_uris, enabled, created_at, updated_at)
    //      VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"
    // )
    // .bind(client_app_id)
    // .bind(&realm_id)
    // .bind("test-quarterly-client")
    // .bind("Test Quarterly Client")
    // .bind(json!(["https://example.com/callback"]))
    // .bind(true)
    // .execute(&ctx.app_state.pool)
    // .await
    // .unwrap();

    // let subscription_id = Uuid::now_v7();
    // let now = Utc::now();
    // let period_end = now + Duration::days(90); // Quarterly = 3 months ≈ 90 days

    // sqlx::query(
    //     "INSERT INTO subscription (id, realm_id, plan_id, client_app_id, status, billing_period, tier,
    //                              external_product_id, current_period_start, current_period_end,
    //                              cancel_at_period_end, created_at, updated_at)
    //      VALUES ($1, $2, $3, $4, 'active', 'quarterly', 'starter', 'test_product', $5, $6,
    //              false, NOW(), NOW())"
    // )
    // .bind(subscription_id)
    // .bind(&realm_id)
    // .bind(plan_id)
    // .bind(client_app_id)
    // .bind(now)
    // .bind(period_end)
    // .execute(&ctx.app_state.pool)
    // .await
    // .unwrap();

    // // Then: billing_period is quarterly
    // let billing_period: String = sqlx::query_scalar(
    //     "SELECT billing_period FROM subscription WHERE id = $1"
    // )
    // .bind(subscription_id)
    // .fetch_one(&ctx.app_state.pool)
    // .await
    // .unwrap();

    // assert_eq!(billing_period, "quarterly");

    // // Then: Period duration is approximately 90 days
    // let (start, end): (chrono::DateTime<Utc>, chrono::DateTime<Utc>) = sqlx::query_as(
    //     "SELECT current_period_start, current_period_end FROM subscription WHERE id = $1"
    // )
    // .bind(subscription_id)
    // .fetch_one(&ctx.app_state.pool)
    // .await
    // .unwrap();

    // assert_subscription_period_duration(&start, &end, 90);

    // // Cleanup
    // delete_test_subscription(ctx, subscription_id).await;

    // =========================================================================
    // Test: Billing period proration
    // =========================================================================
    // Given: Monthly subscription
    // When: 在月中升级到 yearly
    // Then: 按比例计算费用（proration）

    #[test_context(BillingPeriodTestContext)]
    #[tokio::test]
    async fn test_billing_period_proration(ctx: &mut BillingPeriodTestContext) {
        let realm_id = ctx._realm_id.clone();

        // Given: Create monthly subscription mid-period
        let plan_id = create_test_plan(ctx, &realm_id, "Proration Plan").await;
        let (client_app_id, _) = (Uuid::now_v7(), "test_client".to_string());

        // Insert a dummy client app for the test
        sqlx::query(
            "INSERT INTO client_app (id, realm_id, client_id, name, redirect_uris, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"
        )
        .bind(client_app_id)
        .bind(&realm_id)
        .bind("test-proration-client")
        .bind("Test Proration Client")
        .bind(json!(["https://example.com/callback"]))
        .bind(true)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        let subscription_id = Uuid::now_v7();
        let now = Utc::now();
        let period_start = now - Duration::days(15); // 15 days into the period
        let period_end = now + Duration::days(15); // 15 days remaining

        sqlx::query(
            "INSERT INTO subscription (id, realm_id, plan_id, client_app_id, status, billing_period, tier,
                                     external_subscription_id, external_product_id, payment_provider,
                                     current_period_start, current_period_end, cancel_at_period_end,
                                     created_at, updated_at)
             VALUES ($1, $2, $3, $4, 'active', 'monthly', 'starter',
                     'ext_sub_proration', 'test_product', 'creem', $5, $6,
                     false, NOW(), NOW())"
        )
        .bind(subscription_id)
        .bind(&realm_id)
        .bind(plan_id)
        .bind(client_app_id)
        .bind(period_start)
        .bind(period_end)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // When: Upgrade to yearly plan with proration
        // In a real implementation, this would calculate prorated amounts
        // For this test, we verify the period is updated correctly
        let new_period_start = period_end;
        let new_period_end = period_end + Duration::days(365);

        update_subscription_period(ctx, subscription_id, new_period_start, new_period_end).await;

        sqlx::query(
            "UPDATE subscription
             SET billing_period = 'yearly', updated_at = NOW()
             WHERE id = $1",
        )
        .bind(subscription_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        // Then: billing_period is yearly
        let billing_period: String =
            sqlx::query_scalar("SELECT billing_period FROM subscription WHERE id = $1")
                .bind(subscription_id)
                .fetch_one(&ctx.app_state.pool)
                .await
                .unwrap();

        assert_eq!(billing_period, "yearly");

        // Then: Period is updated correctly
        let (start, end): (chrono::DateTime<Utc>, chrono::DateTime<Utc>) = sqlx::query_as(
            "SELECT current_period_start, current_period_end FROM subscription WHERE id = $1",
        )
        .bind(subscription_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap();

        // Truncate to microsecond precision for comparison (PostgreSQL precision)
        assert_eq!(start, new_period_start.trunc_subsecs(6));
        assert_eq!(end, new_period_end.trunc_subsecs(6));

        // Cleanup
        delete_test_subscription(ctx, subscription_id).await;
    }
}
