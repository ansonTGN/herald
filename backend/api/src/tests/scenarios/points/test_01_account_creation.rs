// =============================================================================
// Points System Scenario Test 1: Account Creation
// =============================================================================
//
// **User Story**: US-PU-01 (View My Points Balance), US-PO-02 (View All User Points Accounts)
// **Priority**: P0
//
// **Scenario**: Create Account When User First Recharges
//
// **Given**:
// - A user with valid authentication
// - No existing points account for the user
// - A plan configuration with points on subscribe
//
// **When**:
// - The user subscribes to a plan (triggers webhook)
// - The webhook calls `/api/internal/points/recharge`
//
// **Then**:
// - A new points account is created with balance = points_on_subscribe
// - The account status is "active"
// - A transaction record is created with type="recharge"
// - The transaction amount matches points_on_subscribe
// - The balance_after equals the initial balance
//
// =============================================================================

use crate::tests::scenarios::points::fixtures::*;
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use test_context::test_context;
use uuid::Uuid;

/// ============================================================================
/// Scenario 1.1: Create Account When User First Recharges
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_account_creation_on_subscribe(ctx: &mut TestContext) {
    // ============================================================================
    // Given: A user with valid authentication, no existing points account, and a plan config
    // ============================================================================
    println!("[Step 1] Create test user");
    let user_id = create_test_user(&ctx._app_state.pool, &ctx._realm_id, "user1@example.com").await;

    let plan_id = create_test_plan(&ctx._app_state.pool, &ctx._realm_id, "pro-monthly", 2999).await;
    let points_on_subscribe = 1000;

    let config_id = create_test_plan_config(
        &ctx._app_state.pool,
        &ctx._realm_id,
        plan_id,
        points_on_subscribe,
        1000,
    )
    .await;

    let subscription_id =
        create_test_subscription(&ctx._app_state.pool, user_id, plan_id, &ctx._realm_id).await;

    println!(
        "[Step 1] ✓ Test data created: user={}, plan={}, config={}, subscription={}",
        user_id, plan_id, config_id, subscription_id
    );

    use herald_core::domain::points::entities::RechargeType;

    // ============================================================================
    // When: The webhook triggers recharge on subscribe
    // ============================================================================
    println!("[Step 2] Trigger recharge via service");

    // Call recharge_points_internal directly (simulating webhook behavior)
    // Note: We pass plan_id as the third parameter, not subscription_id
    // because find_plan_config expects a plan_id, not subscription_id
    let transaction = ctx
        ._app_state
        .points_service
        .recharge_points_internal(
            &ctx._realm_id,
            user_id,
            Some(plan_id), // Use plan_id instead of subscription_id
            points_on_subscribe,
            RechargeType::Subscribe,
            Some(subscription_id.to_string()), // Use subscription_id as external_ref_id
        )
        .await
        .expect("Recharge should succeed");

    println!(
        "[Step 2] ✓ Recharge succeeded: transaction_id={}",
        transaction.id
    );

    // ============================================================================
    // Then: Verify account creation and transaction
    // ============================================================================
    println!("[Step 3] Verify account was created");

    // Verify account was created in database
    let account: (Uuid, String, i64, String) = sqlx::query_as(
        "SELECT id, user_id::text, total_balance, status FROM points_accounts WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to fetch points account");

    assert_eq!(
        account.1,
        user_id.to_string(),
        "Account user_id should match"
    );
    assert_eq!(
        account.2, points_on_subscribe,
        "Account balance should equal points_on_subscribe"
    );
    assert_eq!(account.3, "active", "Account status should be active");

    println!(
        "[Step 3] ✓ Account created: balance={}, status={}",
        account.2, account.3
    );

    // Verify transaction was created
    let transaction: (String, String, i64, i64) = sqlx::query_as(
        "SELECT id::text, type::text, amount, balance_after
         FROM points_transactions
         WHERE user_id = $1 AND type = 'recharge'
         ORDER BY created_at DESC
         LIMIT 1",
    )
    .bind(user_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to fetch transaction");

    assert_eq!(
        transaction.1, "recharge",
        "Transaction type should be recharge"
    );
    assert_eq!(
        transaction.2, points_on_subscribe,
        "Transaction amount should match points_on_subscribe"
    );
    assert_eq!(
        transaction.3, points_on_subscribe,
        "Transaction balance_after should equal initial balance"
    );

    println!(
        "[Step 3] ✓ Transaction created: type={}, amount={}, balance_after={}",
        transaction.1, transaction.2, transaction.3
    );

    println!("\n✅ Scenario 1.1 完成：用户首次订阅时成功创建积分账户");
}
