// =============================================================================
// Points Refund Scenarios
// =============================================================================
//
// **User Story**: US-PO-04 (Manage Points Plan Configuration - Extended)
// **Priority**: P2 (Extended to cover refund processing)
//
// **Scenario**: Points refund when subscription is canceled
//
// **Given**:
// - A user with an active subscription and points
// - User cancels subscription
//
// **When**:
// - The refund webhook is received
//
// **Then**:
// - Refunded points are deducted from balance
// - Refund transaction is recorded
// - User's points balance is updated correctly
//
// =============================================================================

use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use herald_core::domain::points::entities::RechargeType;
use test_context::test_context;
use uuid::Uuid;

/// ============================================================================
/// Scenario 1: Full Points Refund on Subscription Cancellation
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_full_points_refund_on_cancellation(ctx: &mut TestContext) {
    // ============================================================================
    // Given: A user with active subscription and points
    // ============================================================================
    println!("[Step 1] Create user with subscription and points");

    let user_id = Uuid::now_v7();

    // Create user
    sqlx::query(
        "INSERT INTO account (id, realm_id, email, password, status)
         VALUES ($1, $2, $3, $4, 1)",
    )
    .bind(user_id)
    .bind(&ctx._realm_id)
    .bind("refund-user@test.com")
    .bind(bcrypt::hash("password123", bcrypt::DEFAULT_COST).unwrap())
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create user");

    // Create points account
    let account_id = Uuid::now_v7();
    let initial_balance = 2000;

    sqlx::query(
        "INSERT INTO points_accounts (id, realm_id, user_id, topup_balance, subscription_balance, status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, 'active', NOW(), NOW())",
    )
    .bind(account_id)
    .bind(&ctx._realm_id)
    .bind(user_id)
    .bind(initial_balance)
    .bind(0)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create points account");

    // Create plan and subscription
    // Ensure default product exists for the plan
    let product_id: Uuid = sqlx::query_scalar(
        "INSERT INTO products (id, realm_id, name, title, description, sort_order, enabled, created_at, updated_at)
         VALUES ($1, $2, 'default', 'Default Product', 'Default test product', 0, true, NOW(), NOW())
         ON CONFLICT (realm_id, name) DO UPDATE SET updated_at = products.updated_at
         RETURNING id"
    )
    .bind(Uuid::now_v7())
    .bind(&ctx._realm_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to ensure default product");

    let plan_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO plan (id, realm_id, name, description, title, type, price, currency,
                          active, trial_days, sort_order, product_id, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, NOW(), NOW())",
    )
    .bind(plan_id)
    .bind(&ctx._realm_id)
    .bind("Premium Plan")
    .bind("Premium subscription")
    .bind("Premium")
    .bind("monthly")
    .bind(2999)
    .bind("USD")
    .bind(true)
    .bind(0)
    .bind(1)
    .bind(product_id)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create plan");

    // Create plan config with 1000 points per period
    let config_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO points_plan_configs (id, realm_id, plan_id, grant_period_type, points_per_period,
                                          validity_days, grant_on_subscribe, max_periods, active, created_at, updated_at)
         VALUES ($1, $2, $3, 'monthly', 1000, 30, true, NULL, true, NOW(), NOW())",
    )
    .bind(config_id)
    .bind(&ctx._realm_id)
    .bind(plan_id)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create plan config");

    // Create subscription
    let subscription_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO subscription (id, realm_id, plan_id, client_app_id, status, billing_period, tier,
                                    external_product_id, external_subscription_id, payment_provider,
                                    current_period_start, current_period_end, cancel_at_period_end,
                                    created_at, updated_at)
         VALUES ($1, $2, $3, NULL, 'active', 'monthly', 'free', $4, $5, 'creem',
                 NOW(), NOW() + INTERVAL '30 days', false, NOW(), NOW())",
    )
    .bind(subscription_id)
    .bind(&ctx._realm_id)
    .bind(plan_id)
    .bind(format!("prod_{}", subscription_id))
    .bind(format!("sub_{}", subscription_id))
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create subscription");

    // Create initial recharge transaction (simulating subscription)
    // Note: Don't set external_ref_id to avoid idempotency constraint conflict with refund
    let recharge_amount = 1000;
    let _transaction = ctx
        ._app_state
        .points_service
        .recharge_points_internal(
            &ctx._realm_id,
            user_id,
            Some(plan_id),
            recharge_amount,
            RechargeType::Subscribe,
            None, // No external_ref_id - refund will use subscription_id as external_ref_id
        )
        .await
        .expect("Recharge should succeed");

    println!(
        "[Step 1] ✓ User created: balance={}, subscription={}",
        initial_balance, subscription_id
    );

    // ============================================================================
    // When: Subscription is canceled and refund is processed
    // ============================================================================
    println!("[Step 2] Process subscription cancellation refund");

    let refund_amount = 1000; // Full refund of the subscribed points

    let _transaction = ctx
        ._app_state
        .points_service
        .refund_points(
            &ctx._realm_id,
            user_id,
            subscription_id.to_string(),
            refund_amount,
            "Subscription canceled".to_string(),
        )
        .await
        .expect("Refund should succeed");

    println!("[Step 2] ✓ Refund processed: amount={}", refund_amount);

    // ============================================================================
    // Then: Verify points are refunded and deducted
    // ============================================================================
    println!("[Step 3] Verify refund");

    let (final_balance,): (i64,) =
        sqlx::query_as("SELECT total_balance FROM points_accounts WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&ctx._app_state.pool)
            .await
            .expect("Failed to fetch final balance");

    let expected_balance = initial_balance + recharge_amount - refund_amount;
    assert_eq!(
        final_balance, expected_balance,
        "Balance should be initial + recharge - refund"
    );

    // Verify refund transaction was created
    let (transaction_type, amount): (String, i64) = sqlx::query_as(
        "SELECT type::text, amount FROM points_transactions
         WHERE user_id = $1 AND type = 'refund'
         ORDER BY created_at DESC
         LIMIT 1",
    )
    .bind(user_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to fetch refund transaction");

    assert_eq!(
        transaction_type, "refund",
        "Transaction type should be 'refund'"
    );
    assert_eq!(amount, -refund_amount, "Refund amount should be negative");

    println!(
        "[Step 3] ✓ Refund verified: final_balance={}, refund_amount={}",
        final_balance, amount
    );
    println!("\n✅ Scenario 1 完成：订阅取消时积分全额退款");
}

/// ============================================================================
/// Scenario 2: Partial Refund (Prorated Based on Usage)
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_partial_refund_prorated(ctx: &mut TestContext) {
    // ============================================================================
    // Given: A user with subscription used for partial period
    // ============================================================================
    println!("[Step 1] Create user with partial period subscription");

    let user_id = Uuid::now_v7();

    // Create user
    sqlx::query(
        "INSERT INTO account (id, realm_id, email, password, status)
         VALUES ($1, $2, $3, $4, 1)",
    )
    .bind(user_id)
    .bind(&ctx._realm_id)
    .bind("partial-refund-user@test.com")
    .bind(bcrypt::hash("password123", bcrypt::DEFAULT_COST).unwrap())
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create user");

    // Create points account
    let account_id = Uuid::now_v7();
    let initial_balance = 3000;

    sqlx::query(
        "INSERT INTO points_accounts (id, realm_id, user_id, topup_balance, subscription_balance, status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, 'active', NOW(), NOW())",
    )
    .bind(account_id)
    .bind(&ctx._realm_id)
    .bind(user_id)
    .bind(initial_balance)
    .bind(0)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create points account");

    // Create plan and subscription
    // Ensure default product exists for the plan
    let product_id: Uuid = sqlx::query_scalar(
        "INSERT INTO products (id, realm_id, name, title, description, sort_order, enabled, created_at, updated_at)
         VALUES ($1, $2, 'default', 'Default Product', 'Default test product', 0, true, NOW(), NOW())
         ON CONFLICT (realm_id, name) DO UPDATE SET updated_at = products.updated_at
         RETURNING id"
    )
    .bind(Uuid::now_v7())
    .bind(&ctx._realm_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to ensure default product");

    let plan_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO plan (id, realm_id, name, description, title, type, price, currency,
                          active, trial_days, sort_order, product_id, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, NOW(), NOW())",
    )
    .bind(plan_id)
    .bind(&ctx._realm_id)
    .bind("Standard Plan")
    .bind("Standard subscription")
    .bind("Standard")
    .bind("monthly")
    .bind(1999)
    .bind("USD")
    .bind(true)
    .bind(0)
    .bind(2)
    .bind(product_id)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create plan");

    let subscription_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO subscription (id, realm_id, plan_id, client_app_id, status, billing_period, tier,
                                    external_product_id, external_subscription_id, payment_provider,
                                    current_period_start, current_period_end, cancel_at_period_end,
                                    created_at, updated_at)
         VALUES ($1, $2, $3, NULL, 'active', 'monthly', 'free', $4, $5, 'creem',
                 NOW() - INTERVAL '15 days', NOW() + INTERVAL '15 days', false, NOW(), NOW())",
    )
    .bind(subscription_id)
    .bind(&ctx._realm_id)
    .bind(plan_id)
    .bind(format!("prod_{}", subscription_id))
    .bind(format!("sub_{}", subscription_id))
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create subscription");

    println!("[Step 1] ✓ User created with partial period subscription");

    // ============================================================================
    // When: Process partial refund (50% of period used)
    // ============================================================================
    println!("[Step 2] Process partial refund");

    let full_recharge_amount = 1000;
    let prorated_refund = 500; // 50% refund based on usage

    // First recharge (full amount)
    // Note: Don't set external_ref_id to avoid idempotency constraint conflict with refund
    ctx._app_state
        .points_service
        .recharge_points_internal(
            &ctx._realm_id,
            user_id,
            Some(plan_id),
            full_recharge_amount,
            RechargeType::Subscribe,
            None, // No external_ref_id - refund will use subscription_id as external_ref_id
        )
        .await
        .expect("Recharge should succeed");

    // Then process partial refund
    let _transaction = ctx
        ._app_state
        .points_service
        .refund_points(
            &ctx._realm_id,
            user_id,
            subscription_id.to_string(),
            prorated_refund,
            "Prorated refund - 50% of period used".to_string(),
        )
        .await
        .expect("Refund should succeed");

    println!(
        "[Step 2] ✓ Partial refund processed: amount={}",
        prorated_refund
    );

    // ============================================================================
    // Then: Verify partial refund is applied
    // ============================================================================
    println!("[Step 3] Verify partial refund");

    let (final_balance,): (i64,) =
        sqlx::query_as("SELECT total_balance FROM points_accounts WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&ctx._app_state.pool)
            .await
            .expect("Failed to fetch final balance");

    let expected_balance = initial_balance + full_recharge_amount - prorated_refund;
    assert_eq!(
        final_balance, expected_balance,
        "Balance should reflect partial refund"
    );

    println!(
        "[Step 3] ✓ Partial refund verified: final_balance={}, expected={}",
        final_balance, expected_balance
    );
    println!("\n✅ Scenario 2 完成：按使用比例部分退款");
}

/// ============================================================================
/// Scenario 3: Multiple Refunds for Different Periods
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_multiple_refunds_different_periods(ctx: &mut TestContext) {
    // ============================================================================
    // Given: A user with multiple subscription periods
    // ============================================================================
    println!("[Step 1] Create user with multiple periods");

    let user_id = Uuid::now_v7();

    // Create user
    sqlx::query(
        "INSERT INTO account (id, realm_id, email, password, status)
         VALUES ($1, $2, $3, $4, 1)",
    )
    .bind(user_id)
    .bind(&ctx._realm_id)
    .bind("multi-period-user@test.com")
    .bind(bcrypt::hash("password123", bcrypt::DEFAULT_COST).unwrap())
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create user");

    // Create points account
    let account_id = Uuid::now_v7();
    let initial_balance = 5000;

    sqlx::query(
        "INSERT INTO points_accounts (id, realm_id, user_id, topup_balance, subscription_balance, status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, 'active', NOW(), NOW())",
    )
    .bind(account_id)
    .bind(&ctx._realm_id)
    .bind(user_id)
    .bind(initial_balance)
    .bind(0)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create points account");

    // Create plan
    // Ensure default product exists for the plan
    let product_id: Uuid = sqlx::query_scalar(
        "INSERT INTO products (id, realm_id, name, title, description, sort_order, enabled, created_at, updated_at)
         VALUES ($1, $2, 'default', 'Default Product', 'Default test product', 0, true, NOW(), NOW())
         ON CONFLICT (realm_id, name) DO UPDATE SET updated_at = products.updated_at
         RETURNING id"
    )
    .bind(Uuid::now_v7())
    .bind(&ctx._realm_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to ensure default product");

    let plan_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO plan (id, realm_id, name, description, title, type, price, currency,
                          active, trial_days, sort_order, product_id, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, NOW(), NOW())",
    )
    .bind(plan_id)
    .bind(&ctx._realm_id)
    .bind("Annual Plan")
    .bind("Annual subscription")
    .bind("Annual")
    .bind("yearly")
    .bind(29999)
    .bind("USD")
    .bind(true)
    .bind(0)
    .bind(3)
    .bind(product_id)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create plan");

    let subscription_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO subscription (id, realm_id, plan_id, client_app_id, status, billing_period, tier,
                                    external_product_id, external_subscription_id, payment_provider,
                                    current_period_start, current_period_end, cancel_at_period_end,
                                    created_at, updated_at)
         VALUES ($1, $2, $3, NULL, 'active', 'yearly', 'free', $4, $5, 'creem',
                 NOW(), NOW() + INTERVAL '365 days', false, NOW(), NOW())",
    )
    .bind(subscription_id)
    .bind(&ctx._realm_id)
    .bind(plan_id)
    .bind(format!("prod_{}", subscription_id))
    .bind(format!("sub_{}", subscription_id))
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create subscription");

    println!("[Step 1] ✓ User created with annual subscription");

    // ============================================================================
    // When: Process multiple refunds for different periods
    // ============================================================================
    println!("[Step 2] Process multiple period refunds");

    let period1_refund = 2000;
    let period2_refund = 1500;
    let period3_refund = 1000;

    // Process first period refund
    ctx._app_state
        .points_service
        .refund_points(
            &ctx._realm_id,
            user_id,
            format!("{}_period1", subscription_id),
            period1_refund,
            "Refund period 1".to_string(),
        )
        .await
        .expect("First refund should succeed");

    // Process second period refund
    ctx._app_state
        .points_service
        .refund_points(
            &ctx._realm_id,
            user_id,
            format!("{}_period2", subscription_id),
            period2_refund,
            "Refund period 2".to_string(),
        )
        .await
        .expect("Second refund should succeed");

    // Process third period refund
    ctx._app_state
        .points_service
        .refund_points(
            &ctx._realm_id,
            user_id,
            format!("{}_period3", subscription_id),
            period3_refund,
            "Refund period 3".to_string(),
        )
        .await
        .expect("Third refund should succeed");

    let total_refund = period1_refund + period2_refund + period3_refund;
    println!(
        "[Step 2] ✓ Multiple refunds processed: total={}",
        total_refund
    );

    // ============================================================================
    // Then: Verify all refunds are applied
    // ============================================================================
    println!("[Step 3] Verify multiple refunds");

    let (final_balance,): (i64,) =
        sqlx::query_as("SELECT total_balance FROM points_accounts WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&ctx._app_state.pool)
            .await
            .expect("Failed to fetch final balance");

    let expected_balance = initial_balance - total_refund;
    assert_eq!(
        final_balance, expected_balance,
        "Balance should reflect all refunds"
    );

    // Verify all refund transactions exist
    let (refund_count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM points_transactions
         WHERE user_id = $1 AND type = 'refund'",
    )
    .bind(user_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to count refund transactions");

    assert_eq!(refund_count, 3, "Should have 3 refund transactions");

    println!(
        "[Step 3] ✓ Multiple refunds verified: final_balance={}, refund_count={}",
        final_balance, refund_count
    );
    println!("\n✅ Scenario 3 完成：多期退款处理正确");
}
