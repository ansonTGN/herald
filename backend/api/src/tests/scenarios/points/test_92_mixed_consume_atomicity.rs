use crate::tests::helpers::points_helpers::*;
use crate::tests::scenarios::points::fixtures::create_test_user;
use crate::tests::schema_test_context::SchemaTestContext;
use chrono::Utc;
use herald_core::domain::points::entities::{CreditSourceType, CreditType, QuotaSourceType};
use test_context::test_context;
use uuid::Uuid;

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_mixed_consume_window_then_pool(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(
        &ctx.app_state.pool,
        &realm_id,
        "mixed-window-pool@example.com",
    )
    .await;
    create_points_wallet(ctx, user_id, &realm_id).await;
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;
    grant_quota_entitlement_for_test(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
        QuotaSourceType::SubscriptionInitial,
        "sub-mixed",
        &[(86_400, 100, "day")],
        Utc::now(),
        None,
    )
    .await;
    let topup_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::TopupCredit,
        CreditSourceType::Topup,
        Uuid::now_v7().to_string(),
        500,
        None,
    )
    .await;
    seed_quota_consume_for_test(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
        100,
        Utc::now(),
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
    let topup = get_ledger_by_id(ctx, topup_id).await;
    assert_eq!(
        topup.remaining_amount, 500,
        "window use must not mutate pool ledgers"
    );
}

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_mixed_consume_insufficient_total_rollback(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id =
        create_test_user(&ctx.app_state.pool, &realm_id, "mixed-rollback@example.com").await;
    create_points_wallet(ctx, user_id, &realm_id).await;
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;
    grant_quota_entitlement_for_test(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
        QuotaSourceType::SubscriptionInitial,
        "sub-rollback",
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
        CreditType::SubscriptionCredit,
        30,
    )
    .await;
}

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_mixed_consume_window_exhausted_atomic_topup(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(
        &ctx.app_state.pool,
        &realm_id,
        "mixed-exhausted@example.com",
    )
    .await;
    create_points_wallet(ctx, user_id, &realm_id).await;
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;
    grant_quota_entitlement_for_test(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
        QuotaSourceType::SubscriptionInitial,
        "sub-exhausted",
        &[(86_400, 100, "day")],
        Utc::now(),
        None,
    )
    .await;
    seed_quota_consume_for_test(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
        100,
        Utc::now(),
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

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_mixed_consume_free_window_after_subscription(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "mixed-free@example.com").await;
    create_points_wallet(ctx, user_id, &realm_id).await;
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;
    grant_quota_entitlement_for_test(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
        QuotaSourceType::SubscriptionInitial,
        "sub-free-order",
        &[(86_400, 100, "day")],
        Utc::now(),
        None,
    )
    .await;
    grant_quota_entitlement_for_test(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::FreePeriodicCredit,
        QuotaSourceType::FreePeriodicGrant,
        "free-order",
        &[(86_400, 200, "day")],
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
        100,
    )
    .await;
    assert_window_available(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::FreePeriodicCredit,
        200,
    )
    .await;
}

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_concurrent_consume_no_overspend(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(
        &ctx.app_state.pool,
        &realm_id,
        "mixed-concurrent@example.com",
    )
    .await;
    create_points_wallet(ctx, user_id, &realm_id).await;
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;
    grant_quota_entitlement_for_test(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
        QuotaSourceType::SubscriptionInitial,
        "sub-concurrent",
        &[(86_400, 100, "day")],
        Utc::now(),
        None,
    )
    .await;
    seed_quota_consume_for_test(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
        100,
        Utc::now(),
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
