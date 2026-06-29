use crate::tests::helpers::points_helpers::*;
use crate::tests::scenarios::points::fixtures::create_test_user;
use crate::tests::schema_test_context::SchemaTestContext;
use chrono::{Duration, Utc};
use herald_core::domain::points::entities::{CreditType, QuotaSourceType};
use test_context::test_context;

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_midperiod_user_gets_quota(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "midperiod@example.com").await;
    create_points_wallet(ctx, user_id, &realm_id).await;
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;
    grant_quota_entitlement_for_test(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::FreePeriodicCredit,
        QuotaSourceType::FreePeriodicGrant,
        "free-midperiod",
        &[(86_400, 50, "day")],
        Utc::now(),
        None,
    )
    .await;
    seed_quota_consume_for_test(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::FreePeriodicCredit,
        10,
        Utc::now(),
    )
    .await;
    assert_window_available(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::FreePeriodicCredit,
        40,
    )
    .await;
}

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_inactive_user_zero_overhead(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id =
        create_test_user(&ctx.app_state.pool, &realm_id, "inactive-zero@example.com").await;
    create_points_wallet(ctx, user_id, &realm_id).await;
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;
    grant_quota_entitlement_for_test(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::FreePeriodicCredit,
        QuotaSourceType::FreePeriodicGrant,
        "free-inactive",
        &[(86_400, 50, "day")],
        Utc::now() - Duration::days(10),
        None,
    )
    .await;
    let txn_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM points_transactions WHERE realm_id = $1 AND user_id = $2",
    )
    .bind(&realm_id)
    .bind(user_id)
    .fetch_one(&ctx.app_state.pool)
    .await
    .expect("count txns");
    let ledger_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM points_credit_ledger WHERE realm_id = $1 AND user_id = $2",
    )
    .bind(&realm_id)
    .bind(user_id)
    .fetch_one(&ctx.app_state.pool)
    .await
    .expect("count ledgers");
    assert_eq!(
        txn_count, 0,
        "inactive quota users must not receive pregrant transactions"
    );
    assert_eq!(
        ledger_count, 0,
        "free periodic quota must not prewrite ledger rows"
    );
}
