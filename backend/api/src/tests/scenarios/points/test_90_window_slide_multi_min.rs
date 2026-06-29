use crate::tests::helpers::points_helpers::*;
use crate::tests::scenarios::points::fixtures::create_test_user;
use crate::tests::schema_test_context::SchemaTestContext;
use chrono::{Duration, Utc};
use herald_core::domain::points::entities::{CreditType, QuotaSourceType};
use test_context::test_context;

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_window_slide_recovery(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "win-slide@example.com").await;
    create_points_wallet(ctx, user_id, &realm_id).await;
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;

    grant_quota_entitlement_for_test(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
        QuotaSourceType::SubscriptionInitial,
        "sub-window-slide",
        &[(18_000, 100, "5h")],
        Utc::now() - Duration::minutes(30),
        Some(Utc::now() + Duration::days(30)),
    )
    .await;
    let consume_id = seed_quota_consume_for_test(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
        60,
        Utc::now(),
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

    sqlx::query(
        "UPDATE points_transactions SET created_at = NOW() - INTERVAL '6 hours' WHERE id = $1",
    )
    .bind(consume_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("move consume outside 5h window");
    assert_window_available(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
        100,
    )
    .await;
}

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_multi_window_min(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "multi-min@example.com").await;
    create_points_wallet(ctx, user_id, &realm_id).await;
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;

    grant_quota_entitlement_for_test(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
        QuotaSourceType::SubscriptionInitial,
        "sub-multi-min",
        &[
            (18_000, 100, "5h"),
            (604_800, 500, "week"),
            (2_592_000, 2_000, "month"),
        ],
        Utc::now(),
        Some(Utc::now() + Duration::days(30)),
    )
    .await;
    seed_quota_consume_for_test(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
        80,
        Utc::now(),
    )
    .await;
    assert_window_available(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
        20,
    )
    .await;
}
