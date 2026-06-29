use crate::tests::helpers::points_helpers::*;
use crate::tests::scenarios::points::fixtures::create_test_user;
use crate::tests::schema_test_context::SchemaTestContext;
use chrono::{Duration, Utc};
use herald_core::domain::points::entities::{CreditType, QuotaSourceType};
use test_context::test_context;

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_worker_down_window_balance_free(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "worker-free@example.com").await;
    create_points_wallet(ctx, user_id, &realm_id).await;
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;
    grant_quota_entitlement_for_test(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::FreePeriodicCredit,
        QuotaSourceType::FreePeriodicGrant,
        "worker-free",
        &[(86_400, 30, "day")],
        Utc::now(),
        None,
    )
    .await;
    assert_window_available(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::FreePeriodicCredit,
        30,
    )
    .await;
}

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_worker_down_window_balance_subscription(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "worker-sub@example.com").await;
    create_points_wallet(ctx, user_id, &realm_id).await;
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;
    grant_quota_entitlement_for_test(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
        QuotaSourceType::SubscriptionInitial,
        "worker-sub",
        &[(86_400, 40, "day")],
        Utc::now(),
        None,
    )
    .await;
    assert_window_available(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
        40,
    )
    .await;
}

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_worker_down_no_pregrant_write(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(
        &ctx.app_state.pool,
        &realm_id,
        "worker-no-write@example.com",
    )
    .await;
    create_points_wallet(ctx, user_id, &realm_id).await;
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM points_grant_records WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&ctx.app_state.pool)
            .await
            .expect("count grant records");
    assert_eq!(
        count, 0,
        "quota model must not require grant-record prewrites"
    );
}

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_worker_down_quota_entitlement_expiry_only(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id =
        create_test_user(&ctx.app_state.pool, &realm_id, "worker-expiry@example.com").await;
    create_points_wallet(ctx, user_id, &realm_id).await;
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;
    grant_quota_entitlement_for_test(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
        QuotaSourceType::SubscriptionInitial,
        "worker-expired",
        &[(86_400, 40, "day")],
        Utc::now() - Duration::days(2),
        Some(Utc::now() - Duration::days(1)),
    )
    .await;
    assert_window_available(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
        0,
    )
    .await;
}
