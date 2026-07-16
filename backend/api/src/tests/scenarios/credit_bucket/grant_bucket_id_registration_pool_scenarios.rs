// =============================================================================
// Scenario Tests: explicit bucketId grant + registration pool resolution
// =============================================================================
//
// Covers design `.ai/design/credit-bucket.md`:
//   - (non-purchase grant Bucket target resolution)
//   - (`receives_registration_credits` + partial unique index
//     `uq_credit_buckets_registration_pool` — at most one per Realm)
//   - grant bucketId
//   - every grant carries an EXPLICIT `bucketId`; no implicit
//     resolution. SDK/admin grants missing `bucketId` → 400
//     `grant_bucket_required`. Registration / free-periodic grants resolve to
//     the Realm's single registration-pool Bucket; no marked Bucket → fail-safe
//     skip (no grant, no implicit pool). Registration pool and subscription
//     Bucket are independent — no cross-pool leak.
//
// All scenarios exercise the real production paths:
//   - HTTP: `POST /api/ext/points/{realmId}/grant` (SDK)
//   - HTTP: `POST /api/points/{realmId}/grant` (admin, api-points/grant.rs)
//   - Service: `RegistrationService::handle_user_registration`
//     (resolves pool via `RegistrationPoolResolver::resolve_registration_pool_bucket`)
//   - DB: `points_credit_ledger.bucket_id` / `points_grant_schedules.bucket_id`
//     / `credit_buckets.receives_registration_credits`
//
// Per authoring rules: tests target the intended design contract. Where the
// landed production signature/behavior differs from the item's assumptions,
// the gap is recorded inline (`RUNTIME GAP`) and the test is written against
// the intended contract — the runner will triage runtime failures.
//
// Authoritative runtime gaps surfaced by these tests:
//   1. (none expected at compile time — all targets below use stable,
//      already-landed production APIs.)
//
// =============================================================================

#![allow(clippy::too_many_arguments)]

use crate::tests::helpers::billing_helpers::setup_billing_admin_session;
use crate::tests::helpers::credit_bucket_helpers::{
    CreditBucketOpts, count_ledger_outside_bucket, create_test_credit_bucket,
    find_registration_pool_for_realm, grant_points_admin_with_bucket_via_api,
    grant_points_ext_with_bucket_via_api, mark_bucket_as_registration_pool,
    read_receives_registration_credits, read_wallet_total_balance, sum_ledger_granted_in_bucket,
    try_mark_registration_pool_raw,
};
use crate::tests::scenarios::points::fixtures::{
    create_test_api_key, create_test_client_app, create_test_user,
};
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use herald_core::domain::points::entities::CreditSourceType;
use serde_json::{Value, json};
use sqlx::PgPool;
use test_context::test_context;
use tower::ServiceExt;
use uuid::Uuid;

// =============================================================================
// Local SQL helpers
// =============================================================================

/// Upsert a `realm_default_configs` row (the registration-service input). The
/// registration service reads `registration_bonus_points`,
/// `free_periodic_points_amount`, `free_periodic_validity_days`, and
/// `free_periodic_grant_period_type` from this row before resolving the
/// registration pool Bucket.
async fn set_realm_default_config(
    pool: &PgPool,
    realm_id: &str,
    registration_bonus: i64,
    free_periodic_amount: i64,
    free_periodic_validity_days: i32,
    period_type: &str,
) {
    sqlx::query(
        r#"INSERT INTO realm_default_configs
             (realm_id, registration_bonus_points, free_periodic_points_amount,
              free_periodic_validity_days, free_periodic_grant_period_type)
           VALUES ($1, $2, $3, $4, $5)
           ON CONFLICT (realm_id) DO UPDATE SET
             registration_bonus_points     = EXCLUDED.registration_bonus_points,
             free_periodic_points_amount   = EXCLUDED.free_periodic_points_amount,
             free_periodic_validity_days   = EXCLUDED.free_periodic_validity_days,
             free_periodic_grant_period_type = EXCLUDED.free_periodic_grant_period_type"#,
    )
    .bind(realm_id)
    .bind(registration_bonus)
    .bind(free_periodic_amount)
    .bind(free_periodic_validity_days)
    .bind(period_type)
    .execute(pool)
    .await
    .expect("Failed to upsert realm_default_configs");
}

/// Read the first `points_grant_schedules.bucket_id` for a user (returns None
/// when no schedule row exists). Used to assert the periodic grant schedule
/// targets the registration pool Bucket.
async fn read_first_grant_schedule_bucket(pool: &PgPool, user_id: Uuid) -> Option<Uuid> {
    sqlx::query_scalar::<_, Option<Uuid>>(
        "SELECT bucket_id FROM points_grant_schedules
          WHERE user_id = $1 ORDER BY created_at ASC LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .expect("Failed to read grant_schedule.bucket_id")
    .flatten()
}

/// Extract the `code` field from an error JSON body, or None if absent.
fn error_code(body: &Option<Value>) -> Option<&str> {
    body.as_ref()
        .and_then(|v| v.get("code"))
        .and_then(|c| c.as_str())
}

// =============================================================================
// Scenario 1: SDK grant without bucketId → 400 grant_bucket_required
// =============================================================================

/// User Story: US-CB-007 (SDK grants must carry an explicit target Bucket).
/// Covers:
///   - SDK grant missing `bucketId` → 400
///     `grant_bucket_required`. No implicit bucket resolution is permitted.
///   - `backend/api-ext/src/points.rs::grant_points_ext` rejects with a
///     structured `grant_bucket_required` body before reaching the service.
#[test_context(TestContext)]
#[tokio::test]
async fn sdk_grant_without_bucket_id_returns_400_grant_bucket_required(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();
    let pool = &ctx.app_state.pool;

    let user_id = create_test_user(pool, &realm_id, "cb_t03_sdk_nobucket@example.com").await;
    let client_app_id = create_test_client_app(pool, &realm_id).await;
    let api_key = create_test_api_key(pool, &realm_id, client_app_id).await;

    // Sanity: the user has no bucket rows yet.
    assert_eq!(
        count_ledger_outside_bucket(pool, user_id, Uuid::nil()).await,
        0,
        "precondition: user has no ledger rows"
    );

    // --- When: SDK grant with NO bucketId. ---------------------------------
    let (status, body) = grant_points_ext_with_bucket_via_api(
        ctx,
        &realm_id,
        user_id,
        100,
        "promo grant",
        None,
        None, // no bucketId
        &api_key,
    )
    .await;

    // --- Then: 400 with code=grant_bucket_required. -----------------------
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "SDK grant without bucketId must be 400, got {}: {:?}",
        status,
        body
    );
    assert_eq!(
        error_code(&body),
        Some("grant_bucket_required"),
        "expected grant_bucket_required error code, got: {:?}",
        body
    );

    // --- And: no ledger row was written. ----------------------------------
    assert_eq!(
        count_ledger_outside_bucket(pool, user_id, Uuid::nil()).await,
        0,
        "no grant should land when bucketId is missing"
    );
}

// =============================================================================
// Scenario 2: SDK grant with a valid bucketId → lands in that pool
// =============================================================================

/// User Story: US-CB-007 (SDK grants route to the explicitly targeted Bucket).
/// Covers:
///   - SDK grant with a valid `bucketId` grants to that
///     exact pool; the response echoes `bucketId`.
///   - DB check: `points_credit_ledger.bucket_id` and the wallet total balance
///     reflect the targeted Bucket (no cross-pool leak).
#[test_context(TestContext)]
#[tokio::test]
async fn sdk_grant_with_bucket_id_grants_to_specified_pool(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();
    let pool = &ctx.app_state.pool;

    let user_id = create_test_user(pool, &realm_id, "cb_t03_sdk_withbucket@example.com").await;
    let client_app_id = create_test_client_app(pool, &realm_id).await;
    let api_key = create_test_api_key(pool, &realm_id, client_app_id).await;

    let target_bucket = create_test_credit_bucket(
        pool,
        &realm_id,
        CreditBucketOpts {
            name: Some("SDK Target Bucket".into()),
            bucket_key: Some(format!("sdk-target-{}", Uuid::now_v7())),
            ..Default::default()
        },
    )
    .await;

    // --- When: SDK grant with the explicit target bucketId. ---------------
    let (status, body) = grant_points_ext_with_bucket_via_api(
        ctx,
        &realm_id,
        user_id,
        250,
        "sdk grant to bucket",
        None,
        Some(target_bucket),
        &api_key,
    )
    .await;

    assert_eq!(
        status,
        StatusCode::OK,
        "SDK grant with valid bucketId should succeed, got {}: {:?}",
        status,
        body
    );

    // --- Then: response echoes bucketId. ----------------------------------
    let echoed = body
        .as_ref()
        .and_then(|v| v.get("bucketId"))
        .and_then(|b| b.as_str());
    assert_eq!(
        echoed,
        Some(target_bucket.to_string()).as_deref(),
        "response must echo the explicit bucketId, got: {:?}",
        body
    );

    // --- And: ledger + wallet reflect the targeted Bucket. ----------------
    let granted_granted =
        sum_ledger_granted_in_bucket(pool, user_id, target_bucket, "granted_credit").await;
    assert_eq!(
        granted_granted, 250,
        "SDK grant must land in the explicitly targeted pool"
    );

    let wallet_total = read_wallet_total_balance(pool, &realm_id, user_id, target_bucket).await;
    assert_eq!(
        wallet_total, 250,
        "wallet total_balance must reflect the SDK grant"
    );

    // No cross-pool leak.
    assert_eq!(
        count_ledger_outside_bucket(pool, user_id, target_bucket).await,
        0,
        "no ledger rows should land outside the targeted Bucket"
    );
}

// =============================================================================
// Scenario 3: admin grant without bucketId → rejected
// =============================================================================

/// User Story: US-CB-004 (admin grants must carry an explicit target Bucket).
/// Covers:
///   - admin (`POST /api/points/{realmId}/grant`) grant
///     missing `bucketId` → 400 `grant_bucket_required`. Admin grants have the
///     same explicit-bucketId contract as SDK grants.
///   - `backend/api-points/src/grant.rs::grant_points` rejects with a
///     structured `grant_bucket_required` body before reaching the service.
#[test_context(TestContext)]
#[tokio::test]
async fn admin_grant_without_bucket_id_rejected(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();

    // Admin session setup borrows ctx mutably; do it before any immutable
    // borrow of ctx.app_state.pool.
    let admin_token = setup_billing_admin_session(ctx, "cb_t03_admin_nobucket@example.com").await;

    let pool = &ctx.app_state.pool;
    let user_id = create_test_user(pool, &realm_id, "cb_t03_grant_target@example.com").await;

    // --- When: admin grant with NO bucketId. ------------------------------
    let (status, body) = grant_points_admin_with_bucket_via_api(
        ctx,
        &realm_id,
        user_id,
        500,
        "admin grant attempt",
        None,
        None, // no bucketId
        &admin_token,
    )
    .await;

    // --- Then: 400 with code=grant_bucket_required. ----------------------
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "admin grant without bucketId must be 400, got {}: {:?}",
        status,
        body
    );
    assert_eq!(
        error_code(&body),
        Some("grant_bucket_required"),
        "expected grant_bucket_required error code, got: {:?}",
        body
    );

    // --- And: no ledger row was written. ----------------------------------
    assert_eq!(
        count_ledger_outside_bucket(pool, user_id, Uuid::nil()).await,
        0,
        "no grant should land when bucketId is missing"
    );
}

// =============================================================================
// Scenario 4: registration grant lands in the marked registration pool
// =============================================================================

/// User Story: US-FU-001 (registration bonus lands in the Realm's registration
/// pool Bucket).
/// Covers:
///   - `handle_user_registration` resolves the Realm's
///     registration-pool Bucket (`receives_registration_credits = true`) via
///     `RegistrationPoolResolver::resolve_registration_pool_bucket` and grants
///     the registration bonus into that exact Bucket.
///   - DB check: the registration bonus lands in `points_credit_ledger` with
///     `credit_type = 'registration_credit'` and `bucket_id` = the marked pool.
#[test_context(TestContext)]
#[tokio::test]
async fn registration_grant_lands_in_marked_registration_pool(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();
    let pool = &ctx.app_state.pool;

    // Realm config: 800 registration bonus, no free periodic (isolated check).
    set_realm_default_config(pool, &realm_id, 800, 0, 0, "daily").await;

    // Mark exactly one Bucket as the registration pool.
    let pool_bucket = create_test_credit_bucket(
        pool,
        &realm_id,
        CreditBucketOpts {
            name: Some("Registration Pool".into()),
            bucket_key: Some(format!("reg-pool-{}", Uuid::now_v7())),
            ..Default::default()
        },
    )
    .await;
    mark_bucket_as_registration_pool(pool, pool_bucket).await;

    // Sanity: resolver-equivalent lookup sees the marked Bucket.
    assert_eq!(
        find_registration_pool_for_realm(pool, &realm_id).await,
        Some(pool_bucket),
        "precondition: marked registration pool must be resolvable"
    );

    let user_id = create_test_user(pool, &realm_id, "cb_t03_reg_pool@example.com").await;

    // --- When: registration handler runs for a new user. -----------------
    let result = ctx
        .app_state
        .registration_service
        .handle_user_registration(user_id, &realm_id)
        .await;
    assert!(result.is_ok(), "registration should succeed: {:?}", result);

    // --- Then: registration bonus lands in the marked pool Bucket. --------
    let registration_credit =
        sum_ledger_granted_in_bucket(pool, user_id, pool_bucket, "registration_credit").await;
    assert_eq!(
        registration_credit, 800,
        "registration bonus must land in the marked registration pool Bucket"
    );

    // No cross-pool leak (no ledger rows outside the marked pool).
    assert_eq!(
        count_ledger_outside_bucket(pool, user_id, pool_bucket).await,
        0,
        "registration bonus must not leak outside the marked pool"
    );
}

// =============================================================================
// Scenario 5: free periodic grant lands in the registration pool
// =============================================================================

/// User Story: US-FU-002 (free periodic grant schedule targets the Realm's
/// registration pool Bucket).
/// Covers:
///   - registration builds the periodic `points_grant_schedules`
///     row with `bucket_id` = the marked registration pool Bucket; the first
///     periodic grant (issued synchronously by `handle_user_registration`)
///     lands in that Bucket via the schedule's `bucket_id`.
///   - DB check: `points_grant_schedules.bucket_id` = marked pool; first
///     periodic grant lands as `free_periodic_credit` in that Bucket.
#[test_context(TestContext)]
#[tokio::test]
async fn free_periodic_grant_lands_in_registration_pool(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();
    let pool = &ctx.app_state.pool;

    // Realm config: 100 registration bonus + 30 free-periodic (daily, 7d valid).
    set_realm_default_config(pool, &realm_id, 100, 30, 7, "daily").await;

    let pool_bucket = create_test_credit_bucket(
        pool,
        &realm_id,
        CreditBucketOpts {
            name: Some("Periodic Pool".into()),
            bucket_key: Some(format!("periodic-pool-{}", Uuid::now_v7())),
            ..Default::default()
        },
    )
    .await;
    mark_bucket_as_registration_pool(pool, pool_bucket).await;

    let user_id = create_test_user(pool, &realm_id, "cb_t03_periodic@example.com").await;

    // --- When: registration runs (registration bonus + first periodic). ---
    let result = ctx
        .app_state
        .registration_service
        .handle_user_registration(user_id, &realm_id)
        .await;
    assert!(result.is_ok(), "registration should succeed: {:?}", result);

    // --- Then: grant_schedule.bucket_id = marked pool. -------------------
    let schedule_bucket = read_first_grant_schedule_bucket(pool, user_id).await;
    assert_eq!(
        schedule_bucket,
        Some(pool_bucket),
        "periodic grant_schedule.bucket_id must be the registration pool Bucket"
    );

    // --- And: first free-periodic grant landed in that Bucket. -----------
    let free_periodic =
        sum_ledger_granted_in_bucket(pool, user_id, pool_bucket, "free_periodic_credit").await;
    assert_eq!(
        free_periodic, 30,
        "first periodic grant must land in the registration pool Bucket"
    );

    // --- And: registration bonus also landed in the same pool. -----------
    let registration =
        sum_ledger_granted_in_bucket(pool, user_id, pool_bucket, "registration_credit").await;
    assert_eq!(registration, 100, "registration bonus in the same pool");

    // No cross-pool leak.
    assert_eq!(
        count_ledger_outside_bucket(pool, user_id, pool_bucket).await,
        0,
        "no grants should leak outside the registration pool Bucket"
    );
}

// =============================================================================
// Scenario 6: registration pool absent → fail-safe skip (no grant)
// =============================================================================

/// User Story: US-FU-001 (no implicit pool — fail-safe skip when no Bucket is
/// marked).
/// Covers:
///   - when no Bucket in the Realm is flagged
///     `receives_registration_credits = true`, the resolver returns `None` and
///     `handle_user_registration` SKIPS both the registration bonus and the
///     periodic grant schedule fail-safe — it never falls back to any implicit
///     pool.
///   - DB check: no `points_credit_ledger` rows, no
///     `points_grant_schedules` rows for the user.
#[test_context(TestContext)]
#[tokio::test]
async fn registration_pool_absent_skips_grant_fail_safe(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();
    let pool = &ctx.app_state.pool;

    // Realm config: positive amounts so grants WOULD fire if a pool existed.
    set_realm_default_config(pool, &realm_id, 500, 50, 7, "daily").await;

    // Sanity: no Bucket in this Realm is marked as the registration pool.
    assert_eq!(
        find_registration_pool_for_realm(pool, &realm_id).await,
        None,
        "precondition: no registration pool Bucket configured"
    );

    let user_id = create_test_user(pool, &realm_id, "cb_t03_nopool@example.com").await;

    // --- When: registration runs with NO registration pool configured. ---
    let result = ctx
        .app_state
        .registration_service
        .handle_user_registration(user_id, &realm_id)
        .await;

    // --- Then: fail-safe skip (Ok), NOT an error. ------------------------
    assert!(
        result.is_ok(),
        "registration must succeed (fail-safe skip) when no pool is configured: {:?}",
        result
    );

    // --- And: no registration bonus was granted. -------------------------
    let any_ledger = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)::BIGINT FROM points_credit_ledger WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("count ledger");
    assert_eq!(
        any_ledger, 0,
        "fail-safe skip must NOT grant any credits (no implicit pool fallback)"
    );

    // --- And: no periodic grant schedule was created. --------------------
    let schedule_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)::BIGINT FROM points_grant_schedules WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("count schedules");
    assert_eq!(
        schedule_count, 0,
        "fail-safe skip must NOT create a periodic grant schedule"
    );
}

// =============================================================================
// Scenario 7: second registration pool in same Realm → 409
// =============================================================================

/// User Story: US-CB-001 (at most one registration pool per Realm).
/// Covers:
///   - the partial unique index
///     `uq_credit_buckets_registration_pool ON credit_buckets(realm_id)
///      WHERE receives_registration_credits = true` enforces "at most one per
///     Realm". A second marker in the same Realm raises a DB-level unique
///     violation; the production write path
///     (`PostgresBillingRepository::create_credit_bucket` /
///      `update_credit_bucket`) surfaces this as 409
///     `registration_pool_conflict`.
///   - This scenario asserts BOTH layers:
///       (a) raw DB unique violation when bypassing the handler, and
///       (b) HTTP 409 `registration_pool_conflict` via the real create handler.
#[test_context(TestContext)]
#[tokio::test]
async fn second_registration_pool_in_same_realm_returns_409_registration_pool_conflict(
    ctx: &mut TestContext,
) {
    let realm_id = ctx._realm_id.clone();

    // Admin session setup borrows ctx mutably; do it before any immutable
    // borrow of ctx.app_state.pool.
    let admin_token = setup_billing_admin_session(ctx, "cb_t03_conflict_admin@example.com").await;

    let pool = &ctx.app_state.pool;

    // First Bucket: marked as the registration pool.
    let first_pool = create_test_credit_bucket(
        pool,
        &realm_id,
        CreditBucketOpts {
            name: Some("First Registration Pool".into()),
            bucket_key: Some(format!("first-reg-pool-{}", Uuid::now_v7())),
            ..Default::default()
        },
    )
    .await;
    mark_bucket_as_registration_pool(pool, first_pool).await;

    // Sanity: the first pool is resolvable and marked.
    assert_eq!(
        find_registration_pool_for_realm(pool, &realm_id).await,
        Some(first_pool),
        "precondition: first registration pool is marked"
    );
    assert!(
        read_receives_registration_credits(pool, first_pool).await,
        "precondition: first pool has receives_registration_credits = true"
    );

    // --- (a) Raw DB layer: a second marker must violate the partial unique
    // index. --------------------------------------------------------------
    let second_pool = create_test_credit_bucket(
        pool,
        &realm_id,
        CreditBucketOpts {
            name: Some("Second Registration Pool".into()),
            bucket_key: Some(format!("second-reg-pool-{}", Uuid::now_v7())),
            ..Default::default()
        },
    )
    .await;
    let raw_result = try_mark_registration_pool_raw(pool, second_pool).await;
    assert!(
        raw_result.is_err(),
        "DB partial unique index uq_credit_buckets_registration_pool must reject a \
         second registration pool in the same Realm; got Ok"
    );

    // Reset the second bucket so it can be reused as the "would-be second
    // pool" through the HTTP handler below (the handler writes the flag during
    // create, so the bucket must start unmarked).
    sqlx::query("UPDATE credit_buckets SET receives_registration_credits = false WHERE id = $1")
        .bind(second_pool)
        .execute(pool)
        .await
        .expect("reset second pool flag");

    // --- (b) HTTP layer: POST a new Bucket with receivesRegistrationCredits
    // = true in the same Realm must return 409 registration_pool_conflict. -
    let client_app_id = create_test_client_app(pool, &realm_id).await;

    let app = ctx.create_unified_test_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/realms/{}/billing/credit-buckets", realm_id))
                .header("content-type", "application/json")
                .header(
                    axum::http::header::AUTHORIZATION,
                    format!("Bearer {}", admin_token),
                )
                .body(Body::from(
                    json!({
                        "bucketKey": format!("http-second-pool-{}", Uuid::now_v7()),
                        "name": "HTTP Second Pool",
                        "clientAppIds": [client_app_id],
                        "receivesRegistrationCredits": true,
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: Option<Value> = if body_bytes.is_empty() {
        None
    } else {
        Some(serde_json::from_slice(&body_bytes).unwrap_or(Value::Null))
    };

    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "second registration pool via HTTP must be 409, got {}: {:?}",
        status,
        body
    );
    assert_eq!(
        error_code(&body),
        Some("registration_pool_conflict"),
        "expected registration_pool_conflict error code, got: {:?}",
        body
    );
}

// =============================================================================
// Scenario 8: registration pool independent of subscription bucket
// =============================================================================

/// User Story: US-FU-003 + US-CB-008 (registration pool and subscription
/// Bucket do not cross-contaminate).
/// Covers:
///   - registration grants target the registration pool Bucket;
///     subscription lifecycle grants target `subscription.bucket_id`. The two
///     are independent — a registration grant MUST NOT land in the subscription
///     Bucket and vice versa.
///   - This scenario constructs two distinct Buckets, grants registration
///     credits into the registration pool, then grants subscription credits
///     into the subscription Bucket via the same domain write path, and
///     asserts no cross-pool leak in either direction.
#[test_context(TestContext)]
#[tokio::test]
async fn registration_pool_independent_of_subscription_bucket(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();
    let pool = &ctx.app_state.pool;

    // Realm config: 200 registration bonus (no periodic, to isolate the check).
    set_realm_default_config(pool, &realm_id, 200, 0, 0, "daily").await;

    // Two DISTINCT buckets: one registration pool, one subscription bucket.
    let reg_bucket = create_test_credit_bucket(
        pool,
        &realm_id,
        CreditBucketOpts {
            name: Some("Reg-Only Pool".into()),
            bucket_key: Some(format!("reg-only-{}", Uuid::now_v7())),
            ..Default::default()
        },
    )
    .await;
    mark_bucket_as_registration_pool(pool, reg_bucket).await;

    let sub_bucket = create_test_credit_bucket(
        pool,
        &realm_id,
        CreditBucketOpts {
            name: Some("Sub-Only Bucket".into()),
            bucket_key: Some(format!("sub-only-{}", Uuid::now_v7())),
            ..Default::default()
        },
    )
    .await;

    let user_id = create_test_user(pool, &realm_id, "cb_t03_independent@example.com").await;

    // --- When: registration handler grants the registration bonus. -------
    let result = ctx
        .app_state
        .registration_service
        .handle_user_registration(user_id, &realm_id)
        .await;
    assert!(result.is_ok(), "registration should succeed: {:?}", result);

    // --- And: a subscription-cycle grant lands in sub_bucket (mirrors
    // `subscription_service.handle_subscription_paid` routing via
    // `subscription.bucket_id`). We use the real domain write
    // path `points_service.grant_points_internal` with the subscription
    // Bucket as the explicit target. ---
    ctx.app_state
        .points_service
        .grant_points_internal(
            &realm_id,
            user_id,
            sub_bucket,
            herald_core::domain::points::entities::CreditType::SubscriptionCredit,
            CreditSourceType::SubscriptionInitial,
            1_000,
            None,
            // effective_at: None ⟺ immediately available.
            None,
            Some(format!("sub-grant-{}", Uuid::now_v7())),
            Some("Subscription cycle grant".to_string()),
            None,
        )
        .await
        .expect("subscription grant should succeed");

    // --- Then: registration bonus in reg_bucket ONLY. --------------------
    let reg_in_reg =
        sum_ledger_granted_in_bucket(pool, user_id, reg_bucket, "registration_credit").await;
    assert_eq!(reg_in_reg, 200, "registration bonus in the reg pool Bucket");

    let reg_in_sub =
        sum_ledger_granted_in_bucket(pool, user_id, sub_bucket, "registration_credit").await;
    assert_eq!(
        reg_in_sub, 0,
        "registration bonus must NOT leak into the subscription Bucket"
    );

    // --- And: subscription credit in sub_bucket ONLY. --------------------
    let sub_in_sub =
        sum_ledger_granted_in_bucket(pool, user_id, sub_bucket, "subscription_credit").await;
    assert_eq!(sub_in_sub, 1_000, "subscription credit in the sub Bucket");

    let sub_in_reg =
        sum_ledger_granted_in_bucket(pool, user_id, reg_bucket, "subscription_credit").await;
    assert_eq!(
        sub_in_reg, 0,
        "subscription credit must NOT leak into the registration pool Bucket"
    );

    // --- And: per-bucket wallet totals are isolated. ---------------------
    let reg_total = read_wallet_total_balance(pool, &realm_id, user_id, reg_bucket).await;
    let sub_total = read_wallet_total_balance(pool, &realm_id, user_id, sub_bucket).await;
    assert_eq!(reg_total, 200, "registration pool wallet total");
    assert_eq!(sub_total, 1_000, "subscription bucket wallet total");
}
