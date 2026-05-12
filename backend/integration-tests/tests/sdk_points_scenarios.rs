// =============================================================================
// SDK Points Scenarios
// =============================================================================
//
// Test SDK points API against real API
// =============================================================================

use herald_sdk::Client;
use herald_test_support::SchemaTestContext;
use sqlx::query;
use test_context::test_context;
use uuid::Uuid;

/// Helper function to create a points account with initial balance
async fn create_points_account_with_balance(
    ctx: &mut SchemaTestContext,
    user_id: &str,
    initial_balance: i64,
) -> Uuid {
    let account_uuid = Uuid::now_v7();
    let user_uuid = Uuid::parse_str(user_id).expect("Invalid user_id UUID format");
    query(
        "INSERT INTO points_accounts (
            id, user_id, realm_id,
            topup_balance, subscription_balance,
            total_topup_granted, total_subscription_granted,
            total_recharged, total_consumed, status
         )
         VALUES ($1, $2, $3, 0, $4, 0, $4, 0, 0, 'active')",
    )
    .bind(account_uuid)
    .bind(user_uuid)
    .bind(&ctx._realm_id)
    .bind(initial_balance)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create points account");

    query(
        "INSERT INTO points_credit_ledger (
            id, user_id, realm_id, credit_type, source_type, source_id,
            granted_amount, used_amount, revoked_amount, status
         )
         VALUES ($1, $2, $3, 'subscription_credit', 'system_grant', $4, $5, 0, 0, 'active')",
    )
    .bind(Uuid::now_v7())
    .bind(user_uuid)
    .bind(&ctx._realm_id)
    .bind(format!("test-grant-{}", account_uuid))
    .bind(initial_balance)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create credit ledger entry");

    account_uuid
}

/// ============================================================================
/// Points API Scenarios
/// ============================================================================
/// Test scenario: SDK get balance - success
///
/// Verifies that SDK correctly retrieves user points balance
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_sdk_get_balance_success(ctx: &mut SchemaTestContext) {
    // 1. Setup: Create admin session
    let email = "points_admin@example.com";
    let (_session_token, user_id) =
        herald_test_support::helpers::create_admin_session_with_user(ctx, email, 1800).await;
    herald_test_support::helpers::grant_realm_admin_role(ctx, &user_id).await;

    // 2. Start test server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", addr);

    let router = ctx.create_unified_test_router();
    let handle = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    // 3. Create API key
    let (api_key, _api_key_entity) =
        herald_test_support::helpers::create_test_api_key(ctx, "SDK Test Key", true, None).await;

    // 4. Create points account with initial balance
    create_points_account_with_balance(ctx, &user_id, 1000).await;

    // 5. Create SDK client and get balance
    let client = Client::new(base_url, api_key, None);

    let result = client.get_balance(&ctx._realm_id, &user_id).await;

    // 6. Verify
    assert!(
        result.is_ok(),
        "Get balance should succeed: {:?}",
        result.err()
    );
    let balance = result.unwrap();
    assert_eq!(balance.user_id, user_id);
    assert_eq!(balance.balance, 1000);

    // 7. Cleanup
    handle.abort();
}

/// Test scenario: SDK consume points - success
///
/// Verifies that SDK correctly consumes points from user account
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_sdk_consume_points_success(ctx: &mut SchemaTestContext) {
    // 1. Setup: Create admin session
    let email = "points_admin@example.com";
    let (_session_token, user_id) =
        herald_test_support::helpers::create_admin_session_with_user(ctx, email, 1800).await;
    herald_test_support::helpers::grant_realm_admin_role(ctx, &user_id).await;

    // 2. Start test server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", addr);

    let router = ctx.create_unified_test_router();
    let handle = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    // 3. Create API key
    let (api_key, _api_key_entity) =
        herald_test_support::helpers::create_test_api_key(ctx, "SDK Test Key", true, None).await;

    // 4. Create points account with initial balance
    create_points_account_with_balance(ctx, &user_id, 500).await;

    // 5. Create SDK client and consume points
    let client = Client::new(base_url, api_key, None);

    let result = client
        .consume_points(
            &ctx._realm_id,
            &user_id,
            &ctx._client_app_id,
            100,
            Some("Test purchase".to_string()),
            None,
        )
        .await;

    // 6. Verify
    assert!(
        result.is_ok(),
        "Consume points should succeed: {:?}",
        result.err()
    );
    let response = result.unwrap();
    assert_eq!(response.amount, -100); // Negative for consumption
    assert_eq!(response.balance_after, 400);

    // 7. Cleanup
    handle.abort();
}

/// Test scenario: SDK consume points - insufficient balance
///
/// Verifies that SDK correctly handles insufficient balance error
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_sdk_consume_points_insufficient(ctx: &mut SchemaTestContext) {
    // 1. Setup: Create admin session
    let email = "points_admin@example.com";
    let (_session_token, user_id) =
        herald_test_support::helpers::create_admin_session_with_user(ctx, email, 1800).await;
    herald_test_support::helpers::grant_realm_admin_role(ctx, &user_id).await;

    // 2. Start test server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", addr);

    let router = ctx.create_unified_test_router();
    let handle = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    // 3. Create API key
    let (api_key, _api_key_entity) =
        herald_test_support::helpers::create_test_api_key(ctx, "SDK Test Key", true, None).await;

    // 4. Create points account with small balance
    create_points_account_with_balance(ctx, &user_id, 50).await;

    // 5. Create SDK client and try to consume more than available
    let client = Client::new(base_url, api_key, None);

    let result = client
        .consume_points(
            &ctx._realm_id,
            &user_id,
            &ctx._client_app_id,
            100, // Try to consume more than available (only 50)
            Some("Test purchase".to_string()),
            None,
        )
        .await;

    // 6. Verify: Should fail with insufficient points error
    assert!(
        result.is_err(),
        "Consume with insufficient balance should return error"
    );

    // 7. Cleanup
    handle.abort();
}
