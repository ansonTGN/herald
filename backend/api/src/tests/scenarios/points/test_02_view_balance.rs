// =============================================================================
// Points System Scenario Test 2: View Balance
// =============================================================================
//
// **User Story**: US-PU-01 (View My Points Balance)
// **Priority**: P0
//
// **Scenario**: User Views Their Own Balance
//
// **Given**:
// - A user with authentication
// - An existing points account with balance 5000
// - Total recharged: 10000
// - Total consumed: 5000
//
// **When**:
// - The user calls `GET /api/{realmId}/points/balance`
//
// **Then**:
// - The response returns balance: 5000
// - The response returns total_recharged: 10000
// - The response returns total_consumed: 5000
// - The response returns currency: "points"
// - HTTP status is 200 OK
//
// =============================================================================

use crate::tests::helpers::points_helpers::{
    assert_derived_balance, count_future_effective_active_rows,
    create_credit_ledger_entry_with_effective_at,
};
use crate::tests::helpers::test_setup_helpers::record_test_user_consent;
use crate::tests::scenarios::points::fixtures::*;
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{Duration, Utc};
use serde_json::json;
use test_context::test_context;
use tower::ServiceExt;

/// ============================================================================
/// Scenario 1.2: User Views Their Own Balance
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_user_view_own_balance(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    // ============================================================================
    // Given: A user with authentication and points account
    // ============================================================================
    println!("[Step 1] Create test user and points account");

    let email = "user2@example.com";
    let password = "password123";
    let user_id =
        create_test_user_with_auth(&ctx._app_state.pool, &ctx._realm_id, email, password).await;
    record_test_user_consent(&ctx._app_state.pool, user_id, &ctx._realm_id).await;

    let balance = 5000;
    let total_recharged = 10000;
    let total_consumed = 5000;

    let wallet_id = create_test_points_wallet(&ctx._app_state.pool, user_id, balance).await;

    // Update total_recharged and total_consumed
    sqlx::query(
        "UPDATE points_wallets SET total_recharged = $1, total_consumed = $2 WHERE id = $3",
    )
    .bind(total_recharged)
    .bind(total_consumed)
    .bind(wallet_id)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to update account totals");

    println!(
        "[Step 1] ✓ Test data created: user={}, account={}, balance={}, recharged={}, consumed={}",
        user_id, wallet_id, balance, total_recharged, total_consumed
    );

    // ============================================================================
    // When: The user logs in and views their balance
    // ============================================================================
    println!("[Step 2] User logs in");

    // Login to get session token
    let login_payload = json!({
        "clientId": ctx._client_id,
        "email": email,
        "password": password,
        "turnstileToken": "dummy"
    });

    let login_request = Request::builder()
        .method("POST")
        .uri(format!("/api/auth/{}/login", ctx._realm_id))
        .header("content-type", "application/json")
        .header("x-forwarded-for", "3.3.3.3")
        .body(Body::from(login_payload.to_string()))
        .unwrap();

    let login_response = app.clone().oneshot(login_request).await.unwrap();
    assert_eq!(login_response.status(), StatusCode::OK);

    // Extract session token
    let (_response, token) = crate::tests::extract_bearer_token(login_response).await;
    let token = token.expect("Login should return accessToken");

    println!("[Step 2] ✓ User logged in: token={}", token);

    println!("[Step 3] User requests balance");

    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/points/{}/wallets/{}", ctx._realm_id, user_id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();

    // ============================================================================
    // Then: Verify balance response
    // ============================================================================
    println!("[Step 4] Verify balance response");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Get balance should return 200 OK"
    );

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");
    let body: serde_json::Value =
        serde_json::from_slice(&body_bytes).expect("Failed to parse JSON");

    assert_eq!(
        body["balance"].as_i64(),
        Some(balance),
        "Balance should be 5000"
    );
    assert_eq!(
        body["totalRecharged"].as_i64(),
        Some(total_recharged),
        "Total recharged should be 10000"
    );
    assert_eq!(
        body["totalConsumed"].as_i64(),
        Some(total_consumed),
        "Total consumed should be 5000"
    );
    assert_eq!(
        body["currency"].as_str(),
        Some("points"),
        "Currency should be 'points'"
    );
    assert_eq!(
        body["userId"].as_str(),
        Some(user_id.to_string().as_str()),
        "User ID should match"
    );

    println!(
        "[Step 4] ✓ Balance verified: balance={}, recharged={}, consumed={}",
        balance, total_recharged, total_consumed
    );

    println!("\n✅ Scenario 1.2 完成：用户成功查看自己的积分余额");
}

/// ============================================================================
/// Scenario 1.3: GET wallet returns a zero-balance user-total view (no row)
/// ============================================================================
///
/// Credit Buckets model: a wallet is per-(user, bucket).
/// A bare GET for a user with no wallet returns a synthesized zero-balance
/// user-total view and does NOT persist a `bucket_id = NULL` row (the column is
/// NOT NULL). Wallet rows are created lazily only when a grant/consume targets
/// a specific bucket.
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_get_wallet_auto_creates_empty_wallet(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    // Given: A user exists but has no points wallet yet.
    let email = "user2-auto-create@example.com";
    let password = "password123";
    let user_id =
        create_test_user_with_auth(&ctx._app_state.pool, &ctx._realm_id, email, password).await;
    record_test_user_consent(&ctx._app_state.pool, user_id, &ctx._realm_id).await;

    let wallet_count_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM points_wallets WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&ctx._app_state.pool)
            .await
            .expect("Failed to count wallets before request");
    assert_eq!(
        wallet_count_before, 0,
        "Precondition matters: this scenario verifies GET returns a usable view when no wallet exists"
    );

    // When: The user logs in and requests their own wallet.
    let login_payload = json!({
        "clientId": ctx._client_id,
        "email": email,
        "password": password,
        "turnstileToken": "dummy"
    });

    let login_request = Request::builder()
        .method("POST")
        .uri(format!("/api/auth/{}/login", ctx._realm_id))
        .header("content-type", "application/json")
        .header("x-forwarded-for", "3.3.3.4")
        .body(Body::from(login_payload.to_string()))
        .unwrap();

    let login_response = app.clone().oneshot(login_request).await.unwrap();
    assert_eq!(login_response.status(), StatusCode::OK);

    let (_response, token) = crate::tests::extract_bearer_token(login_response).await;
    let token = token.expect("Login should return accessToken");

    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/points/{}/wallets/{}", ctx._realm_id, user_id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();

    // Then: The response is a zero-balance active view (no row persisted).
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Get wallet should return 200 OK with a zero-balance view"
    );

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");
    let body: serde_json::Value =
        serde_json::from_slice(&body_bytes).expect("Failed to parse JSON");

    assert_eq!(body["balance"].as_i64(), Some(0));
    assert_eq!(body["status"].as_str(), Some("active"));
    assert_eq!(body["totalRecharged"].as_i64(), Some(0));
    assert_eq!(body["totalConsumed"].as_i64(), Some(0));
    assert_eq!(body["userId"].as_str(), Some(user_id.to_string().as_str()));

    // No wallet row is persisted by a bare GET — the bucket-wallet is created
    // lazily only when a grant/consume targets a specific bucket.
    let wallet_count_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM points_wallets WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&ctx._app_state.pool)
            .await
            .expect("Failed to count wallets after request");
    assert_eq!(
        wallet_count_after, 0,
        "GET must NOT persist a bucket-less wallet row; bucket-wallets are created lazily on grant/consume"
    );
}

/// ============================================================================
/// Scenario 1.4: user PointsWalletResponse does NOT leak
/// future-effective rows
/// ============================================================================
///
/// User Story: US-PU-001 / US-PU-004 / US-PU-005 (future-period credits must
/// not be visible to regular users before their effective time).
///
/// Covers design `.ai/design/point-time.md` P1 "响应不泄漏未来期积分" +
/// risk "wallet Stored 列读点遗漏：get_balance 之外的 list_wallets ... 若
/// 继续读 points_wallets.total_balance 会泄漏未来期积分" (P1).
///
/// Why this test exists: the GET `/wallets/{userId}` response assembles
/// `balance`/typed balances from the DERIVED SUM (`compute_available_balance`,
/// derived predicate), whose predicate includes
/// `(effective_at IS NULL OR effective_at <= NOW())`. A future-effective
/// pre-grant row must therefore NOT show up in the regular-user balance
/// response — otherwise the invariant "balance you see == balance you can
/// spend" breaks and a future period silently leaks into the user-visible
/// balance. The test also confirms the same row IS present in the ledger
/// (so the non-leak is a predicate effect, not "row missing").
#[test_context(TestContext)]
#[tokio::test]
async fn test_user_balance_excludes_future_effective(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();
    let realm_id = ctx._realm_id.clone();

    // Given: A user with two subscription_credit ledger rows on the same
    // (user, realm, bucket): one immediately available (effective_at=NULL,
    // amount A), one future-effective (effective_at=now+1d, amount B).
    let email = "user-be-t09-noleak@example.com";
    let password = "password123";
    let user_id =
        create_test_user_with_auth(&ctx._app_state.pool, &realm_id, email, password).await;
    record_test_user_consent(&ctx._app_state.pool, user_id, &realm_id).await;

    let amount_immediate = 2_000;
    let amount_future = 3_000;
    let future_effective_at = Utc::now() + Duration::days(1);

    let _imm_ledger = create_credit_ledger_entry_with_effective_at(
        ctx,
        user_id,
        &realm_id,
        herald_core::domain::points::entities::CreditType::SubscriptionCredit,
        herald_core::domain::points::entities::CreditSourceType::SubscriptionInitial,
        format!("be-t09-noleak-imm-{}", uuid::Uuid::now_v7()),
        amount_immediate,
        None,
        None, // effective_at=NULL ⟺ immediately available
    )
    .await;
    let _fut_ledger = create_credit_ledger_entry_with_effective_at(
        ctx,
        user_id,
        &realm_id,
        herald_core::domain::points::entities::CreditType::SubscriptionCredit,
        herald_core::domain::points::entities::CreditSourceType::SubscriptionInitial,
        format!("be-t09-noleak-fut-{}", uuid::Uuid::now_v7()),
        amount_future,
        None,
        Some(future_effective_at), // future ⟺ excluded from derived SUM
    )
    .await;

    // Cross-check (a): the derived balance predicate excludes B; the row IS
    // present in the ledger (count=1 future-effective active row).
    assert_derived_balance(
        ctx,
        user_id,
        &realm_id,
        herald_core::domain::points::entities::CreditType::SubscriptionCredit,
        amount_immediate,
    )
    .await;
    assert_eq!(
        count_future_effective_active_rows(ctx, user_id, &realm_id).await,
        1,
        "future-effective row is present in the ledger (the non-leak is a predicate effect, not a missing row)"
    );

    // When: the user logs in and requests their wallet.
    let login_payload = json!({
        "clientId": ctx._client_id,
        "email": email,
        "password": password,
        "turnstileToken": "dummy"
    });
    let login_request = Request::builder()
        .method("POST")
        .uri(format!("/api/auth/{}/login", realm_id))
        .header("content-type", "application/json")
        .header("x-forwarded-for", "3.3.3.9")
        .body(Body::from(login_payload.to_string()))
        .unwrap();
    let login_response = app.clone().oneshot(login_request).await.unwrap();
    assert_eq!(login_response.status(), StatusCode::OK);
    let (_response, token) = crate::tests::extract_bearer_token(login_response).await;
    let token = token.expect("Login should return accessToken");

    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/points/{}/wallets/{}", realm_id, user_id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();

    // Then: the response balance == A (immediate only), NOT A+B — the future
    // row does not leak into the user-visible PointsWalletResponse.balance.
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Get wallet should return 200 OK"
    );
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");
    let body: serde_json::Value =
        serde_json::from_slice(&body_bytes).expect("Failed to parse JSON");

    assert_eq!(
        body["balance"].as_i64(),
        Some(amount_immediate),
        "PointsWalletResponse.balance must be the derived SUM excluding future-effective rows; \
         expected {} (immediate only), got {:?}",
        amount_immediate,
        body["balance"].as_i64()
    );

    println!(
        "\n✅ Scenario 1.4 完成：普通用户 PointsWalletResponse 不含 future-effective（balance={}，未泄漏未来期 {}）",
        amount_immediate, amount_future
    );
}
