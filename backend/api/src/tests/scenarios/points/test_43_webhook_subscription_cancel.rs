// =============================================================================
// Test: Subscription Cancel Webhook
// =============================================================================
//
// Tests for subscription.canceled webhook events under the window-quota model.
//
// User Story: docs/user-stories/points-billing-events.md
// Covers: US-BI-011 (Subscription cancel behavior)
//
// =============================================================================

use crate::tests::helpers::points_helpers::{
    assert_derived_balance, create_points_wallet, ensure_test_bucket_for_realm,
    get_user_quota_entitlements, grant_quota_entitlement_for_test,
    seed_attributed_subscription_quota,
};
use crate::tests::helpers::webhook_helpers::{
    assert_webhook_success, build_subscription_canceled_event, generate_test_event_id,
    send_webhook_with_signature, setup_test_plan_config,
};
use crate::tests::scenarios::points::fixtures::*;
use crate::tests::schema_test_context::SchemaTestContext;
use chrono::{Duration, Utc};
use herald_core::domain::points::entities::{CreditType, QuotaEntitlementStatus, QuotaSourceType};
use serde_json::json;
use test_context::test_context;
use uuid::Uuid;

/// Seed a subscription row that the cancel webhook can resolve and update in place.
///
/// `subscription_id` is used both as the row primary key and as the `source_id`
/// for the quota entitlement so that `handle_subscription_cancel` can locate the
/// entitlement to revoke.
async fn seed_test_subscription(
    ctx: &SchemaTestContext,
    realm_id: &str,
    user_id: Uuid,
    subscription_id: Uuid,
    external_subscription_id: &str,
    entitlement_key: &str,
) {
    sqlx::query(
        "INSERT INTO subscription
            (id, realm_id, user_id, external_subscription_id, external_product_id,
             payment_provider, status, entitlement_key, current_period_start,
             current_period_end, cancel_at_period_end, created_at, updated_at, billing_type)
         VALUES ($1, $2, $3, $4, 'prod_test_monthly', 'creem', 'active', $5, NOW(),
                 NOW() + INTERVAL '30 days', false, NOW(), NOW(), 'recurring')",
    )
    .bind(subscription_id)
    .bind(realm_id)
    .bind(user_id)
    .bind(external_subscription_id)
    .bind(entitlement_key)
    .execute(&ctx.app_state.pool)
    .await
    .expect("seed_test_subscription: subscription insert");
}

/// Cancellation locates all original rule results rather than one current configured wallet.
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_multi_wallet_grant_rule_cancel_revokes_all_source_accounts_idempotently(
    ctx: &mut SchemaTestContext,
) {
    super::multi_wallet_grant_rule_scenarios::assert_two_account_fixed_event(
        ctx,
        herald_core::domain::points::DistributionTrigger::SubscriptionInitial,
    )
    .await;
}

/// Build a cancel event and point its `subscriptionId` at the seeded subscription.
fn build_cancel_event_for_subscription(
    event_id: String,
    user_id: Uuid,
    cancel_at_period_end: bool,
    realm_id: &str,
    external_subscription_id: &str,
) -> serde_json::Value {
    let mut event =
        build_subscription_canceled_event(event_id, user_id, cancel_at_period_end, realm_id);
    event["data"]["object"]["subscriptionId"] = json!(external_subscription_id);
    event
}

// ============================================================================
// Test 1: Default Cancel (Period End) - Retain Points
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-BI-011 场景 1 - 默认取消保留积分到周期结束
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_subscription_cancel_default_retains_points(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user1@example.com").await;
    let plan_id = Uuid::now_v7();
    let subscription_id = Uuid::now_v7();
    let external_subscription_id = format!("sub_{}", subscription_id);
    let event_id = generate_test_event_id();
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;
    let now = Utc::now();

    ctx.with_creem_config(&realm_id, None, None, None).await;
    setup_test_plan_config(ctx, &realm_id, plan_id).await;
    create_points_wallet(ctx, user_id, &realm_id).await;

    seed_test_subscription(
        ctx,
        &realm_id,
        user_id,
        subscription_id,
        &external_subscription_id,
        &plan_id.to_string(),
    )
    .await;

    grant_quota_entitlement_for_test(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
        QuotaSourceType::SubscriptionInitial,
        &subscription_id.to_string(),
        &[(2_592_000, 10_000, "period")],
        now - Duration::hours(1),
        Some(now + Duration::days(30)),
    )
    .await;

    // When: Cancel with cancel_at_period_end = true
    let event = build_cancel_event_for_subscription(
        event_id,
        user_id,
        true,
        &realm_id,
        &external_subscription_id,
    );

    let app = ctx.create_unified_test_router();
    let response = send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;

    // Then
    assert_webhook_success(&response);

    let entitlements =
        get_user_quota_entitlements(ctx, user_id, CreditType::SubscriptionCredit).await;
    assert_eq!(
        entitlements.len(),
        1,
        "Exactly one subscription entitlement should exist"
    );

    let entitlement = &entitlements[0];
    assert_eq!(
        entitlement.status,
        QuotaEntitlementStatus::Active,
        "Default cancel should leave the entitlement active"
    );
    assert_eq!(
        entitlement.quota_windows.first().map(|w| w.limit),
        Some(10_000),
        "Quota limit should remain unchanged"
    );

    assert_derived_balance(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        10_000,
    )
    .await;
}

// ============================================================================
// Test 2: Immediate Cancel - Revoke Unused Points
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-BI-011 场景 2 - 立即取消回收未使用会员积分
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_subscription_cancel_immediate_revokes_unused(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user2@example.com").await;
    let plan_id = Uuid::now_v7();
    let subscription_id = Uuid::now_v7();
    let external_subscription_id = format!("sub_{}", subscription_id);
    let event_id = generate_test_event_id();
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;
    let now = Utc::now();

    ctx.with_creem_config(&realm_id, None, None, None).await;
    setup_test_plan_config(ctx, &realm_id, plan_id).await;
    create_points_wallet(ctx, user_id, &realm_id).await;

    seed_test_subscription(
        ctx,
        &realm_id,
        user_id,
        subscription_id,
        &external_subscription_id,
        &plan_id.to_string(),
    )
    .await;

    seed_attributed_subscription_quota(
        ctx,
        &realm_id,
        user_id,
        subscription_id,
        bucket_id,
        CreditType::SubscriptionCredit,
        QuotaSourceType::SubscriptionInitial,
        &[(2_592_000, 10_000, "period")],
        now - Duration::hours(1),
        Some(now + Duration::days(30)),
    )
    .await;

    // When: Cancel immediately (cancel_at_period_end = false)
    let event = build_cancel_event_for_subscription(
        event_id,
        user_id,
        false,
        &realm_id,
        &external_subscription_id,
    );

    let app = ctx.create_unified_test_router();
    let response = send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;

    // Then
    assert_webhook_success(&response);

    let entitlements =
        get_user_quota_entitlements(ctx, user_id, CreditType::SubscriptionCredit).await;
    assert_eq!(entitlements.len(), 1);

    let entitlement = &entitlements[0];
    assert_eq!(
        entitlement.status,
        QuotaEntitlementStatus::Revoked,
        "Immediate cancel should revoke the entitlement"
    );
    assert!(
        entitlement.effective_until.is_some(),
        "Revoke should set effective_until"
    );

    assert_derived_balance(ctx, user_id, &realm_id, CreditType::SubscriptionCredit, 0).await;
}

// ============================================================================
// Test 3: Cancel Event Idempotency
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-BI-011 场景 4 - 取消事件幂等性
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_subscription_cancel_idempotency(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user3@example.com").await;
    let plan_id = Uuid::now_v7();
    let subscription_id = Uuid::now_v7();
    let external_subscription_id = format!("sub_{}", subscription_id);
    let event_id = generate_test_event_id();
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;
    let now = Utc::now();

    ctx.with_creem_config(&realm_id, None, None, None).await;
    setup_test_plan_config(ctx, &realm_id, plan_id).await;
    create_points_wallet(ctx, user_id, &realm_id).await;

    seed_test_subscription(
        ctx,
        &realm_id,
        user_id,
        subscription_id,
        &external_subscription_id,
        &plan_id.to_string(),
    )
    .await;

    seed_attributed_subscription_quota(
        ctx,
        &realm_id,
        user_id,
        subscription_id,
        bucket_id,
        CreditType::SubscriptionCredit,
        QuotaSourceType::SubscriptionInitial,
        &[(2_592_000, 10_000, "period")],
        now - Duration::hours(1),
        Some(now + Duration::days(30)),
    )
    .await;

    let event = build_cancel_event_for_subscription(
        event_id.clone(),
        user_id,
        false,
        &realm_id,
        &external_subscription_id,
    );

    let app = ctx.create_unified_test_router();

    // When: First processing
    let response1 =
        send_webhook_with_signature(&app, &realm_id, event.clone(), "test_webhook_secret").await;
    assert_webhook_success(&response1);

    // When: Second processing (same event_id)
    let response2 =
        send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;
    assert_webhook_success(&response2);

    // Then: Entitlement should still be revoked and there should be exactly one row
    let entitlements =
        get_user_quota_entitlements(ctx, user_id, CreditType::SubscriptionCredit).await;
    assert_eq!(
        entitlements.len(),
        1,
        "Redelivery must not create additional entitlement rows"
    );

    let entitlement = &entitlements[0];
    assert_eq!(
        entitlement.status,
        QuotaEntitlementStatus::Revoked,
        "Entitlement should remain revoked after redelivery"
    );

    assert_derived_balance(ctx, user_id, &realm_id, CreditType::SubscriptionCredit, 0).await;
}
