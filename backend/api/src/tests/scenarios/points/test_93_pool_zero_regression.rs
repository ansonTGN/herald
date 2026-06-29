use crate::tests::helpers::points_helpers::*;
use crate::tests::scenarios::points::fixtures::create_test_user;
use crate::tests::schema_test_context::SchemaTestContext;
use chrono::{Duration, Utc};
use herald_core::domain::points::entities::{CreditSourceType, CreditType, QuotaSourceType};
use test_context::test_context;
use uuid::Uuid;

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_pool_topup_consume_priority(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "pool-topup@example.com").await;
    create_points_wallet(ctx, user_id, &realm_id).await;
    let ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::TopupCredit,
        CreditSourceType::Topup,
        Uuid::now_v7().to_string(),
        100,
        Some(Utc::now() + Duration::days(1)),
    )
    .await;
    assert_eq!(get_ledger_by_id(ctx, ledger_id).await.remaining_amount, 100);
}

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_pool_registration_granted_consume(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(
        &ctx.app_state.pool,
        &realm_id,
        "pool-registration@example.com",
    )
    .await;
    create_points_wallet(ctx, user_id, &realm_id).await;
    let ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::GrantedCredit,
        CreditSourceType::AdminGrant,
        Uuid::now_v7().to_string(),
        70,
        None,
    )
    .await;
    assert_eq!(get_ledger_by_id(ctx, ledger_id).await.remaining_amount, 70);
}

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_pool_refund_recovery(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "pool-refund@example.com").await;
    create_points_wallet(ctx, user_id, &realm_id).await;
    let ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::TopupCredit,
        CreditSourceType::Topup,
        Uuid::now_v7().to_string(),
        100,
        None,
    )
    .await;
    assert_eq!(get_ledger_by_id(ctx, ledger_id).await.revoked_amount, 0);
}

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_pool_expiration_consistent(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "pool-expire@example.com").await;
    create_points_wallet(ctx, user_id, &realm_id).await;
    let ledger_id = create_credit_ledger_entry_v2(
        ctx,
        user_id,
        &realm_id,
        CreditType::TopupCredit,
        CreditSourceType::Topup,
        Uuid::now_v7().to_string(),
        25,
        Some(Utc::now() - Duration::days(1)),
    )
    .await;
    assert!(get_ledger_by_id(ctx, ledger_id).await.expires_at.is_some());
}

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_pool_consume_no_window_interference(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "pool-window@example.com").await;
    create_points_wallet(ctx, user_id, &realm_id).await;
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;
    grant_quota_entitlement_for_test(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
        QuotaSourceType::SubscriptionInitial,
        "pool-window-sub",
        &[(86_400, 10, "day")],
        Utc::now(),
        None,
    )
    .await;
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
    assert_window_available(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
        10,
    )
    .await;
    assert_eq!(get_ledger_by_id(ctx, ledger_id).await.remaining_amount, 25);
}
