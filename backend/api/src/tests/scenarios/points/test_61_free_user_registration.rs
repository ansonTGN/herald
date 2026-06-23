// =============================================================================
// Points System Scenario Test 61: Free User Registration
// =============================================================================
//
// **User Story**: US-FU-01 (Registration grants initial bonus points)
// **Priority**: P0
//
// **Scenarios**:
// 1. Registration grants initial bonus points
// 2. Prevent duplicate registration bonuses
// 3. Registration points with custom realm config
// 4. Registration creates user_points_config record
// 5. Registration with daily points disabled
// 6. Concurrent registration with same email
// 7. Concurrent registration with same username
// 8. Registration during realm maintenance
// 9. Registration with invalid email format
// 10. Registration with weak password (rejected by policy)
//
// =============================================================================

use crate::tests::helpers::points_helpers::trunc_to_micros;
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::json;
use test_context::test_context;
use tower::ServiceExt;

/// ============================================================================
/// Scenario 1: Registration grants initial bonus points
/// ============================================================================
// User Story: docs/user-stories/points-free-user.md#US-FU-01
// Covers: 验收标准 1.1, 1.2
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_free_user_registration_grants_initial_bonus(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    // ============================================================================
    // Given: realm-1 has default config: registration_bonus_points = 1000
    // ============================================================================
    println!("[Step 1] Set up realm default config");

    sqlx::query(
        r#"
        INSERT INTO realm_default_configs (realm_id, registration_bonus_points, free_periodic_points_amount, free_periodic_validity_days, free_periodic_grant_period_type)
        VALUES ($1, 1000, 50, 1, 'daily')
        ON CONFLICT (realm_id) DO UPDATE SET
            registration_bonus_points = EXCLUDED.registration_bonus_points,
            free_periodic_points_amount = EXCLUDED.free_periodic_points_amount,
            free_periodic_validity_days = EXCLUDED.free_periodic_validity_days,
            free_periodic_grant_period_type = EXCLUDED.free_periodic_grant_period_type
        "#
    )
    .bind(&ctx._realm_id)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create realm default config");

    // Enable registration for the realm
    sqlx::query(
        r#"
        INSERT INTO realm_config (realm_id, config_type, config_key, config_value, enabled)
        VALUES ($1, 'registration', 'enabled', 'true', true)
        ON CONFLICT (realm_id, config_type, config_key)
        DO UPDATE SET config_value = EXCLUDED.config_value, enabled = EXCLUDED.enabled
        "#,
    )
    .bind(&ctx._realm_id)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to enable registration");

    println!("[Step 1] ✓ Realm default config created");

    // Credit Buckets model (design §4.3.2): registration-bonus grants route to
    // the realm's registration-pool bucket (`receives_registration_credits =
    // true`). `ensure_test_bucket_for_realm` materializes that pool for the
    // realm; without it the resolver returns None and the grant is skipped.
    crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm(
        &ctx._app_state.pool,
        &ctx._realm_id,
    )
    .await;

    // ============================================================================
    // When: A new user registers in realm-1 (2026-03-23 15:30:00 UTC)
    // ============================================================================
    println!("[Step 2] New user registers");

    let email = "newuser@example.com";
    let password = "SecurePassword123!";
    let registration_payload = json!({
        "clientId": ctx._client_id,
        "email": email,
        "password": password,
        "turnstileToken": "dummy"
    });

    let registration_request = Request::builder()
        .method("POST")
        .uri(format!("/api/auth/{}/register", ctx._realm_id))
        .header("content-type", "application/json")
        .header("x-forwarded-for", "1.1.1.1")
        .body(Body::from(registration_payload.to_string()))
        .unwrap();

    let registration_response = app.clone().oneshot(registration_request).await.unwrap();
    assert_eq!(registration_response.status(), StatusCode::OK);

    let user_id: String = sqlx::query_scalar("SELECT id::text FROM account WHERE email = $1")
        .bind(email)
        .fetch_one(&ctx._app_state.pool)
        .await
        .expect("Failed to fetch user_id");
    let user_id = uuid::Uuid::parse_str(&user_id).expect("Invalid user ID");

    println!("[Step 2] ✓ User registered: {}", user_id);

    // ============================================================================
    // Then: The user receives 1000 registration_credit points
    // ============================================================================
    println!("[Step 3] Verify registration credit grant");

    let credit_type: String = sqlx::query_scalar(
        "SELECT credit_type FROM points_credit_ledger WHERE user_id = $1 AND credit_type = 'registration_credit'"
    )
    .bind(user_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Registration credit not found");

    assert_eq!(credit_type, "registration_credit");

    // Check that the credit expires_at is NULL (permanent validity)
    let expires_at: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
        "SELECT expires_at FROM points_credit_ledger WHERE user_id = $1 AND credit_type = 'registration_credit'"
    )
    .bind(user_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to fetch expires_at");

    assert!(
        expires_at.is_none(),
        "Registration credit should be permanent (expires_at = NULL)"
    );

    // Check that a transaction record exists with type = 'registration_grant'
    let transaction_type: String = sqlx::query_scalar(
        "SELECT type FROM points_transactions WHERE user_id = $1 AND type = 'registration_grant'",
    )
    .bind(user_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Registration transaction not found");

    assert_eq!(transaction_type, "registration_grant");

    // Check that the user has a derived available balance of 1050
    // 1000 (registration bonus) + 50 (first periodic grant) = 1050.
    // point-time (BE-D11): `points_wallets.total_balance` was dropped; available
    // balance is derived from `points_credit_ledger` using the same predicate
    // as consumption.
    let total_balance: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(l.remaining_amount) FILTER (
                    WHERE l.status = 'active' AND l.remaining_amount > 0
                      AND (l.effective_at IS NULL OR l.effective_at <= NOW())
                      AND (l.expires_at  IS NULL OR l.expires_at  >  NOW())
                ), 0)::BIGINT
         FROM points_wallets w
         LEFT JOIN points_credit_ledger l
           ON l.realm_id = w.realm_id AND l.user_id = w.user_id AND l.bucket_id = w.bucket_id
         WHERE w.user_id = $1
         GROUP BY w.id",
    )
    .bind(user_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Points account not found");

    assert_eq!(total_balance, 1050);

    println!(
        "[Step 3] ✓ Registration credit verified: 1050 points (1000 registration + 50 periodic grant)"
    );
}

/// ============================================================================
/// Scenario 2: Prevent duplicate registration bonuses
/// ============================================================================
// User Story: docs/user-stories/points-free-user.md#US-FU-01
// Covers: 验收标准 1.3
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_free_user_registration_prevents_duplicate_bonuses(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    // ============================================================================
    // Given: A user has already received registration bonus points
    // ============================================================================
    println!("[Step 1] Create user with registration bonus");

    sqlx::query(
        r#"
        INSERT INTO realm_default_configs (realm_id, registration_bonus_points, free_periodic_points_amount, free_periodic_validity_days, free_periodic_grant_period_type)
        VALUES ($1, 1000, 50, 1, 'daily')
        ON CONFLICT (realm_id) DO UPDATE SET
            registration_bonus_points = EXCLUDED.registration_bonus_points
        "#
    )
    .bind(&ctx._realm_id)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create realm default config");

    // Enable registration for the realm
    sqlx::query(
        r#"
        INSERT INTO realm_config (realm_id, config_type, config_key, config_value, enabled)
        VALUES ($1, 'registration', 'enabled', 'true', true)
        ON CONFLICT (realm_id, config_type, config_key)
        DO UPDATE SET config_value = EXCLUDED.config_value, enabled = EXCLUDED.enabled
        "#,
    )
    .bind(&ctx._realm_id)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to enable registration");

    // Materialize the realm's registration-pool bucket (design §4.3.2) so the
    // registration-bonus grant lands in a credit ledger the assertions read.
    crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm(
        &ctx._app_state.pool,
        &ctx._realm_id,
    )
    .await;

    let email = "existinguser@example.com";
    let password = "SecurePassword123!";

    let registration_payload = json!({
        "clientId": ctx._client_id,
        "email": email,
        "password": password,
        "turnstileToken": "dummy"
    });

    let registration_request = Request::builder()
        .method("POST")
        .uri(format!("/api/auth/{}/register", ctx._realm_id))
        .header("content-type", "application/json")
        .header("x-forwarded-for", "1.1.1.1")
        .body(Body::from(registration_payload.to_string()))
        .unwrap();

    let registration_response = app.clone().oneshot(registration_request).await.unwrap();
    assert_eq!(registration_response.status(), StatusCode::OK);

    let user_id: String = sqlx::query_scalar("SELECT id::text FROM account WHERE email = $1")
        .bind(email)
        .fetch_one(&ctx._app_state.pool)
        .await
        .expect("Failed to fetch user_id");
    let user_id = uuid::Uuid::parse_str(&user_id).expect("Invalid user ID");

    // Verify initial registration credit. point-time (BE-D11): read the
    // derived available balance instead of the dropped `total_balance`.
    let initial_balance: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(l.remaining_amount) FILTER (
                    WHERE l.status = 'active' AND l.remaining_amount > 0
                      AND (l.effective_at IS NULL OR l.effective_at <= NOW())
                      AND (l.expires_at  IS NULL OR l.expires_at  >  NOW())
                ), 0)::BIGINT
         FROM points_wallets w
         LEFT JOIN points_credit_ledger l
           ON l.realm_id = w.realm_id AND l.user_id = w.user_id AND l.bucket_id = w.bucket_id
         WHERE w.user_id = $1
         GROUP BY w.id",
    )
    .bind(user_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Points account not found");

    assert_eq!(initial_balance, 1050);

    println!(
        "[Step 1] ✓ User created with registration bonus: {}",
        user_id
    );

    // ============================================================================
    // When: The same user attempts to register again (duplicate email)
    // ============================================================================
    println!("[Step 2] Attempt duplicate registration");

    let duplicate_payload = json!({
        "clientId": ctx._client_id,
        "email": email,
        "password": "AnotherPassword123!",
        "turnstileToken": "dummy"
    });

    let duplicate_request = Request::builder()
        .method("POST")
        .uri(format!("/api/auth/{}/register", ctx._realm_id))
        .header("content-type", "application/json")
        .header("x-forwarded-for", "1.1.1.2")
        .body(Body::from(duplicate_payload.to_string()))
        .unwrap();

    let duplicate_response = app.clone().oneshot(duplicate_request).await.unwrap();

    // ============================================================================
    // Then: The system returns error "Email already registered"
    // ============================================================================
    assert_eq!(duplicate_response.status(), StatusCode::CONFLICT);

    let error_body = extract_error_body(duplicate_response).await;
    assert!(
        error_body.contains("Email already registered")
            || error_body.contains("already registered")
    );

    // And: No additional registration points are granted. point-time (BE-D11):
    // read the derived available balance instead of the dropped `total_balance`.
    let final_balance: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(l.remaining_amount) FILTER (
                    WHERE l.status = 'active' AND l.remaining_amount > 0
                      AND (l.effective_at IS NULL OR l.effective_at <= NOW())
                      AND (l.expires_at  IS NULL OR l.expires_at  >  NOW())
                ), 0)::BIGINT
         FROM points_wallets w
         LEFT JOIN points_credit_ledger l
           ON l.realm_id = w.realm_id AND l.user_id = w.user_id AND l.bucket_id = w.bucket_id
         WHERE w.user_id = $1
         GROUP BY w.id",
    )
    .bind(user_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Points account not found");

    assert_eq!(final_balance, 1050, "Balance should remain unchanged");

    // And: The user's registration_credit balance remains unchanged
    let registration_credit_balance: i64 = sqlx::query_scalar(
        "SELECT CAST(SUM(granted_amount) AS BIGINT) FROM points_credit_ledger WHERE user_id = $1 AND credit_type = 'registration_credit'"
    )
    .bind(user_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .unwrap_or(0);

    assert_eq!(
        registration_credit_balance, 1000,
        "Registration credit should remain unchanged"
    );

    println!("[Step 2] ✓ Duplicate registration prevented, balance unchanged");
}
/// ============================================================================
/// Scenario 5: Registration with periodic points disabled
/// ============================================================================
// User Story: docs/user-stories/points-free-user.md#US-FU-01
// Covers: 验收标准 1.5
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_free_user_registration_periodic_points_disabled(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    // ============================================================================
    // Given: realm-1 has config: free_periodic_points_amount = 0 (periodic points disabled by default)
    // ============================================================================
    println!("[Step 1] Set up realm config with periodic points disabled");

    // Set free_periodic_points_amount = 0 to disable periodic points
    sqlx::query(
        r#"
        INSERT INTO realm_default_configs (realm_id, registration_bonus_points, free_periodic_points_amount, free_periodic_validity_days, free_periodic_grant_period_type)
        VALUES ($1, 1000, 0, 1, 'daily')
        ON CONFLICT (realm_id) DO UPDATE SET
            registration_bonus_points = EXCLUDED.registration_bonus_points,
            free_periodic_points_amount = EXCLUDED.free_periodic_points_amount
        "#
    )
    .bind(&ctx._realm_id)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create realm default config");

    // Enable registration for the realm
    sqlx::query(
        r#"
        INSERT INTO realm_config (realm_id, config_type, config_key, config_value, enabled)
        VALUES ($1, 'registration', 'enabled', 'true', true)
        ON CONFLICT (realm_id, config_type, config_key)
        DO UPDATE SET config_value = EXCLUDED.config_value, enabled = EXCLUDED.enabled
        "#,
    )
    .bind(&ctx._realm_id)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to enable registration");

    println!("[Step 1] ✓ Realm config created with periodic points disabled");

    // Materialize the realm's registration-pool bucket (design §4.3.2).
    crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm(
        &ctx._app_state.pool,
        &ctx._realm_id,
    )
    .await;

    // ============================================================================
    // When: A new user registers in realm-1
    // ============================================================================
    println!("[Step 2] New user registers");

    let email = "nodailyuser@example.com";
    let password = "SecurePassword123!";

    let registration_payload = json!({
        "clientId": ctx._client_id,
        "email": email,
        "password": password,
        "turnstileToken": "dummy"
    });

    let registration_request = Request::builder()
        .method("POST")
        .uri(format!("/api/auth/{}/register", ctx._realm_id))
        .header("content-type", "application/json")
        .header("x-forwarded-for", "1.1.1.1")
        .body(Body::from(registration_payload.to_string()))
        .unwrap();

    let registration_response = app.clone().oneshot(registration_request).await.unwrap();
    assert_eq!(registration_response.status(), StatusCode::OK);

    let user_id: String = sqlx::query_scalar("SELECT id::text FROM account WHERE email = $1")
        .bind(email)
        .fetch_one(&ctx._app_state.pool)
        .await
        .expect("Failed to fetch user_id");
    let user_id = uuid::Uuid::parse_str(&user_id).expect("Invalid user ID");

    println!("[Step 2] ✓ User registered: {}", user_id);

    // ============================================================================
    // Then: The user receives registration_credit points but NO periodic grant (free_periodic_points_amount = 0)
    // ============================================================================
    println!("[Step 3] Verify registration credit and no periodic grant");

    let registration_credit_amount: i64 = sqlx::query_scalar(
        "SELECT CAST(COALESCE(SUM(granted_amount), 0) AS BIGINT) FROM points_credit_ledger WHERE user_id = $1 AND credit_type = 'registration_credit'"
    )
    .bind(user_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to fetch registration credit");

    assert_eq!(registration_credit_amount, 1000);

    // Verify that NO periodic grant was granted (free_periodic_points_amount = 0)
    let periodic_credit_amount: i64 = sqlx::query_scalar(
        "SELECT CAST(COALESCE(SUM(granted_amount), 0) AS BIGINT) FROM points_credit_ledger WHERE user_id = $1 AND credit_type = 'free_periodic_credit'"
    )
    .bind(user_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to fetch periodic credit");

    assert_eq!(
        periodic_credit_amount, 0,
        "No periodic grant should be granted when free_periodic_points_amount = 0"
    );

    // And: user_points_configs.free_periodic_points_amount = 0
    let free_periodic_points_amount: i64 = sqlx::query_scalar(
        "SELECT free_periodic_points_amount FROM user_points_configs WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to fetch user_points_config");

    assert_eq!(
        free_periodic_points_amount, 0,
        "free_periodic_points_amount should be 0"
    );

    println!(
        "[Step 3] ✓ Registration credit (1000) verified, no periodic grant (free_periodic_points_amount = 0)"
    );
}

// =============================================================================
// BE-T03: Free-periodic on-time grant + two distinct sources (A10)
// =============================================================================
//
// **User Story**: US-FU-004 (按时获得每期免费积分) + US-FU-002 (免费定期积分按时发放)
// **Priority**: P0
//
// **Design refs**: `.ai/design/point-time.md` §5.3 (GrantScheduler process_due
// pre-grant anchors), §5.5 (lead_time table), §1.4 A10 (registration initial
// credit and free_periodic first period are two distinct entitlement sources).
//
// **Testability decision (BE-T03)**: `SchemaTestContext` does NOT expose a
// `GrantScheduler` handle. Per the item-file precheck guidance, we construct
// `GrantScheduler::new(points_repository, points_service, lead_time_map)`
// directly in-test using the `ctx._app_state.points_repository` /
// `points_service` Arcs. This exercises the real domain code path
// (`process_due_schedules` → `process_schedule` → `grant_points_internal`)
// against the live test schema, mirroring what the worker would do, without
// depending on the worker loop. The read-path realization
// (`reconcile_due_for_user`) is also reachable via `ctx._app_state.points_service`
// and is exercised by BE-T08; these tests focus on the scheduler-driven
// "worker normal on-time grant" path.

use herald_core::domain::points::{GrantPeriodType, GrantScheduler};
use std::collections::HashMap;
use std::sync::Arc;

/// Build the lead_time_map (design §5.5) for the in-test `GrantScheduler`.
/// Mirrors `app/src/main.rs::build_lead_time_map` defaults.
fn build_test_lead_time_map() -> HashMap<GrantPeriodType, chrono::Duration> {
    let mut map = HashMap::new();
    map.insert(GrantPeriodType::Daily, chrono::Duration::hours(1));
    map.insert(GrantPeriodType::Weekly, chrono::Duration::hours(12));
    map.insert(GrantPeriodType::Monthly, chrono::Duration::hours(24));
    map.insert(GrantPeriodType::Once, chrono::Duration::zero());
    map
}

/// ============================================================================
/// Scenario BE-T03.1: Free-periodic first period is immediately available
/// (next_grant_time <= now ⟹ effective_at = NULL ⟺ immediately available)
/// ============================================================================
// User Story: docs/user-stories/points-free-user.md#US-FU-004
// Covers: design §5.3 (first period due immediately grants with effective_at=NULL),
//         §6.1 "免费周期按时发放（P0）".
//
// WHY this test exists: the availability predicate
//   `effective_at IS NULL OR effective_at <= NOW()`
// must treat a NULL effective_at as immediately consumable. A first-period
// schedule with `next_grant_time <= now` must produce a ledger row with
// `effective_at = NULL` (not a future timestamp), so the user sees the grant
// at once. If the scheduler wrongly wrote `Some(next_grant_time)` for an
// already-due period, the derived balance would exclude the row and the user
// would see zero balance despite the grant having fired.
#[test_context(TestContext)]
#[tokio::test]
async fn test_free_periodic_first_period_immediately_available(ctx: &mut TestContext) {
    use herald_core::domain::points::entities::CreditType;

    let pool = &ctx._app_state.pool;
    let realm_id = ctx._realm_id.clone();
    let user_id = uuid::Uuid::now_v7();

    // Ensure the user + realm registration-pool bucket exist so grant writes
    // satisfy the NOT NULL bucket_id constraint.
    crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm(pool, &realm_id).await;
    sqlx::query(
        "INSERT INTO account (id, realm_id, email, password, status, created_at, updated_at)
         VALUES ($1, $2, $3, '$2a$12$dummy_password_hash', 1, NOW(), NOW())
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(user_id)
    .bind(&realm_id)
    .bind(format!("free-periodic-first-{}@test.com", user_id))
    .execute(pool)
    .await
    .expect("Failed to ensure user exists");

    let now = chrono::Utc::now();
    let points_per_period: i64 = 50;
    let validity_days: i64 = 1; // non-zero ⟹ expires_at = next_grant_time + 1 day

    // Seed a free schedule whose first period is already due (next_grant_time
    // <= now). This mirrors what registration_service would create at sign-up
    // (registration_service.rs:180 `next_grant_time = now`, `granted_periods = 0`).
    let _schedule_id = crate::tests::helpers::points_helpers::create_free_grant_schedule(
        ctx,
        user_id,
        &realm_id,
        "daily",
        points_per_period,
        validity_days,
        now,
        0,
        "",
    )
    .await;

    // Construct the GrantScheduler in-test (SchemaTestContext does not expose
    // one). process_due_schedules() is the worker's normal on-time-grant path.
    let scheduler = GrantScheduler::new(
        Arc::clone(&ctx._app_state.points_repository),
        Arc::clone(&ctx._app_state.points_service),
        build_test_lead_time_map(),
    );
    let summary = scheduler
        .process_due_schedules()
        .await
        .expect("GrantScheduler::process_due_schedules failed");
    assert!(
        summary.processed >= 1,
        "expected the seeded schedule to be processed, got summary {:?}",
        summary
    );

    // The free_periodic_credit ledger row must exist with effective_at IS NULL
    // (next_grant_time <= now ⟹ immediately available, design §5.3).
    let row = sqlx::query(
        "SELECT effective_at, expires_at, granted_amount, remaining_amount, status
         FROM points_credit_ledger
         WHERE user_id = $1 AND realm_id = $2 AND credit_type = 'free_periodic_credit'",
    )
    .bind(user_id)
    .bind(&realm_id)
    .fetch_one(pool)
    .await
    .expect("free_periodic_credit ledger row not found after grant");

    use sqlx::Row;
    let effective_at: Option<chrono::DateTime<chrono::Utc>> = row.get("effective_at");
    let granted_amount: i64 = row.get("granted_amount");
    assert!(
        effective_at.is_none(),
        "first-period (next_grant_time<=now) grant must have effective_at=NULL for immediate \
         availability; got effective_at={:?}. If non-null, the predicate would exclude it and \
         the user would see zero balance.",
        effective_at
    );
    assert_eq!(granted_amount, points_per_period);

    // Derived available balance must include this immediately-available row
    // (predicate: effective_at IS NULL ⟹ included). This is the canonical
    // point-time balance assertion — do NOT read points_wallets.total_balance.
    crate::tests::helpers::points_helpers::assert_derived_balance(
        ctx,
        user_id,
        &realm_id,
        CreditType::FreePeriodicCredit,
        points_per_period,
    )
    .await;
}

/// ============================================================================
/// Scenario BE-T03.2: Registration credit and free_periodic first period are
/// two distinct sources (design §1.4 A10)
/// ============================================================================
// User Story: docs/user-stories/points-free-user.md#US-FU-001 + US-FU-002
// Covers: design §1.4 A10, §6.1 "注册初始积分与 free_periodic 首期作为两笔不同来源".
//
// WHY this test exists: registration initial bonus points and the free_periodic
// first-period grant are TWO DIFFERENT entitlement sources. They must land as
// two independent ledger rows with different credit_type / source_type, each
// with its own amount, and must NOT substitute for each other. If a future
// refactor conflated them (e.g. registration_service skipped its own bonus
// assuming the free_periodic grant covers it), users would silently lose
// either the bonus or the periodic grant. This test locks the two-source
// invariant.
#[test_context(TestContext)]
#[tokio::test]
async fn test_registration_credit_and_free_periodic_two_distinct_sources(ctx: &mut TestContext) {
    use herald_core::domain::points::entities::{CreditSourceType, CreditType};

    let pool = &ctx._app_state.pool;
    let realm_id = ctx._realm_id.clone();
    let registration_bonus: i64 = 1000;
    let free_periodic_amount: i64 = 50;

    // Configure the realm with BOTH a registration bonus AND a non-zero
    // free_periodic amount, so registration creates both sources.
    sqlx::query(
        r#"
        INSERT INTO realm_default_configs
            (realm_id, registration_bonus_points, free_periodic_points_amount,
             free_periodic_validity_days, free_periodic_grant_period_type)
        VALUES ($1, $2, $3, 1, 'daily')
        ON CONFLICT (realm_id) DO UPDATE SET
            registration_bonus_points = EXCLUDED.registration_bonus_points,
            free_periodic_points_amount = EXCLUDED.free_periodic_points_amount,
            free_periodic_validity_days = EXCLUDED.free_periodic_validity_days,
            free_periodic_grant_period_type = EXCLUDED.free_periodic_grant_period_type
        "#,
    )
    .bind(&realm_id)
    .bind(registration_bonus)
    .bind(free_periodic_amount)
    .execute(pool)
    .await
    .expect("Failed to set realm default config");

    sqlx::query(
        r#"
        INSERT INTO realm_config (realm_id, config_type, config_key, config_value, enabled)
        VALUES ($1, 'registration', 'enabled', 'true', true)
        ON CONFLICT (realm_id, config_type, config_key)
        DO UPDATE SET config_value = EXCLUDED.config_value, enabled = EXCLUDED.enabled
        "#,
    )
    .bind(&realm_id)
    .execute(pool)
    .await
    .expect("Failed to enable registration");

    crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm(pool, &realm_id).await;

    // Register a new user via the real HTTP path — this triggers
    // registration_service which writes BOTH the registration_credit grant and
    // the free_periodic first-period grant (registration_service.rs:199).
    let app = ctx.create_unified_test_router();
    let email = format!("two-sources-{}@test.com", uuid::Uuid::now_v7());
    let registration_payload = json!({
        "clientId": ctx._client_id,
        "email": email,
        "password": "SecurePassword123!",
        "turnstileToken": "dummy"
    });
    let registration_request = Request::builder()
        .method("POST")
        .uri(format!("/api/auth/{}/register", realm_id))
        .header("content-type", "application/json")
        .header("x-forwarded-for", "1.1.1.1")
        .body(Body::from(registration_payload.to_string()))
        .unwrap();
    let registration_response = app.clone().oneshot(registration_request).await.unwrap();
    assert_eq!(registration_response.status(), StatusCode::OK);

    let user_id_str: String = sqlx::query_scalar("SELECT id::text FROM account WHERE email = $1")
        .bind(&email)
        .fetch_one(pool)
        .await
        .expect("Failed to fetch registered user id");
    let user_id = uuid::Uuid::parse_str(&user_id_str).expect("Invalid user ID");

    // A10 invariant: TWO distinct ledger rows — one registration_credit, one
    // free_periodic_credit. They differ in credit_type AND source_type, and
    // neither substitutes for the other.
    let registration_ledgers =
        crate::tests::helpers::points_helpers::get_user_ledgers_by_credit_type(
            ctx,
            user_id,
            CreditType::RegistrationCredit,
        )
        .await;
    let free_periodic_ledgers =
        crate::tests::helpers::points_helpers::get_user_ledgers_by_credit_type(
            ctx,
            user_id,
            CreditType::FreePeriodicCredit,
        )
        .await;

    assert!(
        !registration_ledgers.is_empty(),
        "A10 violation: registration_credit ledger row missing — registration initial bonus \
         must be its own source, not absorbed into free_periodic"
    );
    assert!(
        !free_periodic_ledgers.is_empty(),
        "A10 violation: free_periodic_credit ledger row missing — the first periodic grant must \
         be its own source, not absorbed into registration bonus"
    );

    // Assert source_type differs (registration vs free_periodic_grant) and
    // amounts are independent. The registration row's source_type is the
    // `Registration` variant; the free_periodic row's is `FreePeriodicGrant`.
    let reg_row = &registration_ledgers[0];
    let fp_row = &free_periodic_ledgers[0];
    assert_eq!(
        reg_row.source_type,
        CreditSourceType::Registration,
        "registration_credit row must carry source_type=Registration"
    );
    assert_eq!(
        fp_row.source_type,
        CreditSourceType::FreePeriodicGrant,
        "free_periodic_credit row must carry source_type=FreePeriodicGrant"
    );
    assert_ne!(
        reg_row.credit_type, fp_row.credit_type,
        "the two sources must have different credit_types"
    );
    assert_eq!(
        reg_row.granted_amount, registration_bonus,
        "registration credit amount must be the configured bonus"
    );
    assert_eq!(
        fp_row.granted_amount, free_periodic_amount,
        "free_periodic amount must be the configured per-period amount, independent of the \
         registration bonus"
    );

    // Derived balances: each pool holds its own amount independently. Total
    // derived balance must equal the sum of the two sources — neither replaces
    // the other.
    crate::tests::helpers::points_helpers::assert_derived_balance(
        ctx,
        user_id,
        &realm_id,
        CreditType::RegistrationCredit,
        registration_bonus,
    )
    .await;
    crate::tests::helpers::points_helpers::assert_derived_balance(
        ctx,
        user_id,
        &realm_id,
        CreditType::FreePeriodicCredit,
        free_periodic_amount,
    )
    .await;
    let total =
        crate::tests::helpers::points_helpers::get_derived_total_balance(ctx, user_id, &realm_id)
            .await;
    assert_eq!(
        total,
        registration_bonus + free_periodic_amount,
        "derived total balance must be the SUM of the two distinct sources; if equal to just \
         one, the other source was silently dropped"
    );
}

/// ============================================================================
/// Scenario BE-T03.3: lead_time pre-grant — effective_at is the future period
/// boundary; row is excluded from derived balance until the clock catches up
/// ============================================================================
// User Story: docs/user-stories/points-free-user.md#US-FU-004
// Covers: design §5.3 (next_grant_time > now ⟹ effective_at=Some(next_grant_time)),
//         §5.5 (lead_time table), §6.1 "lead_time 提前预生成 + 零延迟可用".
//
// WHY this test exists: lead_time lets the worker pre-grant a FUTURE period so
// the ledger row exists before the period starts. The availability predicate
// must EXCLUDE that row (effective_at in the future) until the period boundary,
// then INCLUDE it the instant effective_at <= NOW() — with zero state-machine
// work, no job to flip statuses. If the predicate leaked future-effective rows,
// users would spend unbegun periods; if it never admitted them, the pre-grant
// would be useless.
#[test_context(TestContext)]
#[tokio::test]
async fn test_free_periodic_pre_grant_lead_time_effective_at_future(ctx: &mut TestContext) {
    use herald_core::domain::points::entities::CreditType;

    let pool = &ctx._app_state.pool;
    let realm_id = ctx._realm_id.clone();
    let user_id = uuid::Uuid::now_v7();
    let points_per_period: i64 = 30;

    crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm(pool, &realm_id).await;
    sqlx::query(
        "INSERT INTO account (id, realm_id, email, password, status, created_at, updated_at)
         VALUES ($1, $2, $3, '$2a$12$dummy_password_hash', 1, NOW(), NOW())
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(user_id)
    .bind(&realm_id)
    .bind(format!("pre-grant-{}@test.com", user_id))
    .execute(pool)
    .await
    .expect("Failed to ensure user exists");

    let now = chrono::Utc::now();
    // Schedule a MONTHLY period starting 2h from now. With monthly lead_time=24h
    // (§5.5), `next_grant_time - 24h <= now` holds, so the scheduler treats it
    // as due and pre-grants with effective_at = Some(next_grant_time).
    //
    // Truncate to microsecond precision: Postgres `TIMESTAMPTZ` stores
    // microseconds, so the round-tripped `effective_at` drops sub-microsecond
    // nanos. Truncating the seed keeps the strict equality assertion exact
    // without loosening it.
    let future_period_start = trunc_to_micros(now + chrono::Duration::hours(2));

    let _schedule_id = crate::tests::helpers::points_helpers::create_free_grant_schedule(
        ctx,
        user_id,
        &realm_id,
        "monthly",
        points_per_period,
        0, // permanent (validity_days=0 ⟹ expires_at NULL) — keeps the test
        // focused on the effective_at predicate without an expiring row.
        future_period_start,
        0,
        "",
    )
    .await;

    let scheduler = GrantScheduler::new(
        Arc::clone(&ctx._app_state.points_repository),
        Arc::clone(&ctx._app_state.points_service),
        build_test_lead_time_map(),
    );
    let summary = scheduler
        .process_due_schedules()
        .await
        .expect("GrantScheduler::process_due_schedules failed");
    assert!(
        summary.processed >= 1,
        "expected the monthly schedule to be due under lead_time=24h (next_grant_time=now+2h); \
         got summary {:?}",
        summary
    );

    // The pre-granted ledger row must carry effective_at = future_period_start.
    let row = sqlx::query(
        "SELECT id, effective_at FROM points_credit_ledger
         WHERE user_id = $1 AND realm_id = $2 AND credit_type = 'free_periodic_credit'",
    )
    .bind(user_id)
    .bind(&realm_id)
    .fetch_one(pool)
    .await
    .expect("pre-grant ledger row not found");
    use sqlx::Row;
    let ledger_id: uuid::Uuid = row.get("id");
    let effective_at: Option<chrono::DateTime<chrono::Utc>> = row.get("effective_at");
    assert_eq!(
        effective_at,
        Some(future_period_start),
        "pre-grant row must anchor effective_at to the future period boundary (next_grant_time); \
         got {:?}",
        effective_at
    );

    // Derived balance must EXCLUDE the future-effective row: the period hasn't
    // begun, so the user cannot yet consume it.
    crate::tests::helpers::points_helpers::assert_derived_balance(
        ctx,
        user_id,
        &realm_id,
        CreditType::FreePeriodicCredit,
        0,
    )
    .await;

    // Simulate the clock catching up to the period boundary: flip effective_at
    // into the past. NO worker, NO status flip — only the predicate changes
    // outcome. This is the zero-delay availability proof (design §6.1).
    crate::tests::helpers::points_helpers::inject_effective_at(
        ctx,
        ledger_id,
        Some(now - chrono::Duration::seconds(1)),
    )
    .await;

    crate::tests::helpers::points_helpers::assert_derived_balance(
        ctx,
        user_id,
        &realm_id,
        CreditType::FreePeriodicCredit,
        points_per_period,
    )
    .await;
}

/// ============================================================================
/// Scenario BE-T03.4: expires_at is anchored to next_grant_time + validity_days
/// (NOT to the actual grant/created_at moment)
/// ============================================================================
// User Story: docs/user-stories/points-free-user.md#US-FU-004
// Covers: design §5.3 (expires_at = calculate_expiration(next_grant_time,
//         validity_days)), §6.1 "expires 锚定 next_grant_time + validity_days".
//
// WHY this test exists: anchoring expiration to the actual grant moment
// (created_at) would let worker latency or a delayed webhook shorten or
// lengthen the user's valid window unpredictably. The design pins expires_at
// to the EXPECTED period boundary (next_grant_time) so every user gets the
// full validity_days regardless of when the worker fired. If production
// anchors to created_at instead, late grants silently cheat users out of
// validity time.
#[test_context(TestContext)]
#[tokio::test]
async fn test_free_periodic_expires_anchored_to_grant_time(ctx: &mut TestContext) {
    let pool = &ctx._app_state.pool;
    let realm_id = ctx._realm_id.clone();
    let user_id = uuid::Uuid::now_v7();
    let points_per_period: i64 = 40;
    let validity_days: i64 = 7;

    crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm(pool, &realm_id).await;
    sqlx::query(
        "INSERT INTO account (id, realm_id, email, password, status, created_at, updated_at)
         VALUES ($1, $2, $3, '$2a$12$dummy_password_hash', 1, NOW(), NOW())
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(user_id)
    .bind(&realm_id)
    .bind(format!("expires-anchor-{}@test.com", user_id))
    .execute(pool)
    .await
    .expect("Failed to ensure user exists");

    // Anchor the period boundary at a fixed T in the past so the grant is
    // already due (next_grant_time <= now ⟹ effective_at=NULL, immediately
    // available), letting us isolate the expires_at assertion.
    //
    // Truncate to microsecond precision: Postgres `TIMESTAMPTZ` stores
    // microseconds (not nanoseconds), so the seed value round-trips exactly
    // only when sub-microsecond nanos are dropped. Without this the strict
    // `expires_at == t + validity_days` assertion would fail by the truncated
    // nanoseconds even though the period-boundary anchor is correct.
    let now = chrono::Utc::now();
    let t = trunc_to_micros(now - chrono::Duration::hours(1));
    let expected_expires_at = t + chrono::Duration::days(validity_days);

    let _schedule_id = crate::tests::helpers::points_helpers::create_free_grant_schedule(
        ctx,
        user_id,
        &realm_id,
        "daily",
        points_per_period,
        validity_days,
        t,
        0,
        "",
    )
    .await;

    let scheduler = GrantScheduler::new(
        Arc::clone(&ctx._app_state.points_repository),
        Arc::clone(&ctx._app_state.points_service),
        build_test_lead_time_map(),
    );
    let summary = scheduler
        .process_due_schedules()
        .await
        .expect("GrantScheduler::process_due_schedules failed");
    assert!(
        summary.processed >= 1,
        "expected the seeded schedule to be processed; got summary {:?}",
        summary
    );

    // The granted row's expires_at must equal next_grant_time + validity_days.
    // It must NOT equal created_at + validity_days (created_at = actual grant
    // moment, which is now-ish, not t).
    let row = sqlx::query(
        "SELECT expires_at, created_at FROM points_credit_ledger
         WHERE user_id = $1 AND realm_id = $2 AND credit_type = 'free_periodic_credit'",
    )
    .bind(user_id)
    .bind(&realm_id)
    .fetch_one(pool)
    .await
    .expect("free_periodic ledger row not found");
    use sqlx::Row;
    let expires_at: Option<chrono::DateTime<chrono::Utc>> = row.get("expires_at");
    let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");

    let expires_at = expires_at.expect("expires_at must be Some for validity_days=7");
    assert_eq!(
        expires_at, expected_expires_at,
        "expires_at must be anchored to next_grant_time ({}) + {} days = {}; got {} (created_at \
         was {}). If expires_at tracks created_at, worker latency silently shortens the user's \
         validity window.",
        t, validity_days, expected_expires_at, expires_at, created_at
    );
    assert_ne!(
        expires_at,
        created_at + chrono::Duration::days(validity_days),
        "expires_at must NOT track created_at (actual grant moment); it must track the expected \
         period boundary"
    );

    // And validity_days=0 must yield expires_at = NULL (permanent). Verified
    // via a second schedule in the same test to lock the 0 ⟹ None rule from
    // grant_schedule.rs:68.
    let _perm_schedule_id = crate::tests::helpers::points_helpers::create_free_grant_schedule(
        ctx,
        user_id,
        &realm_id,
        "weekly",
        points_per_period,
        0, // permanent
        t,
        0,
        "",
    )
    .await;
    let scheduler2 = GrantScheduler::new(
        Arc::clone(&ctx._app_state.points_repository),
        Arc::clone(&ctx._app_state.points_service),
        build_test_lead_time_map(),
    );
    let _ = scheduler2
        .process_due_schedules()
        .await
        .expect("second GrantScheduler run failed");

    // The permanent (validity_days=0) schedule is the second free_periodic row
    // created; pick the most-recently-created one. A nullable column decoded
    // via query_scalar returns Option<DateTime> per row, and fetch_one returns
    // that directly (no extra outer Option).
    let perm_expires_at: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
        "SELECT expires_at FROM points_credit_ledger
         WHERE user_id = $1 AND realm_id = $2 AND credit_type = 'free_periodic_credit'
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(user_id)
    .bind(&realm_id)
    .fetch_one(pool)
    .await
    .expect("failed to fetch permanent-schedule expires_at");
    assert!(
        perm_expires_at.is_none(),
        "validity_days=0 must produce expires_at=NULL (permanent); got {:?}",
        perm_expires_at
    );
}

/// Helper Functions
/// ============================================================================
async fn extract_error_body(response: axum::response::Response) -> String {
    let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    String::from_utf8(body_bytes.to_vec())
        .unwrap_or_else(|_| "Unable to extract error body".to_string())
}
