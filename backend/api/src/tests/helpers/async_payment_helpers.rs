// =============================================================================
// Async Payment Test Helpers
// =============================================================================
//
// Shared helpers for async payment points strategy and revocation scenario tests.
// Provides functions for creating test users, wallets, payment attempts, and
// querying balances and attempt status.
//
// =============================================================================

#![allow(dead_code)]

use crate::tests::helpers::billing_helpers::setup_test_entitlement_mapping_full;
use crate::tests::schema_test_context::SchemaTestContext;
use uuid::Uuid;

/// Create a test user in the test realm and return their UUID.
pub async fn create_test_user(ctx: &SchemaTestContext, realm_id: &str, email: &str) -> Uuid {
    let user_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO account (id, realm_id, email, password, status)
         VALUES ($1, $2, $3, $4, 1)
         ON CONFLICT (realm_id, email) DO NOTHING",
    )
    .bind(user_id)
    .bind(realm_id)
    .bind(email)
    .bind("$2a$12$dummy_password_hash")
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create test user");
    user_id
}

/// Create a points wallet for a user with zero balances.
pub async fn create_points_wallet(ctx: &SchemaTestContext, user_id: Uuid, realm_id: &str) {
    let bucket_id = crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm(
        &ctx.app_state.pool,
        realm_id,
    )
    .await;
    sqlx::query(
        "INSERT INTO points_wallets (id, user_id, realm_id, bucket_id, total_topup_granted, total_subscription_granted, total_recharged, total_consumed, status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, 0, 0, 0, 0, 'active', NOW(), NOW())
         ON CONFLICT (realm_id, user_id, bucket_id) DO NOTHING",
    )
    .bind(Uuid::now_v7())
    .bind(user_id)
    .bind(realm_id)
    .bind(bucket_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create points wallet");
}

/// Set the async_points_strategy for a realm.
/// Uses ON CONFLICT ... DO UPDATE for idempotency across test runs.
pub async fn set_async_points_strategy(
    ctx: &SchemaTestContext,
    realm_id: &str,
    strategy_value: &str,
) {
    sqlx::query(
        "INSERT INTO realm_config (id, realm_id, config_type, config_key, config_value, is_secret, enabled, created_at, updated_at)
         VALUES (uuidv7(), $1, 'stripe', 'async_points_strategy', $2, false, true, NOW(), NOW())
         ON CONFLICT (realm_id, config_type, config_key)
         DO UPDATE SET config_value = $2, enabled = true, updated_at = NOW()",
    )
    .bind(realm_id)
    .bind(strategy_value)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to set async_points_strategy");
}

/// Create a one-time entitlement mapping with points_per_period.
/// Returns the mapping ID.
pub async fn create_one_time_mapping(
    ctx: &mut SchemaTestContext,
    realm_id: &str,
    entitlement_key: &str,
    points_per_period: i64,
) -> Uuid {
    setup_test_entitlement_mapping_full(
        ctx,
        realm_id,
        "stripe",
        &format!("prod_stripe_{}", entitlement_key),
        None,
        entitlement_key,
        Some("one_time"),
        None,
        Some(points_per_period),
        None,
        None,
        false,
        None,
        true,
        None,
    )
    .await
}

/// Create a recurring entitlement mapping with points_per_period.
/// Returns the mapping ID.
pub async fn create_recurring_mapping(
    ctx: &mut SchemaTestContext,
    realm_id: &str,
    entitlement_key: &str,
    points_per_period: i64,
) -> Uuid {
    setup_test_entitlement_mapping_full(
        ctx,
        realm_id,
        "stripe",
        &format!("prod_stripe_{}", entitlement_key),
        None,
        entitlement_key,
        Some("recurring"),
        Some("monthly"),
        Some(points_per_period),
        None,
        None,
        true,
        None,
        true,
        None,
    )
    .await
}

/// Create a pending payment attempt targeting the given mapping.
/// Returns the attempt ID.
pub async fn create_pending_payment_attempt(
    ctx: &SchemaTestContext,
    realm_id: &str,
    user_id: Uuid,
    mapping_id: Uuid,
) -> Uuid {
    let bucket_id = crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm(
        &ctx.app_state.pool,
        realm_id,
    )
    .await;
    let attempt_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO payment_attempts
            (id, realm_id, user_id, payment_provider, target_type, target_id,
             bucket_id, amount, currency, status, expires_at, created_at, updated_at)
         VALUES ($1, $2, $3, 'stripe', 'entitlement_mapping', $4,
             $5, 1000, 'usd', 'Pending', NOW() + INTERVAL '1 hour', NOW(), NOW())",
    )
    .bind(attempt_id)
    .bind(realm_id)
    .bind(user_id)
    .bind(mapping_id)
    .bind(bucket_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create pending payment attempt");
    attempt_id
}

/// Get payment attempt status by ID.
pub async fn get_payment_attempt_status(
    ctx: &SchemaTestContext,
    attempt_id: Uuid,
) -> Option<String> {
    sqlx::query_scalar::<_, String>("SELECT status FROM payment_attempts WHERE id = $1")
        .bind(attempt_id)
        .fetch_optional(&ctx.app_state.pool)
        .await
        .unwrap_or(None)
}

/// Get topup_balance for a user's wallet.
///
/// BE-D11: `points_wallets.topup_balance` was dropped; available topup balance
/// is derived from `points_credit_ledger` (topup + registration + free_periodic).
pub async fn get_topup_balance(ctx: &SchemaTestContext, user_id: Uuid, realm_id: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(SUM(l.remaining_amount) FILTER (
                    WHERE l.status = 'active' AND l.remaining_amount > 0
                      AND l.credit_type IN ('topup_credit','registration_credit','free_periodic_credit')
                      AND (l.effective_at IS NULL OR l.effective_at <= NOW())
                      AND (l.expires_at  IS NULL OR l.expires_at  >  NOW())
                ), 0)::BIGINT
         FROM points_wallets w
         LEFT JOIN points_credit_ledger l
           ON l.realm_id = w.realm_id AND l.user_id = w.user_id AND l.bucket_id = w.bucket_id
         WHERE w.user_id = $1 AND w.realm_id = $2
         GROUP BY w.id",
    )
    .bind(user_id)
    .bind(realm_id)
    .fetch_optional(&ctx.app_state.pool)
    .await
    .unwrap()
    .unwrap_or(0)
}

/// Get subscription_balance for a user's wallet.
///
/// BE-D11: `points_wallets.subscription_balance` was dropped; available
/// subscription balance is derived from `points_credit_ledger`.
pub async fn get_subscription_balance(
    ctx: &SchemaTestContext,
    user_id: Uuid,
    realm_id: &str,
) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(SUM(l.remaining_amount) FILTER (
                    WHERE l.status = 'active' AND l.remaining_amount > 0
                      AND l.credit_type = 'subscription_credit'
                      AND (l.effective_at IS NULL OR l.effective_at <= NOW())
                      AND (l.expires_at  IS NULL OR l.expires_at  >  NOW())
                ), 0)::BIGINT
         FROM points_wallets w
         LEFT JOIN points_credit_ledger l
           ON l.realm_id = w.realm_id AND l.user_id = w.user_id AND l.bucket_id = w.bucket_id
         WHERE w.user_id = $1 AND w.realm_id = $2
         GROUP BY w.id",
    )
    .bind(user_id)
    .bind(realm_id)
    .fetch_optional(&ctx.app_state.pool)
    .await
    .unwrap()
    .unwrap_or(0)
}
