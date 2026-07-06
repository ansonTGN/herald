// =============================================================================
// Test: Subscription Paid Webhook
// =============================================================================
//
// Tests for subscription.paid webhook events (initial subscription and renewals)
// under the window-quota model.
//
// User Story: docs/user-stories/points-billing-events.md
// Covers: US-PO-06 (Subscription grants and renewals)
//
// Under the quota model, subscription.paid creates ONE PointsQuotaEntitlement
// row per (subscription, period). There are no points_credit_ledger rows, no
// points_grant_schedules, no points_grant_records, and no chained next-period
// pre-grants.
//
// =============================================================================

use crate::tests::helpers::points_helpers::*;
use crate::tests::helpers::webhook_helpers::*;
use crate::tests::scenarios::points::fixtures::*;
use crate::tests::schema_test_context::SchemaTestContext;
use herald_core::domain::billing::BillingRepository;
use herald_core::domain::points::entities::{CreditType, QuotaEntitlementStatus, QuotaSourceType};
use test_context::test_context;
use uuid::Uuid;

/// Resolve the price-level EntitlementMapping for a key in these scenarios.
///
/// The price-level mapping refactor changed `handle_subscription_paid` to consume
/// the price-level mapping directly. These scenarios seed a single mapping per
/// entitlement_key, so resolving by key is identity-equivalent to the price-level
/// mapping the webhook path resolves.
async fn mapping_for_key(
    ctx: &SchemaTestContext,
    realm_id: &str,
    key: &str,
) -> herald_core::domain::billing::entities::EntitlementMapping {
    ctx.app_state
        .billing_repository
        .find_entitlement_mapping_by_key(realm_id, key)
        .await
        .unwrap_or_else(|_| panic!("mapping for key '{key}' should exist"))
        .unwrap_or_else(|| panic!("mapping for key '{key}' should be Some"))
}

/// Seed a `subscription` row bound to the realm's legacy test bucket and
/// return its id. Used by subscription tests so the `subscription_id` and
/// the grant target `bucket_id` are known ahead of the service call.
async fn seed_subscription_row(
    ctx: &mut SchemaTestContext,
    user_id: Uuid,
    realm_id: &str,
    entitlement_key: &str,
) -> Uuid {
    use crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm;
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, realm_id).await;
    let subscription_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO subscription
            (id, realm_id, user_id, status, entitlement_key,
             external_subscription_id, external_product_id, payment_provider,
             current_period_start, current_period_end, cancel_at_period_end,
             bucket_id, created_at, updated_at)
         VALUES ($1, $2, $3, 'active', $4, $5, $6, 'creem',
                 NOW(), NOW() + INTERVAL '30 days', false, $7, NOW(), NOW())",
    )
    .bind(subscription_id)
    .bind(realm_id)
    .bind(user_id)
    .bind(entitlement_key)
    .bind(format!("sub_be_t04_{}", subscription_id))
    .bind(format!("prod_be_t04_{}", entitlement_key))
    .bind(bucket_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to seed subscription row");
    subscription_id
}

// ============================================================================
// Test 1: Initial Subscription Grant
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-PO-06 场景 - Initial subscription grants points
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_subscription_paid_initial_grant(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user1@example.com").await;
    let plan_id = Uuid::now_v7();
    let event_id = generate_test_event_id();

    // Configure Creem webhook for this realm
    ctx.with_creem_config(&realm_id, None, None, None).await;

    // Setup plan config for the test
    setup_test_plan_config(ctx, &realm_id, plan_id).await;

    // Create points account for user
    create_points_wallet(ctx, user_id, &realm_id).await;

    // Build subscription.paid event (is_renewal = false)
    let event = build_subscription_paid_event(
        event_id, user_id, plan_id, false, // initial subscription
        &realm_id,
    );

    // When
    let app = ctx.create_unified_test_router();
    let response = send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;

    // Then
    assert_webhook_success(&response);

    // Verify subscription_credit quota entitlement was created
    let entitlements =
        get_user_quota_entitlements(ctx, user_id, CreditType::SubscriptionCredit).await;
    assert_eq!(
        entitlements.len(),
        1,
        "Should create one subscription quota entitlement"
    );

    let entitlement = &entitlements[0];
    assert_eq!(entitlement.credit_type, CreditType::SubscriptionCredit);
    assert_eq!(
        entitlement.source_type,
        QuotaSourceType::SubscriptionInitial
    );
    assert_eq!(
        entitlement.quota_windows.len(),
        1,
        "Should have one quota window"
    );
    assert_eq!(
        entitlement.quota_windows[0].limit,
        1000, // Amount from setup_test_plan_config
        "Window limit should equal points_per_period"
    );
    assert_eq!(entitlement.status, QuotaEntitlementStatus::Active);
    assert!(
        entitlement.effective_until.is_some(),
        "Subscription entitlement should have effective_until"
    );
}

// ============================================================================
// Test 2: Subscription Renewal
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-PO-06 场景 - Subscription renewal grants points
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_subscription_paid_renewal_grant(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user2@example.com").await;
    let plan_id = Uuid::now_v7();
    let event_id = generate_test_event_id();

    // Configure Creem webhook for this realm
    ctx.with_creem_config(&realm_id, None, None, None).await;

    // Setup plan config for the test
    setup_test_plan_config(ctx, &realm_id, plan_id).await;

    create_points_wallet(ctx, user_id, &realm_id).await;

    // Build subscription.paid event (is_renewal = true)
    let event = build_subscription_paid_event(
        event_id, user_id, plan_id, true, // renewal
        &realm_id,
    );

    // When
    let app = ctx.create_unified_test_router();
    let response = send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;

    // Then
    assert_webhook_success(&response);

    // Verify subscription_credit quota entitlement was created with renewal source type
    let entitlements =
        get_user_quota_entitlements(ctx, user_id, CreditType::SubscriptionCredit).await;
    assert_eq!(entitlements.len(), 1);

    let entitlement = &entitlements[0];
    assert_eq!(entitlement.credit_type, CreditType::SubscriptionCredit);
    assert_eq!(
        entitlement.source_type,
        QuotaSourceType::SubscriptionRenewal
    );
    assert_eq!(
        entitlement.quota_windows[0].limit, 1000,
        "Window limit should equal points_per_period"
    );
    assert_eq!(entitlement.status, QuotaEntitlementStatus::Active);
}

// ============================================================================
// Test 3: Subscription Paid Event Idempotency
// ============================================================================

// User Story: docs/user-stories/points-billing-events.md
// Covers: US-PO-06 场景 - subscription.paid 幂等性，相同 event_id 不重复发放积分
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_subscription_paid_idempotency(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(&ctx.app_state.pool, &realm_id, "user3@example.com").await;
    let plan_id = Uuid::now_v7();
    let event_id = generate_test_event_id();

    // Configure Creem webhook for this realm
    ctx.with_creem_config(&realm_id, None, None, None).await;

    // Setup plan config for the test
    setup_test_plan_config(ctx, &realm_id, plan_id).await;

    create_points_wallet(ctx, user_id, &realm_id).await;

    // Build subscription.paid event with a shared event_id
    let event = build_subscription_paid_event(
        event_id.clone(),
        user_id,
        plan_id,
        false, // initial subscription
        &realm_id,
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

    // Then: Should only create one subscription quota entitlement row
    let count = count_subscription_quota_entitlements(ctx, user_id).await;
    assert_eq!(count, 1, "Should not duplicate quota entitlement on retry");

    let entitlements =
        get_user_quota_entitlements(ctx, user_id, CreditType::SubscriptionCredit).await;
    let entitlement = &entitlements[0];
    assert_eq!(
        entitlement.quota_windows[0].limit, 1000,
        "Granted window limit should be exactly one plan allocation"
    );
    assert_eq!(entitlement.status, QuotaEntitlementStatus::Active);
}

// ============================================================================
// Subscription activation / renewal window-quota idempotency
// ============================================================================
//
// These tests exercise the period-aware `handle_subscription_paid` path
// directly with a pre-seeded `subscription` row. Direct service invocation
// lets the test bind a known `subscription_id` and assert on the resulting
// `points_quota_entitlements` rows deterministically.

/// User Story: US-PU-009 (use current-period points without distribution delay).
/// Covers (P0 — 订阅当前周期配额):
///   - Subscription activation grants the CURRENT period only.
///   - Derived available balance equals one period's worth.
///   - NO next-period pre-grant entitlement is written.
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_subscription_activation_grants_current_period_only(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(
        &ctx.app_state.pool,
        &realm_id,
        "be_t04_activate@example.com",
    )
    .await;

    let entitlement_key = format!("be-t04-act-{}", Uuid::now_v7());
    let points_per_period: i64 = 1000;

    setup_test_entitlement_mapping_for_webhook(
        ctx,
        &realm_id,
        "creem",
        &format!("prod_be_t04_{}", entitlement_key),
        &entitlement_key,
        points_per_period,
        true,
        true,
    )
    .await;

    let subscription_id = seed_subscription_row(ctx, user_id, &realm_id, &entitlement_key).await;
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;

    let now = chrono::Utc::now();
    let current_period_start = now - chrono::Duration::seconds(10);
    let current_period_end = now + chrono::Duration::days(30);

    // --- When: subscription activation fires handle_subscription_paid -------
    let mapping = mapping_for_key(ctx, &realm_id, &entitlement_key).await;
    let result = ctx
        .app_state
        .subscription_service
        .handle_subscription_paid(
            user_id,
            subscription_id,
            bucket_id,
            &realm_id,
            &mapping,
            false, // initial activation
            current_period_start,
            current_period_end,
            format!("evt_be_t04_act_{}", Uuid::now_v7()),
        )
        .await;
    assert!(result.is_ok(), "activation grant failed: {:?}", result);

    // --- Then: exactly one active entitlement for the current period ---------
    let entitlements =
        get_user_quota_entitlements(ctx, user_id, CreditType::SubscriptionCredit).await;
    assert_eq!(
        entitlements.len(),
        1,
        "activation should create exactly one current-period quota entitlement, got {}",
        entitlements.len()
    );

    let entitlement = &entitlements[0];
    assert_eq!(entitlement.credit_type, CreditType::SubscriptionCredit);
    assert_eq!(
        entitlement.source_type,
        QuotaSourceType::SubscriptionInitial
    );
    assert_eq!(entitlement.status, QuotaEntitlementStatus::Active);
    assert_eq!(
        entitlement.quota_windows.first().map(|w| w.limit),
        Some(points_per_period)
    );
    assert!(
        entitlement.effective_until.is_some(),
        "current-period entitlement should have effective_until"
    );

    assert_derived_balance(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        points_per_period,
    )
    .await;

    assert_eq!(
        count_subscription_quota_entitlements(ctx, user_id).await,
        1,
        "no next-period pre-grant should exist"
    );
}

/// User Story: US-PU-009 (renewal must not double-grant the same period).
/// Covers (P0 — 续费周期幂等): calling `handle_subscription_paid` twice with
/// the same `(subscription_id, period_start)` produces exactly one quota
/// entitlement row.
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_subscription_renewal_period_idempotency(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(
        &ctx.app_state.pool,
        &realm_id,
        "be_t04_period_idem@example.com",
    )
    .await;

    let entitlement_key = format!("be-t04-pi-{}", Uuid::now_v7());
    let points_per_period: i64 = 500;

    setup_test_entitlement_mapping_for_webhook(
        ctx,
        &realm_id,
        "creem",
        &format!("prod_be_t04_{}", entitlement_key),
        &entitlement_key,
        points_per_period,
        true,
        true,
    )
    .await;

    let subscription_id = seed_subscription_row(ctx, user_id, &realm_id, &entitlement_key).await;
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;

    let now = chrono::Utc::now();
    let period_start = now - chrono::Duration::seconds(10);
    let period_end = now + chrono::Duration::days(30);

    let mapping = mapping_for_key(ctx, &realm_id, &entitlement_key).await;
    let event_id = format!("evt_be_t04_pi_{}", Uuid::now_v7());

    // --- When: first renewal for this period --------------------------------
    let result1 = ctx
        .app_state
        .subscription_service
        .handle_subscription_paid(
            user_id,
            subscription_id,
            bucket_id,
            &realm_id,
            &mapping,
            true, // renewal
            period_start,
            period_end,
            event_id.clone(),
        )
        .await;
    assert!(
        result1.is_ok(),
        "first renewal should succeed: {:?}",
        result1
    );

    // --- Then: one entitlement exists ---------------------------------------
    let count1 = count_subscription_quota_entitlements(ctx, user_id).await;
    assert_eq!(
        count1, 1,
        "first renewal should create exactly one quota entitlement"
    );

    // --- When: same period is processed again -------------------------------
    let result2 = ctx
        .app_state
        .subscription_service
        .handle_subscription_paid(
            user_id,
            subscription_id,
            bucket_id,
            &realm_id,
            &mapping,
            true,
            period_start,
            period_end,
            event_id,
        )
        .await;
    assert!(
        result2.is_ok(),
        "duplicate renewal should be idempotent: {:?}",
        result2
    );

    // --- Then: still exactly one entitlement, still active ------------------
    let count2 = count_subscription_quota_entitlements(ctx, user_id).await;
    assert_eq!(
        count2, 1,
        "duplicate renewal must not create additional quota entitlement rows"
    );

    let entitlements =
        get_user_quota_entitlements(ctx, user_id, CreditType::SubscriptionCredit).await;
    let entitlement = &entitlements[0];
    assert_eq!(entitlement.status, QuotaEntitlementStatus::Active);
    assert_eq!(
        entitlement.source_type,
        QuotaSourceType::SubscriptionRenewal
    );
}

/// User Story: US-PU-009 (duplicate provider webhook delivery must not
/// double-grant).
/// Covers (P0 — provider event-level idempotency preserved):
/// when the SAME `event_id` is delivered twice, the webhook layer deduplicates
/// the delivery and does NOT re-enter `handle_subscription_paid`.
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_subscription_renewal_event_idempotency(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_test_user(
        &ctx.app_state.pool,
        &realm_id,
        "be_t04_event_idem@example.com",
    )
    .await;
    let plan_id = Uuid::now_v7();
    let event_id = generate_test_event_id();

    ctx.with_creem_config(&realm_id, None, None, None).await;
    setup_test_plan_config(ctx, &realm_id, plan_id).await;
    create_points_wallet(ctx, user_id, &realm_id).await;

    // Build a subscription.paid renewal webhook with explicit period bounds.
    let now = chrono::Utc::now();
    let period_start_str = now.to_rfc3339();
    let period_end_str = (now + chrono::Duration::days(30)).to_rfc3339();
    let base = build_subscription_paid_event(
        event_id.clone(),
        user_id,
        plan_id,
        true, // renewal
        &realm_id,
    );
    let mut event = base.clone();
    event["data"]["object"]["currentPeriodStart"] = serde_json::Value::String(period_start_str);
    event["data"]["object"]["currentPeriodEnd"] = serde_json::Value::String(period_end_str);

    let app = ctx.create_unified_test_router();

    // --- When: first webhook delivery ---------------------------------------
    let response1 =
        send_webhook_with_signature(&app, &realm_id, event.clone(), "test_webhook_secret").await;
    assert_webhook_success(&response1);

    let count_after_first = count_subscription_quota_entitlements(ctx, user_id).await;
    assert_eq!(
        count_after_first, 1,
        "first delivery should create exactly one quota entitlement"
    );

    // --- When: second webhook delivery (SAME event_id) ----------------------
    let response2 =
        send_webhook_with_signature(&app, &realm_id, event, "test_webhook_secret").await;
    assert_webhook_success(&response2);

    // --- Then: the duplicate delivery must NOT add any additional row -------
    let count_after_second = count_subscription_quota_entitlements(ctx, user_id).await;
    assert_eq!(
        count_after_first, count_after_second,
        "duplicate webhook event_id must not create additional entitlement rows"
    );

    // Verify the business idempotency key is anchored to (subscription, period).
    // The quota model stores this key on the entitlement row itself.
    let entitlements =
        get_user_quota_entitlements(ctx, user_id, CreditType::SubscriptionCredit).await;
    assert_eq!(entitlements.len(), 1);
    let entitlement = &entitlements[0];
    let expected_key = format!("sub:{}:period:{}", entitlement.source_id, now.timestamp());
    assert_eq!(
        entitlement.idempotency_key, expected_key,
        "entitlement idempotency key must be sub:{{subscription_id}}:period:{{period_start}}"
    );
}

/// User Story: US-PU-009 (no chained next-period pre-grant under window-quota).
/// Covers (P0 — 续费不预生成下一周期): after a renewal webhook hits, the service
/// creates exactly one active entitlement for the current period and nothing
/// else.
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_subscription_renewal_does_not_pregrant_next_period(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id =
        create_test_user(&ctx.app_state.pool, &realm_id, "be_t04_chain@example.com").await;

    let entitlement_key = format!("be-t04-chain-{}", Uuid::now_v7());
    let points_per_period: i64 = 800;

    setup_test_entitlement_mapping_for_webhook(
        ctx,
        &realm_id,
        "creem",
        &format!("prod_be_t04_{}", entitlement_key),
        &entitlement_key,
        points_per_period,
        true,
        true,
    )
    .await;

    let subscription_id = seed_subscription_row(ctx, user_id, &realm_id, &entitlement_key).await;
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;

    let now = chrono::Utc::now();
    let current_period_start = now - chrono::Duration::seconds(10);
    let current_period_end = now + chrono::Duration::days(30);

    // --- When: renewal webhook fires ----------------------------------------
    let mapping = mapping_for_key(ctx, &realm_id, &entitlement_key).await;
    let result = ctx
        .app_state
        .subscription_service
        .handle_subscription_paid(
            user_id,
            subscription_id,
            bucket_id,
            &realm_id,
            &mapping,
            true, // renewal
            current_period_start,
            current_period_end,
            format!("evt_be_t04_chain_{}", Uuid::now_v7()),
        )
        .await;
    assert!(result.is_ok(), "renewal grant failed: {:?}", result);

    // --- Then: exactly one active entitlement for the current period --------
    let entitlements =
        get_user_quota_entitlements(ctx, user_id, CreditType::SubscriptionCredit).await;
    assert_eq!(
        entitlements.len(),
        1,
        "renewal should create exactly one current-period quota entitlement"
    );

    let entitlement = &entitlements[0];
    assert_eq!(entitlement.credit_type, CreditType::SubscriptionCredit);
    assert_eq!(
        entitlement.source_type,
        QuotaSourceType::SubscriptionRenewal
    );
    assert_eq!(entitlement.status, QuotaEntitlementStatus::Active);

    assert_eq!(
        count_subscription_quota_entitlements(ctx, user_id).await,
        1,
        "no next-period pre-grant should exist"
    );

    assert_derived_balance(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        points_per_period,
    )
    .await;
}
