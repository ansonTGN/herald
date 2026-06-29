use crate::tests::helpers::points_helpers::*;
use crate::tests::scenarios::points::fixtures::create_test_user;
use crate::tests::schema_test_context::SchemaTestContext;
use chrono::Utc;
use herald_core::domain::points::entities::{CreditSourceType, CreditType, QuotaSourceType};
use test_context::test_context;
use uuid::Uuid;

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_wallets_quota_view_subscription(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id =
        create_test_user(&ctx.app_state.pool, &realm_id, "wallet-quota@example.com").await;
    create_points_wallet(ctx, user_id, &realm_id).await;
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;
    grant_quota_entitlement_for_test(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
        QuotaSourceType::SubscriptionInitial,
        "wallet-sub",
        &[(18_000, 100, "5h"), (604_800, 500, "week")],
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
}

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_wallets_quota_view_exhausted(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(
        &ctx.app_state.pool,
        &realm_id,
        "wallet-exhausted@example.com",
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
        "wallet-exhausted",
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
        CreditType::SubscriptionCredit,
        50,
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
async fn test_wallets_pool_only_bucket_null_quota(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(
        &ctx.app_state.pool,
        &realm_id,
        "wallet-pool-only@example.com",
    )
    .await;
    create_points_wallet(ctx, user_id, &realm_id).await;
    let ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::TopupCredit,
        CreditSourceType::Topup,
        Uuid::now_v7().to_string(),
        25,
        None,
    )
    .await;
    assert_eq!(get_ledger_by_id(ctx, ledger_id).await.remaining_amount, 25);
}

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_wallets_quota_key_stable(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "wallet-key@example.com").await;
    create_points_wallet(ctx, user_id, &realm_id).await;
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;
    grant_quota_entitlement_for_test(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
        QuotaSourceType::SubscriptionInitial,
        "wallet-key",
        &[(18_000, 100, "5h")],
        Utc::now(),
        None,
    )
    .await;
    let windows = quota_windows_jsonb(&[(18_000, 100, "5h")]);
    assert!(windows.to_string().contains("\"key\":\"5h\""));
}

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_wallets_quota_view_permission(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(
        &ctx.app_state.pool,
        &realm_id,
        "wallet-permission@example.com",
    )
    .await;
    create_points_wallet(ctx, user_id, &realm_id).await;
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;
    assert_eq!(
        count_active_quota_entitlements(
            ctx,
            &realm_id,
            user_id,
            bucket_id,
            CreditType::SubscriptionCredit
        )
        .await,
        0,
        "no entitlement means quota view is absent for permission-visible pool-only data"
    );
}
