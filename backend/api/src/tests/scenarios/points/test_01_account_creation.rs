// =============================================================================
// Points System Scenario Test 1: Account Creation
// =============================================================================
//
// **User Story**: US-PU-01 (View My Points Balance), US-PO-02 (View All User Wallets)
// **Priority**: P0
//
// **Scenario**: Create Account When User First Recharges
//
// **Given**:
// - A user with valid authentication
// - No existing points account for the user
// - An entitlement points policy with points on subscribe
//
// **When**:
// - The user subscribes to an entitlement (triggers webhook)
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
    // Given: A user with valid authentication, no existing points account, and an entitlement points policy
    // ============================================================================
    println!("[Step 1] Create test user");
    let user_id = create_test_user(&ctx._app_state.pool, &ctx._realm_id, "user1@example.com").await;

    let mapping_id =
        create_test_entitlement_mapping(&ctx._app_state.pool, &ctx._realm_id, "pro-monthly", 2999)
            .await;
    let points_on_subscribe = 1000;

    let config_id = configure_test_entitlement_points(
        &ctx._app_state.pool,
        &ctx._realm_id,
        mapping_id,
        points_on_subscribe,
        1000,
    )
    .await;

    let subscription_id =
        create_test_subscription(&ctx._app_state.pool, user_id, mapping_id, &ctx._realm_id).await;

    println!(
        "[Step 1] ✓ Test data created: user={}, mapping={}, config={}, subscription={}",
        user_id, mapping_id, config_id, subscription_id
    );

    use herald_core::domain::points::entities::RechargeType;

    // ============================================================================
    // When: The webhook triggers recharge on subscribe
    // ============================================================================
    println!("[Step 2] Trigger recharge via service");

    // Credit-bucket: recharge now requires an explicit bucket_id target.
    // Create a real bucket so the operation has a valid target.
    use crate::tests::helpers::credit_bucket_helpers::{
        CreditBucketOpts, attach_bucket_client_app, create_test_credit_bucket,
    };
    let bucket_id = create_test_credit_bucket(
        &ctx._app_state.pool,
        &ctx._realm_id,
        CreditBucketOpts::default(),
    )
    .await;
    let _client_app_uuid: Uuid = ctx
        ._client_app_id
        .parse()
        .expect("_client_app_id should be a valid UUID");
    attach_bucket_client_app(
        &ctx._app_state.pool,
        &ctx._realm_id,
        bucket_id,
        _client_app_uuid,
    )
    .await;

    let transaction = ctx
        ._app_state
        .points_service
        .recharge_points_internal(
            &ctx._realm_id,
            user_id,
            bucket_id,
            points_on_subscribe,
            RechargeType::Subscribe,
            Some(subscription_id.to_string()),
            None, // expires_at: no expiration for test
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
        "SELECT w.id, w.user_id::text,
                COALESCE(SUM(l.remaining_amount) FILTER (
                    WHERE l.status = 'active' AND l.remaining_amount > 0
                      AND (l.effective_at IS NULL OR l.effective_at <= NOW())
                      AND (l.expires_at  IS NULL OR l.expires_at  >  NOW())
                ), 0)::BIGINT AS total_balance,
                w.status
         FROM points_wallets w
         LEFT JOIN points_credit_ledger l
           ON l.realm_id = w.realm_id AND l.user_id = w.user_id AND l.bucket_id = w.bucket_id
         WHERE w.user_id = $1
         GROUP BY w.id, w.user_id, w.status",
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
         WHERE user_id = $1 AND type = 'subscription_grant'
         ORDER BY created_at DESC
         LIMIT 1",
    )
    .bind(user_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to fetch transaction");

    assert_eq!(
        transaction.1, "subscription_grant",
        "Transaction type should be subscription_grant"
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
