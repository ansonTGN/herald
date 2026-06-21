// =============================================================================
// BE-T02 — Scenario Tests: fulfillment + subscription lifecycle bucket routing
// =============================================================================
//
// Covers design `.ai/design/credit-bucket.md`:
//   - §5.3 (grant/fulfillment Bucket routing)
//   - §5.5 (subscription lifecycle reclamation)
//   - §6.1 履约 / 订阅生命周期
//   - decision A7: in-flight attempt is NOT rerouted when the mapping's
//     bucket_id is changed after purchase.
//   - decision A8: routing source = `payment_attempt.bucket_id` snapshot
//     (taken at purchase creation); first fulfillment freezes
//     `subscription.bucket_id` from that snapshot; missing snapshot/subscription
//     bucket fails loud.
//
// All tests exercise the real production services via `ctx.app_state`:
//   - `purchase_service.create_payment_attempt` (HTTP path, scenario 1 only)
//   - `fulfillment_service.fulfill_subscription_purchase(&attempt, …)`
//   - `subscription_service.handle_subscription_paid / upgrade / cancel /
//     downgrade`
//   - `points_service.revoke_points_by_credit_type` (refund path, routed via
//     `subscription.bucket_id`)
//
// Per authoring rules: tests target the intended design contract. Where the
// landed production signature/behavior differs from the item's assumptions,
// the gap is recorded inline (`RUNTIME GAP`) and the test is written against
// the intended contract — the runner (BE-T06) will triage runtime failures.
//
// Authoritative runtime gaps surfaced by these tests:
//   1. (none expected at compile time — all targets below use stable,
//      already-landed production APIs from BE-D05/BE-D06.)
//
// =============================================================================

#![allow(clippy::too_many_arguments)]

use crate::tests::helpers::credit_bucket_helpers::{
    CreditBucketOpts, count_ledger_in_bucket, count_ledger_outside_bucket,
    create_test_credit_bucket, read_subscription_bucket, read_wallet_total_balance,
    sum_ledger_granted_in_bucket,
};
use crate::tests::scenarios::points::fixtures::{
    create_test_client_app, create_test_user, create_test_user_with_auth,
};
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use herald_core::domain::common::entities::app_errors::CoreError;
use herald_core::domain::payment_attempt::entities::PaymentAttempt;
use herald_core::domain::points::{
    entities::{CreditType, RevocationType},
    subscription_service::CancelMode,
};
use herald_core::domain::purchase::FulfillmentService;
use sqlx::PgPool;
use test_context::test_context;
use uuid::Uuid;

// =============================================================================
// Local SQL helpers — direct row construction for fulfillment scenarios
// =============================================================================

/// Create a `subscription`-billing entitlement mapping attached to a Bucket,
/// with `grant_on_subscribe = true`, a positive `points_per_period`, and
/// `billing_type = 'recurring'`. Returns the mapping_id.
async fn create_subscription_mapping_in_bucket(
    pool: &PgPool,
    realm_id: &str,
    entitlement_key: &str,
    bucket_id: Uuid,
    points_per_period: i64,
) -> Uuid {
    let mapping_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO provider_entitlement_mappings
            (id, realm_id, payment_provider, external_product_id, entitlement_key,
             billing_type, billing_period, points_per_period, grant_on_subscribe, enabled,
             bucket_id, created_at, updated_at)
         VALUES ($1, $2, 'stripe', $3, $4, 'recurring', 'monthly', $5, true, true, $6, NOW(), NOW())",
    )
    .bind(mapping_id)
    .bind(realm_id)
    .bind(format!("prod_{}", mapping_id))
    .bind(entitlement_key)
    .bind(points_per_period)
    .bind(bucket_id)
    .execute(pool)
    .await
    .expect("Failed to insert subscription entitlement mapping");
    mapping_id
}

/// Insert a `payment_attempts` row directly with a specific `bucket_id`
/// snapshot, `target_type = 'entitlement_mapping'` (the only value allowed by
/// migration 20260609_points_package_one_time.sql `chk_target_type`), and
/// `status = 'Succeeded'` (so fulfillment can proceed without another status
/// transition). Returns the attempt_id; use [`load_attempt`] to materialize the
/// `PaymentAttempt` for the fulfillment service.
async fn insert_attempt_with_bucket_snapshot(
    pool: &PgPool,
    realm_id: &str,
    user_id: Uuid,
    mapping_id: Uuid,
    bucket_id: Option<Uuid>,
) -> Uuid {
    let attempt_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO payment_attempts
            (id, realm_id, user_id, payment_provider, target_type, target_id,
             bucket_id, amount, currency, status, provider_reference,
             expires_at, created_at, updated_at)
         VALUES ($1, $2, $3, 'stripe', 'entitlement_mapping', $4,
                 $5, 999, 'USD', 'Succeeded', $6,
                 NOW() + INTERVAL '2 hours', NOW(), NOW())",
    )
    .bind(attempt_id)
    .bind(realm_id)
    .bind(user_id)
    .bind(mapping_id)
    .bind(bucket_id)
    .bind(format!("pi_test_{}", attempt_id))
    .execute(pool)
    .await
    .expect("Failed to insert payment_attempts row");
    attempt_id
}

/// Load a `PaymentAttempt` from the DB via the real service.
async fn load_attempt(ctx: &TestContext, attempt_id: Uuid) -> PaymentAttempt {
    ctx.app_state
        .payment_attempt_service
        .get_payment_attempt_by_id_only(attempt_id)
        .await
        .expect("payment attempt not found after insert")
}

/// Create a `subscription` row directly with `bucket_id = bucket` (non-null,
/// per the eager-binding contract), bypassing fulfillment (used by lifecycle
/// scenarios that start from an already-existing subscription).
async fn insert_subscription_in_bucket(
    pool: &PgPool,
    realm_id: &str,
    user_id: Uuid,
    client_app_id: Uuid,
    entitlement_key: &str,
    bucket_id: Uuid,
) -> Uuid {
    let subscription_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO subscription
            (id, realm_id, user_id, client_app_id, status, entitlement_key,
             external_price_id, external_subscription_id, external_product_id,
             payment_provider, current_period_start, current_period_end,
             cancel_at_period_end, bucket_id, created_at, updated_at)
         VALUES ($1, $2, $3, $4, 'active', $5,
                 'price_test', $6, $7, 'stripe',
                 NOW(), NOW() + INTERVAL '30 days',
                 false, $8, NOW(), NOW())",
    )
    .bind(subscription_id)
    .bind(realm_id)
    .bind(user_id)
    .bind(client_app_id)
    .bind(entitlement_key)
    .bind(format!("sub_test_{}", subscription_id))
    .bind(format!("prod_{}", entitlement_key))
    .bind(bucket_id)
    .execute(pool)
    .await
    .expect("Failed to insert subscription row");
    subscription_id
}

// =============================================================================
// Scenario 1 (REMOVED): mapping with bucket_id=NULL rejects purchase creation
// =============================================================================
//
// `provider_entitlement_mappings.bucket_id` is NOT NULL in the base schema
// (`20260607_product_reduce.sql`). A bucket-less mapping can therefore no longer exist,
// so the purchase-time runtime check — CoreError::EntitlementMappingNotAttachedToBucket
// (design A8, §5.3) — was removed from `resolve_target`. The invariant this
// scenario guarded ("a mapping without a credit bucket cannot be purchased") is
// now enforced structurally at the schema layer instead of at request time.

// =============================================================================
// Scenario 2: fulfillment grants to the attempt-snapshot Bucket (A8)
// =============================================================================

/// User Story: US-CB-004 (purchase Bucket plan), US-PA-003 (payment success
/// fulfillment).
/// Covers (BE-T02 scope):
///   - `fulfill_subscription_purchase` reads `attempt.bucket_id` snapshot (NOT
///     live `mapping.bucket_id`) and grants initial subscription credits to
///     that Bucket's pool (design A8 / §5.3).
///   - DB check: the new `points_credit_ledger.bucket_id` equals the snapshot
///     Bucket, and no ledger row leaks to any other Bucket.
#[test_context(TestContext)]
#[tokio::test]
async fn fulfillment_grants_to_attempt_snapshot_bucket(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();
    let pool = &ctx.app_state.pool;

    let user_id = create_test_user(pool, &realm_id, "cb_t02_fulfill_snapshot@example.com").await;

    // --- Given: a Bucket A and a subscription mapping attached to A. --------
    let bucket_a = create_test_credit_bucket(
        pool,
        &realm_id,
        CreditBucketOpts {
            name: Some("Snapshot Target".into()),
            bucket_key: Some("snapshot-target".into()),
            ..Default::default()
        },
    )
    .await;
    let entitlement_key = format!("cb-t02-snap-{}", Uuid::now_v7());
    let mapping_id =
        create_subscription_mapping_in_bucket(pool, &realm_id, &entitlement_key, bucket_a, 1_000)
            .await;

    // --- And: a Succeeded payment attempt whose snapshot bucket = A. --------
    let attempt_id =
        insert_attempt_with_bucket_snapshot(pool, &realm_id, user_id, mapping_id, Some(bucket_a))
            .await;
    let attempt = load_attempt(ctx, attempt_id).await;
    assert_eq!(attempt.bucket_id, bucket_a, "snapshot bucket = A");

    // --- When: fulfilling the attempt via the real fulfillment service. ----
    let provider_tx_id = format!("sub_snap_{}", attempt_id);
    let result = ctx
        .app_state
        .fulfillment_service
        .fulfill_subscription_purchase(&attempt, provider_tx_id.clone())
        .await;

    assert!(result.is_ok(), "fulfillment should succeed: {:?}", result);

    // --- Then: ledger row landed in Bucket A, with the right amount. --------
    let ledger_count_a =
        count_ledger_in_bucket(pool, user_id, bucket_a, "subscription_credit").await;
    assert_eq!(
        ledger_count_a, 1,
        "exactly one subscription_credit ledger in bucket A"
    );

    let balance_a =
        sum_ledger_granted_in_bucket(pool, user_id, bucket_a, "subscription_credit").await;
    assert_eq!(
        balance_a, 1_000,
        "granted amount == mapping.points_per_period"
    );

    // --- And: no ledger row leaked to any other Bucket. --------------------
    let leak_count = count_ledger_outside_bucket(pool, user_id, bucket_a).await;
    assert_eq!(leak_count, 0, "no ledger row in any other bucket");

    // --- And: subscription.bucket_id is frozen to the snapshot Bucket. ------
    let subscription_id = result
        .as_ref()
        .ok()
        .and_then(|r| r.subscription_id)
        .expect("fulfillment returned a subscription_id");
    let frozen_bucket = read_subscription_bucket(pool, subscription_id).await;
    assert_eq!(
        frozen_bucket, bucket_a,
        "subscription.bucket_id frozen to the attempt snapshot bucket"
    );
}

// =============================================================================
// Scenario 3: first fulfillment freezes subscription.bucket_id (A8)
// =============================================================================

/// User Story: US-CB-008 (subscription lifecycle by Bucket).
/// Covers (BE-T02 scope):
///   - After the first `fulfill_subscription_purchase`, `subscription.bucket_id`
///     is non-NULL and equals the attempt snapshot. This is the freeze event
///     that makes the subscription's lifecycle resolve deterministically to
///     one pool (design A8 / §5.5).
///   - Subsequent subscription lifecycle events (renewal/upgrade/cancel) read
///     this frozen value as their routing source — so a NULL here is a
///     data-integrity breach, not a valid state.
#[test_context(TestContext)]
#[tokio::test]
async fn fulfillment_freezes_subscription_bucket_on_first_renewal(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();
    let pool = &ctx.app_state.pool;

    let user_id = create_test_user(pool, &realm_id, "cb_t02_freeze@example.com").await;

    let bucket = create_test_credit_bucket(
        pool,
        &realm_id,
        CreditBucketOpts {
            name: Some("Freeze Bucket".into()),
            bucket_key: Some("freeze-bucket".into()),
            ..Default::default()
        },
    )
    .await;
    let entitlement_key = format!("cb-t02-freeze-{}", Uuid::now_v7());
    let mapping_id =
        create_subscription_mapping_in_bucket(pool, &realm_id, &entitlement_key, bucket, 500).await;

    let attempt_id =
        insert_attempt_with_bucket_snapshot(pool, &realm_id, user_id, mapping_id, Some(bucket))
            .await;
    let attempt = load_attempt(ctx, attempt_id).await;

    let result = ctx
        .app_state
        .fulfillment_service
        .fulfill_subscription_purchase(&attempt, format!("sub_freeze_{}", attempt_id))
        .await
        .expect("first fulfillment should succeed");

    let subscription_id = result.subscription_id.expect("subscription_id present");

    // --- Then: subscription.bucket_id equals the snapshot. The column is NOT
    // NULL by schema (eager binding), so the only meaningful assertion is the
    // value identity; the previous `is_some()` check is now tautological and
    // was removed.
    let frozen = read_subscription_bucket(pool, subscription_id).await;
    assert_eq!(
        frozen, bucket,
        "subscription.bucket_id frozen to the attempt snapshot bucket"
    );
    assert_eq!(
        attempt.bucket_id, frozen,
        "snapshot == frozen subscription bucket"
    );
}

// =============================================================================
// Scenario 4: A7 regression — mapping bucket change after purchase does not
// reroute an in-flight attempt
// =============================================================================

/// User Story: US-CB-003 (coverage-set / mapping changes affect only future
/// purchases), A7 (覆盖集变更不回溯).
/// Covers (BE-T02 scope):
///   - Purchase attempt is created with snapshot Bucket A.
///   - Mapping is then re-pointed to Bucket B.
///   - Fulfilling the in-flight attempt STILL grants to Bucket A (reads the
///     snapshot, NOT the live mapping).
///   - No credit leaks to Bucket B from this in-flight attempt.
#[test_context(TestContext)]
#[tokio::test]
async fn mapping_bucket_change_after_purchase_does_not_reroute_inflight_attempt(
    ctx: &mut TestContext,
) {
    let realm_id = ctx._realm_id.clone();
    let pool = &ctx.app_state.pool;

    let user_id =
        create_test_user_with_auth(pool, &realm_id, "cb_t02_a7@example.com", "pw123").await;

    // --- Given: two Buckets A and B. ---------------------------------------
    let bucket_a = create_test_credit_bucket(
        pool,
        &realm_id,
        CreditBucketOpts {
            name: Some("A7 Original".into()),
            bucket_key: Some("a7-original".into()),
            ..Default::default()
        },
    )
    .await;
    let bucket_b = create_test_credit_bucket(
        pool,
        &realm_id,
        CreditBucketOpts {
            name: Some("A7 Repoint Target".into()),
            bucket_key: Some("a7-repoint".into()),
            ..Default::default()
        },
    )
    .await;

    let entitlement_key = format!("cb-t02-a7-{}", Uuid::now_v7());
    // Mapping starts attached to Bucket A.
    let mapping_id =
        create_subscription_mapping_in_bucket(pool, &realm_id, &entitlement_key, bucket_a, 800)
            .await;

    // --- And: a Succeeded attempt snapshotting Bucket A. -------------------
    let attempt_id =
        insert_attempt_with_bucket_snapshot(pool, &realm_id, user_id, mapping_id, Some(bucket_a))
            .await;
    let attempt_snapshot = load_attempt(ctx, attempt_id).await;
    assert_eq!(attempt_snapshot.bucket_id, bucket_a);

    // --- When: the mapping is re-pointed to Bucket B AFTER the snapshot. ---
    sqlx::query(
        "UPDATE provider_entitlement_mappings SET bucket_id = $1, updated_at = NOW() WHERE id = $2",
    )
    .bind(bucket_b)
    .bind(mapping_id)
    .execute(pool)
    .await
    .expect("re-point mapping bucket");

    // Sanity: live mapping now points to B.
    let live_mapping_bucket: Option<Uuid> =
        sqlx::query_scalar("SELECT bucket_id FROM provider_entitlement_mappings WHERE id = $1")
            .bind(mapping_id)
            .fetch_one(pool)
            .await
            .expect("read mapping bucket");
    assert_eq!(
        live_mapping_bucket,
        Some(bucket_b),
        "live mapping now points to B"
    );

    // Reload the attempt so we read what fulfillment will see.
    let attempt = load_attempt(ctx, attempt_id).await;
    assert_eq!(
        attempt.bucket_id, bucket_a,
        "attempt snapshot is unchanged after mapping re-point"
    );

    // --- Then: fulfillment still grants to Bucket A (the snapshot). --------
    let _result = ctx
        .app_state
        .fulfillment_service
        .fulfill_subscription_purchase(&attempt, format!("sub_a7_{}", attempt_id))
        .await
        .expect("fulfillment should succeed (snapshot route)");

    let ledger_a = count_ledger_in_bucket(pool, user_id, bucket_a, "subscription_credit").await;
    assert_eq!(ledger_a, 1, "ledger row landed in snapshot bucket A");

    let ledger_b = count_ledger_in_bucket(pool, user_id, bucket_b, "subscription_credit").await;
    assert_eq!(
        ledger_b, 0,
        "no ledger row in bucket B (mapping re-point not retroactive)"
    );

    let balance_a =
        sum_ledger_granted_in_bucket(pool, user_id, bucket_a, "subscription_credit").await;
    assert_eq!(balance_a, 800, "granted amount to snapshot bucket A");
}

// =============================================================================
// Scenario 5: renewal grant lands in subscription.bucket_id pool (§5.5)
// =============================================================================

/// User Story: US-CB-008 (subscription lifecycle by Bucket), US-PU subscription
/// renewal.
/// Covers (BE-T02 scope):
///   - `handle_subscription_paid` (renewal path) grants to `subscription.bucket_id`
///     (design §5.5). The grant ledger row's `bucket_id` matches the
///     subscription's bound Bucket; no leak.
#[test_context(TestContext)]
#[tokio::test]
async fn subscription_paid_renews_to_same_bucket_pool(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();
    let pool = &ctx.app_state.pool;

    let user_id = create_test_user(pool, &realm_id, "cb_t02_renew@example.com").await;
    let client_app_id = create_test_client_app(pool, &realm_id).await;

    let bucket = create_test_credit_bucket(
        pool,
        &realm_id,
        CreditBucketOpts {
            name: Some("Renewal Bucket".into()),
            bucket_key: Some("renewal-bucket".into()),
            ..Default::default()
        },
    )
    .await;
    let entitlement_key = format!("cb-t02-renew-{}", Uuid::now_v7());
    // Mapping in the same bucket (so handle_subscription_paid can resolve
    // points policy by entitlement_key and route to subscription.bucket_id).
    let _mapping_id =
        create_subscription_mapping_in_bucket(pool, &realm_id, &entitlement_key, bucket, 750).await;

    // --- Given: a subscription already bound to the Bucket. ----------------
    let subscription_id = insert_subscription_in_bucket(
        pool,
        &realm_id,
        user_id,
        client_app_id,
        &entitlement_key,
        bucket,
    )
    .await;
    assert_eq!(
        read_subscription_bucket(pool, subscription_id).await,
        bucket
    );

    // --- When: a renewal grant fires (is_renewal = true). ------------------
    let period_end = chrono::Utc::now() + chrono::Duration::days(30);
    let event_id = format!("evt_renew_{}", Uuid::now_v7());
    let result = ctx
        .app_state
        .subscription_service
        .handle_subscription_paid(
            user_id,
            subscription_id,
            bucket,
            &realm_id,
            &entitlement_key,
            true, // is_renewal
            period_end,
            event_id,
        )
        .await;

    assert!(result.is_ok(), "renewal grant should succeed: {:?}", result);

    // --- Then: ledger row landed in subscription.bucket_id pool. -----------
    let ledger_count = count_ledger_in_bucket(pool, user_id, bucket, "subscription_credit").await;
    assert_eq!(
        ledger_count, 1,
        "renewal grant ledger in subscription bucket pool"
    );

    let balance = sum_ledger_granted_in_bucket(pool, user_id, bucket, "subscription_credit").await;
    assert_eq!(
        balance, 750,
        "renewal grant amount == mapping.points_per_period"
    );

    // --- And: no leak outside the subscription bucket pool. ----------------
    let leak = count_ledger_outside_bucket(pool, user_id, bucket).await;
    assert_eq!(
        leak, 0,
        "renewal grant did not leak outside the subscription bucket"
    );
}

// =============================================================================
// Scenario 6: upgrade revokes old + grants new within the same Bucket (§5.5)
// =============================================================================

/// User Story: US-CB-008 (subscription lifecycle by Bucket), US-PU upgrade.
/// Covers (BE-T02 scope):
///   - `handle_subscription_upgrade` revokes the old plan's subscription credits
///     and grants the new plan's credits, both routed to `subscription.bucket_id`
///     (design §5.5). No cross-pool leak: the revoke and the grant share one
///     Bucket.
#[test_context(TestContext)]
#[tokio::test]
async fn subscription_upgrade_revokes_old_and_grants_new_within_same_bucket(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();
    let pool = &ctx.app_state.pool;

    let user_id = create_test_user(pool, &realm_id, "cb_t02_upgrade@example.com").await;
    let client_app_id = create_test_client_app(pool, &realm_id).await;

    let bucket = create_test_credit_bucket(
        pool,
        &realm_id,
        CreditBucketOpts {
            name: Some("Upgrade Bucket".into()),
            bucket_key: Some("upgrade-bucket".into()),
            ..Default::default()
        },
    )
    .await;

    let old_key = format!("cb-t02-upg-old-{}", Uuid::now_v7());
    let new_key = format!("cb-t02-upg-new-{}", Uuid::now_v7());
    // Both mappings live in the same Bucket (upgrade is a same-pool swap).
    let _old_mapping =
        create_subscription_mapping_in_bucket(pool, &realm_id, &old_key, bucket, 400).await;
    let _new_mapping =
        create_subscription_mapping_in_bucket(pool, &realm_id, &new_key, bucket, 1_200).await;

    let subscription_id =
        insert_subscription_in_bucket(pool, &realm_id, user_id, client_app_id, &old_key, bucket)
            .await;

    // Seed the user with old-plan subscription credits in the SAME bucket so
    // the upgrade revoke has something to revoke.
    let period_end = chrono::Utc::now() + chrono::Duration::days(30);
    let seed_event = format!("evt_upg_seed_{}", Uuid::now_v7());
    ctx.app_state
        .subscription_service
        .handle_subscription_paid(
            user_id,
            subscription_id,
            bucket,
            &realm_id,
            &old_key,
            false,
            period_end,
            seed_event,
        )
        .await
        .expect("seed old-plan grant should succeed");

    let balance_after_seed =
        sum_ledger_granted_in_bucket(pool, user_id, bucket, "subscription_credit").await;
    assert_eq!(balance_after_seed, 400, "old-plan grant seeded");

    // --- When: upgrade old -> new within the same bucket. ------------------
    let result = ctx
        .app_state
        .subscription_service
        .handle_subscription_upgrade(user_id, bucket, &realm_id, &old_key, &new_key, period_end)
        .await;

    assert!(result.is_ok(), "upgrade should succeed: {:?}", result);

    // --- Then: old credits revoked, new credits granted, same bucket. ------
    let net_balance =
        sum_ledger_granted_in_bucket(pool, user_id, bucket, "subscription_credit").await;
    assert_eq!(
        net_balance, 1_200,
        "after upgrade the net subscription balance == new plan amount (old revoked, new granted) in the same bucket"
    );

    // --- And: no cross-pool leak. ------------------------------------------
    let leak = count_ledger_outside_bucket(pool, user_id, bucket).await;
    assert_eq!(
        leak, 0,
        "upgrade did not leak outside the subscription bucket"
    );
}

// =============================================================================
// Scenario 7: cancel revokes only the subscription bucket pool (§5.5)
// =============================================================================

/// User Story: US-CB-008, US-PU cancel.
/// Covers (BE-T02 scope):
///   - `handle_subscription_cancel` (ImmediateCancel) revokes subscription
///     credits routed to `subscription.bucket_id` only; an unrelated Bucket's
///     balance is untouched (no cross-pool revoke).
#[test_context(TestContext)]
#[tokio::test]
async fn subscription_cancel_revokes_only_subscription_bucket_pool(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();
    let pool = &ctx.app_state.pool;

    let user_id = create_test_user(pool, &realm_id, "cb_t02_cancel@example.com").await;
    let client_app_id = create_test_client_app(pool, &realm_id).await;

    let bucket_sub = create_test_credit_bucket(
        pool,
        &realm_id,
        CreditBucketOpts {
            name: Some("Cancel Sub Bucket".into()),
            bucket_key: Some("cancel-sub-bucket".into()),
            ..Default::default()
        },
    )
    .await;
    let bucket_other = create_test_credit_bucket(
        pool,
        &realm_id,
        CreditBucketOpts {
            name: Some("Cancel Other Bucket".into()),
            bucket_key: Some("cancel-other-bucket".into()),
            ..Default::default()
        },
    )
    .await;

    let entitlement_key = format!("cb-t02-cancel-{}", Uuid::now_v7());
    let _mapping =
        create_subscription_mapping_in_bucket(pool, &realm_id, &entitlement_key, bucket_sub, 600)
            .await;

    let subscription_id = insert_subscription_in_bucket(
        pool,
        &realm_id,
        user_id,
        client_app_id,
        &entitlement_key,
        bucket_sub,
    )
    .await;

    // Seed subscription credits in the subscription bucket AND unrelated
    // granted credits in another bucket. The cancel must only touch the
    // subscription bucket pool.
    let period_end = chrono::Utc::now() + chrono::Duration::days(30);
    let seed_event = format!("evt_cancel_seed_{}", Uuid::now_v7());
    ctx.app_state
        .subscription_service
        .handle_subscription_paid(
            user_id,
            subscription_id,
            bucket_sub,
            &realm_id,
            &entitlement_key,
            false,
            period_end,
            seed_event,
        )
        .await
        .expect("seed sub grant should succeed");

    // Grant 5_000 of GrantedCredit into the OTHER bucket — cancel must not
    // touch this.
    crate::tests::helpers::credit_bucket_helpers::admin_grant_to_bucket(
        ctx,
        &realm_id,
        user_id,
        bucket_other,
        5_000,
        None,
    )
    .await;

    let sub_balance_before =
        sum_ledger_granted_in_bucket(pool, user_id, bucket_sub, "subscription_credit").await;
    let other_balance_before =
        read_wallet_total_balance(pool, &realm_id, user_id, bucket_other).await;
    assert_eq!(sub_balance_before, 600, "subscription credits seeded");
    assert_eq!(other_balance_before, 5_000, "other-bucket credits seeded");

    // --- When: ImmediateCancel routed to subscription.bucket_id. -----------
    let result = ctx
        .app_state
        .subscription_service
        .handle_subscription_cancel(
            user_id,
            bucket_sub,
            &realm_id,
            CancelMode::ImmediateCancel,
            None,
            Some(&entitlement_key),
        )
        .await;

    assert!(result.is_ok(), "cancel should succeed: {:?}", result);
    let revoke_output = result.unwrap();
    assert!(
        revoke_output.total_revoked > 0,
        "cancel revoked unused subscription credits in the subscription bucket"
    );

    // --- Then: subscription bucket pool drained; other bucket untouched. ---
    let sub_balance_after =
        sum_ledger_granted_in_bucket(pool, user_id, bucket_sub, "subscription_credit").await;
    assert_eq!(
        sub_balance_after, 0,
        "subscription bucket pool fully drained by cancel"
    );
    let other_balance_after =
        read_wallet_total_balance(pool, &realm_id, user_id, bucket_other).await;
    assert_eq!(
        other_balance_after, 5_000,
        "other bucket pool NOT touched by cancel (no cross-pool revoke)"
    );
}

// =============================================================================
// Scenario 8: refund revokes only the subscription bucket pool (§5.5)
// =============================================================================

/// User Story: US-CB-008, US-PU refund.
/// Covers (BE-T02 scope):
///   - Refund (revoke by credit type) routed to `subscription.bucket_id` only;
///     design §5.5 "退款同上" maps to revoke routed to the subscription bucket.
///   - An unrelated Bucket's balance is NOT touched (no cross-pool leak).
#[test_context(TestContext)]
#[tokio::test]
async fn subscription_refund_revokes_only_subscription_bucket_pool(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();
    let pool = &ctx.app_state.pool;

    let user_id = create_test_user(pool, &realm_id, "cb_t02_refund@example.com").await;
    let client_app_id = create_test_client_app(pool, &realm_id).await;

    let bucket_sub = create_test_credit_bucket(
        pool,
        &realm_id,
        CreditBucketOpts {
            name: Some("Refund Sub Bucket".into()),
            bucket_key: Some("refund-sub-bucket".into()),
            ..Default::default()
        },
    )
    .await;
    let bucket_other = create_test_credit_bucket(
        pool,
        &realm_id,
        CreditBucketOpts {
            name: Some("Refund Other Bucket".into()),
            bucket_key: Some("refund-other-bucket".into()),
            ..Default::default()
        },
    )
    .await;

    let entitlement_key = format!("cb-t02-refund-{}", Uuid::now_v7());
    let _mapping =
        create_subscription_mapping_in_bucket(pool, &realm_id, &entitlement_key, bucket_sub, 900)
            .await;

    let subscription_id = insert_subscription_in_bucket(
        pool,
        &realm_id,
        user_id,
        client_app_id,
        &entitlement_key,
        bucket_sub,
    )
    .await;

    // Seed: subscription credits in bucket_sub + GrantedCredit in bucket_other.
    let period_end = chrono::Utc::now() + chrono::Duration::days(30);
    let seed_event = format!("evt_refund_seed_{}", Uuid::now_v7());
    ctx.app_state
        .subscription_service
        .handle_subscription_paid(
            user_id,
            subscription_id,
            bucket_sub,
            &realm_id,
            &entitlement_key,
            false,
            period_end,
            seed_event,
        )
        .await
        .expect("seed sub grant should succeed");

    crate::tests::helpers::credit_bucket_helpers::admin_grant_to_bucket(
        ctx,
        &realm_id,
        user_id,
        bucket_other,
        3_000,
        None,
    )
    .await;

    let sub_before =
        sum_ledger_granted_in_bucket(pool, user_id, bucket_sub, "subscription_credit").await;
    let other_before = read_wallet_total_balance(pool, &realm_id, user_id, bucket_other).await;
    assert_eq!(sub_before, 900);
    assert_eq!(other_before, 3_000);

    // --- When: refund revokes subscription_credit routed to the sub bucket.
    let result = ctx
        .app_state
        .points_service
        .revoke_points_by_credit_type(
            &realm_id,
            user_id,
            bucket_sub, // subscription.bucket_id — the refund routing source (§5.5)
            CreditType::SubscriptionCredit,
            RevocationType::RefundRevoke,
            "Subscription refund".to_string(),
        )
        .await;

    assert!(result.is_ok(), "refund revoke should succeed: {:?}", result);
    let revoke_output = result.unwrap();
    assert!(
        revoke_output.total_revoked > 0,
        "refund revoked subscription credits in the subscription bucket"
    );

    // --- Then: subscription bucket drained; other bucket NOT touched. ------
    let sub_after =
        sum_ledger_granted_in_bucket(pool, user_id, bucket_sub, "subscription_credit").await;
    assert_eq!(sub_after, 0, "subscription bucket drained by refund");

    let other_after = read_wallet_total_balance(pool, &realm_id, user_id, bucket_other).await;
    assert_eq!(
        other_after, 3_000,
        "other bucket NOT touched by refund (no cross-pool leak)"
    );
}

// =============================================================================
// Scenario 9: downgrade preserves current cycle; next cycle same pool (§5.5)
// =============================================================================

/// User Story: US-CB-008, US-PU downgrade.
/// Covers (BE-T02 scope):
///   - `handle_subscription_downgrade` does NOT revoke any current-cycle
///     balance (design §5.5); it only validates the entitlement keys and
///     records the intent. The next-cycle grant_schedule (routed via the same
///     `subscription.bucket_id`) uses the new entitlement.
///   - The current-cycle balance is unchanged immediately after the downgrade.
#[test_context(TestContext)]
#[tokio::test]
async fn subscription_downgrade_preserves_current_cycle(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();
    let pool = &ctx.app_state.pool;

    let user_id = create_test_user(pool, &realm_id, "cb_t02_downgrade@example.com").await;
    let client_app_id = create_test_client_app(pool, &realm_id).await;

    let bucket = create_test_credit_bucket(
        pool,
        &realm_id,
        CreditBucketOpts {
            name: Some("Downgrade Bucket".into()),
            bucket_key: Some("downgrade-bucket".into()),
            ..Default::default()
        },
    )
    .await;

    let old_key = format!("cb-t02-dg-old-{}", Uuid::now_v7());
    let new_key = format!("cb-t02-dg-new-{}", Uuid::now_v7());
    let _old_mapping =
        create_subscription_mapping_in_bucket(pool, &realm_id, &old_key, bucket, 1_000).await;
    let _new_mapping =
        create_subscription_mapping_in_bucket(pool, &realm_id, &new_key, bucket, 300).await;

    let subscription_id =
        insert_subscription_in_bucket(pool, &realm_id, user_id, client_app_id, &old_key, bucket)
            .await;

    // Seed current-cycle credits at the old-plan amount.
    let period_end = chrono::Utc::now() + chrono::Duration::days(30);
    let seed_event = format!("evt_dg_seed_{}", Uuid::now_v7());
    ctx.app_state
        .subscription_service
        .handle_subscription_paid(
            user_id,
            subscription_id,
            bucket,
            &realm_id,
            &old_key,
            false,
            period_end,
            seed_event,
        )
        .await
        .expect("seed old-plan grant should succeed");

    let balance_before =
        sum_ledger_granted_in_bucket(pool, user_id, bucket, "subscription_credit").await;
    assert_eq!(balance_before, 1_000, "current-cycle old-plan balance");

    // --- When: downgrade old -> new (same bucket). -------------------------
    let result = ctx
        .app_state
        .subscription_service
        .handle_subscription_downgrade(
            user_id,
            subscription_id,
            bucket,
            &realm_id,
            &old_key,
            &new_key,
        )
        .await;

    assert!(result.is_ok(), "downgrade should succeed: {:?}", result);

    // --- Then: current-cycle balance unchanged. ----------------------------
    let balance_after =
        sum_ledger_granted_in_bucket(pool, user_id, bucket, "subscription_credit").await;
    assert_eq!(
        balance_after, 1_000,
        "downgrade does not change current-cycle balance (next cycle uses new plan in the same pool)"
    );

    // --- And: no leak outside the subscription bucket. ---------------------
    let leak = count_ledger_outside_bucket(pool, user_id, bucket).await;
    assert_eq!(
        leak, 0,
        "downgrade did not leak outside the subscription bucket"
    );

    // --- And: the subscription stays bound to the same Bucket (next-cycle
    // grant_schedule will route to subscription.bucket_id per design §5.5). --
    assert_eq!(
        read_subscription_bucket(pool, subscription_id).await,
        bucket,
        "subscription stays bound to the same bucket for next-cycle routing"
    );
}

// =============================================================================
// Scenario 10: entitlement-mapping missing fails loud (graceful skip)
// =============================================================================
//
// History: this scenario originally forced `subscription.bucket_id = NULL` and
// asserted `CoreError::SubscriptionBucketNotResolved`. After the eager-binding
// migration `subscription.bucket_id` became NOT NULL (webhook path
// `resolve_bucket_id_for_entitlement` resolves the bucket at subscription
// creation), so the None-bucket fail-loud case can no longer be constructed.
// The column-level NOT NULL constraint now enforces the invariant this test
// used to guard at the service layer; the runtime fail-loud path
// (`SubscriptionBucketNotResolved`) is dead code that the production signature
// change has made unreachable.
//
// To preserve the test's underlying intent — "a renewal that cannot be resolved
// is rejected loudly and credits nothing" — the scenario now exercises the
// analogous graceful-skip precondition that the service STILL checks before
// any grant: a missing entitlement-mapping points policy. The Creem webhook
// handler relies on this `EntitlementMappingNotFound` result to skip the event
// without retrying or crediting any implicit pool (see `handle_subscription_paid`
// inline comment in `subscription_service.rs`).

/// User Story: US-CB-008 — fail-loud contract for an unresolvable renewal.
/// Covers (BE-T02 scope):
///   - A subscription bound to a valid Bucket, but whose `entitlement_key` has
///     NO `provider_entitlement_mappings` row (points policy missing).
///   - `handle_subscription_paid` returns `CoreError::EntitlementMappingNotFound`
///     BEFORE any grant is attempted; NO credit is written to any pool, and no
///     wallet row is created.
///   - This is the graceful-skip precondition the webhook handler relies on to
///     drop unreadable events without implicit crediting.
#[test_context(TestContext)]
#[tokio::test]
async fn subscription_with_unresolved_bucket_fails_loud(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();
    let pool = &ctx.app_state.pool;

    let user_id = create_test_user(pool, &realm_id, "cb_t02_unresolved@example.com").await;
    let client_app_id = create_test_client_app(pool, &realm_id).await;

    // A real, enabled Bucket — the subscription IS bound (eager binding contract).
    let bucket = create_test_credit_bucket(
        pool,
        &realm_id,
        CreditBucketOpts {
            name: Some("Bound Bucket".into()),
            bucket_key: Some("bound-bucket".into()),
            ..Default::default()
        },
    )
    .await;

    // Deliberately NO `create_subscription_mapping_in_bucket` call: the
    // entitlement_key below has no points policy, so the service cannot
    // resolve the grant and must fail loud.
    let entitlement_key = format!("cb-t02-unresolved-{}", Uuid::now_v7());

    // --- Given: a subscription bound to the Bucket but with no mapping. ----
    let subscription_id = insert_subscription_in_bucket(
        pool,
        &realm_id,
        user_id,
        client_app_id,
        &entitlement_key,
        bucket,
    )
    .await;
    assert_eq!(
        read_subscription_bucket(pool, subscription_id).await,
        bucket,
        "precondition: subscription is bound to the bucket (eager binding)"
    );

    // --- When: a renewal grant fires against the unmapped entitlement. -----
    let period_end = chrono::Utc::now() + chrono::Duration::days(30);
    let event_id = format!("evt_unresolved_{}", Uuid::now_v7());
    let result = ctx
        .app_state
        .subscription_service
        .handle_subscription_paid(
            user_id,
            subscription_id,
            bucket,
            &realm_id,
            &entitlement_key,
            true,
            period_end,
            event_id,
        )
        .await;

    // --- Then: domain fails loud with EntitlementMappingNotFound. ----------
    assert!(
        matches!(result, Err(CoreError::EntitlementMappingNotFound)),
        "expected EntitlementMappingNotFound for missing points policy, got {:?}",
        result
    );

    // --- And: no ledger row was written to ANY bucket (no implicit pool). --
    let ledger_count_all: i64 =
        sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM points_credit_ledger WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(pool)
            .await
            .expect("count ledger");
    assert_eq!(
        ledger_count_all, 0,
        "no ledger row written — fail loud prevents implicit-pool crediting"
    );

    // --- And: no wallet row was created for the user (no implicit pool). ---
    let wallet_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM points_wallets WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(pool)
            .await
            .expect("count wallets");
    assert_eq!(
        wallet_count, 0,
        "no wallet row created — fail loud prevents implicit-pool wallet creation"
    );
}
