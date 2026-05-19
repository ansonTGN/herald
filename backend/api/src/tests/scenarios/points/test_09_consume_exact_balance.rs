// =============================================================================
// Points System Scenario Test 9: Consume Exact Balance (balance -> 0)
// =============================================================================
//
// Boundary tests for consuming points when the amount exactly equals available
// balance. This is a classic off-by-one edge case not covered by test_08
// (partial consumption) or test_60 (mixed balance with remainder).
//
// User Story: docs/user-stories/points-billing-events.md
// Covers: US-PU-01 exact-balance boundary, US-BI-013 zero-balance transitions
//
// =============================================================================

use crate::tests::helpers::points_helpers::*;
use crate::tests::scenarios::points::fixtures::*;
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use herald_core::domain::points::dtos::ConsumePointsInput;
use herald_core::domain::points::entities::{CreditSourceType, CreditType};
use serde_json::json;
use test_context::test_context;
use tower::ServiceExt;
use uuid::Uuid;

// ============================================================================
// Scenario 1: Single ledger exact consumption
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-PU-01 exact-balance boundary - single ledger
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_consume_exact_single_ledger(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    // Given: user with balance 5000 from a single ledger
    let user_id =
        create_test_user(&ctx._app_state.pool, &ctx._realm_id, "exact1@example.com").await;
    let initial_balance = 5000;

    let account_id =
        create_test_points_account(&ctx._app_state.pool, user_id, initial_balance).await;

    let client_app_id = create_test_client_app(&ctx._app_state.pool, &ctx._realm_id).await;
    let api_key = create_test_api_key(&ctx._app_state.pool, &ctx._realm_id, client_app_id).await;

    // When: consume exactly 5000 (the full balance)
    let consume_payload = json!({
        "userId": user_id.to_string(),
        "clientAppId": client_app_id.to_string(),
        "amount": initial_balance,
        "description": "exact consumption"
    });

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/ext/points/{}/consume", ctx._realm_id))
        .header("content-type", "application/json")
        .header("X-API-Key", &api_key)
        .body(Body::from(consume_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();

    // Then: success, balance = 0
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Exact-balance consumption should succeed with 200 OK"
    );

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");
    let body: serde_json::Value =
        serde_json::from_slice(&body_bytes).expect("Failed to parse JSON");

    assert_eq!(
        body["balanceAfter"].as_i64(),
        Some(0),
        "Balance after should be exactly 0, got {:?}",
        body["balanceAfter"]
    );

    // Verify account balance in database is 0
    let (db_balance, db_consumed): (i64, i64) =
        sqlx::query_as("SELECT total_balance, total_consumed FROM points_accounts WHERE id = $1")
            .bind(account_id)
            .fetch_one(&ctx._app_state.pool)
            .await
            .expect("Failed to fetch account");

    assert_eq!(
        db_balance, 0,
        "Account total_balance should be 0 after exact consumption"
    );
    assert_eq!(
        db_consumed, initial_balance,
        "Total consumed should equal initial balance"
    );

    // Verify transaction record exists
    let txn_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM points_transactions WHERE user_id = $1 AND type = 'consume'",
    )
    .bind(user_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .unwrap();

    assert_eq!(txn_count, 1, "Should have exactly 1 consume transaction");

    // Verify ledger remaining is 0
    let ledger_remaining: i64 = sqlx::query_scalar(
        "SELECT remaining_amount FROM points_credit_ledger WHERE user_id = $1 ORDER BY created_at LIMIT 1",
    )
    .bind(user_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .unwrap();

    assert_eq!(
        ledger_remaining, 0,
        "Ledger remaining_amount should be 0 after exact consumption"
    );
}

// ============================================================================
// Scenario 2: Mixed ledger exact consumption
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-BI-013 exact-balance boundary - multiple ledgers fully drained
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_consume_exact_mixed_ledgers(ctx: &mut TestContext) {
    // Given: subscription ledger 3000 + topup ledger 2000 = 5000 total
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx._app_state.pool, &realm_id, "exact2@example.com").await;
    let plan_id = Uuid::now_v7();

    create_points_account(ctx, user_id, &realm_id).await;

    let sub_ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        CreditSourceType::SubscriptionInitial,
        plan_id.to_string(),
        3000,
        None,
    )
    .await;

    let topup_ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::TopupCredit,
        CreditSourceType::Topup,
        Uuid::now_v7().to_string(),
        2000,
        None,
    )
    .await;

    // When: consume exactly 5000 (the full combined balance)
    let identity = create_test_third_party_identity(&realm_id);
    let input = ConsumePointsInput {
        user_id: user_id.to_string(),
        client_app_id: ctx._client_app_id.clone(),
        amount: 5000,
        description: Some("exact mixed consumption".to_string()),
    };
    let result = ctx
        .app_state
        .points_service
        .consume_points(identity, &realm_id, input)
        .await;

    // Then: success, both ledgers fully drained
    assert!(
        result.is_ok(),
        "Exact mixed-balance consumption should succeed"
    );
    let transaction = result.unwrap();
    assert_eq!(transaction.amount, -5000);
    assert_eq!(transaction.balance_after, 0);

    // Subscription ledger should be fully consumed (FIFO: consumed first)
    let sub_ledger = get_ledger_by_id(ctx, sub_ledger_id).await;
    assert_eq!(
        sub_ledger.remaining_amount, 0,
        "Subscription ledger should be fully drained"
    );
    assert_eq!(sub_ledger.used_amount, 3000);

    // Topup ledger should also be fully consumed
    let topup_ledger = get_ledger_by_id(ctx, topup_ledger_id).await;
    assert_eq!(
        topup_ledger.remaining_amount, 0,
        "Topup ledger should be fully drained"
    );
    assert_eq!(topup_ledger.used_amount, 2000);

    // Verify account balance is 0
    let account = get_points_account_by_user(ctx, user_id)
        .await
        .expect("Account should exist");
    let (_, total_balance, _, _) = account;
    assert_eq!(total_balance, 0, "Account total_balance should be 0");

    // Verify 2 allocation records (one per ledger)
    let allocations = get_consumption_allocations(ctx, user_id).await;
    assert_eq!(
        allocations.len(),
        2,
        "Should have 2 allocation records for exact mixed consumption"
    );
}

// ============================================================================
// Scenario 3: Rejection after balance reaches 0
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-PU-01 zero-balance rejection - consume after balance exhausted
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_consume_after_balance_zero_rejected(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    // Given: user with balance 5000 that will be fully consumed
    let user_id =
        create_test_user(&ctx._app_state.pool, &ctx._realm_id, "exact3@example.com").await;
    let initial_balance = 5000;

    create_test_points_account(&ctx._app_state.pool, user_id, initial_balance).await;

    let client_app_id = create_test_client_app(&ctx._app_state.pool, &ctx._realm_id).await;
    let api_key = create_test_api_key(&ctx._app_state.pool, &ctx._realm_id, client_app_id).await;

    // First consumption: drain all 5000
    let drain_payload = json!({
        "userId": user_id.to_string(),
        "clientAppId": client_app_id.to_string(),
        "amount": initial_balance,
        "description": "drain all"
    });

    let drain_request = Request::builder()
        .method("POST")
        .uri(format!("/api/ext/points/{}/consume", ctx._realm_id))
        .header("content-type", "application/json")
        .header("X-API-Key", &api_key)
        .body(Body::from(drain_payload.to_string()))
        .unwrap();

    let drain_response = app.clone().oneshot(drain_request).await.unwrap();
    assert_eq!(
        drain_response.status(),
        StatusCode::OK,
        "Drain consumption should succeed"
    );

    // When: attempt to consume 1 more point
    let retry_payload = json!({
        "userId": user_id.to_string(),
        "clientAppId": client_app_id.to_string(),
        "amount": 1,
        "description": "should fail"
    });

    let retry_request = Request::builder()
        .method("POST")
        .uri(format!("/api/ext/points/{}/consume", ctx._realm_id))
        .header("content-type", "application/json")
        .header("X-API-Key", &api_key)
        .body(Body::from(retry_payload.to_string()))
        .unwrap();

    let retry_response = app.clone().oneshot(retry_request).await.unwrap();

    // Then: rejected with error
    assert_eq!(
        retry_response.status(),
        StatusCode::BAD_REQUEST,
        "Consumption on zero-balance account should be rejected"
    );

    let body_bytes = axum::body::to_bytes(retry_response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");
    let body: serde_json::Value =
        serde_json::from_slice(&body_bytes).expect("Failed to parse JSON");

    let body_str = body.to_string().to_lowercase();
    assert!(
        body_str.contains("insufficient"),
        "Error should indicate insufficient balance, got: {:?}",
        body
    );

    // Verify balance is still 0
    let (db_balance,): (i64,) =
        sqlx::query_as("SELECT total_balance FROM points_accounts WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&ctx._app_state.pool)
            .await
            .unwrap();

    assert_eq!(
        db_balance, 0,
        "Balance should remain 0 after rejected attempt"
    );
}

// ============================================================================
// Scenario 4: Recharge after balance exhaustion
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-PU-01 zero-balance recovery via topup
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_recharge_after_balance_exhausted(ctx: &mut TestContext) {
    // Given: user with balance that will be drained to 0
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx._app_state.pool, &realm_id, "exact4@example.com").await;

    create_points_account(ctx, user_id, &realm_id).await;

    // Initial subscription credit: 5000
    let sub_ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        CreditSourceType::SubscriptionInitial,
        Uuid::now_v7().to_string(),
        5000,
        None,
    )
    .await;

    // Drain all 5000
    let identity = create_test_third_party_identity(&realm_id);
    let drain_input = ConsumePointsInput {
        user_id: user_id.to_string(),
        client_app_id: ctx._client_app_id.clone(),
        amount: 5000,
        description: Some("drain all".to_string()),
    };
    let drain_result = ctx
        .app_state
        .points_service
        .consume_points(identity.clone(), &realm_id, drain_input)
        .await;

    assert!(drain_result.is_ok(), "Drain should succeed");
    assert_eq!(drain_result.unwrap().balance_after, 0);

    // Verify balance is 0
    let account = get_points_account_by_user(ctx, user_id)
        .await
        .expect("Account should exist");
    let (_, total_before_topup, _, _) = account;
    assert_eq!(total_before_topup, 0, "Balance should be 0 after drain");

    // When: recharge 3000 via topup, then consume 1000
    let topup_ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::TopupCredit,
        CreditSourceType::Topup,
        Uuid::now_v7().to_string(),
        3000,
        None,
    )
    .await;

    let consume_input = ConsumePointsInput {
        user_id: user_id.to_string(),
        client_app_id: ctx._client_app_id.clone(),
        amount: 1000,
        description: Some("after recharge".to_string()),
    };
    let consume_result = ctx
        .app_state
        .points_service
        .consume_points(identity, &realm_id, consume_input)
        .await;

    // Then: success, balance = 2000
    assert!(
        consume_result.is_ok(),
        "Consumption after recharge should succeed, got error: {:?}",
        consume_result.err()
    );
    let transaction = consume_result.unwrap();
    assert_eq!(transaction.amount, -1000);
    assert_eq!(transaction.balance_after, 2000);

    // Verify ledger states
    let sub_ledger = get_ledger_by_id(ctx, sub_ledger_id).await;
    assert_eq!(
        sub_ledger.remaining_amount, 0,
        "Subscription ledger should remain drained"
    );

    let topup_ledger = get_ledger_by_id(ctx, topup_ledger_id).await;
    assert_eq!(
        topup_ledger.remaining_amount, 2000,
        "Topup ledger should have 2000 remaining"
    );
    assert_eq!(topup_ledger.used_amount, 1000);

    // Verify final account balance
    let account = get_points_account_by_user(ctx, user_id)
        .await
        .expect("Account should exist");
    let (_, total_final, topup_final, _) = account;
    assert_eq!(total_final, 2000, "Final total balance should be 2000");
    assert_eq!(topup_final, 2000, "Final topup balance should be 2000");
}
