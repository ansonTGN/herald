// =============================================================================
// Points Package Purchase Scenario Tests
// =============================================================================
//
// Tests for points package purchase flow and fulfillment:
// - Payment attempt creation
// - Payment success processing
// - Points granting with correct credit type (topup_credit)
// - Fulfillment idempotency (CRITICAL - prevents double-grants)
// - Purchase history tracking
//
// User Story: docs/user-stories/10-points-package-user-stories.md
// User Story: docs/user-stories/11-points-package-purchase-user-stories.md
//
// =============================================================================

use crate::tests::helpers::points_helpers::*;
use crate::tests::helpers::points_package_helpers::*;
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::Json;
use axum::extract::{Path, State};
use serde_json::json;
use uuid::Uuid;

#[cfg(test)]
mod tests {

    use super::*;
    use test_context::test_context;

    /// ============================================================================
    /// P0-PUR-01: Idempotent fulfillment prevents double-grants (CRITICAL)
    /// ============================================================================
    /// User Story: docs/user-stories/11-points-package-purchase-user-stories.md
    /// Covers: Idempotency requirement
    ///
    /// **CRITICAL**: This test prevents duplicate points grants from webhook retries
    ///
    /// Scenario: Duplicate webhook callbacks should not double-grant points
    /// Given: A points package grants 1000 topup_credit points
    /// When: Payment webhook is called twice (simulating retry)
    /// Then: User receives exactly 1000 topup_credit points (not 2000)

    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_idempotent_fulfillment_prevents_double_grants(ctx: &mut TestContext) {
        let realm_id = ctx._realm_id.clone();
        let user_id = Uuid::now_v7();

        // Given: Create a points package
        let package_id = create_points_package(
            ctx,
            &realm_id,
            "test_package_1000",
            "1000 Points Package",
            1000, // points
            999,  // price in cents
            "USD",
            true, // enabled
        )
        .await;

        // And: Create a user with a points account
        create_points_account(ctx, user_id, &realm_id).await;

        // When: Create payment attempt
        let payment_attempt_id = create_payment_attempt(
            ctx,
            &realm_id,
            user_id,
            "points_package",
            package_id,
            "wechat",
            999, // amount
            "USD",
        )
        .await;

        let provider_transaction_id = format!("wx_{}", payment_attempt_id);

        // First fulfillment call
        let payload1 = json!({
            "providerStatus": "success",
            "providerTransactionId": provider_transaction_id,
            "completedAt": chrono::Utc::now().to_rfc3339(),
        });

        let response1 = crate::application::http::billing::purchase_handlers::fulfill_payment(
            State((*ctx.app_state).clone()),
            Path(payment_attempt_id),
            Json(serde_json::from_value(payload1).unwrap()),
        )
        .await;

        // Then: First fulfillment succeeds
        assert!(response1.is_ok(), "First fulfillment should succeed");

        // And: User has 1000 topup_credit points
        let account = get_points_account_by_user(ctx, user_id).await;
        assert!(account.is_some());
        let (_account_id, _total_balance, topup_balance, subscription_balance) = account.unwrap();
        assert_eq!(
            topup_balance, 1000,
            "User should have 1000 topup_credit after first webhook"
        );
        assert_eq!(
            subscription_balance, 0,
            "User should have 0 subscription_credit"
        );

        // When: Second fulfillment call with same provider_transaction_id (simulating retry)
        let payload2 = json!({
            "providerStatus": "success",
            "providerTransactionId": provider_transaction_id,
            "completedAt": chrono::Utc::now().to_rfc3339(),
        });

        let response2 = crate::application::http::billing::purchase_handlers::fulfill_payment(
            State((*ctx.app_state).clone()),
            Path(payment_attempt_id), // Same attempt ID
            Json(serde_json::from_value(payload2).unwrap()),
        )
        .await;

        // Then: Second fulfillment also succeeds (idempotent)
        assert!(
            response2.is_ok(),
            "Second fulfillment should succeed (idempotent)"
        );

        // And: User still has exactly 1000 topup_credit points (NOT 2000)
        let account = get_points_account_by_user(ctx, user_id).await;
        assert!(account.is_some());
        let (_account_id, _total_balance, topup_balance, subscription_balance) = account.unwrap();
        assert_eq!(
            topup_balance, 1000,
            "User should still have 1000 topup_credit after second webhook (idempotency)"
        );
        assert_eq!(
            subscription_balance, 0,
            "User should still have 0 subscription_credit"
        );

        // And: Only ONE purchase record exists
        let purchase_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM points_package_purchases WHERE user_id = $1 AND points_package_id = $2"
        )
        .bind(user_id)
        .bind(package_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap();
        assert_eq!(purchase_count, 1, "Should have exactly 1 purchase record");
    }

    /// ============================================================================
    /// P0-PUR-02: Points package grants topup_credit only
    /// ============================================================================
    /// User Story: docs/user-stories/10-points-package-user-stories.md
    /// Covers: Credit type separation requirement
    ///
    /// **CRITICAL**: This test verifies points packages grant topup_credit, not subscription_credit
    ///
    /// Scenario: Points package purchase should only grant topup_credit
    /// Given: A points package grants 1000 points
    /// When: Payment webhook is processed
    /// Then: User receives 1000 topup_credit points (NOT subscription_credit)

    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_points_package_grants_topup_credit_only(ctx: &mut TestContext) {
        let realm_id = ctx._realm_id.clone();
        let user_id = Uuid::now_v7();

        // Given: Create a points package
        let package_id = create_points_package(
            ctx,
            &realm_id,
            "test_package_500",
            "500 Points Package",
            500, // points
            499, // price in cents
            "USD",
            true, // enabled
        )
        .await;

        // And: Create a user with a points account
        create_points_account(ctx, user_id, &realm_id).await;

        // When: Create payment attempt and fulfill it
        let payment_attempt_id = create_payment_attempt(
            ctx,
            &realm_id,
            user_id,
            "points_package",
            package_id,
            "wechat",
            499, // amount
            "USD",
        )
        .await;

        let provider_transaction_id = format!("wx_{}", payment_attempt_id);

        let payload = json!({
            "providerStatus": "success",
            "providerTransactionId": provider_transaction_id,
            "completedAt": chrono::Utc::now().to_rfc3339(),
        });

        let response = crate::application::http::billing::purchase_handlers::fulfill_payment(
            State((*ctx.app_state).clone()),
            Path(payment_attempt_id),
            Json(serde_json::from_value(payload).unwrap()),
        )
        .await;

        // Then: Fulfillment succeeds
        assert!(response.is_ok(), "Fulfillment should succeed");

        // DEBUG: Check balance right after webhook
        let account_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM points_accounts WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(&ctx.app_state.pool)
                .await
                .unwrap();
        println!("DEBUG: Number of accounts for user: {}", account_count);

        let balances: Vec<(Uuid, i64)> = sqlx::query_as(
            "SELECT id, topup_balance FROM points_accounts WHERE user_id = $1 ORDER BY created_at",
        )
        .bind(user_id)
        .fetch_all(&ctx.app_state.pool)
        .await
        .unwrap();
        println!("DEBUG: All balances for user: {:?}", balances);

        // And: User has exactly 500 topup_credit points
        let account = get_points_account_by_user(ctx, user_id).await;
        assert!(account.is_some());
        let (_account_id, _total_balance, topup_balance, subscription_balance) = account.unwrap();
        assert_eq!(topup_balance, 500, "User should have 500 topup_credit");
        assert_eq!(
            subscription_balance, 0,
            "User should have 0 subscription_credit"
        );

        // And: Credit ledger shows topup_credit entry
        let ledgers = get_user_ledgers_by_credit_type(
            ctx,
            user_id,
            herald_core::domain::points::entities::CreditType::TopupCredit,
        )
        .await;
        assert_eq!(ledgers.len(), 1, "Should have 1 topup_credit ledger entry");
        assert_eq!(
            ledgers[0].granted_amount, 500,
            "Ledger should show 500 topup_credit granted"
        );

        // And: No subscription_credit ledger entries exist
        let subscription_ledgers = get_user_ledgers_by_credit_type(
            ctx,
            user_id,
            herald_core::domain::points::entities::CreditType::SubscriptionCredit,
        )
        .await;
        assert_eq!(
            subscription_ledgers.len(),
            0,
            "Should have 0 subscription_credit ledger entries"
        );
    }

    /// ============================================================================
    /// P1-PUR-03: Complete purchase flow - from creation to fulfillment
    /// ============================================================================
    /// User Story: docs/user-stories/11-points-package-purchase-user-stories.md
    /// Covers: Complete purchase flow
    ///
    /// Scenario: Complete purchase flow from payment attempt to points granting
    /// Given: A user wants to buy a points package
    /// When: They complete the payment flow
    /// Then: Points are granted to their account with correct credit type

    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_points_package_purchase_complete_flow(ctx: &mut TestContext) {
        let realm_id = ctx._realm_id.clone();
        let user_id = Uuid::now_v7();

        // Given: Create a points package
        let package_id = create_points_package(
            ctx,
            &realm_id,
            "complete_flow_package",
            "Complete Flow Package",
            2000, // points
            1999, // price in cents
            "USD",
            true, // enabled
        )
        .await;

        // And: Create a user with a points account
        let account_id = create_points_account(ctx, user_id, &realm_id).await;

        // When: Create payment attempt
        let payment_attempt_id = create_payment_attempt(
            ctx,
            &realm_id,
            user_id,
            "points_package",
            package_id,
            "wechat",
            1999, // amount
            "USD",
        )
        .await;

        // And: Simulate successful payment webhook
        let provider_transaction_id = format!("wx_complete_{}", payment_attempt_id);

        let payload = json!({
            "providerStatus": "success",
            "providerTransactionId": provider_transaction_id,
            "completedAt": chrono::Utc::now().to_rfc3339(),
        });

        let response = crate::application::http::billing::purchase_handlers::fulfill_payment(
            State((*ctx.app_state).clone()),
            Path(payment_attempt_id),
            Json(serde_json::from_value(payload).unwrap()),
        )
        .await;

        // Then: Payment fulfillment succeeds
        assert!(response.is_ok(), "Fulfillment should succeed");

        // And: User has 2000 topup_credit points
        assert_points_balance(ctx, account_id, 2000, 2000, 0).await;

        // And: Purchase record exists
        let purchase_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM points_package_purchases WHERE user_id = $1 AND points_package_id = $2"
        )
        .bind(user_id)
        .bind(package_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap();
        assert_eq!(purchase_count, 1, "Should have 1 purchase record");

        // And: Payment attempt status is succeeded
        assert_payment_attempt_exists(ctx, payment_attempt_id, "Succeeded").await;
    }

    /// ============================================================================
    /// P1-PUR-04: Purchase history tracking
    /// ============================================================================
    /// User Story: docs/user-stories/11-points-package-purchase-user-stories.md
    /// Covers: Purchase history requirement
    ///
    /// Scenario: User can view their purchase history
    /// Given: A user has purchased multiple points packages
    /// When: They query their purchase history
    /// Then: All purchases are returned in chronological order

    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_purchase_history_tracking(ctx: &mut TestContext) {
        let realm_id = ctx._realm_id.clone();
        let user_id = Uuid::now_v7();

        // Given: Create user and account
        create_points_account(ctx, user_id, &realm_id).await;

        // And: Create two points packages
        let package1_id = create_points_package(
            ctx,
            &realm_id,
            "history_package_1",
            "History Package 1",
            1000,
            999,
            "USD",
            true,
        )
        .await;

        let package2_id = create_points_package(
            ctx,
            &realm_id,
            "history_package_2",
            "History Package 2",
            2000,
            1999,
            "USD",
            true,
        )
        .await;

        // When: User purchases both packages
        let attempt1_id = create_payment_attempt(
            ctx,
            &realm_id,
            user_id,
            "points_package",
            package1_id,
            "wechat",
            999,
            "USD",
        )
        .await;

        let tx_id1 = format!("wx_history_1_{}", attempt1_id);
        let payload1 = json!({
            "providerStatus": "success",
            "providerTransactionId": tx_id1,
            "completedAt": chrono::Utc::now().to_rfc3339(),
        });

        let response1 = crate::application::http::billing::purchase_handlers::fulfill_payment(
            State((*ctx.app_state).clone()),
            Path(attempt1_id),
            Json(serde_json::from_value(payload1).unwrap()),
        )
        .await;
        assert!(response1.is_ok(), "First fulfillment should succeed");

        // Small delay to ensure different timestamps
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let attempt2_id = create_payment_attempt(
            ctx,
            &realm_id,
            user_id,
            "points_package",
            package2_id,
            "wechat",
            1999,
            "USD",
        )
        .await;

        let tx_id2 = format!("wx_history_2_{}", attempt2_id);
        let payload2 = json!({
            "providerStatus": "success",
            "providerTransactionId": tx_id2,
            "completedAt": chrono::Utc::now().to_rfc3339(),
        });

        let response2 = crate::application::http::billing::purchase_handlers::fulfill_payment(
            State((*ctx.app_state).clone()),
            Path(attempt2_id),
            Json(serde_json::from_value(payload2).unwrap()),
        )
        .await;
        assert!(response2.is_ok(), "Second fulfillment should succeed");

        // Then: Purchase history shows both purchases
        let purchases: Vec<(Uuid, Uuid, i64, i64)> = sqlx::query_as(
            "SELECT id, points_package_id, points, amount FROM points_package_purchases
             WHERE user_id = $1 ORDER BY created_at ASC",
        )
        .bind(user_id)
        .fetch_all(&ctx.app_state.pool)
        .await
        .unwrap();

        assert_eq!(purchases.len(), 2, "Should have 2 purchases");
        assert_eq!(
            purchases[0].1, package1_id,
            "First purchase should be package1"
        );
        assert_eq!(purchases[0].2, 1000, "First purchase should be 1000 points");
        assert_eq!(
            purchases[1].1, package2_id,
            "Second purchase should be package2"
        );
        assert_eq!(
            purchases[1].2, 2000,
            "Second purchase should be 2000 points"
        );
    }

    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_internal_fulfill_webhook_persists_purchase_and_is_idempotent(
        ctx: &mut TestContext,
    ) {
        let realm_id = ctx._realm_id.clone();
        let user_id = Uuid::now_v7();

        let package_id = create_points_package(
            ctx,
            &realm_id,
            "webhook_purchase_package",
            "Webhook Purchase Package",
            750,
            699,
            "USD",
            true,
        )
        .await;

        create_points_account(ctx, user_id, &realm_id).await;

        let attempt_id = create_payment_attempt(
            ctx,
            &realm_id,
            user_id,
            "points_package",
            package_id,
            "wechat",
            699,
            "USD",
        )
        .await;

        let payload = json!({
            "providerStatus": "success",
            "providerTransactionId": format!("wx_webhook_{}", attempt_id),
            "completedAt": chrono::Utc::now().to_rfc3339(),
        });

        let response = crate::application::http::billing::purchase_handlers::fulfill_payment(
            State((*ctx.app_state).clone()),
            Path(attempt_id),
            Json(serde_json::from_value(payload).unwrap()),
        )
        .await;
        assert!(response.is_ok());

        let second_payload = json!({
            "providerStatus": "success",
            "providerTransactionId": format!("wx_webhook_{}", attempt_id),
            "completedAt": chrono::Utc::now().to_rfc3339(),
        });

        let second_response =
            crate::application::http::billing::purchase_handlers::fulfill_payment(
                State((*ctx.app_state).clone()),
                Path(attempt_id),
                Json(serde_json::from_value(second_payload).unwrap()),
            )
            .await;
        assert!(second_response.is_ok());

        let purchase_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM points_package_purchases WHERE payment_attempt_id = $1",
        )
        .bind(attempt_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap();
        assert_eq!(purchase_count, 1, "Should persist a single purchase record");

        let purchase: (i64, i64, String, Option<Uuid>) = sqlx::query_as(
            "SELECT points, amount, currency, points_transaction_id
             FROM points_package_purchases
             WHERE payment_attempt_id = $1",
        )
        .bind(attempt_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap();
        assert_eq!(purchase.0, 750);
        assert_eq!(purchase.1, 699);
        assert_eq!(purchase.2, "USD");
        assert!(purchase.3.is_some(), "Purchase should link transaction id");

        let account = get_points_account_by_user(ctx, user_id).await.unwrap();
        assert_eq!(account.2, 750, "Webhook should grant topup credits once");
        assert_eq!(account.3, 0, "Webhook must not grant subscription credits");
    }
}
