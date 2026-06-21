// =============================================================================
// SDK Points Scenarios
// =============================================================================
//
// Test SDK points API against real API
// =============================================================================

use herald_sdk::Client;
use herald_test_support::SchemaTestContext;
use sqlx::{query, query_scalar};
use test_context::test_context;
use uuid::Uuid;

/// Ensure a single legacy `credit_buckets` row exists for the realm and attach
/// every client app in the realm to it.
///
/// The credit-bucket migration made `points_wallets.bucket_id` and
/// `points_credit_ledger.bucket_id` NOT NULL, and the consume path resolves
/// pool coverage from `credit_bucket_client_apps` (no default-bucket merging).
/// This mirrors `api::tests::helpers::points_helpers::ensure_test_bucket_for_realm`
/// so this crate's raw-SQL wallet fixtures satisfy the same constraints.
async fn ensure_legacy_bucket_for_realm(pool: &sqlx::PgPool, realm_id: &str) -> Uuid {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    use std::hash::Hasher;
    hasher.write(realm_id.as_bytes());
    let slug = format!("legacy-{:016x}", hasher.finish());

    query(
        r#"INSERT INTO credit_buckets
             (id, realm_id, bucket_key, name, display_order, enabled,
              receives_registration_credits, created_at, updated_at)
           VALUES ($1, $2, $3, 'Legacy Test Bucket', 0, true, true, NOW(), NOW())
           ON CONFLICT (realm_id, bucket_key) DO NOTHING"#,
    )
    .bind(Uuid::now_v7())
    .bind(realm_id)
    .bind(&slug)
    .execute(pool)
    .await
    .expect("Failed to ensure legacy credit bucket");

    let bucket_id: Uuid =
        query_scalar("SELECT id FROM credit_buckets WHERE realm_id = $1 AND bucket_key = $2")
            .bind(realm_id)
            .bind(&slug)
            .fetch_one(pool)
            .await
            .expect("Failed to fetch legacy credit bucket");

    query(
        r#"INSERT INTO credit_bucket_client_apps
             (bucket_id, client_app_id, realm_id, created_at)
           SELECT $1, id, $2, NOW()
           FROM client_app
           WHERE realm_id = $2
           ON CONFLICT (bucket_id, client_app_id) DO NOTHING"#,
    )
    .bind(bucket_id)
    .bind(realm_id)
    .execute(pool)
    .await
    .expect("Failed to attach client apps to legacy credit bucket");

    bucket_id
}

/// Helper function to create a points account with initial balance
async fn create_points_wallet_with_balance(
    ctx: &mut SchemaTestContext,
    user_id: &str,
    initial_balance: i64,
) -> Uuid {
    let account_uuid = Uuid::now_v7();
    let user_uuid = Uuid::parse_str(user_id).expect("Invalid user_id UUID format");
    let bucket_id = ensure_legacy_bucket_for_realm(&ctx.app_state.pool, &ctx._realm_id).await;

    query(
        "INSERT INTO points_wallets (
            id, user_id, realm_id, bucket_id,
            topup_balance, subscription_balance,
            total_topup_granted, total_subscription_granted,
            total_recharged, total_consumed, status
         )
         VALUES ($1, $2, $3, $4, 0, $5, 0, $5, 0, 0, 'active')",
    )
    .bind(account_uuid)
    .bind(user_uuid)
    .bind(&ctx._realm_id)
    .bind(bucket_id)
    .bind(initial_balance)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create points account");

    query(
        "INSERT INTO points_credit_ledger (
            id, user_id, realm_id, bucket_id, credit_type, source_type, source_id,
            granted_amount, used_amount, revoked_amount, status
         )
         VALUES ($1, $2, $3, $4, 'subscription_credit', 'system_grant', $5, $6, 0, 0, 'active')",
    )
    .bind(Uuid::now_v7())
    .bind(user_uuid)
    .bind(&ctx._realm_id)
    .bind(bucket_id)
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
    let (api_key, api_key_entity) =
        herald_test_support::helpers::create_test_api_key(ctx, "SDK Test Key", true, None).await;
    herald_test_support::helpers::grant_api_key_permissions(
        ctx,
        &api_key_entity.id,
        &[("points", "view"), ("points", "manage")],
    )
    .await;

    // 4. Create points account with initial balance
    create_points_wallet_with_balance(ctx, &user_id, 1000).await;

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
    let (api_key, api_key_entity) =
        herald_test_support::helpers::create_test_api_key(ctx, "SDK Test Key", true, None).await;
    herald_test_support::helpers::grant_api_key_permissions(
        ctx,
        &api_key_entity.id,
        &[("points", "view"), ("points", "manage")],
    )
    .await;

    // 4. Create points account with initial balance
    create_points_wallet_with_balance(ctx, &user_id, 500).await;

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
    // Consume response `amount` is the deduction magnitude (positive) — see the
    // SDK `ConsumePointsResponse` / `BucketTransaction.amount` docs.
    assert_eq!(response.amount, 100);
    assert_eq!(response.transactions[0].balance_after, 400);

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
    let (api_key, api_key_entity) =
        herald_test_support::helpers::create_test_api_key(ctx, "SDK Test Key", true, None).await;
    herald_test_support::helpers::grant_api_key_permissions(
        ctx,
        &api_key_entity.id,
        &[("points", "view"), ("points", "manage")],
    )
    .await;

    // 4. Create points account with small balance
    create_points_wallet_with_balance(ctx, &user_id, 50).await;

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
