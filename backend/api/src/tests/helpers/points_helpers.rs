#![allow(dead_code)]

use crate::tests::schema_test_context::SchemaTestContext;
use herald_core::domain::authentication::Identity;
use herald_core::domain::client_api_keys::entities::ClientApiKey;
use herald_core::domain::points::entities::{CreditType, PointsQuotaEntitlement, TransactionType};
use herald_core::domain::user::entities::User;
use sqlx::Row;
use uuid::Uuid;

/// ============================================================================
/// Points Wallet Creation Helpers
/// ============================================================================
/// Ensure a user row exists in the account table (needed by grant_points_atomic).
async fn ensure_test_user_exists(ctx: &SchemaTestContext, user_id: Uuid, realm_id: &str) {
    sqlx::query(
        "INSERT INTO account (id, realm_id, email, password, status, created_at, updated_at)
         VALUES ($1, $2, $3, '$2a$12$dummy_password_hash', 1, NOW(), NOW())
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(user_id)
    .bind(realm_id)
    .bind(format!("wallet-user-{}@test.com", user_id))
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to ensure user exists");
}

/// Ensure a single legacy `credit_buckets` row exists for `realm_id` and return
/// its id.
///
/// The credit-bucket migration made `points_wallets.bucket_id`,
/// `points_transactions.bucket_id` and `points_credit_ledger.bucket_id` NOT
/// NULL. Legacy scenario tests predate the Bucket model and never create a
/// bucket; without one every legacy INSERT into these tables violates the NOT
/// NULL constraint. This helper materializes a deterministic, realm-scoped
/// legacy bucket (`bucket_key = "legacy-<hash>"`, `enabled = true`,
/// `receives_registration_credits = true`) once per realm and reuses it on
/// subsequent calls. The bucket is shared by all legacy wallets / ledgers /
/// transactions in that realm, mirroring the pre-bucket single-pool semantics
/// these tests were written against. Marking it as the realm's registration
/// pool means registration-bonus grants (which require a registration-pool
/// bucket) land in the same legacy pool the tests assert on.
pub async fn ensure_test_bucket_for_realm(pool: &sqlx::PgPool, realm_id: &str) -> Uuid {
    use sqlx::Row;

    // Deterministic legacy bucket key. realm_id is not guaranteed to match the
    // `^[a-z0-9-]{1,64}$` bucket_key constraint, so derive a short stable slug
    // from a hex hash of the realm_id.
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    use std::hash::Hasher;
    hasher.write(realm_id.as_bytes());
    let slug = format!("legacy-{:016x}", hasher.finish());

    sqlx::query(
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

    let row = sqlx::query("SELECT id FROM credit_buckets WHERE realm_id = $1 AND bucket_key = $2")
        .bind(realm_id)
        .bind(&slug)
        .fetch_one(pool)
        .await
        .expect("Failed to fetch legacy credit bucket");
    let bucket_id: Uuid = row.get("id");

    // Legacy consume paths resolve coverage from `credit_bucket_client_apps`
    // (production behavior: no default-bucket merging). Legacy scenario tests
    // were written against a single shared pool per realm, so attach every
    // existing client app in this realm to the legacy bucket. Idempotent:
    // client apps created later are attached at their own creation site (see
    // `attach_client_app_to_legacy_bucket`).
    sqlx::query(
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
    .expect("Failed to attach existing client apps to legacy credit bucket");

    bucket_id
}

/// Attach a client app to this realm's legacy credit bucket (no-op if the
/// legacy bucket has not been materialized yet).
///
/// Legacy scenario tests create client apps ad-hoc via several helpers; each
/// such helper should call this so the consume-path coverage resolution
/// includes the new client app.
pub async fn attach_client_app_to_legacy_bucket(
    pool: &sqlx::PgPool,
    realm_id: &str,
    client_app_id: Uuid,
) {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    use std::hash::Hasher;
    hasher.write(realm_id.as_bytes());
    let slug = format!("legacy-{:016x}", hasher.finish());

    sqlx::query(
        r#"INSERT INTO credit_bucket_client_apps
             (bucket_id, client_app_id, realm_id, created_at)
           SELECT b.id, $1, b.realm_id, NOW()
           FROM credit_buckets b
           WHERE b.realm_id = $2 AND b.bucket_key = $3
           ON CONFLICT (bucket_id, client_app_id) DO NOTHING"#,
    )
    .bind(client_app_id)
    .bind(realm_id)
    .bind(&slug)
    .execute(pool)
    .await
    .expect("Failed to attach client app to legacy credit bucket");
}

/// Create a points wallet for a user
///
/// Also ensures the user exists in the account table (needed by grant_points_atomic).
/// Returns the wallet_id
pub async fn create_points_wallet(
    ctx: &mut SchemaTestContext,
    user_id: Uuid,
    realm_id: &str,
) -> Uuid {
    ensure_test_user_exists(ctx, user_id, realm_id).await;

    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, realm_id).await;
    let wallet_id = Uuid::now_v7();

    // point-time: the 5 per-type balance columns and the
    // `total_balance` GENERATED column were dropped from `points_wallets`;
    // available balance is derived from `points_credit_ledger`. This INSERT
    // therefore seeds only the retained lifetime-analytics columns.
    sqlx::query(
        "INSERT INTO points_wallets (id, user_id, realm_id, bucket_id, total_topup_granted, total_subscription_granted, total_recharged, total_consumed, status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, 0, 0, 0, 0, 'active', NOW(), NOW())
         ON CONFLICT (realm_id, user_id, bucket_id) DO NOTHING",
    )
    .bind(wallet_id)
    .bind(user_id)
    .bind(realm_id)
    .bind(bucket_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create points wallet");

    // Re-read in case a wallet already existed for this (realm, user, bucket)
    // pool and our minted id collided with the unique row.
    let row = sqlx::query(
        "SELECT id FROM points_wallets WHERE realm_id = $1 AND user_id = $2 AND bucket_id = $3",
    )
    .bind(realm_id)
    .bind(user_id)
    .bind(bucket_id)
    .fetch_one(&ctx.app_state.pool)
    .await
    .expect("Failed to fetch points wallet after ensure");
    let existing: Uuid = row.get("id");
    existing
}

/// Create a user, ensure the legacy test wallet exists, and return
/// `(user_id, bucket_id)`. New quota-window scenario tests use this to avoid
/// repeating setup boilerplate in every case.
pub async fn create_user_wallet_and_bucket_for_test(
    ctx: &mut SchemaTestContext,
    realm_id: &str,
    email: &str,
) -> (Uuid, Uuid) {
    let user_id = crate::tests::scenarios::points::fixtures::create_test_user(
        &ctx.app_state.pool,
        realm_id,
        email,
    )
    .await;
    create_points_wallet(ctx, user_id, realm_id).await;
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, realm_id).await;
    (user_id, bucket_id)
}

/// Create a points wallet with initial balance
///
/// Under point-time `points_wallets` no longer holds Stored per-type
/// balances; available balance is derived from `points_credit_ledger`. To keep
/// this helper's contract ("after this returns, the wallet shows
/// `topup_balance` of topup credit and `subscription_balance` of subscription
/// credit") we seed ledger rows of the requested amounts. Lifetime analytics
/// columns are bumped to mirror the legacy behavior.
pub async fn create_points_wallet_with_balance(
    ctx: &mut SchemaTestContext,
    user_id: Uuid,
    realm_id: &str,
    _initial_balance: i64,
    topup_balance: i64,
    subscription_balance: i64,
) -> Uuid {
    ensure_test_user_exists(ctx, user_id, realm_id).await;

    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, realm_id).await;
    let wallet_id = Uuid::now_v7();

    sqlx::query(
        "INSERT INTO points_wallets (id, user_id, realm_id, bucket_id, total_topup_granted, total_subscription_granted, total_recharged, total_consumed, status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $5 + $6, 0, 'active', NOW(), NOW())
         ON CONFLICT (realm_id, user_id, bucket_id) DO NOTHING",
    )
    .bind(wallet_id)
    .bind(user_id)
    .bind(realm_id)
    .bind(bucket_id)
    .bind(topup_balance)
    .bind(subscription_balance)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create points wallet with balance");

    let row = sqlx::query(
        "SELECT id FROM points_wallets WHERE realm_id = $1 AND user_id = $2 AND bucket_id = $3",
    )
    .bind(realm_id)
    .bind(user_id)
    .bind(bucket_id)
    .fetch_one(&ctx.app_state.pool)
    .await
    .expect("Failed to fetch points wallet with balance after ensure");
    let existing: Uuid = row.get("id");

    // Seed ledger rows so the derived balance reflects the requested amounts.
    if topup_balance > 0 {
        sqlx::query(
            "INSERT INTO points_credit_ledger (id, user_id, realm_id, bucket_id, credit_type, source_type, source_id, granted_amount, used_amount, revoked_amount, status, created_at, updated_at)
             VALUES ($1, $2, $3, $4, 'topup_credit', 'system_grant', $5, $6, 0, 0, 'active', NOW(), NOW())",
        )
        .bind(Uuid::now_v7())
        .bind(user_id)
        .bind(realm_id)
        .bind(bucket_id)
        .bind(format!("seed-topup-{}", existing))
        .bind(topup_balance)
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to seed topup ledger for create_points_wallet_with_balance");
    }
    if subscription_balance > 0 {
        sqlx::query(
            "INSERT INTO points_credit_ledger (id, user_id, realm_id, bucket_id, credit_type, source_type, source_id, granted_amount, used_amount, revoked_amount, status, created_at, updated_at)
             VALUES ($1, $2, $3, $4, 'subscription_credit', 'system_grant', $5, $6, 0, 0, 'active', NOW(), NOW())",
        )
        .bind(Uuid::now_v7())
        .bind(user_id)
        .bind(realm_id)
        .bind(bucket_id)
        .bind(format!("seed-sub-{}", existing))
        .bind(subscription_balance)
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to seed subscription ledger for create_points_wallet_with_balance");
    }

    existing
}

/// ============================================================================
/// Points Transaction Helpers
/// ============================================================================
/// Create a points transaction record
pub async fn create_points_transaction(
    ctx: &mut SchemaTestContext,
    wallet_id: Uuid,
    user_id: Uuid,
    realm_id: &str,
    transaction_type: TransactionType,
    amount: i64,
    balance_after: i64,
    topup_balance_after: Option<i64>,
    subscription_balance_after: Option<i64>,
    credit_type: Option<CreditType>,
    description: Option<&str>,
) -> Uuid {
    // Resolve the bucket_id bound to this wallet so the NOT NULL
    // points_transactions.bucket_id constraint holds.
    let bucket_id: Uuid =
        sqlx::query_scalar::<_, Uuid>("SELECT bucket_id FROM points_wallets WHERE id = $1")
            .bind(wallet_id)
            .fetch_one(&ctx.app_state.pool)
            .await
            .expect("Failed to resolve bucket_id from points_wallets");

    let transaction_id = Uuid::now_v7();

    sqlx::query(
        "INSERT INTO points_transactions (id, wallet_id, user_id, realm_id, bucket_id, type, amount, balance_after,
         topup_balance_after, subscription_balance_after, credit_type, description, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, NOW())"
    )
    .bind(transaction_id)
    .bind(wallet_id)
    .bind(user_id)
    .bind(realm_id)
    .bind(bucket_id)
    .bind(transaction_type.to_string())
    .bind(amount)
    .bind(balance_after)
    .bind(topup_balance_after)
    .bind(subscription_balance_after)
    .bind(credit_type.map(|t| t.to_string()))
    .bind(description)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create points transaction");

    transaction_id
}

/// ============================================================================
/// Points Ledger Helpers
/// ============================================================================
/// Create a credit ledger entry (for tracking credit grants).
///
/// Legacy helper kept for completeness; new tests should prefer
/// `create_credit_ledger_entry_v2`. The credit-bucket migration reshaped
/// `points_credit_ledger` (no more `wallet_id` / `transaction_id` / `amount` /
/// `remaining_amount` columns; `bucket_id` is NOT NULL, `remaining_amount` is a
/// generated column). This helper resolves the user/realm/bucket from the
/// wallet row and writes the new-schema columns.
pub async fn create_credit_ledger_entry(
    ctx: &mut SchemaTestContext,
    wallet_id: Uuid,
    transaction_id: Uuid,
    credit_type: CreditType,
    amount: i64,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
) {
    let row = sqlx::query("SELECT user_id, realm_id, bucket_id FROM points_wallets WHERE id = $1")
        .bind(wallet_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .expect("Failed to resolve wallet for credit ledger entry");
    let user_id: Uuid = row.get("user_id");
    let realm_id: String = row.get("realm_id");
    let bucket_id: Uuid = row.get("bucket_id");

    sqlx::query(
        "INSERT INTO points_credit_ledger
            (id, user_id, realm_id, bucket_id, credit_type, source_type, source_id,
             granted_amount, used_amount, revoked_amount, expires_at, status,
             created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, 'system_grant', $6, $7, 0, 0, $8, 'active', NOW(), NOW())",
    )
    .bind(Uuid::now_v7())
    .bind(user_id)
    .bind(&realm_id)
    .bind(bucket_id)
    .bind(credit_type.to_string())
    .bind(transaction_id.to_string())
    .bind(amount)
    .bind(expires_at)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create credit ledger entry");
}

/// ============================================================================
/// Query Helpers
/// ============================================================================
/// Get points wallet by user ID
///
/// Returns (wallet_id, balance, topup_balance, subscription_balance).
/// Under point-time the balance figures are DERIVED from
/// `points_credit_ledger` (same predicate as consumption) — the wallet row no
/// longer carries Stored per-type balances. The grouping matches the legacy
/// meaning: `topup` = topup + registration + free_periodic credit types.
///
/// Subscription balance is sourced from the window-quota model
/// (`points_quota_entitlements` + `points_transactions` window aggregation),
/// not from ledger rows.
pub async fn get_points_wallet_by_user(
    ctx: &SchemaTestContext,
    user_id: Uuid,
) -> Option<(Uuid, i64, i64, i64)> {
    let row = sqlx::query(
        "SELECT w.id, w.realm_id, w.bucket_id,
                COALESCE(SUM(l.remaining_amount) FILTER (
                    WHERE l.status = 'active' AND l.remaining_amount > 0
                      AND (l.effective_at IS NULL OR l.effective_at <= NOW())
                      AND (l.expires_at  IS NULL OR l.expires_at  >  NOW())
                ), 0)::BIGINT AS total_balance,
                COALESCE(SUM(l.remaining_amount) FILTER (
                    WHERE l.status = 'active' AND l.remaining_amount > 0
                      AND l.credit_type IN ('topup_credit','registration_credit','free_periodic_credit')
                      AND (l.effective_at IS NULL OR l.effective_at <= NOW())
                      AND (l.expires_at  IS NULL OR l.expires_at  >  NOW())
                ), 0)::BIGINT AS topup_balance
         FROM points_wallets w
         LEFT JOIN points_credit_ledger l
           ON l.realm_id = w.realm_id AND l.user_id = w.user_id AND l.bucket_id = w.bucket_id
         WHERE w.user_id = $1
         GROUP BY w.id
         ORDER BY MAX(w.created_at) DESC
         LIMIT 1"
    )
    .bind(user_id)
    .fetch_optional(&ctx.app_state.pool)
    .await
    .unwrap()?;

    use sqlx::Row;
    let wallet_id: Uuid = row.get("id");
    let realm_id: String = row.get("realm_id");
    let bucket_id: Uuid = row.get("bucket_id");
    let ledger_total: i64 = row.get("total_balance");
    let topup_balance: i64 = row.get("topup_balance");

    // Subscription availability comes from the quota window model.
    let subscription_balance = compute_window_available(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
    )
    .await;

    Some((
        wallet_id,
        ledger_total + subscription_balance,
        topup_balance,
        subscription_balance,
    ))
}

/// Get points wallet balance
///
/// Returns (total, topup, subscription). Under point-time the balance
/// figures are DERIVED from `points_credit_ledger` using the same predicate as
/// consumption — `points_wallets` no longer carries Stored per-type balances.
///
/// Subscription balance is sourced from the window-quota model
/// (`points_quota_entitlements` + `points_transactions` window aggregation).
pub async fn get_points_balance(
    ctx: &SchemaTestContext,
    wallet_id: Uuid,
) -> Option<(i64, i64, i64)> {
    let row = sqlx::query(
        "SELECT w.realm_id, w.bucket_id, w.user_id,
                COALESCE(SUM(l.remaining_amount) FILTER (
                    WHERE l.status = 'active' AND l.remaining_amount > 0
                      AND (l.effective_at IS NULL OR l.effective_at <= NOW())
                      AND (l.expires_at  IS NULL OR l.expires_at  >  NOW())
                ), 0)::BIGINT AS total_balance,
                COALESCE(SUM(l.remaining_amount) FILTER (
                    WHERE l.status = 'active' AND l.remaining_amount > 0
                      AND l.credit_type IN ('topup_credit','registration_credit','free_periodic_credit')
                      AND (l.effective_at IS NULL OR l.effective_at <= NOW())
                      AND (l.expires_at  IS NULL OR l.expires_at  >  NOW())
                ), 0)::BIGINT AS topup_balance
         FROM points_wallets w
         LEFT JOIN points_credit_ledger l
           ON l.realm_id = w.realm_id AND l.user_id = w.user_id AND l.bucket_id = w.bucket_id
         WHERE w.id = $1
         GROUP BY w.id"
    )
    .bind(wallet_id)
    .fetch_optional(&ctx.app_state.pool)
    .await
    .unwrap()?;

    use sqlx::Row;
    let realm_id: String = row.get("realm_id");
    let bucket_id: Uuid = row.get("bucket_id");
    let user_id: Uuid = row.get("user_id");
    let ledger_total: i64 = row.get("total_balance");
    let topup_balance: i64 = row.get("topup_balance");

    let subscription_balance = compute_window_available(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
    )
    .await;

    Some((
        ledger_total + subscription_balance,
        topup_balance,
        subscription_balance,
    ))
}

/// Get total credit amount from ledger for a specific credit type.
///
/// For window-quota credit types (subscription_credit, free_periodic_credit)
/// this sums the first-window `limit` from `points_quota_entitlements` instead
/// of ledger `granted_amount`.
pub async fn get_total_credit_by_type(
    ctx: &SchemaTestContext,
    user_id: Uuid,
    credit_type: CreditType,
) -> i64 {
    if credit_type == CreditType::SubscriptionCredit
        || credit_type == CreditType::FreePeriodicCredit
    {
        let entitlements = get_user_quota_entitlements(ctx, user_id, credit_type).await;
        return entitlements
            .iter()
            .map(|e| e.quota_windows.first().map(|w| w.limit).unwrap_or(0))
            .sum();
    }

    sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(SUM(granted_amount), 0)::BIGINT FROM points_credit_ledger
         WHERE user_id = $1 AND credit_type = $2",
    )
    .bind(user_id)
    .bind(credit_type.to_string())
    .fetch_one(&ctx.app_state.pool)
    .await
    .unwrap()
}

/// Get remaining credit amount from ledger for a specific credit type.
///
/// For window-quota credit types this returns `compute_window_available`
/// instead of summing ledger `remaining_amount`.
pub async fn get_remaining_credit_by_type(
    ctx: &SchemaTestContext,
    user_id: Uuid,
    credit_type: CreditType,
) -> i64 {
    if credit_type == CreditType::SubscriptionCredit
        || credit_type == CreditType::FreePeriodicCredit
    {
        let realm_id = ctx._realm_id.clone();
        let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;
        return compute_window_available(ctx, &realm_id, user_id, bucket_id, credit_type).await;
    }

    sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(SUM(remaining_amount), 0)::BIGINT FROM points_credit_ledger
         WHERE user_id = $1 AND credit_type = $2",
    )
    .bind(user_id)
    .bind(credit_type.to_string())
    .fetch_one(&ctx.app_state.pool)
    .await
    .unwrap()
}

/// Get transactions for a user
///
/// Returns Vec of (transaction_id, transaction_type, amount)
pub async fn get_user_transactions(
    ctx: &SchemaTestContext,
    user_id: Uuid,
    limit: i64,
) -> Vec<(Uuid, String, i64)> {
    sqlx::query(
        "SELECT id, type, amount FROM points_transactions WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2"
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(&ctx.app_state.pool)
    .await
    .unwrap()
    .into_iter()
    .map(|row| (
        row.get("id"),
        row.get("type"),
        row.get("amount"),
    ))
    .collect()
}

/// Count transactions by type for a user
pub async fn count_transactions_by_type(
    ctx: &SchemaTestContext,
    user_id: Uuid,
    transaction_type: TransactionType,
) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM points_transactions WHERE user_id = $1 AND type = $2",
    )
    .bind(user_id)
    .bind(transaction_type.to_string())
    .fetch_one(&ctx.app_state.pool)
    .await
    .unwrap()
}

/// ============================================================================
/// Assertion Helpers
/// ============================================================================
/// Assert points wallet balance matches expected values
pub async fn assert_points_balance(
    ctx: &SchemaTestContext,
    wallet_id: Uuid,
    expected_total: i64,
    expected_topup: i64,
    expected_subscription: i64,
) {
    let balance = get_points_balance(ctx, wallet_id).await;

    if let Some((total, topup, subscription)) = balance {
        assert_eq!(
            total, expected_total,
            "Total balance mismatch: expected {}, got {}",
            expected_total, total
        );
        assert_eq!(
            topup, expected_topup,
            "Topup balance mismatch: expected {}, got {}",
            expected_topup, topup
        );
        assert_eq!(
            subscription, expected_subscription,
            "Subscription balance mismatch: expected {}, got {}",
            expected_subscription, subscription
        );
    } else {
        panic!("Points wallet not found");
    }
}

/// Assert credit ledger entries exist for a specific credit type
pub async fn assert_credit_ledger_exists(
    ctx: &SchemaTestContext,
    user_id: Uuid,
    credit_type: CreditType,
    expected_total_amount: i64,
) {
    let total = get_total_credit_by_type(ctx, user_id, credit_type).await;

    assert_eq!(
        total, expected_total_amount,
        "Total {} credit mismatch: expected {}, got {}",
        credit_type, expected_total_amount, total
    );
}

/// Assert transaction exists for a user
pub async fn assert_transaction_exists(
    ctx: &SchemaTestContext,
    user_id: Uuid,
    transaction_type: TransactionType,
    expected_amount: i64,
) {
    let transactions = get_user_transactions(ctx, user_id, 100).await;

    let found = transactions.iter().any(|(_id, tx_type, amount)| {
        tx_type == &transaction_type.to_string() && *amount == expected_amount
    });

    assert!(
        found,
        "Expected transaction of type {:?} with amount {} not found for user",
        transaction_type, expected_amount
    );
}

/// ============================================================================
/// Cleanup Helpers
/// ============================================================================
/// Delete all points data for a user
pub async fn cleanup_user_points(ctx: &mut SchemaTestContext, user_id: Uuid) {
    sqlx::query("DELETE FROM points_transactions WHERE user_id = $1")
        .bind(user_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

    sqlx::query("DELETE FROM points_credit_ledger WHERE user_id = $1")
        .bind(user_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

    sqlx::query("DELETE FROM points_wallets WHERE user_id = $1")
        .bind(user_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();
}

/// Delete all points data for an account
pub async fn cleanup_wallet_points(ctx: &mut SchemaTestContext, wallet_id: Uuid) {
    sqlx::query("DELETE FROM points_transactions WHERE wallet_id = $1")
        .bind(wallet_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

    sqlx::query("DELETE FROM points_credit_ledger WHERE wallet_id IN (SELECT id FROM points_wallets WHERE id = $1)")
        .bind(wallet_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

    sqlx::query("DELETE FROM points_wallets WHERE id = $1")
        .bind(wallet_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();
}

/// Create a credit ledger entry (user-focused version)
pub async fn create_credit_ledger_entry_v2(
    ctx: &SchemaTestContext,
    user_id: Uuid,
    realm_id: &str,
    credit_type: herald_core::domain::points::entities::CreditType,
    source_type: herald_core::domain::points::entities::CreditSourceType,
    source_id: String,
    amount: i64,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
) -> Uuid {
    let ledger_id = Uuid::now_v7();

    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, realm_id).await;

    sqlx::query(
        "INSERT INTO points_credit_ledger (id, user_id, realm_id, bucket_id, credit_type, source_type, source_id, granted_amount, used_amount, revoked_amount, expires_at, status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 0, 0, $9, 'active', NOW(), NOW())"
    )
    .bind(ledger_id)
    .bind(user_id)
    .bind(realm_id)
    .bind(bucket_id)
    .bind(credit_type.to_string())
    .bind(source_type.to_string())
    .bind(source_id)
    .bind(amount)
    .bind(expires_at)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create credit ledger entry");

    // point-time: `points_wallets` no longer holds Stored per-type
    // balances; available balance is derived from `points_credit_ledger`. We
    // only bump the retained lifetime-analytics columns (topup / subscription
    // / recharged) so analytics-style assertions still hold.
    let (topup_bump, sub_bump) = match credit_type {
        herald_core::domain::points::entities::CreditType::TopupCredit => (amount, 0),
        herald_core::domain::points::entities::CreditType::SubscriptionCredit => (0, amount),
        _ => (0, 0),
    };
    sqlx::query(
        "UPDATE points_wallets
         SET total_topup_granted = total_topup_granted + $1,
             total_subscription_granted = total_subscription_granted + $2,
             total_recharged = total_recharged + $1 + $2,
             updated_at = NOW()
         WHERE user_id = $3 AND realm_id = $4",
    )
    .bind(topup_bump)
    .bind(sub_bump)
    .bind(user_id)
    .bind(realm_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to update account analytics after creating ledger");

    ledger_id
}

/// Resolve the `bucket_id` of the wallet a realm/user's credits live in.
/// Refund-via-webhook tests need this to seed a `payment_attempts` snapshot
/// scoped to the same pool (see `create_payment_attempt_snapshot`).
pub async fn get_wallet_bucket_id(ctx: &SchemaTestContext, realm_id: &str, user_id: Uuid) -> Uuid {
    let row =
        sqlx::query("SELECT bucket_id FROM points_wallets WHERE realm_id = $1 AND user_id = $2")
            .bind(realm_id)
            .bind(user_id)
            .fetch_one(&ctx.app_state.pool)
            .await
            .expect("Failed to fetch wallet bucket_id");
    let bucket_id: Uuid = row.get("bucket_id");
    bucket_id
}

/// Create a `payment_attempts` snapshot row so the Creem refund webhook can
/// resolve the routing `bucket_id` for `provider_reference` (the original
/// payment id). The refund handler (`webhook_handlers.rs` refund path)
/// requires a payment_attempt with a non-null `bucket_id` to scope revocation
/// to the same pool the original grant targeted (fail-loud) — tests
/// that grant a ledger directly via `create_credit_ledger_entry_v2` must also
/// seed this snapshot before sending a refund webhook, otherwise the handler
/// rejects with "no payment_attempt for payment_id".
///
/// `provider_reference` is the value the webhook will look up (the test's
/// `payment_id`); `bucket_id` should match the wallet the credits live in.
pub async fn create_payment_attempt_snapshot(
    ctx: &SchemaTestContext,
    realm_id: &str,
    user_id: Uuid,
    provider_reference: &str,
    bucket_id: Uuid,
    original_amount: i64,
) {
    sqlx::query(
        "INSERT INTO payment_attempts
            (id, realm_id, user_id, payment_provider, target_type, target_id,
             bucket_id, amount, currency, status, provider_reference,
             provider_status, expires_at, created_at, updated_at)
         VALUES ($1, $2, $3, 'creem', 'entitlement_mapping', $4,
                 $5, $6, 'usd', 'Succeeded', $7,
                 'succeeded', NOW() + INTERVAL '1 hour', NOW(), NOW())
         ON CONFLICT DO NOTHING",
    )
    .bind(Uuid::now_v7())
    .bind(realm_id)
    .bind(user_id)
    .bind(Uuid::now_v7())
    .bind(bucket_id)
    .bind(original_amount)
    .bind(provider_reference)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create payment_attempt snapshot");
}

/// Helper function to convert a database row to PointsCreditLedger
fn row_to_credit_ledger(
    row: &sqlx::postgres::PgRow,
) -> herald_core::domain::points::entities::PointsCreditLedger {
    herald_core::domain::points::entities::PointsCreditLedger {
        id: row.get("id"),
        user_id: row.get("user_id"),
        realm_id: row.get("realm_id"),
        bucket_id: row.get("bucket_id"),
        credit_type: row.get::<String, _>("credit_type").parse().unwrap(),
        source_type: row.get::<String, _>("source_type").parse().unwrap(),
        source_id: row.get("source_id"),
        granted_amount: row.get("granted_amount"),
        used_amount: row.get("used_amount"),
        revoked_amount: row.get("revoked_amount"),
        remaining_amount: row.get("remaining_amount"),
        expires_at: row.get("expires_at"),
        effective_at: row.get("effective_at"),
        status: row.get::<String, _>("status").parse().unwrap(),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

/// Get all ledgers for a user
pub async fn get_user_ledgers(
    ctx: &SchemaTestContext,
    user_id: Uuid,
) -> Vec<herald_core::domain::points::entities::PointsCreditLedger> {
    let rows = sqlx::query(
        "SELECT * FROM points_credit_ledger WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(&ctx.app_state.pool)
    .await
    .unwrap();

    rows.into_iter()
        .map(|row| row_to_credit_ledger(&row))
        .collect()
}

/// Get ledgers by credit type
pub async fn get_user_ledgers_by_credit_type(
    ctx: &SchemaTestContext,
    user_id: Uuid,
    credit_type: herald_core::domain::points::entities::CreditType,
) -> Vec<herald_core::domain::points::entities::PointsCreditLedger> {
    let rows = sqlx::query(
        "SELECT * FROM points_credit_ledger WHERE user_id = $1 AND credit_type = $2 ORDER BY created_at DESC"
    )
    .bind(user_id)
    .bind(credit_type.to_string())
    .fetch_all(&ctx.app_state.pool)
    .await
    .unwrap();

    rows.into_iter()
        .map(|row| row_to_credit_ledger(&row))
        .collect()
}

/// Get ledger by ID
pub async fn get_ledger_by_id(
    ctx: &SchemaTestContext,
    ledger_id: Uuid,
) -> herald_core::domain::points::entities::PointsCreditLedger {
    let row = sqlx::query("SELECT * FROM points_credit_ledger WHERE id = $1")
        .bind(ledger_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap();

    row_to_credit_ledger(&row)
}

/// Consume points from a specific ledger.
///
/// Bumps the ledger's `used_amount` (which recomputes the GENERATED
/// `remaining_amount`) AND bumps the matching wallet's `total_consumed`
/// lifetime-analytics column. Under point-time there is no per-type
/// Stored balance column to decrement — the available balance is derived from
/// `points_credit_ledger`, so bumping `used_amount` is sufficient to reduce
/// the derived balance.
pub async fn consume_points_from_ledger(ctx: &SchemaTestContext, ledger_id: Uuid, amount: i64) {
    sqlx::query(
        "UPDATE points_credit_ledger
         SET used_amount = used_amount + $1,
             updated_at = NOW()
         WHERE id = $2",
    )
    .bind(amount)
    .bind(ledger_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to consume points from ledger");

    // Keep the wallet `total_consumed` lifetime-analytics column in sync with
    // the ledger deduction. Match the wallet by the ledger's
    // (realm_id, user_id, bucket_id).
    sqlx::query(
        "UPDATE points_wallets w
         SET total_consumed = w.total_consumed + $1,
             updated_at = NOW()
         FROM points_credit_ledger l
         WHERE l.id = $2 AND w.realm_id = l.realm_id
           AND w.user_id = l.user_id AND w.bucket_id = l.bucket_id",
    )
    .bind(amount)
    .bind(ledger_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to sync wallet total_consumed after manual ledger consume");
}

/// Get revocation records for a user
pub async fn get_revocation_records(
    ctx: &SchemaTestContext,
    user_id: Uuid,
) -> Vec<herald_core::domain::points::entities::PointsRevocationRecord> {
    let rows = sqlx::query(
        "SELECT * FROM points_revocation_records WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(&ctx.app_state.pool)
    .await
    .unwrap();

    rows.into_iter()
        .map(
            |row| herald_core::domain::points::entities::PointsRevocationRecord {
                id: row.get("id"),
                ledger_id: row.get("ledger_id"),
                user_id: row.get("user_id"),
                realm_id: row.get("realm_id"),
                revocation_type: row.get::<String, _>("revocation_type").parse().unwrap(),
                revoked_amount: row.get("revoked_amount"),
                reason: row.get("reason"),
                reference_id: row.get("reference_id"),
                created_at: row.get("created_at"),
            },
        )
        .collect()
}

/// Get consumption allocations for a user
pub async fn get_consumption_allocations(
    ctx: &SchemaTestContext,
    user_id: Uuid,
) -> Vec<herald_core::domain::points::entities::PointsConsumptionAllocation> {
    let rows = sqlx::query(
        "SELECT * FROM points_consumption_allocations WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(&ctx.app_state.pool)
    .await
    .unwrap();

    rows.into_iter()
        .map(
            |row| herald_core::domain::points::entities::PointsConsumptionAllocation {
                id: row.get("id"),
                transaction_id: row.get("transaction_id"),
                ledger_id: row.get("ledger_id"),
                wallet_id: row.get("wallet_id"),
                user_id: row.get("user_id"),
                realm_id: row.get("realm_id"),
                bucket_id: row.get("bucket_id"),
                allocated_amount: row.get("allocated_amount"),
                ledger_remaining_after: row.get("ledger_remaining_after"),
                created_at: row.get("created_at"),
            },
        )
        .collect()
}

/// Check if idempotency key exists
pub async fn assert_idempotency_key_exists(ctx: &SchemaTestContext, key: &str) {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM idempotency_keys WHERE idempotency_key = $1)",
    )
    .bind(key)
    .fetch_one(&ctx.app_state.pool)
    .await
    .unwrap();

    assert!(exists, "Idempotency key {} should exist", key);
}

/// Assert transaction exists by type
pub async fn assert_transaction_exists_by_type(
    ctx: &SchemaTestContext,
    user_id: Uuid,
    transaction_type: herald_core::domain::points::entities::TransactionType,
    expected_amount: i64,
) {
    let transactions =
        sqlx::query("SELECT * FROM points_transactions WHERE user_id = $1 AND type = $2")
            .bind(user_id)
            .bind(transaction_type.to_string())
            .fetch_all(&ctx.app_state.pool)
            .await
            .unwrap();

    let found = transactions
        .iter()
        .any(|row| row.get::<i64, _>("amount") == expected_amount);

    assert!(
        found,
        "Expected transaction of type {:?} with amount {} not found for user",
        transaction_type, expected_amount
    );
}

/// ============================================================================
/// Test Identity Creation Helper
/// ============================================================================
/// Create a test Identity for a user
pub fn create_test_identity(user_id: Uuid, realm_id: &str) -> Identity {
    let user = User {
        id: user_id,
        realm_id: realm_id.to_string(),
        email: "test@example.com".to_string(),
        nickname: None,
        password_hash: None,
        provider_ids: Vec::new(),
        status: herald_core::domain::user::entities::UserStatus::Normal,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    Identity::User(user)
}

/// Create a test ThirdParty identity for points.consume scenarios.
pub fn create_test_third_party_identity(realm_id: &str) -> Identity {
    Identity::ThirdParty(ClientApiKey {
        id: Uuid::now_v7().to_string(),
        name: "Test API Key".to_string(),
        api_key_hash: "sha256:test".to_string(),
        realm_id: realm_id.to_string(),
        client_app_id: None,
        enabled: true,
        expires_at: None,
        created_at: chrono::Utc::now(),
        last_used_at: None,
    })
}

/// Assert all account balances are non-negative.
///
/// Aggregates across every wallet row the user owns in the realm (one row per
/// `(user, bucket)` pool under the multi-bucket model). Returns the summed
/// `(total_balance, topup_balance, subscription_balance)`.
///
/// Under point-time the balance figures are DERIVED from
/// `points_credit_ledger` (same predicate as consumption) and from the quota
/// window model; the assertion that each is >= 0 still holds because
/// `remaining_amount` and window `remaining` are non-negative.
pub async fn assert_balances_non_negative(
    ctx: &SchemaTestContext,
    user_id: Uuid,
    realm_id: &str,
) -> (i64, i64, i64) {
    let account = sqlx::query(
        "SELECT COALESCE(SUM(l.remaining_amount) FILTER (
                    WHERE l.status = 'active' AND l.remaining_amount > 0
                      AND (l.effective_at IS NULL OR l.effective_at <= NOW())
                      AND (l.expires_at  IS NULL OR l.expires_at  >  NOW())
                ), 0)::BIGINT AS total_balance,
                COALESCE(SUM(l.remaining_amount) FILTER (
                    WHERE l.status = 'active' AND l.remaining_amount > 0
                      AND l.credit_type IN ('topup_credit','registration_credit','free_periodic_credit')
                      AND (l.effective_at IS NULL OR l.effective_at <= NOW())
                      AND (l.expires_at  IS NULL OR l.expires_at  >  NOW())
                ), 0)::BIGINT AS topup_balance
         FROM points_wallets w
         LEFT JOIN points_credit_ledger l
           ON l.realm_id = w.realm_id AND l.user_id = w.user_id AND l.bucket_id = w.bucket_id
         WHERE w.user_id = $1 AND w.realm_id = $2",
    )
    .bind(user_id)
    .bind(realm_id)
    .fetch_one(&ctx.app_state.pool)
    .await
    .expect("Failed to fetch account");

    use sqlx::Row;
    let ledger_total: i64 = account.get("total_balance");
    let topup_balance: i64 = account.get("topup_balance");

    // Aggregate subscription availability across all of the user's wallets in
    // this realm (typically one).
    let wallet_rows =
        sqlx::query("SELECT bucket_id FROM points_wallets WHERE user_id = $1 AND realm_id = $2")
            .bind(user_id)
            .bind(realm_id)
            .fetch_all(&ctx.app_state.pool)
            .await
            .expect("Failed to fetch wallet buckets");

    let mut subscription_balance: i64 = 0;
    for row in wallet_rows {
        let bucket_id: Uuid = row.get("bucket_id");
        subscription_balance += compute_window_available(
            ctx,
            realm_id,
            user_id,
            bucket_id,
            CreditType::SubscriptionCredit,
        )
        .await;
    }

    let total_balance = ledger_total + subscription_balance;

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

    (total_balance, topup_balance, subscription_balance)
}

/// Assert ledger accounting invariant: granted == used + revoked + remaining, and remaining >= 0
pub async fn assert_ledger_invariants(ctx: &SchemaTestContext, user_id: Uuid) {
    let ledgers = get_user_ledgers(ctx, user_id).await;
    for ledger in &ledgers {
        assert!(
            ledger.remaining_amount >= 0,
            "ledger {} remaining_amount must be >= 0, got {}",
            ledger.id,
            ledger.remaining_amount
        );
        assert_eq!(
            ledger.granted_amount,
            ledger.used_amount + ledger.revoked_amount + ledger.remaining_amount,
            "ledger accounting invariant broken for ledger {}: granted={} != used={} + revoked={} + remaining={}",
            ledger.id,
            ledger.granted_amount,
            ledger.used_amount,
            ledger.revoked_amount,
            ledger.remaining_amount
        );
    }
}

/// Assert caller-supplied balance figures match SUM(ledger.remaining_amount)
/// grouped by credit type, using the same derived predicate as consumption
/// (status / effective_at / expires_at). Under point-time this is a
/// derived-vs-derived comparison: callers should obtain the values from
/// `assert_balances_non_negative` (which uses the same predicate and grouping).
///
/// Subscription balance is compared against the quota window model, not ledger
/// rows.
pub async fn assert_account_matches_ledger_sums(
    ctx: &SchemaTestContext,
    user_id: Uuid,
    realm_id: &str,
    topup_balance: i64,
    subscription_balance: i64,
) {
    let topup_ledger_sum: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(remaining_amount), 0)::BIGINT FROM points_credit_ledger
         WHERE user_id = $1 AND realm_id = $2
           AND credit_type IN ('topup_credit', 'registration_credit', 'free_periodic_credit')
           AND status = 'active' AND remaining_amount > 0
           AND (effective_at IS NULL OR effective_at <= NOW())
           AND (expires_at  IS NULL OR expires_at  >  NOW())",
    )
    .bind(user_id)
    .bind(realm_id)
    .fetch_one(&ctx.app_state.pool)
    .await
    .expect("Failed to sum topup ledger remaining");

    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, realm_id).await;
    let sub_quota_sum = compute_window_available(
        ctx,
        realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
    )
    .await;

    assert_eq!(
        topup_balance, topup_ledger_sum,
        "topup_balance ({}) must match ledger sum ({})",
        topup_balance, topup_ledger_sum
    );
    assert_eq!(
        subscription_balance, sub_quota_sum,
        "subscription_balance ({}) must match quota window availability ({})",
        subscription_balance, sub_quota_sum
    );
}

/// Verify points were granted with correct entitlement_key association.
///
/// Under the window-quota model subscription grants live in
/// `points_quota_entitlements`; `source_id` is the subscription id. Join with
/// `subscription` to aggregate by `entitlement_key` and sum the first window
/// limit (matching the legacy "granted amount" semantics).
pub async fn verify_points_granted_for_entitlement(
    ctx: &SchemaTestContext,
    user_id: Uuid,
    entitlement_key: &str,
    expected_amount: i64,
) {
    let rows = sqlx::query(
        r#"SELECT q.quota_windows
           FROM points_quota_entitlements q
           JOIN subscription s ON s.id = q.source_id::uuid
           WHERE q.user_id = $1
             AND q.credit_type = 'subscription_credit'
             AND s.entitlement_key = $2"#,
    )
    .bind(user_id)
    .bind(entitlement_key)
    .fetch_all(&ctx.app_state.pool)
    .await
    .unwrap();

    use sqlx::Row;
    let total: i64 = rows
        .iter()
        .map(|row| {
            let windows: serde_json::Value = row.get("quota_windows");
            windows
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|w| w.get("limit").and_then(|v| v.as_i64()))
                .unwrap_or(0)
        })
        .sum();

    assert!(
        total >= expected_amount,
        "Expected at least {} subscription credit granted for entitlement {}, got {}",
        expected_amount,
        entitlement_key,
        total
    );
}

/// Get current points balance for a user (derived SUM from ledger + quota windows).
///
/// Returns 0 if the user has no wallet. Under point-time the balance
/// is DERIVED from `points_credit_ledger` AND from `points_quota_entitlements`
/// window availability. `points_wallets` no longer carries a Stored `total_balance`.
pub async fn get_points_balance_for_user(ctx: &SchemaTestContext, user_id: Uuid) -> i64 {
    let ledger_balance: Option<i64> = sqlx::query_scalar(
        "SELECT COALESCE(SUM(l.remaining_amount) FILTER (
                    WHERE l.status = 'active' AND l.remaining_amount > 0
                      AND (l.effective_at IS NULL OR l.effective_at <= NOW())
                      AND (l.expires_at  IS NULL OR l.expires_at  >  NOW())
                ), 0)::BIGINT
         FROM points_wallets w
         LEFT JOIN points_credit_ledger l
           ON l.realm_id = w.realm_id AND l.user_id = w.user_id AND l.bucket_id = w.bucket_id
         WHERE w.user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(&ctx.app_state.pool)
    .await
    .unwrap()
    .flatten();

    let ledger_balance = ledger_balance.unwrap_or(0);

    // Add subscription window availability from the quota model. Scope to the
    // first wallet found for the user (tests use a single wallet per user).
    let wallet =
        sqlx::query("SELECT realm_id, bucket_id FROM points_wallets WHERE user_id = $1 LIMIT 1")
            .bind(user_id)
            .fetch_optional(&ctx.app_state.pool)
            .await
            .unwrap();

    if let Some(row) = wallet {
        use sqlx::Row;
        let realm_id: String = row.get("realm_id");
        let bucket_id: Uuid = row.get("bucket_id");
        let sub_available = compute_window_available(
            ctx,
            &realm_id,
            user_id,
            bucket_id,
            CreditType::SubscriptionCredit,
        )
        .await;
        ledger_balance + sub_available
    } else {
        ledger_balance
    }
}

/// Get points grant schedule by entitlement_key.
///
/// Returns Vec of (id, entitlement_key, points_per_period, granted_periods, max_periods, active).
pub async fn get_points_grant_schedule_by_entitlement(
    ctx: &SchemaTestContext,
    entitlement_key: &str,
) -> Vec<(Uuid, String, i64, i64, Option<i64>, bool)> {
    let rows = sqlx::query(
        "SELECT id, entitlement_key, points_per_period, granted_periods, max_periods, active
         FROM points_grant_schedules
         WHERE entitlement_key = $1
         ORDER BY created_at DESC",
    )
    .bind(entitlement_key)
    .fetch_all(&ctx.app_state.pool)
    .await
    .unwrap();

    rows.into_iter()
        .map(|row| {
            use sqlx::Row;
            (
                row.get("id"),
                row.get("entitlement_key"),
                row.get("points_per_period"),
                row.get("granted_periods"),
                row.get("max_periods"),
                row.get("active"),
            )
        })
        .collect()
}

/// Truncate a UTC datetime to microsecond precision.
///
/// Postgres `TIMESTAMPTZ` stores microsecond precision (not nanoseconds), so a
/// seed value derived from `chrono::Utc::now()` (nanosecond precision) loses
/// its sub-microsecond nanos on the DB round-trip. Truncating the seed before
/// writing keeps strict equality assertions (e.g. `expires_at == seed + days`)
/// exact without loosening them.
pub fn trunc_to_micros(ts: chrono::DateTime<chrono::Utc>) -> chrono::DateTime<chrono::Utc> {
    use chrono::Timelike;
    let nanos = ts.timestamp_subsec_nanos();
    let truncated_nanos = (nanos / 1_000) * 1_000;
    ts.with_nanosecond(truncated_nanos)
        .expect("truncated_nanos is a valid subsec nanos value (< 2e9)")
}
// Shared helpers for the point-time feature (design `.ai/design/point-time.md`).
// These cover three reuse needs expressed by the
// scenario matrix:
//   1. Future-effective row injection — `inject_effective_at`,
//      `create_credit_ledger_entry_with_effective_at`. The new entry helper
//      intentionally does NOT call `create_credit_ledger_entry_v2`'s UPDATE
//      `points_wallets` branch, so tests asserting the **derived** available
//      balance (not the Stored column) are not polluted by Stored writes.
//   2. Derived-predicate balance assertion — `get_derived_balance_by_credit_type`,
//      `get_derived_total_balance`, `assert_derived_balance`. The SQL predicate
//      mirrors production `compute_available_balance` /
//      `compute_bucket_available_balances` verbatim:
//        status='active' AND remaining_amount>0
//          AND (effective_at IS NULL OR effective_at <= NOW())
//          AND (expires_at  IS NULL OR expires_at  >  NOW())
//      Downstream scenario items MUST use these instead of reading
//      `points_wallets.total_balance` — the Stored column is not the available
//      balance authority under point-time.
//   3. Schedule / grant_record seed + advance + row-level reclaim positioning
//      — `create_free_grant_schedule`, `create_subscription_grant_schedule`,
//      `advance_schedule`, `create_grant_record`, `grant_record_exists`,
//      `find_ledger_by_source_id`, `find_ledger_id_by_schedule_period`,
//      `count_future_effective_active_rows`.
// # `entitlement_key` decision (Scope item 3 / P2-4)
// Free-periodic points are resolved by production `reconcile_due_for_user`
// (domain/src/points/service.rs:218) → `pregrant_next_period_atomic`, which
// reads `schedule.points_per_period` DIRECTLY. It does NOT consult
// `provider_entitlement_mappings` or any `entitlement_key`-keyed lookup for
// free schedules. Correspondingly, `registration_service.rs:177` seeds the
// free-periodic schedule with `entitlement_key: None`, which infra persists
// as the column default `''` (NOT NULL DEFAULT '', migration line 407).
// `resolve_entitlement_points` (subscription_service.rs:33) operates on the
// subscription `EntitlementMapping` row (a different table), not on the free
// schedule. Therefore the faithful mirror for a FREE schedule seed is
// `entitlement_key = ''` (callers may pass `""`). Subscription schedules
// carry the entitlement_key from their `subscription.entitlement_key` and
// should pass that non-empty value.

/// Derived-predicate fragment mirroring production
/// `compute_available_balance` / consumption selection.
/// Kept inline (not a constant) so the full SQL stays readable at call sites.
const DERIVED_AVAILABLE_PREDICATE: &str = concat!(
    "status = 'active'",
    " AND remaining_amount > 0",
    " AND (effective_at IS NULL OR effective_at <= NOW())",
    " AND (expires_at IS NULL OR expires_at > NOW())"
);

/// UPDATE the `effective_at` column of an existing ledger row.
///
/// Use this to flip an immediately-available row to future-effective (or vice
/// versa) without re-creating it. The DB CHECK
/// `points_credit_ledger_effective_before_expires` rejects an inverted window
/// (effective_at > expires_at); callers must keep `effective_at <= expires_at`.
pub async fn inject_effective_at(
    ctx: &SchemaTestContext,
    ledger_id: Uuid,
    effective_at: Option<chrono::DateTime<chrono::Utc>>,
) {
    sqlx::query(
        "UPDATE points_credit_ledger SET effective_at = $1, updated_at = NOW() WHERE id = $2",
    )
    .bind(effective_at)
    .bind(ledger_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to inject effective_at on ledger row");
}

/// INSERT a credit ledger row directly with an explicit `effective_at`.
///
/// Unlike `create_credit_ledger_entry_v2`, this does NOT touch the
/// `points_wallets` Stored balance columns — tests asserting the DERIVED
/// available balance (via `assert_derived_balance`) must not have the Stored
/// write masking regressions. If a test also needs the Stored column in sync
/// (e.g. legacy analytics assertions), call `create_credit_ledger_entry_v2`
/// separately or invoke the v2 helper then `inject_effective_at`.
///
/// `effective_at = None`  ⟺ immediately available (zero-regression default).
/// `effective_at = Some(t)` ⟺ enters the available set only when `t <= NOW()`.
pub async fn create_credit_ledger_entry_with_effective_at(
    ctx: &SchemaTestContext,
    user_id: Uuid,
    realm_id: &str,
    credit_type: herald_core::domain::points::entities::CreditType,
    source_type: herald_core::domain::points::entities::CreditSourceType,
    source_id: String,
    amount: i64,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
    effective_at: Option<chrono::DateTime<chrono::Utc>>,
) -> Uuid {
    let ledger_id = Uuid::now_v7();
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, realm_id).await;

    sqlx::query(
        "INSERT INTO points_credit_ledger
            (id, user_id, realm_id, bucket_id, credit_type, source_type, source_id,
             granted_amount, used_amount, revoked_amount,
             expires_at, effective_at, status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 0, 0, $9, $10, 'active', NOW(), NOW())",
    )
    .bind(ledger_id)
    .bind(user_id)
    .bind(realm_id)
    .bind(bucket_id)
    .bind(credit_type.to_string())
    .bind(source_type.to_string())
    .bind(source_id)
    .bind(amount)
    .bind(expires_at)
    .bind(effective_at)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create credit ledger entry with effective_at");

    ledger_id
}

/// Derived available balance for one `(user, realm, credit_type)` pool.
///
/// Mirrors production `compute_available_balance` verbatim — does NOT
/// read `points_wallets.total_balance`. Future-effective and expired active
/// rows are excluded by the shared predicate.
pub async fn get_derived_balance_by_credit_type(
    ctx: &SchemaTestContext,
    user_id: Uuid,
    realm_id: &str,
    credit_type: herald_core::domain::points::entities::CreditType,
) -> i64 {
    // Window-quota model: subscription/free_periodic availability is computed
    // from active entitlements + consume transactions, not ledger rows.
    if credit_type == CreditType::SubscriptionCredit
        || credit_type == CreditType::FreePeriodicCredit
    {
        let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, realm_id).await;
        return compute_window_available(ctx, realm_id, user_id, bucket_id, credit_type).await;
    }

    let sql = format!(
        "SELECT COALESCE(SUM(remaining_amount), 0)::BIGINT FROM points_credit_ledger
         WHERE user_id = $1 AND realm_id = $2 AND credit_type = $3 AND ({pred})",
        pred = DERIVED_AVAILABLE_PREDICATE
    );
    sqlx::query_scalar(&sql)
        .bind(user_id)
        .bind(realm_id)
        .bind(credit_type.to_string())
        .fetch_one(&ctx.app_state.pool)
        .await
        .expect("Failed to compute derived balance by credit_type")
}

/// Derived available balance summed across all credit types for a user.
///
/// Mirrors production total derived balance. Use this for "total
/// available" assertions; prefer `get_derived_balance_by_credit_type` for
/// per-pool assertions.
pub async fn get_derived_total_balance(
    ctx: &SchemaTestContext,
    user_id: Uuid,
    realm_id: &str,
) -> i64 {
    let sql = format!(
        "SELECT COALESCE(SUM(remaining_amount), 0)::BIGINT FROM points_credit_ledger
         WHERE user_id = $1 AND realm_id = $2 AND ({pred})",
        pred = DERIVED_AVAILABLE_PREDICATE
    );
    let ledger_total: i64 = sqlx::query_scalar(&sql)
        .bind(user_id)
        .bind(realm_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .expect("Failed to compute derived total balance");

    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, realm_id).await;
    let sub_available = compute_window_available(
        ctx,
        realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
    )
    .await;
    let free_available = compute_window_available(
        ctx,
        realm_id,
        user_id,
        bucket_id,
        CreditType::FreePeriodicCredit,
    )
    .await;

    ledger_total + sub_available + free_available
}

/// Assert the derived available balance for a credit type matches `expected`.
///
/// This is the canonical point-time balance assertion. Downstream scenario
/// items MUST use this instead of reading `points_wallets.total_balance` —
/// the Stored column is not the available-balance authority under point-time
/// and may lag or diverge.
pub async fn assert_derived_balance(
    ctx: &SchemaTestContext,
    user_id: Uuid,
    realm_id: &str,
    credit_type: herald_core::domain::points::entities::CreditType,
    expected: i64,
) {
    let actual = get_derived_balance_by_credit_type(ctx, user_id, realm_id, credit_type).await;
    assert_eq!(
        actual, expected,
        "derived available balance for {:?} (user {}) expected {}, got {}; \
         predicate = status='active' AND remaining_amount>0 AND (effective_at IS NULL OR \
         effective_at<=NOW()) AND (expires_at IS NULL OR expires_at>NOW())",
        credit_type, user_id, expected, actual
    );
}

/// Seed a FREE-periodic `points_grant_schedules` row (`subscription_id IS NULL`,
/// `active = TRUE`).
///
/// `entitlement_key` is passed by the caller. Per the helper-module doc
/// comment above, free-periodic schedules are resolved by production
/// `reconcile_due_for_user` via `schedule.points_per_period` directly (NOT via
/// an entitlement_key lookup), and `registration_service.rs:177` seeds with
/// `entitlement_key: None` (persisted as `''`). Callers should pass `""` to
/// mirror production faithfully unless they are explicitly testing a non-empty
/// informational key.
///
/// Returns the new schedule id.
pub async fn create_free_grant_schedule(
    ctx: &SchemaTestContext,
    user_id: Uuid,
    realm_id: &str,
    grant_period_type: &str,
    points_per_period: i64,
    validity_days: i64,
    next_grant_time: chrono::DateTime<chrono::Utc>,
    granted_periods: i64,
    entitlement_key: &str,
) -> Uuid {
    let schedule_id = Uuid::now_v7();
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, realm_id).await;

    sqlx::query(
        "INSERT INTO points_grant_schedules
            (id, user_id, realm_id, bucket_id, subscription_id, entitlement_key,
             grant_period_type, base_time, next_grant_time, points_per_period,
             validity_days, granted_periods, max_periods, active, created_at, updated_at)
         VALUES ($1, $2, $3, $4, NULL, $5, $6, $7, $8, $9, $10, $11, NULL, TRUE, NOW(), NOW())",
    )
    .bind(schedule_id)
    .bind(user_id)
    .bind(realm_id)
    .bind(bucket_id)
    .bind(entitlement_key)
    .bind(grant_period_type)
    .bind(next_grant_time) // base_time anchors the schedule
    .bind(next_grant_time)
    .bind(points_per_period)
    .bind(validity_days)
    .bind(granted_periods)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create free grant schedule");

    schedule_id
}

/// Seed a SUBSCRIPTION-bound `points_grant_schedules` row.
///
/// `first_period_start` is stored as `base_time` (the period_number derivation
/// anchor). `entitlement_key` mirrors the subscription's key
/// (non-empty for subscription schedules).
pub async fn create_subscription_grant_schedule(
    ctx: &SchemaTestContext,
    user_id: Uuid,
    realm_id: &str,
    subscription_id: Uuid,
    entitlement_key: &str,
    points_per_period: i64,
    next_grant_time: chrono::DateTime<chrono::Utc>,
    first_period_start: chrono::DateTime<chrono::Utc>,
    granted_periods: i64,
) -> Uuid {
    let schedule_id = Uuid::now_v7();
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, realm_id).await;

    sqlx::query(
        "INSERT INTO points_grant_schedules
            (id, user_id, realm_id, bucket_id, subscription_id, entitlement_key,
             grant_period_type, base_time, next_grant_time, points_per_period,
             validity_days, granted_periods, max_periods, active, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, 'monthly', $7, $8, $9, 0, $10, NULL, TRUE, NOW(), NOW())",
    )
    .bind(schedule_id)
    .bind(user_id)
    .bind(realm_id)
    .bind(bucket_id)
    .bind(subscription_id)
    .bind(entitlement_key)
    .bind(first_period_start)
    .bind(next_grant_time)
    .bind(points_per_period)
    .bind(granted_periods)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create subscription grant schedule");

    schedule_id
}

/// Advance a schedule row's `next_grant_time` and `granted_periods`.
///
/// Use this in worker-down scenarios to simulate the progression that
/// `GrantScheduler::process_due_schedules` would have applied, without
/// invoking the worker (worker is preheat, not correctness).
pub async fn advance_schedule(
    ctx: &SchemaTestContext,
    schedule_id: Uuid,
    new_next_grant_time: chrono::DateTime<chrono::Utc>,
    new_granted_periods: i64,
) {
    sqlx::query(
        "UPDATE points_grant_schedules
         SET next_grant_time = $1, granted_periods = $2, updated_at = NOW()
         WHERE id = $3",
    )
    .bind(new_next_grant_time)
    .bind(new_granted_periods)
    .bind(schedule_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to advance grant schedule");
}

/// Seed a `points_grant_records(schedule_id, period_number)` idempotency row.
///
/// `ledger_id` is REQUIRED — `points_grant_records.ledger_id` is a
/// NOT NULL FK (the reclaim row-positioning bridge). Callers should first
/// create the ledger row via `create_credit_ledger_entry_with_effective_at`
/// (or `create_credit_ledger_entry_v2`) and pass the returned id here.
pub async fn create_grant_record(
    ctx: &SchemaTestContext,
    schedule_id: Uuid,
    period_number: i64,
    amount: i64,
    grant_time: chrono::DateTime<chrono::Utc>,
    ledger_id: Uuid,
) {
    let user_id: Uuid =
        sqlx::query_scalar("SELECT user_id FROM points_grant_schedules WHERE id = $1")
            .bind(schedule_id)
            .fetch_one(&ctx.app_state.pool)
            .await
            .expect("Failed to fetch schedule user_id for grant record");
    let realm_id: String =
        sqlx::query_scalar("SELECT realm_id FROM points_grant_schedules WHERE id = $1")
            .bind(schedule_id)
            .fetch_one(&ctx.app_state.pool)
            .await
            .expect("Failed to fetch schedule realm_id for grant record");

    sqlx::query(
        "INSERT INTO points_grant_records
            (id, schedule_id, user_id, realm_id, period_number, granted_amount, grant_time, ledger_id, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())",
    )
    .bind(Uuid::now_v7())
    .bind(schedule_id)
    .bind(user_id)
    .bind(realm_id)
    .bind(period_number)
    .bind(amount)
    .bind(grant_time)
    .bind(ledger_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create grant record");
}

/// Return TRUE iff a `points_grant_records` row exists for
/// `(schedule_id, period_number)`. Use this to assert the period-level
/// business idempotency key.
pub async fn grant_record_exists(
    ctx: &SchemaTestContext,
    schedule_id: Uuid,
    period_number: i64,
) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM points_grant_records WHERE schedule_id = $1 AND period_number = $2)",
    )
    .bind(schedule_id)
    .bind(period_number)
    .fetch_one(&ctx.app_state.pool)
    .await
    .expect("Failed to check grant record existence")
}

/// Locate the ledger id for a `(user, source_id)`. Used by reclaim tests
/// asserting row-level positioning by `source_id`.
pub async fn find_ledger_by_source_id(
    ctx: &SchemaTestContext,
    user_id: Uuid,
    source_id: &str,
) -> Option<Uuid> {
    sqlx::query_scalar::<_, Option<Uuid>>(
        "SELECT id FROM points_credit_ledger WHERE user_id = $1 AND source_id = $2 LIMIT 1",
    )
    .bind(user_id)
    .bind(source_id)
    .fetch_optional(&ctx.app_state.pool)
    .await
    .expect("Failed to find ledger by source_id")
    .flatten()
}

/// Locate the ledger id for a `(schedule_id, period_number)` via the
/// `points_grant_records.ledger_id` FK (reclaim bridge).
///
/// This is the same lookup production reclaim performs — it resolves
/// the business idempotency key `(schedule_id, period_number)` (which lives
/// only in `points_grant_records`) to a unique ledger row. Reclaim tests MUST
/// use this rather than guessing by `source_id`.
pub async fn find_ledger_id_by_schedule_period(
    ctx: &SchemaTestContext,
    schedule_id: Uuid,
    period_number: i64,
) -> Option<Uuid> {
    sqlx::query_scalar::<_, Option<Uuid>>(
        "SELECT ledger_id FROM points_grant_records WHERE schedule_id = $1 AND period_number = $2 LIMIT 1",
    )
    .bind(schedule_id)
    .bind(period_number)
    .fetch_optional(&ctx.app_state.pool)
    .await
    .expect("Failed to find ledger_id by schedule period")
    .flatten()
}

/// Count active ledger rows for a user that are future-effective
/// (`effective_at > NOW()`). Use this to assert response/path non-leak of
/// future-effective rows and pre-grant counts.
pub async fn count_future_effective_active_rows(
    ctx: &SchemaTestContext,
    user_id: Uuid,
    realm_id: &str,
) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM points_credit_ledger
         WHERE user_id = $1 AND realm_id = $2
           AND status = 'active' AND effective_at IS NOT NULL AND effective_at > NOW()",
    )
    .bind(user_id)
    .bind(realm_id)
    .fetch_one(&ctx.app_state.pool)
    .await
    .expect("Failed to count future-effective active rows")
}

// points-grant-redesign helpers (quota entitlement + window view)
// Shared helpers for the window-quota model (design
// `.ai/design/points-grant-redesign.md` §4.3.2 / §5.1 / §5.4). These mirror
// the production read path (`find_active_quota_entitlements` predicate +
// `compute_quota_windows_view`) so scenario tests assert against the SAME
// authority the consumption / balance read path uses, not a second DDL.
// Active-entitlement predicate (infra `find_active_quota_entitlements`,
// postgres_repository.rs:5793):
//   status = 'active' AND effective_from <= now
//     AND (effective_until IS NULL OR effective_until > now)
// A revoked entitlement (status='revoked' + effective_until set) leaves the
// active set, so its windows contribute ZERO availability — the core reclaim
// invariant (US-CB-008 / design §7 P0).

/// JSONB shape of `quota_windows` / `free_periodic_quota_windows` as written by
/// production serde (`QuotaWindowDbJson`, infra postgres_repository.rs:178).
/// `[{"windowSeconds": i64, "limit": i64, "key": "5h"}]`.
pub fn quota_windows_jsonb(windows: &[(i64, i64, &str)]) -> serde_json::Value {
    serde_json::Value::Array(
        windows
            .iter()
            .map(|(secs, limit, key)| {
                serde_json::json!({
                    "windowSeconds": secs,
                    "limit": limit,
                    "key": key,
                })
            })
            .collect(),
    )
}

/// Count ACTIVE quota entitlement rows for `(user, bucket, credit_type)` under
/// the production active predicate. A revoked/expired entitlement is excluded.
pub async fn count_active_quota_entitlements(
    ctx: &SchemaTestContext,
    realm_id: &str,
    user_id: Uuid,
    bucket_id: Uuid,
    credit_type: herald_core::domain::points::entities::CreditType,
) -> i64 {
    sqlx::query_scalar(
        r#"SELECT COUNT(*)::BIGINT FROM points_quota_entitlements
           WHERE realm_id = $1 AND user_id = $2 AND bucket_id = $3
             AND credit_type = $4
             AND status = 'active'
             AND effective_from <= NOW()
             AND (effective_until IS NULL OR effective_until > NOW())"#,
    )
    .bind(realm_id)
    .bind(user_id)
    .bind(bucket_id)
    .bind(credit_type.to_string())
    .fetch_one(&ctx.app_state.pool)
    .await
    .expect("Failed to count active quota entitlements")
}

/// Count ALL quota entitlement rows for `(user, bucket, credit_type)`
/// regardless of status (active + revoked + expired). Use this to assert grant
/// idempotency (exactly one row after a duplicate event) independently of the
/// active predicate.
pub async fn count_all_quota_entitlements(
    ctx: &SchemaTestContext,
    realm_id: &str,
    user_id: Uuid,
    bucket_id: Uuid,
    credit_type: herald_core::domain::points::entities::CreditType,
) -> i64 {
    sqlx::query_scalar(
        r#"SELECT COUNT(*)::BIGINT FROM points_quota_entitlements
           WHERE realm_id = $1 AND user_id = $2 AND bucket_id = $3
             AND credit_type = $4"#,
    )
    .bind(realm_id)
    .bind(user_id)
    .bind(bucket_id)
    .bind(credit_type.to_string())
    .fetch_one(&ctx.app_state.pool)
    .await
    .expect("Failed to count all quota entitlements")
}

/// Return `(status, effective_until_is_set)` for the quota entitlement matched
/// by `(user, bucket, credit_type, source_id)`. `source_id` is the
/// subscription_id (or registration source) string. Used to assert the revoke
/// transition: `status='revoked'` AND `effective_until IS NOT NULL`.
pub async fn quota_entitlement_status(
    ctx: &SchemaTestContext,
    realm_id: &str,
    user_id: Uuid,
    bucket_id: Uuid,
    credit_type: herald_core::domain::points::entities::CreditType,
    source_id: &str,
) -> (String, bool) {
    let row = sqlx::query(
        r#"SELECT status, effective_until IS NOT NULL AS effective_until_set
           FROM points_quota_entitlements
           WHERE realm_id = $1 AND user_id = $2 AND bucket_id = $3
             AND credit_type = $4 AND source_id = $5
           ORDER BY created_at DESC LIMIT 1"#,
    )
    .bind(realm_id)
    .bind(user_id)
    .bind(bucket_id)
    .bind(credit_type.to_string())
    .bind(source_id)
    .fetch_one(&ctx.app_state.pool)
    .await
    .expect("Failed to fetch quota entitlement status");
    use sqlx::Row;
    (
        row.get::<String, _>("status"),
        row.get::<bool, _>("effective_until_set"),
    )
}

/// Window availability for `(user, bucket, credit_type)` = MIN over all ACTIVE
/// entitlement windows of `(limit - SUM(consume in window))`, floored at 0.
/// Mirrors production `compute_window_available_in_tx` (design §5.2). When no
/// active entitlement exists, availability is 0 (the reclaim-zero invariant).
///
/// This recomputes the window aggregation in SQL exactly as production does:
/// for each active entitlement's snapshot window, sum consume rows with
/// `created_at >= now - window_seconds`, take `limit - used`, then take the
/// min across windows (and across entitlements).
pub async fn compute_window_available(
    ctx: &SchemaTestContext,
    realm_id: &str,
    user_id: Uuid,
    bucket_id: Uuid,
    credit_type: herald_core::domain::points::entities::CreditType,
) -> i64 {
    // Active entitlements and their snapshot windows.
    let rows = sqlx::query(
        r#"SELECT id, quota_windows FROM points_quota_entitlements
           WHERE realm_id = $1 AND user_id = $2 AND bucket_id = $3
             AND credit_type = $4
             AND status = 'active'
             AND effective_from <= NOW()
             AND (effective_until IS NULL OR effective_until > NOW())"#,
    )
    .bind(realm_id)
    .bind(user_id)
    .bind(bucket_id)
    .bind(credit_type.to_string())
    .fetch_all(&ctx.app_state.pool)
    .await
    .expect("Failed to fetch active quota entitlements for window view");

    use sqlx::Row;
    let mut min_remaining: Option<i64> = None;
    for row in rows {
        let entitlement_id: Uuid = row.get("id");
        let windows: serde_json::Value = row.get("quota_windows");
        let windows = windows
            .as_array()
            .expect("quota_windows must be a JSON array");
        for win in windows {
            let window_seconds: i64 = win
                .get("windowSeconds")
                .and_then(|v| v.as_i64())
                .expect("windowSeconds present");
            let limit: i64 = win
                .get("limit")
                .and_then(|v| v.as_i64())
                .expect("limit present");
            // Used = SUM(consume) in the sliding window, scoped to this
            // entitlement's credit_type/bucket/user (production scopes by
            // credit_type+bucket+user, not per-entitlement-id, because window
            // availability is the shared rolling cap).
            let _ = entitlement_id; // scope marker; aggregation is per (user,bucket,credit_type)
            let used: i64 = sqlx::query_scalar(
                r#"SELECT COALESCE(SUM(ABS(amount)), 0)::BIGINT FROM points_transactions
                   WHERE realm_id = $1 AND user_id = $2 AND bucket_id = $3
                     AND credit_type = $4 AND type = 'consume'
                     AND created_at >= NOW() - make_interval(secs => $5)"#,
            )
            .bind(realm_id)
            .bind(user_id)
            .bind(bucket_id)
            .bind(credit_type.to_string())
            .bind(window_seconds)
            .fetch_one(&ctx.app_state.pool)
            .await
            .expect("Failed to sum consume in window");
            let remaining = (limit - used).max(0);
            min_remaining = Some(match min_remaining {
                None => remaining,
                Some(cur) => cur.min(remaining),
            });
        }
    }
    min_remaining.unwrap_or(0)
}

/// Assert window availability equals `expected`. The canonical reclaim-zero
/// assertion: after revoke, no active entitlement ⟹ availability 0.
pub async fn assert_window_available(
    ctx: &SchemaTestContext,
    realm_id: &str,
    user_id: Uuid,
    bucket_id: Uuid,
    credit_type: herald_core::domain::points::entities::CreditType,
    expected: i64,
) {
    let actual = compute_window_available(ctx, realm_id, user_id, bucket_id, credit_type).await;
    assert_eq!(
        actual, expected,
        "window availability for {:?} (user {}) expected {}, got {}; \
         active entitlement count determines availability (revoked/expired ⟹ 0)",
        credit_type, user_id, expected, actual
    );
}

/// Get all quota entitlement rows for a user scoped to the test realm and
/// legacy test bucket. Mirrors `get_user_ledgers_by_credit_type` but for the
/// window-quota model. Returns rows regardless of status so callers can assert
/// on active/revoked/expired transitions.
pub async fn get_user_quota_entitlements(
    ctx: &SchemaTestContext,
    user_id: Uuid,
    credit_type: CreditType,
) -> Vec<PointsQuotaEntitlement> {
    let realm_id = ctx._realm_id.clone();
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;

    let rows = sqlx::query(
        r#"SELECT id, user_id, realm_id, bucket_id, credit_type, source_type, source_id,
                  quota_windows, effective_from, effective_until, status, idempotency_key,
                  created_at, updated_at
           FROM points_quota_entitlements
           WHERE realm_id = $1 AND user_id = $2 AND bucket_id = $3 AND credit_type = $4
           ORDER BY created_at DESC"#,
    )
    .bind(&realm_id)
    .bind(user_id)
    .bind(bucket_id)
    .bind(credit_type.to_string())
    .fetch_all(&ctx.app_state.pool)
    .await
    .expect("Failed to fetch user quota entitlements");

    #[derive(Debug, serde::Deserialize)]
    struct QuotaWindowDbJson {
        #[serde(rename = "windowSeconds")]
        window_seconds: i64,
        limit: i64,
        key: String,
    }

    rows.into_iter()
        .map(|row| {
            use sqlx::Row;
            let quota_windows: serde_json::Value = row.get("quota_windows");
            let quota_windows: Vec<QuotaWindowDbJson> = serde_json::from_value(quota_windows)
                .expect("quota_windows must deserialize to Vec<QuotaWindowDbJson>");
            let quota_windows = quota_windows
                .into_iter()
                .map(|w| herald_core::domain::points::entities::QuotaWindow {
                    window_seconds: w.window_seconds,
                    limit: w.limit,
                    key: w.key,
                })
                .collect();
            PointsQuotaEntitlement {
                id: row.get("id"),
                user_id: row.get("user_id"),
                realm_id: row.get("realm_id"),
                bucket_id: row.get("bucket_id"),
                credit_type: row.get::<String, _>("credit_type").parse().unwrap(),
                source_type: row.get::<String, _>("source_type").parse().unwrap(),
                source_id: row.get("source_id"),
                quota_windows,
                effective_from: row.get("effective_from"),
                effective_until: row.get("effective_until"),
                status: row.get::<String, _>("status").parse().unwrap(),
                idempotency_key: row.get("idempotency_key"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            }
        })
        .collect()
}

/// Count quota entitlement rows for `subscription_credit` in the test realm
/// and legacy test bucket (any status). Convenience wrapper for the common
/// "exactly one subscription grant" assertion.
pub async fn count_subscription_quota_entitlements(
    ctx: &SchemaTestContext,
    user_id: Uuid,
) -> usize {
    let realm_id = ctx._realm_id.clone();
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;
    count_all_quota_entitlements(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
    )
    .await as usize
}

/// Sum the first-window `limit` across all active quota entitlements for a
/// user/credit_type in the test realm and legacy bucket. Use this as the
/// quota-model equivalent of `get_total_credit_by_type` for subscription or
/// free_periodic credit.
pub async fn get_total_quota_limit_by_type(
    ctx: &SchemaTestContext,
    user_id: Uuid,
    credit_type: CreditType,
) -> i64 {
    let entitlements = get_user_quota_entitlements(ctx, user_id, credit_type).await;
    entitlements
        .iter()
        .filter(|e| {
            e.status == herald_core::domain::points::entities::QuotaEntitlementStatus::Active
        })
        .map(|e| e.quota_windows.first().map(|w| w.limit).unwrap_or(0))
        .sum()
}

/// Seed a `provider_entitlement_mappings` row WITH `quota_windows` attached,
/// routed to the realm's legacy test bucket. Non-empty `quota_windows`
/// switches the grant to the window-quota model (design §4.3.2). Returns the
/// mapping id.
pub async fn create_entitlement_mapping_with_quota_windows(
    ctx: &SchemaTestContext,
    realm_id: &str,
    provider: &str,
    external_product_id: &str,
    entitlement_key: &str,
    quota_windows: &[(i64, i64, &str)],
    grant_on_subscribe: bool,
) -> Uuid {
    let mapping_id = Uuid::now_v7();
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, realm_id).await;
    let windows_json = quota_windows_jsonb(quota_windows);
    let points_per_period = quota_windows.first().map(|(_, limit, _)| *limit);

    sqlx::query(
        r#"INSERT INTO provider_entitlement_mappings
             (id, realm_id, payment_provider, external_product_id, entitlement_key,
              points_per_period, grant_on_subscribe, enabled, bucket_id, quota_windows, created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, true, $8, $9, NOW(), NOW())"#,
    )
    .bind(mapping_id)
    .bind(realm_id)
    .bind(provider)
    .bind(external_product_id)
    .bind(entitlement_key)
    .bind(points_per_period)
    .bind(grant_on_subscribe)
    .bind(bucket_id)
    .bind(&windows_json)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create entitlement mapping with quota_windows");

    mapping_id
}

/// Grant a quota entitlement directly for scenario tests. This is intentionally
/// table-level: the authoring tests need stable setup for window aggregation,
/// not webhook/provider behavior.
pub async fn grant_quota_entitlement_for_test(
    ctx: &SchemaTestContext,
    realm_id: &str,
    user_id: Uuid,
    bucket_id: Uuid,
    credit_type: herald_core::domain::points::entities::CreditType,
    source_type: herald_core::domain::points::entities::QuotaSourceType,
    source_id: &str,
    quota_windows: &[(i64, i64, &str)],
    effective_from: chrono::DateTime<chrono::Utc>,
    effective_until: Option<chrono::DateTime<chrono::Utc>>,
) -> Uuid {
    let entitlement_id = Uuid::now_v7();
    sqlx::query(
        r#"INSERT INTO points_quota_entitlements
             (id, user_id, realm_id, bucket_id, credit_type, source_type, source_id,
              quota_windows, effective_from, effective_until, status, idempotency_key,
              created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'active', $11, NOW(), NOW())"#,
    )
    .bind(entitlement_id)
    .bind(user_id)
    .bind(realm_id)
    .bind(bucket_id)
    .bind(credit_type.to_string())
    .bind(source_type.as_str())
    .bind(source_id)
    .bind(quota_windows_jsonb(quota_windows))
    .bind(effective_from)
    .bind(effective_until)
    .bind(format!("test:{source_id}:{entitlement_id}"))
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to grant quota entitlement for test");
    entitlement_id
}

/// Seed a window-side consume transaction. Revoke tests use this to prove
/// lifecycle cleanup does not delete or reverse already-written usage history.
pub async fn seed_quota_consume_for_test(
    ctx: &SchemaTestContext,
    realm_id: &str,
    user_id: Uuid,
    bucket_id: Uuid,
    credit_type: herald_core::domain::points::entities::CreditType,
    amount: i64,
    created_at: chrono::DateTime<chrono::Utc>,
) -> Uuid {
    let wallet_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM points_wallets
         WHERE realm_id = $1 AND user_id = $2 AND bucket_id = $3",
    )
    .bind(realm_id)
    .bind(user_id)
    .bind(bucket_id)
    .fetch_one(&ctx.app_state.pool)
    .await
    .expect("wallet should exist before seeding quota consume");

    let transaction_id = Uuid::now_v7();
    sqlx::query(
        r#"INSERT INTO points_transactions
             (id, wallet_id, user_id, realm_id, bucket_id, type, amount,
              balance_after, credit_type, description, client_app_id, correlation_id,
              created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5, 'consume', $6, 0, $7,
                   'quota consume test seed', $8, $9, $10, $10)"#,
    )
    .bind(transaction_id)
    .bind(wallet_id)
    .bind(user_id)
    .bind(realm_id)
    .bind(bucket_id)
    .bind(amount)
    .bind(credit_type.to_string())
    .bind(Uuid::parse_str(&ctx._client_app_id).expect("client_app_id must be a UUID"))
    .bind(format!("quota-consume-{transaction_id}"))
    .bind(created_at)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to seed quota consume for test");
    transaction_id
}
