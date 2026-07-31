// =============================================================================
// Test: Subscription Upgrade Webhook
// =============================================================================
//
// Tests for subscription.update webhook events (upgrades) under the quota
// entitlement model (points_quota_entitlements).
//
// User Story: docs/user-stories/points-billing-events.md
// Covers: US-BI-009 (Subscription upgrade grants difference points)
//
// =============================================================================

use crate::tests::helpers::points_helpers::*;
use crate::tests::helpers::subscription_test_helpers::*;
use crate::tests::helpers::webhook_helpers::*;
use crate::tests::scenarios::points::fixtures::*;
use crate::tests::schema_test_context::SchemaTestContext;
use chrono::{DateTime, Duration, Utc};
use herald_core::domain::points::entities::{CreditType, QuotaEntitlementStatus, QuotaSourceType};
use test_context::test_context;
use uuid::Uuid;

/// Seed a rule-attributed `subscription_initial` quota entitlement standing in
/// for the user's current basic-plan grant.
///
/// The upgrade revoke half (`revoke_distribution_source_in_tx`) only matches
/// entitlements whose `distribution_rule_id IS NOT NULL` (it joins
/// `points_distribution_events` on `distribution_event_id`). A raw
/// `grant_quota_entitlement_for_test` row has NULL attribution and is silently
/// skipped, so the test would see basic NOT revoked. This helper mirrors what a
/// production initial fulfillment would write — a `subscription_initial` rule
/// owned by the basic mapping, a completed distribution event keyed
/// `subscription:<sub_id>:period:<...>` (the revoke's `event_key LIKE` target),
/// and the entitlement row carrying both attribution columns — so the upgrade
/// revoke correctly revokes the basic entitlement.
#[allow(clippy::too_many_arguments)]
async fn seed_rule_attributed_initial_entitlement(
    ctx: &SchemaTestContext,
    realm_id: &str,
    user_id: Uuid,
    subscription_id: Uuid,
    mapping_id: Uuid,
    bucket_id: Uuid,
    limit: i64,
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
) {
    let rule_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO points_distribution_rules
            (id, realm_id, owner_type, entitlement_mapping_id, bucket_id,
             trigger_sources, grant_mode, validity_days, quota_windows,
             enabled, display_order)
         VALUES ($1, $2, 'entitlement_mapping', $3, $4, $5, 'quota', 0, $6, true, 0)",
    )
    .bind(rule_id)
    .bind(realm_id)
    .bind(mapping_id)
    .bind(bucket_id)
    .bind(&["subscription_initial"][..])
    .bind(serde_json::json!([{"windowSeconds": 2_592_000, "limit": limit, "key": "period"}]))
    .execute(&ctx.app_state.pool)
    .await
    .expect("seed basic subscription_initial rule");

    let event_id = Uuid::now_v7();
    let event_key = format!(
        "subscription:{subscription_id}:period:{}",
        period_start.to_rfc3339()
    );
    sqlx::query(
        "INSERT INTO points_distribution_events
            (id, realm_id, user_id, trigger, event_key, source_id,
             owner_type, entitlement_mapping_id, status, result_count,
             completed_at, created_at)
         VALUES ($1, $2, $3, 'subscription_initial', $4, $5,
                 'entitlement_mapping', $6, 'completed', 1, NOW(), NOW())",
    )
    .bind(event_id)
    .bind(realm_id)
    .bind(user_id)
    .bind(&event_key)
    .bind(subscription_id.to_string())
    .bind(mapping_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("seed basic subscription_initial distribution event");

    let entitlement_id = Uuid::now_v7();
    sqlx::query(
        r#"INSERT INTO points_quota_entitlements
             (id, user_id, realm_id, bucket_id, credit_type, source_type, source_id,
              quota_windows, effective_from, effective_until, status, idempotency_key,
              distribution_event_id, distribution_rule_id, created_at, updated_at)
           VALUES ($1, $2, $3, $4, 'subscription_credit', 'subscription_initial', $5,
                   $6, $7, $8, 'active', $9, $10, $11, NOW(), NOW())"#,
    )
    .bind(entitlement_id)
    .bind(user_id)
    .bind(realm_id)
    .bind(bucket_id)
    .bind(subscription_id.to_string())
    .bind(serde_json::json!([{"windowSeconds": 2_592_000, "limit": limit, "key": "period"}]))
    .bind(period_start)
    .bind(period_end)
    .bind(format!("subscription:{subscription_id}:initial"))
    .bind(event_id)
    .bind(rule_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("seed rule-attributed basic subscription_initial entitlement");
}

// ============================================================================
// Test 1: Upgrade Grants Difference Points
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-BI-009 场景 1 - 升级立即补发差额积分
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_subscription_upgrade_grants_difference(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user1@example.com").await;
    let basic_plan_id = Uuid::now_v7();
    let premium_plan_id = Uuid::now_v7();
    let period_end = Utc::now() + Duration::days(30);

    // Configure Creem webhook for this realm
    ctx.with_creem_config(&realm_id, None, None, None).await;

    // Setup plan configs for the test
    setup_test_plan_config_with_points(ctx, &realm_id, basic_plan_id, 5000).await;
    setup_test_plan_config_with_points(ctx, &realm_id, premium_plan_id, 10000).await;

    create_points_wallet(ctx, user_id, &realm_id).await;

    // Pre-create the subscription so we know the internal subscription_id used
    // by the upgrade handler as the entitlement source_id.
    let subscription_id = create_test_subscription_with_entitlement_key(
        ctx,
        &realm_id,
        Uuid::nil(),
        &basic_plan_id.to_string(),
        "",
        "creem",
        "active",
    )
    .await;
    let event_id = format!("test_{}", subscription_id);

    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;
    let premium_rule_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO points_distribution_rules
            (id, realm_id, owner_type, entitlement_mapping_id, bucket_id,
             trigger_sources, grant_mode, validity_days, quota_windows,
             enabled, display_order)
         VALUES ($1, $2, 'entitlement_mapping', $3, $4, $5, 'quota', 0, $6, true, 0)",
    )
    .bind(premium_rule_id)
    .bind(&realm_id)
    .bind(premium_plan_id)
    .bind(bucket_id)
    .bind(&["subscription_upgrade"][..])
    .bind(serde_json::json!([{"windowSeconds": 2_592_000, "limit": 10000, "key": "period"}]))
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to seed premium mapping subscription_upgrade quota rule");

    // User currently has Basic Plan (5000 points) as an active quota entitlement
    // (rule-attributed so the upgrade revoke half can match + revoke it).
    seed_rule_attributed_initial_entitlement(
        ctx,
        &realm_id,
        user_id,
        subscription_id,
        basic_plan_id,
        bucket_id,
        5000,
        Utc::now() - Duration::days(1),
        period_end,
    )
    .await;

    // When: User upgrades to Premium Plan (10000 points)
    //
    // Use the new plan's own external_product_id (prod_test_<premium_plan_id>)
    // so the price-aware webhook resolver lands on the premium mapping instead
    // of colliding on the shared `prod_test_monthly` product that both plans
    // register via setup_test_plan_config_with_points.
    let event = build_subscription_updated_event_with_product(
        event_id,
        user_id,
        basic_plan_id,
        premium_plan_id,
        &realm_id,
        &format!("prod_test_{}", premium_plan_id),
    );

    let app = ctx.create_unified_test_router();
    let response = send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;

    // Then
    assert_webhook_success(&response);

    // Should have exactly 2 entitlement rows: basic (revoked) + premium (active)
    let entitlements =
        get_user_quota_entitlements(ctx, user_id, CreditType::SubscriptionCredit).await;
    assert_eq!(
        entitlements.len(),
        2,
        "Should have revoked basic and active premium entitlements"
    );
    assert_eq!(
        count_subscription_quota_entitlements(ctx, user_id).await,
        2,
        "Total subscription entitlement rows should be 2"
    );

    // Active total limit should equal the premium window limit
    let active_total =
        get_total_quota_limit_by_type(ctx, user_id, CreditType::SubscriptionCredit).await;
    assert_eq!(
        active_total, 10000,
        "Active quota limit should be premium limit"
    );

    // Basic entitlement should be revoked
    let basic_entitlement = entitlements
        .iter()
        .find(|e| e.source_type == QuotaSourceType::SubscriptionInitial)
        .expect("Basic entitlement should exist");
    assert_eq!(
        basic_entitlement.status,
        QuotaEntitlementStatus::Revoked,
        "Basic entitlement should be revoked after upgrade"
    );

    // Premium entitlement should be active and sourced from the upgrade
    let premium_entitlement = entitlements
        .iter()
        .find(|e| e.source_type == QuotaSourceType::SubscriptionUpgrade)
        .expect("Premium upgrade entitlement should exist");
    assert_eq!(
        premium_entitlement.status,
        QuotaEntitlementStatus::Active,
        "Premium upgrade entitlement should be active"
    );
    assert_eq!(
        premium_entitlement
            .quota_windows
            .first()
            .map(|w| w.limit)
            .unwrap_or(0),
        10000,
        "Premium window limit should be 10000"
    );

    // Window availability should reflect the new premium entitlement
    assert_derived_balance(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        10000,
    )
    .await;
}

/// Upgrade results are attributed per rule so old source results can be revoked across wallets.
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_multi_wallet_grant_rule_upgrade_revokes_old_and_grants_new_accounts(
    ctx: &mut SchemaTestContext,
) {
    super::multi_wallet_grant_rule_scenarios::assert_two_account_fixed_event(
        ctx,
        herald_core::domain::points::DistributionTrigger::SubscriptionUpgrade,
    )
    .await;
}

// ============================================================================
// Test 2: Subscription Upgrade Event Idempotency
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-BI-009 场景 - subscription.upgraded 幂等性，相同 event_id 不重复发放升级差价积分
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_subscription_upgrade_idempotency(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user2@example.com").await;
    let basic_plan_id = Uuid::now_v7();
    let premium_plan_id = Uuid::now_v7();
    let period_end = Utc::now() + Duration::days(30);

    // Configure Creem webhook for this realm
    ctx.with_creem_config(&realm_id, None, None, None).await;

    // Setup plan configs for the test
    setup_test_plan_config_with_points(ctx, &realm_id, basic_plan_id, 5000).await;
    setup_test_plan_config_with_points(ctx, &realm_id, premium_plan_id, 10000).await;

    create_points_wallet(ctx, user_id, &realm_id).await;

    // Pre-create the subscription so the upgrade handler uses a known source_id.
    let subscription_id = create_test_subscription_with_entitlement_key(
        ctx,
        &realm_id,
        Uuid::nil(),
        &basic_plan_id.to_string(),
        "",
        "creem",
        "active",
    )
    .await;
    let event_id = format!("test_{}", subscription_id);

    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;

    // Distribution-rules model: seed the premium mapping's upgrade rule so the
    // upgrade grants the 10000-limit quota entitlement (mirrors test 1; see
    // test 1 for the rationale).
    let premium_rule_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO points_distribution_rules
            (id, realm_id, owner_type, entitlement_mapping_id, bucket_id,
             trigger_sources, grant_mode, validity_days, quota_windows,
             enabled, display_order)
         VALUES ($1, $2, 'entitlement_mapping', $3, $4, $5, 'quota', 0, $6, true, 0)",
    )
    .bind(premium_rule_id)
    .bind(&realm_id)
    .bind(premium_plan_id)
    .bind(bucket_id)
    .bind(&["subscription_upgrade"][..])
    .bind(serde_json::json!([{"windowSeconds": 2_592_000, "limit": 10000, "key": "period"}]))
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to seed premium mapping subscription_upgrade quota rule");

    // User currently has Basic Plan (5000 points) as an active quota entitlement
    // (rule-attributed so the upgrade revoke half can match + revoke it).
    seed_rule_attributed_initial_entitlement(
        ctx,
        &realm_id,
        user_id,
        subscription_id,
        basic_plan_id,
        bucket_id,
        5000,
        Utc::now() - Duration::days(1),
        period_end,
    )
    .await;

    // Build upgrade event with a shared event_id/subscription_id. New plan's own
    // product id so the resolver selects the premium mapping.
    let event = build_subscription_updated_event_with_product(
        event_id,
        user_id,
        basic_plan_id,
        premium_plan_id,
        &realm_id,
        &format!("prod_test_{}", premium_plan_id),
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

    // Then: Should still have exactly 2 entitlement rows, not 3
    let entitlements =
        get_user_quota_entitlements(ctx, user_id, CreditType::SubscriptionCredit).await;
    assert_eq!(
        entitlements.len(),
        2,
        "Should have original and exactly one upgrade entitlement"
    );
    assert_eq!(
        count_subscription_quota_entitlements(ctx, user_id).await,
        2,
        "Total subscription entitlement rows should remain 2 after retry"
    );

    // Verify only one upgrade entitlement was created
    let upgrade_entitlements: Vec<_> = entitlements
        .iter()
        .filter(|e| e.source_type == QuotaSourceType::SubscriptionUpgrade)
        .collect();
    assert_eq!(
        upgrade_entitlements.len(),
        1,
        "Should not duplicate upgrade entitlement on retry"
    );

    // Active total limit should remain the premium limit
    let active_total =
        get_total_quota_limit_by_type(ctx, user_id, CreditType::SubscriptionCredit).await;
    assert_eq!(
        active_total, 10000,
        "Active quota limit should remain premium limit after retry"
    );
}
