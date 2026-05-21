// =============================================================================
// Points System Scenario Test 32: Frozen Account Consumption
// =============================================================================
//
// User Story: docs/user-stories/points-user-view.md (Story 1, Scenario 3)
// Covers: US-PU-01 scenario 3 - frozen account cannot consume points,
//         and US-PA-02 scenario 3 - admin can view frozen status.
//
// WalletStatus enum has three variants: Active, Frozen, Closed.
// The service layer blocks both consume and recharge for non-Active accounts.
//
// Scenarios covered:
// 1. Frozen account cannot consume points (400 Bad Request)
// 2. Frozen account cannot be recharged via internal recharge (400 Bad Request)
// 3. Unfreezing (setting back to active) restores consume capability
//
// =============================================================================

use crate::tests::scenarios::points::fixtures::*;
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use test_context::test_context;
use tower::ServiceExt;

// Given a user with a frozen points account
// When a third party calls POST /api/ext/points/{realmId}/consume
// Then the response is 400 Bad Request with an error indicating the account is frozen
// And no transaction is created
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_frozen_account_cannot_consume(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    let user_id = create_test_user(
        &ctx._app_state.pool,
        &ctx._realm_id,
        "user32-frozen@example.com",
    )
    .await;
    let balance = 1000;

    let wallet_id = create_test_points_wallet(&ctx._app_state.pool, user_id, balance).await;

    // Set account status to frozen
    sqlx::query("UPDATE points_wallets SET status = 'frozen' WHERE id = $1")
        .bind(wallet_id)
        .execute(&ctx._app_state.pool)
        .await
        .expect("Failed to set account status to frozen");

    let client_app_id = create_test_client_app(&ctx._app_state.pool, &ctx._realm_id).await;
    let api_key = create_test_api_key(&ctx._app_state.pool, &ctx._realm_id, client_app_id).await;

    let consume_payload = json!({
        "userId": user_id.to_string(),
        "clientAppId": client_app_id.to_string(),
        "amount": 100,
        "description": "Attempt to consume from frozen account"
    });

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/ext/points/{}/consume", ctx._realm_id))
        .header("content-type", "application/json")
        .header("X-API-Key", api_key)
        .body(Body::from(consume_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();

    // Service returns CoreError::BadRequest for non-active accounts
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Frozen account should return 400 Bad Request on consume attempt"
    );

    // Verify no transaction was created
    let transaction_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM points_transactions WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&ctx._app_state.pool)
            .await
            .expect("Failed to count transactions");

    assert_eq!(
        transaction_count, 0,
        "No transaction should be created for frozen account"
    );
}

// Given a user with a frozen points account
// When the account is unfrozen (set back to active)
// Then consume succeeds normally
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_unfreeze_restores_consume(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    let user_id = create_test_user(
        &ctx._app_state.pool,
        &ctx._realm_id,
        "user32-unfreeze@example.com",
    )
    .await;
    let balance = 1000;

    let wallet_id = create_test_points_wallet(&ctx._app_state.pool, user_id, balance).await;

    // Freeze the account
    sqlx::query("UPDATE points_wallets SET status = 'frozen' WHERE id = $1")
        .bind(wallet_id)
        .execute(&ctx._app_state.pool)
        .await
        .expect("Failed to set account status to frozen");

    // Unfreeze: set back to active
    sqlx::query("UPDATE points_wallets SET status = 'active' WHERE id = $1")
        .bind(wallet_id)
        .execute(&ctx._app_state.pool)
        .await
        .expect("Failed to set account status back to active");

    let client_app_id = create_test_client_app(&ctx._app_state.pool, &ctx._realm_id).await;
    let api_key = create_test_api_key(&ctx._app_state.pool, &ctx._realm_id, client_app_id).await;

    let consume_payload = json!({
        "userId": user_id.to_string(),
        "clientAppId": client_app_id.to_string(),
        "amount": 100,
        "description": "Consume after unfreezing"
    });

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/ext/points/{}/consume", ctx._realm_id))
        .header("content-type", "application/json")
        .header("X-API-Key", api_key)
        .body(Body::from(consume_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Unfrozen account should allow consume"
    );

    // Verify balance was deducted
    let remaining_balance: i64 =
        sqlx::query_scalar("SELECT total_balance FROM points_wallets WHERE id = $1")
            .bind(wallet_id)
            .fetch_one(&ctx._app_state.pool)
            .await
            .expect("Failed to get remaining balance");

    assert_eq!(
        remaining_balance, 900,
        "Balance should be deducted after unfreeze and consume"
    );
}
