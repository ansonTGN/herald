// =============================================================================
// Points Expiration Scenarios
// =============================================================================
//
// **User Story**: US-PO-03 (View User Points Transaction History)
// **Priority**: P1 (Extended to cover expiration processing)
//
// **Scenario**: Points expiration and automatic deduction
//
// **Given**:
// - A user with points that have expiration dates
// - Some points have expired
//
// **When**:
// - The expiration processing job runs
//
// **Then**:
// - Expired points are deducted from balance
// - Expiration transactions are recorded
// - Balance reflects only valid points
//
// =============================================================================

use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use test_context::test_context;
use uuid::Uuid;
use chrono::{Duration, Utc};

/// ============================================================================
/// Scenario 1: Expired Points Are Deducted
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_expired_points_are_deducted(ctx: &mut TestContext) {
    // ============================================================================
    // Given: A user with expired points
    // ============================================================================
    println!("[Step 1] Create user with expired points");

    let user_id = Uuid::now_v7();

    // Create user
    sqlx::query(
        "INSERT INTO account (id, realm_id, email, password, status)
         VALUES ($1, $2, $3, $4, 1)",
    )
    .bind(user_id)
    .bind(&ctx._realm_id)
    .bind("expired-points-user@test.com")
    .bind(&bcrypt::hash("password123", bcrypt::DEFAULT_COST).unwrap())
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create user");

    // Create points account with expired points
    let account_id = Uuid::now_v7();
    let expired_amount = 500;
    let valid_amount = 1000;

    sqlx::query(
        "INSERT INTO points_accounts (id, realm_id, user_id, topup_balance, subscription_balance, status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, 'active', NOW(), NOW())",
    )
    .bind(account_id)
    .bind(&ctx._realm_id)
    .bind(user_id)
    .bind(valid_amount + expired_amount)
    .bind(0)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create points account");

    // Create expired points transaction (expired yesterday)
    let expired_transaction_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO points_transactions (id, realm_id, account_id, user_id, type, amount, balance_after,
                                          expires_at, created_at, updated_at)
         VALUES ($1, $2, $3, $4, 'recharge', $5, $6, $7, NOW(), NOW())",
    )
    .bind(expired_transaction_id)
    .bind(&ctx._realm_id)
    .bind(account_id)
    .bind(user_id)
    .bind(expired_amount)
    .bind(valid_amount + expired_amount)
    .bind(Utc::now() - Duration::days(2))  // Expired 2 days ago
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create expired transaction");

    // Create valid points transaction (expires in future)
    let valid_transaction_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO points_transactions (id, realm_id, account_id, user_id, type, amount, balance_after,
                                          expires_at, created_at, updated_at)
         VALUES ($1, $2, $3, $4, 'recharge', $5, $6, $7, NOW(), NOW())",
    )
    .bind(valid_transaction_id)
    .bind(&ctx._realm_id)
    .bind(account_id)
    .bind(user_id)
    .bind(valid_amount)
    .bind(valid_amount + expired_amount)
    .bind(Utc::now() + Duration::days(30))  // Expires in 30 days
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create valid transaction");

    println!(
        "[Step 1] ✓ User created with expired points: {} expired, {} valid",
        expired_amount, valid_amount
    );

    // ============================================================================
    // When: Expiration processing job runs
    // ============================================================================
    println!("[Step 2] Process expired points");

    // Simulate expiration processing by calling the service
    let expiration_result = ctx
        ._app_state
        .points_service
        .process_expired_points(&ctx._realm_id, user_id)
        .await;

    if let Err(ref e) = expiration_result {
        eprintln!("Expiration processing failed with error: {:?}", e);
    }

    assert!(
        expiration_result.is_ok(),
        "Expiration processing should succeed, got error: {:?}",
        expiration_result
    );

    println!("[Step 2] ✓ Expiration processing completed");

    // ============================================================================
    // Then: Verify expired points are deducted
    // ============================================================================
    println!("[Step 3] Verify points deducted");

    // Check final balance
    let (final_balance,): (i64,) = sqlx::query_as(
        "SELECT total_balance FROM points_accounts WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to fetch final balance");

    assert_eq!(
        final_balance, valid_amount,
        "Final balance should only include valid points"
    );

    // Verify expiration transaction was created
    let (transaction_type, transaction_amount): (String, i64) = sqlx::query_as(
        "SELECT type::text, amount FROM points_transactions
         WHERE user_id = $1 AND type = 'expiration'
         ORDER BY created_at DESC
         LIMIT 1"
    )
    .bind(user_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to fetch expiration transaction");

    assert_eq!(transaction_type, "expiration", "Transaction type should be 'expiration'");
    assert_eq!(transaction_amount, -expired_amount, "Expiration should deduct expired amount");

    println!(
        "[Step 3] ✓ Points deducted correctly: final_balance={}, expiration_amount={}",
        final_balance, transaction_amount
    );
    println!("\n✅ Scenario 1 完成：过期积分已自动扣除");
}

/// ============================================================================
/// Scenario 2: Partial Expiration (Some Points Expired, Some Valid)
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_partial_expiration_mixed_validity(ctx: &mut TestContext) {
    // ============================================================================
    // Given: A user with multiple point batches at different expiration dates
    // ============================================================================
    println!("[Step 1] Create user with mixed expiration points");

    let user_id = Uuid::now_v7();

    // Create user
    sqlx::query(
        "INSERT INTO account (id, realm_id, email, password, status)
         VALUES ($1, $2, $3, $4, 1)",
    )
    .bind(user_id)
    .bind(&ctx._realm_id)
    .bind("mixed-expiry-user@test.com")
    .bind(&bcrypt::hash("password123", bcrypt::DEFAULT_COST).unwrap())
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create user");

    // Create points account
    let account_id = Uuid::now_v7();
    let total_points = 3000;

    sqlx::query(
        "INSERT INTO points_accounts (id, realm_id, user_id, topup_balance, subscription_balance, status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, 'active', NOW(), NOW())",
    )
    .bind(account_id)
    .bind(&ctx._realm_id)
    .bind(user_id)
    .bind(total_points)
    .bind(0)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create points account");

    // Batch 1: 1000 points, expired 2 days ago
    sqlx::query(
        "INSERT INTO points_transactions (id, realm_id, account_id, user_id, type, amount, balance_after,
                                          expires_at, created_at, updated_at)
         VALUES ($1, $2, $3, $4, 'recharge', 1000, 3000, $5, NOW(), NOW())",
    )
    .bind(Uuid::now_v7())
    .bind(&ctx._realm_id)
    .bind(account_id)
    .bind(user_id)
    .bind(Utc::now() - Duration::days(3))
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create batch 1");

    // Batch 2: 1000 points, expired yesterday
    sqlx::query(
        "INSERT INTO points_transactions (id, realm_id, account_id, user_id, type, amount, balance_after,
                                          expires_at, created_at, updated_at)
         VALUES ($1, $2, $3, $4, 'recharge', 1000, 3000, $5, NOW(), NOW())",
    )
    .bind(Uuid::now_v7())
    .bind(&ctx._realm_id)
    .bind(account_id)
    .bind(user_id)
    .bind(Utc::now() - Duration::days(2))
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create batch 2");

    // Batch 3: 1000 points, expires in 30 days (valid)
    sqlx::query(
        "INSERT INTO points_transactions (id, realm_id, account_id, user_id, type, amount, balance_after,
                                          expires_at, created_at, updated_at)
         VALUES ($1, $2, $3, $4, 'recharge', 1000, 3000, $5, NOW(), NOW())",
    )
    .bind(Uuid::now_v7())
    .bind(&ctx._realm_id)
    .bind(account_id)
    .bind(user_id)
    .bind(Utc::now() + Duration::days(30))
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create batch 3");

    println!("[Step 1] ✓ User created with 3 point batches: 2 expired, 1 valid");

    // ============================================================================
    // When: Process expired points
    // ============================================================================
    println!("[Step 2] Process expiration");

    let _result = ctx
        ._app_state
        .points_service
        .process_expired_points(&ctx._realm_id, user_id)
        .await
        .expect("Expiration processing should succeed");

    println!("[Step 2] ✓ Expiration processing completed");

    // ============================================================================
    // Then: Verify only expired points are deducted
    // ============================================================================
    println!("[Step 3] Verify partial expiration");

    let (final_balance,): (i64,) = sqlx::query_as(
        "SELECT total_balance FROM points_accounts WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to fetch final balance");

    assert_eq!(
        final_balance, 1000,
        "Only valid points should remain (batch 3)"
    );

    println!("[Step 3] ✓ Partial expiration verified: {} points remaining", final_balance);
    println!("\n✅ Scenario 2 完成：部分过期积分处理正确");
}

/// ============================================================================
/// Scenario 3: No Points Expired (All Valid)
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_no_expiration_all_valid(ctx: &mut TestContext) {
    // ============================================================================
    // Given: A user with all valid points
    // ============================================================================
    println!("[Step 1] Create user with all valid points");

    let user_id = Uuid::now_v7();

    // Create user
    sqlx::query(
        "INSERT INTO account (id, realm_id, email, password, status)
         VALUES ($1, $2, $3, $4, 1)",
    )
    .bind(user_id)
    .bind(&ctx._realm_id)
    .bind("all-valid-user@test.com")
    .bind(&bcrypt::hash("password123", bcrypt::DEFAULT_COST).unwrap())
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create user");

    // Create points account
    let account_id = Uuid::now_v7();
    let balance = 2000;

    sqlx::query(
        "INSERT INTO points_accounts (id, realm_id, user_id, topup_balance, subscription_balance, status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, 'active', NOW(), NOW())",
    )
    .bind(account_id)
    .bind(&ctx._realm_id)
    .bind(user_id)
    .bind(balance)
    .bind(0)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create points account");

    // Create valid points transaction (expires in future)
    sqlx::query(
        "INSERT INTO points_transactions (id, realm_id, account_id, user_id, type, amount, balance_after,
                                          expires_at, created_at, updated_at)
         VALUES ($1, $2, $3, $4, 'recharge', $5, $6, $7, NOW(), NOW())",
    )
    .bind(Uuid::now_v7())
    .bind(&ctx._realm_id)
    .bind(account_id)
    .bind(user_id)
    .bind(balance)
    .bind(balance)
    .bind(Utc::now() + Duration::days(60))  // Expires in 60 days
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create valid transaction");

    println!("[Step 1] ✓ User created with all valid points: {}", balance);

    // ============================================================================
    // When: Process expired points (none should expire)
    // ============================================================================
    println!("[Step 2] Process expiration");

    let result = ctx
        ._app_state
        .points_service
        .process_expired_points(&ctx._realm_id, user_id)
        .await;

    assert!(result.is_ok(), "Expiration processing should succeed even with no expired points");

    println!("[Step 2] ✓ Expiration processing completed (no points expired)");

    // ============================================================================
    // Then: Verify balance unchanged
    // ============================================================================
    println!("[Step 3] Verify balance unchanged");

    let (final_balance,): (i64,) = sqlx::query_as(
        "SELECT total_balance FROM points_accounts WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to fetch final balance");

    assert_eq!(
        final_balance, balance,
        "Balance should remain unchanged when no points expired"
    );

    println!("[Step 3] ✓ Balance unchanged: {}", final_balance);
    println!("\n✅ Scenario 3 完成：无过期积分时余额不变");
}
