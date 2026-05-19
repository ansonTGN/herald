// =============================================================================
// Test: Consume + Webhook Concurrency Race Conditions
// =============================================================================
//
// Tests for race conditions when points consumption (SDK) and webhook events
// (refund.created, subscription.canceled) arrive concurrently.
//
// User Story: docs/user-stories/points-billing-events.md
// Covers: Concurrency safety for consume + webhook race conditions
//
// Key invariant: balance must never go negative, data must remain consistent
// regardless of which operation wins the race.
//
// =============================================================================

use crate::tests::helpers::points_helpers::*;
use crate::tests::helpers::webhook_helpers::*;
use crate::tests::scenarios::points::fixtures::*;
use crate::tests::schema_test_context::SchemaTestContext;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{Duration, Utc};
use herald_core::domain::points::entities::{CreditSourceType, CreditType};
use serde_json::json;
use sqlx::Row;
use test_context::test_context;
use tower::ServiceExt;
use uuid::Uuid;

async fn assert_account_balance_matches_ledger_remaining(
    ctx: &SchemaTestContext,
    user_id: Uuid,
    realm_id: &str,
    credit_types: &[CreditType],
) {
    let account_column = if credit_types == [CreditType::SubscriptionCredit] {
        "subscription_balance"
    } else {
        "topup_balance"
    };

    let account_balance: i64 = sqlx::query_scalar(&format!(
        "SELECT {} FROM points_accounts WHERE user_id = $1 AND realm_id = $2",
        account_column
    ))
    .bind(user_id)
    .bind(realm_id)
    .fetch_one(&ctx.app_state.pool)
    .await
    .expect("Failed to fetch account balance");

    let credit_type_values = credit_types
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let ledger_remaining: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(remaining_amount), 0)::BIGINT FROM points_credit_ledger
         WHERE user_id = $1 AND realm_id = $2 AND credit_type = ANY($3)",
    )
    .bind(user_id)
    .bind(realm_id)
    .bind(&credit_type_values)
    .fetch_one(&ctx.app_state.pool)
    .await
    .expect("Failed to fetch ledger remaining sum");

    assert_eq!(
        account_balance, ledger_remaining,
        "{} must match remaining ledger sum for {:?}",
        account_column, credit_types
    );
}

// ============================================================================
// Test 1: Consume + Refund Webhook Race
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: Consume + refund.created race condition - balance must not go negative
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_consume_refund_race_balance_not_negative(ctx: &mut SchemaTestContext) {
    // Given: User has topup 10000, already consumed 3000, remaining 7000
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "race_refund@example.com").await;
    let event_id = generate_test_event_id();
    let refund_id = format!("refund_{}", Uuid::now_v7());
    let payment_id = format!("payment_{}", Uuid::now_v7());

    ctx.with_creem_config(&realm_id, None, None, None).await;

    create_points_account(ctx, user_id, &realm_id).await;

    let ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::TopupCredit,
        CreditSourceType::Topup,
        payment_id.clone(),
        10000,
        None,
    )
    .await;

    // Simulate prior consumption of 3000
    consume_points_from_ledger(ctx, ledger_id, 3000).await;

    // Also update account total_consumed to match
    sqlx::query(
        "UPDATE points_accounts SET total_consumed = total_consumed + $1 WHERE user_id = $2 AND realm_id = $3",
    )
    .bind(3000i64)
    .bind(user_id)
    .bind(&realm_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to update total_consumed");

    // Prepare consume request: consume 5000 via SDK API
    let client_app_id = create_test_client_app(&ctx.app_state.pool, &realm_id).await;
    let api_key = create_test_api_key(&ctx.app_state.pool, &realm_id, client_app_id).await;

    let consume_payload = json!({
        "userId": user_id.to_string(),
        "clientAppId": client_app_id.to_string(),
        "amount": 5000,
        "description": "Race consume"
    });

    // Prepare refund webhook: refund 5000 (50% of original 10000)
    let refund_event = build_refund_created_event_with_user(
        event_id, refund_id, payment_id, 5000,  // refund amount
        10000, // original amount
        &realm_id, user_id,
    );

    let app = ctx.create_unified_test_router();

    let consume_request = Request::builder()
        .method("POST")
        .uri(format!("/api/ext/points/{}/consume", realm_id))
        .header("content-type", "application/json")
        .header("X-API-Key", api_key)
        .body(Body::from(consume_payload.to_string()))
        .unwrap();

    // When: Fire consume request and refund webhook concurrently
    let (consume_response, webhook_response) = tokio::join!(
        app.clone().oneshot(consume_request),
        send_webhook_with_signature(&app, &realm_id, refund_event, "test_webhook_secret")
    );

    let consume_response = consume_response.expect("consume response should be returned");
    let consume_status = consume_response.status();

    // Then: Verify data consistency

    // 1. Balance must never go negative
    let account = sqlx::query(
        "SELECT total_balance, topup_balance, subscription_balance FROM points_accounts WHERE user_id = $1 AND realm_id = $2",
    )
    .bind(user_id)
    .bind(&realm_id)
    .fetch_one(&ctx.app_state.pool)
    .await
    .expect("Failed to fetch account");

    let total_balance: i64 = account.get("total_balance");
    let topup_balance: i64 = account.get("topup_balance");
    let subscription_balance: i64 = account.get("subscription_balance");

    assert!(
        total_balance >= 0,
        "total_balance must never go negative, got {}",
        total_balance
    );
    assert!(
        topup_balance >= 0,
        "topup_balance must never go negative, got {}",
        topup_balance
    );
    assert!(
        subscription_balance >= 0,
        "subscription_balance must never go negative, got {}",
        subscription_balance
    );

    // 2. Ledger remaining_amount must never go negative
    let ledger = get_ledger_by_id(ctx, ledger_id).await;
    assert!(
        ledger.remaining_amount >= 0,
        "ledger remaining_amount must never go negative, got {}",
        ledger.remaining_amount
    );

    // 3. Invariant: granted_amount = used_amount + revoked_amount + remaining_amount
    assert_eq!(
        ledger.granted_amount,
        ledger.used_amount + ledger.revoked_amount + ledger.remaining_amount,
        "ledger accounting invariant broken: granted={} != used={} + revoked={} + remaining={}",
        ledger.granted_amount,
        ledger.used_amount,
        ledger.revoked_amount,
        ledger.remaining_amount
    );

    // 4. Account balance must match ledger source of truth
    assert_account_balance_matches_ledger_remaining(
        ctx,
        user_id,
        &realm_id,
        &[
            CreditType::TopupCredit,
            CreditType::RegistrationCredit,
            CreditType::FreePeriodicCredit,
        ],
    )
    .await;

    // 5. Verify webhook response is valid
    assert_webhook_success(&webhook_response);

    // Log the consume status for diagnostics
    if consume_status != StatusCode::OK {
        eprintln!(
            "Consume returned {} (acceptable in race, webhook may have run first)",
            consume_status
        );
    }
}

// ============================================================================
// Test 2: Consume + Subscription Cancel Webhook Race
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: Consume + subscription.canceled race - balance must not go negative
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_consume_cancel_race_balance_not_negative(ctx: &mut SchemaTestContext) {
    // Given: User has subscription credits 5000
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "race_cancel@example.com").await;
    let plan_id = Uuid::now_v7();
    let event_id = generate_test_event_id();
    let period_end = Utc::now() + Duration::days(30);

    ctx.with_creem_config(&realm_id, None, None, None).await;
    setup_test_plan_config(ctx, &realm_id, plan_id).await;

    create_points_account(ctx, user_id, &realm_id).await;

    let ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        CreditSourceType::SubscriptionInitial,
        plan_id.to_string(),
        5000,
        Some(period_end),
    )
    .await;

    // Prepare consume request: consume 3000
    let client_app_id = create_test_client_app(&ctx.app_state.pool, &realm_id).await;
    let api_key = create_test_api_key(&ctx.app_state.pool, &realm_id, client_app_id).await;

    let consume_payload = json!({
        "userId": user_id.to_string(),
        "clientAppId": client_app_id.to_string(),
        "amount": 3000,
        "description": "Race consume with cancel"
    });

    // Prepare cancel webhook: immediate cancel (revoke unused)
    let cancel_event = build_subscription_canceled_event(
        event_id, user_id, false, // immediate cancel
        &realm_id,
    );

    let app = ctx.create_unified_test_router();

    let consume_request = Request::builder()
        .method("POST")
        .uri(format!("/api/ext/points/{}/consume", realm_id))
        .header("content-type", "application/json")
        .header("X-API-Key", api_key)
        .body(Body::from(consume_payload.to_string()))
        .unwrap();

    // When: Fire consume and cancel webhook concurrently
    let (consume_response, webhook_response) = tokio::join!(
        app.clone().oneshot(consume_request),
        send_webhook_with_signature(&app, &realm_id, cancel_event, "test_webhook_secret")
    );

    let consume_response = consume_response.expect("consume response should be returned");
    let consume_status = consume_response.status();

    // Then: Verify data consistency

    // 1. Balance must never go negative
    let account = sqlx::query(
        "SELECT total_balance, topup_balance, subscription_balance FROM points_accounts WHERE user_id = $1 AND realm_id = $2",
    )
    .bind(user_id)
    .bind(&realm_id)
    .fetch_one(&ctx.app_state.pool)
    .await
    .expect("Failed to fetch account");

    let total_balance: i64 = account.get("total_balance");
    let topup_balance: i64 = account.get("topup_balance");
    let subscription_balance: i64 = account.get("subscription_balance");

    assert!(
        total_balance >= 0,
        "total_balance must never go negative, got {}",
        total_balance
    );
    assert!(
        topup_balance >= 0,
        "topup_balance must never go negative, got {}",
        topup_balance
    );
    assert!(
        subscription_balance >= 0,
        "subscription_balance must never go negative, got {}",
        subscription_balance
    );

    // 2. Ledger remaining_amount must never go negative
    let ledger = get_ledger_by_id(ctx, ledger_id).await;
    assert!(
        ledger.remaining_amount >= 0,
        "ledger remaining_amount must never go negative, got {}",
        ledger.remaining_amount
    );

    // 3. Ledger accounting invariant
    assert_eq!(
        ledger.granted_amount,
        ledger.used_amount + ledger.revoked_amount + ledger.remaining_amount,
        "ledger accounting invariant broken: granted={} != used={} + revoked={} + remaining={}",
        ledger.granted_amount,
        ledger.used_amount,
        ledger.revoked_amount,
        ledger.remaining_amount
    );

    // 4. Account balance must match ledger source of truth
    assert_account_balance_matches_ledger_remaining(
        ctx,
        user_id,
        &realm_id,
        &[CreditType::SubscriptionCredit],
    )
    .await;

    // 5. Webhook should succeed
    assert_webhook_success(&webhook_response);

    // 6. If consume succeeded, used_amount should reflect it
    if consume_status == StatusCode::OK {
        assert!(
            ledger.used_amount >= 3000,
            "consume succeeded but used_amount ({}) does not reflect consumption of 3000",
            ledger.used_amount
        );
    }

    if consume_status != StatusCode::OK {
        eprintln!(
            "Consume returned {} (acceptable in race with immediate cancel)",
            consume_status
        );
    }
}

// ============================================================================
// Test 3: Multiple Consumes + Single Refund Race
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: 3 concurrent consumes + 1 refund - total must not exceed available, no negatives
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_multi_consume_refund_race_no_overspending(ctx: &mut SchemaTestContext) {
    // Given: User has topup 10000
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "race_multi@example.com").await;
    let event_id = generate_test_event_id();
    let refund_id = format!("refund_{}", Uuid::now_v7());
    let payment_id = format!("payment_{}", Uuid::now_v7());

    ctx.with_creem_config(&realm_id, None, None, None).await;

    create_points_account(ctx, user_id, &realm_id).await;

    let ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::TopupCredit,
        CreditSourceType::Topup,
        payment_id.clone(),
        10000,
        None,
    )
    .await;

    // Prepare 3 consume requests (each 3000) and 1 refund webhook
    let client_app_id = create_test_client_app(&ctx.app_state.pool, &realm_id).await;
    let api_key = create_test_api_key(&ctx.app_state.pool, &realm_id, client_app_id).await;

    let consume_amount: i64 = 3000;
    let make_consume_payload = || {
        json!({
            "userId": user_id.to_string(),
            "clientAppId": client_app_id.to_string(),
            "amount": consume_amount,
            "description": "Multi race consume"
        })
    };

    let make_consume_request = |payload: serde_json::Value| {
        Request::builder()
            .method("POST")
            .uri(format!("/api/ext/points/{}/consume", realm_id))
            .header("content-type", "application/json")
            .header("X-API-Key", &api_key)
            .body(Body::from(payload.to_string()))
            .unwrap()
    };

    // Refund 5000 (50% of original 10000)
    let refund_event = build_refund_created_event_with_user(
        event_id, refund_id, payment_id, 5000,  // refund amount
        10000, // original amount
        &realm_id, user_id,
    );

    let app = ctx.create_unified_test_router();

    let req1 = make_consume_request(make_consume_payload());
    let req2 = make_consume_request(make_consume_payload());
    let req3 = make_consume_request(make_consume_payload());

    // When: Fire all 4 operations concurrently
    let (res1, res2, res3, webhook_res) = tokio::join!(
        app.clone().oneshot(req1),
        app.clone().oneshot(req2),
        app.clone().oneshot(req3),
        send_webhook_with_signature(&app, &realm_id, refund_event, "test_webhook_secret"),
    );

    let res1 = res1.expect("consume 1 response");
    let res2 = res2.expect("consume 2 response");
    let res3 = res3.expect("consume 3 response");

    let statuses = [res1.status(), res2.status(), res3.status()];
    let success_count = statuses.iter().filter(|&&s| s == StatusCode::OK).count();

    // Then: Verify data consistency

    // 1. Webhook should succeed
    assert_webhook_success(&webhook_res);

    // 2. Balance must never go negative
    let account = sqlx::query(
        "SELECT total_balance, topup_balance, subscription_balance FROM points_accounts WHERE user_id = $1 AND realm_id = $2",
    )
    .bind(user_id)
    .bind(&realm_id)
    .fetch_one(&ctx.app_state.pool)
    .await
    .expect("Failed to fetch account");

    let total_balance: i64 = account.get("total_balance");
    let topup_balance: i64 = account.get("topup_balance");

    assert!(
        total_balance >= 0,
        "total_balance must never go negative, got {}",
        total_balance
    );
    assert!(
        topup_balance >= 0,
        "topup_balance must never go negative, got {}",
        topup_balance
    );

    // 3. Ledger remaining_amount must never go negative
    let ledger = get_ledger_by_id(ctx, ledger_id).await;
    assert!(
        ledger.remaining_amount >= 0,
        "ledger remaining_amount must never go negative, got {}",
        ledger.remaining_amount
    );

    // 4. Ledger accounting invariant
    assert_eq!(
        ledger.granted_amount,
        ledger.used_amount + ledger.revoked_amount + ledger.remaining_amount,
        "ledger accounting invariant broken: granted={} != used={} + revoked={} + remaining={}",
        ledger.granted_amount,
        ledger.used_amount,
        ledger.revoked_amount,
        ledger.remaining_amount
    );

    // 5. Total consumed must not exceed granted amount
    let total_used_plus_remaining =
        ledger.used_amount + ledger.remaining_amount + ledger.revoked_amount;
    assert_eq!(
        total_used_plus_remaining, ledger.granted_amount,
        "total used + remaining + revoked must equal granted"
    );

    // 6. Successful consumes should not exceed available balance
    //    Even with 3 concurrent requests of 3000 each and 10000 granted,
    //    at most floor(10000/3000) = 3 could succeed, but refund reduces availability
    let actual_consumed = ledger.used_amount;
    assert!(
        actual_consumed <= ledger.granted_amount,
        "total consumed ({}) must not exceed granted ({})",
        actual_consumed,
        ledger.granted_amount
    );

    // 7. Account balance must match ledger source of truth
    assert_account_balance_matches_ledger_remaining(
        ctx,
        user_id,
        &realm_id,
        &[
            CreditType::TopupCredit,
            CreditType::RegistrationCredit,
            CreditType::FreePeriodicCredit,
        ],
    )
    .await;

    // 8. Revocation records should exist if webhook processed
    let revocations = get_revocation_records(ctx, user_id).await;
    // The refund webhook should produce exactly one revocation record
    assert!(
        !revocations.is_empty(),
        "expected at least 1 revocation record from refund webhook, found {}",
        revocations.len()
    );

    // 9. Count consume transactions - should match success_count
    let consume_tx_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM points_transactions WHERE user_id = $1 AND type = 'consume'",
    )
    .bind(user_id)
    .fetch_one(&ctx.app_state.pool)
    .await
    .expect("Failed to count consume transactions");

    assert_eq!(
        consume_tx_count, success_count as i64,
        "consume transaction count ({}) should match successful consume responses ({})",
        consume_tx_count, success_count
    );

    eprintln!(
        "Race result: {} of 3 consumes succeeded, balance={}, used={}, revoked={}, remaining={}",
        success_count,
        total_balance,
        ledger.used_amount,
        ledger.revoked_amount,
        ledger.remaining_amount
    );
}
