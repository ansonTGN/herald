// **User Story**: US-FU-01 (Registration grants initial bonus points)
// **Priority**: P0
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

use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use herald_core::domain::points::entities::{
    CreditSourceType, CreditType, QuotaEntitlementStatus, QuotaSourceType,
};
use serde_json::json;
use std::sync::Arc;
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
    let realm_id = ctx._realm_id.clone();
    let registration_bonus: i64 = 1000;
    let free_periodic_amount: i64 = 50;

    // Given: realm-1 has default config: registration_bonus_points = 1000 and a
    // free-periodic quota window (points-grant-redesign BE-D07: the grant is a
    // `points_quota_entitlements` row, not a ledger row).
    println!("[Step 1] Set up realm default config");

    sqlx::query(
        r#"
        INSERT INTO realm_default_configs
            (realm_id, registration_bonus_points, free_periodic_points_amount,
             free_periodic_validity_days, free_periodic_grant_period_type, free_periodic_quota_windows)
        VALUES ($1, $2, $3, 1, 'daily', $4::jsonb)
        ON CONFLICT (realm_id) DO UPDATE SET
            registration_bonus_points = EXCLUDED.registration_bonus_points,
            free_periodic_points_amount = EXCLUDED.free_periodic_points_amount,
            free_periodic_validity_days = EXCLUDED.free_periodic_validity_days,
            free_periodic_grant_period_type = EXCLUDED.free_periodic_grant_period_type,
            free_periodic_quota_windows = EXCLUDED.free_periodic_quota_windows
        "#,
    )
    .bind(&realm_id)
    .bind(registration_bonus)
    .bind(free_periodic_amount)
    .bind(json!([{"windowSeconds": 86400, "limit": free_periodic_amount, "key": "day"}]))
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
    .bind(&realm_id)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to enable registration");

    println!("[Step 1] ✓ Realm default config created");

    // Materialize the realm's registration-pool bucket so both the registration
    // ledger grant and the free-periodic quota entitlement land there.
    crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm(
        &ctx._app_state.pool,
        &realm_id,
    )
    .await;

    // When: A new user registers in realm-1
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
        .uri(format!("/api/auth/{}/register", realm_id))
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

    // Then: The user receives 1000 registration_credit points as a ledger row
    println!("[Step 3] Verify registration credit grant");

    let registration_ledgers =
        crate::tests::helpers::points_helpers::get_user_ledgers_by_credit_type(
            ctx,
            user_id,
            CreditType::RegistrationCredit,
        )
        .await;
    assert_eq!(
        registration_ledgers.len(),
        1,
        "registration_credit must be a single ledger row"
    );
    let reg_row = &registration_ledgers[0];
    assert_eq!(reg_row.granted_amount, registration_bonus);
    assert_eq!(reg_row.source_type, CreditSourceType::Registration);
    assert!(
        reg_row.expires_at.is_none(),
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

    // And: the user receives the free-periodic grant as a quota entitlement.
    let free_entitlements = crate::tests::helpers::points_helpers::get_user_quota_entitlements(
        ctx,
        user_id,
        CreditType::FreePeriodicCredit,
    )
    .await;
    assert_eq!(
        free_entitlements.len(),
        1,
        "free_periodic_credit must be a single quota entitlement"
    );
    let fp = &free_entitlements[0];
    assert_eq!(fp.status, QuotaEntitlementStatus::Active);
    assert_eq!(fp.source_type, QuotaSourceType::FreePeriodicGrant);
    assert_eq!(
        fp.quota_windows.first().map(|w| w.limit).unwrap_or(0),
        free_periodic_amount
    );

    // Total derived available balance includes the ledger-based registration
    // credit plus the window-based free-periodic availability.
    let total_balance =
        crate::tests::helpers::points_helpers::get_derived_total_balance(ctx, user_id, &realm_id)
            .await;
    assert_eq!(total_balance, registration_bonus + free_periodic_amount);

    println!(
        "[Step 3] ✓ Registration credit verified: {} points ({} registration + {} periodic quota)",
        total_balance, registration_bonus, free_periodic_amount
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
    let realm_id = ctx._realm_id.clone();
    let registration_bonus: i64 = 1000;
    let free_periodic_amount: i64 = 50;

    // Given: A user has already received registration bonus points
    println!("[Step 1] Create user with registration bonus");

    sqlx::query(
        r#"
        INSERT INTO realm_default_configs
            (realm_id, registration_bonus_points, free_periodic_points_amount,
             free_periodic_validity_days, free_periodic_grant_period_type, free_periodic_quota_windows)
        VALUES ($1, $2, $3, 1, 'daily', $4::jsonb)
        ON CONFLICT (realm_id) DO UPDATE SET
            registration_bonus_points = EXCLUDED.registration_bonus_points,
            free_periodic_points_amount = EXCLUDED.free_periodic_points_amount,
            free_periodic_validity_days = EXCLUDED.free_periodic_validity_days,
            free_periodic_grant_period_type = EXCLUDED.free_periodic_grant_period_type,
            free_periodic_quota_windows = EXCLUDED.free_periodic_quota_windows
        "#,
    )
    .bind(&realm_id)
    .bind(registration_bonus)
    .bind(free_periodic_amount)
    .bind(json!([{"windowSeconds": 86400, "limit": free_periodic_amount, "key": "day"}]))
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
    .bind(&realm_id)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to enable registration");

    crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm(
        &ctx._app_state.pool,
        &realm_id,
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
        .uri(format!("/api/auth/{}/register", realm_id))
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

    let initial_balance =
        crate::tests::helpers::points_helpers::get_derived_total_balance(ctx, user_id, &realm_id)
            .await;
    assert_eq!(initial_balance, registration_bonus + free_periodic_amount);

    let initial_free_entitlements =
        crate::tests::helpers::points_helpers::get_user_quota_entitlements(
            ctx,
            user_id,
            CreditType::FreePeriodicCredit,
        )
        .await;
    assert_eq!(initial_free_entitlements.len(), 1);

    println!(
        "[Step 1] ✓ User created with registration bonus: {}",
        user_id
    );

    // When: The same user attempts to register again (duplicate email)
    println!("[Step 2] Attempt duplicate registration");

    let duplicate_payload = json!({
        "clientId": ctx._client_id,
        "email": email,
        "password": "AnotherPassword123!",
        "turnstileToken": "dummy"
    });

    let duplicate_request = Request::builder()
        .method("POST")
        .uri(format!("/api/auth/{}/register", realm_id))
        .header("content-type", "application/json")
        .header("x-forwarded-for", "1.1.1.2")
        .body(Body::from(duplicate_payload.to_string()))
        .unwrap();

    let duplicate_response = app.clone().oneshot(duplicate_request).await.unwrap();

    // Then: The system returns error "Email already registered"
    assert_eq!(duplicate_response.status(), StatusCode::CONFLICT);

    let error_body = extract_error_body(duplicate_response).await;
    assert!(
        error_body.contains("Email already registered")
            || error_body.contains("already registered")
    );

    // And: No additional points or quota entitlements are granted.
    let final_balance =
        crate::tests::helpers::points_helpers::get_derived_total_balance(ctx, user_id, &realm_id)
            .await;
    assert_eq!(
        final_balance,
        registration_bonus + free_periodic_amount,
        "Balance should remain unchanged"
    );

    let final_free_entitlements =
        crate::tests::helpers::points_helpers::get_user_quota_entitlements(
            ctx,
            user_id,
            CreditType::FreePeriodicCredit,
        )
        .await;
    assert_eq!(
        final_free_entitlements.len(),
        1,
        "Free-periodic entitlement must stay idempotent"
    );

    let registration_credit_balance: i64 = sqlx::query_scalar(
        "SELECT CAST(COALESCE(SUM(granted_amount), 0) AS BIGINT) FROM points_credit_ledger WHERE user_id = $1 AND credit_type = 'registration_credit'"
    )
    .bind(user_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .unwrap_or(0);

    assert_eq!(
        registration_credit_balance, registration_bonus,
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
    let realm_id = ctx._realm_id.clone();

    // Given: realm-1 has config: free_periodic_quota_windows empty (periodic points disabled)
    println!("[Step 1] Set up realm config with periodic points disabled");

    sqlx::query(
        r#"
        INSERT INTO realm_default_configs
            (realm_id, registration_bonus_points, free_periodic_points_amount,
             free_periodic_validity_days, free_periodic_grant_period_type, free_periodic_quota_windows)
        VALUES ($1, 1000, 0, 1, 'daily', NULL)
        ON CONFLICT (realm_id) DO UPDATE SET
            registration_bonus_points = EXCLUDED.registration_bonus_points,
            free_periodic_points_amount = EXCLUDED.free_periodic_points_amount,
            free_periodic_validity_days = EXCLUDED.free_periodic_validity_days,
            free_periodic_grant_period_type = EXCLUDED.free_periodic_grant_period_type,
            free_periodic_quota_windows = EXCLUDED.free_periodic_quota_windows
        "#,
    )
    .bind(&realm_id)
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
    .bind(&realm_id)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to enable registration");

    println!("[Step 1] ✓ Realm config created with periodic points disabled");

    crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm(
        &ctx._app_state.pool,
        &realm_id,
    )
    .await;

    // When: A new user registers in realm-1
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
        .uri(format!("/api/auth/{}/register", realm_id))
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

    // Then: The user receives registration_credit points but NO periodic quota entitlement
    println!("[Step 3] Verify registration credit and no periodic grant");

    let registration_credit_amount: i64 = sqlx::query_scalar(
        "SELECT CAST(COALESCE(SUM(granted_amount), 0) AS BIGINT) FROM points_credit_ledger WHERE user_id = $1 AND credit_type = 'registration_credit'"
    )
    .bind(user_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to fetch registration credit");

    assert_eq!(registration_credit_amount, 1000);

    let free_entitlements = crate::tests::helpers::points_helpers::get_user_quota_entitlements(
        ctx,
        user_id,
        CreditType::FreePeriodicCredit,
    )
    .await;
    assert!(
        free_entitlements.is_empty(),
        "No periodic quota entitlement should be granted when free_periodic_quota_windows is empty"
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

    println!("[Step 3] ✓ Registration credit (1000) verified, no periodic quota entitlement");
}

/// ============================================================================
/// Free-periodic on-time grant + two distinct sources
/// **User Story**: US-FU-004 (按时获得每期免费积分) + US-FU-002 (免费定期积分按时发放)
/// **Priority**: P0
/// **Design refs**: `.ai/design/point-time.md`
///
/// points-grant-redesign (BE-D07): free-periodic grants are window-quota
/// entitlements (`points_quota_entitlements`), not ledger rows. Registration
/// credits remain ledger rows. These tests assert that split.
/// ============================================================================
use herald_core::domain::points::GrantScheduler;

/// ============================================================================
/// Scenario 1: Free-periodic first period is immediately available
/// ============================================================================
// User Story: docs/user-stories/points-free-user.md#US-FU-004
// Covers: first-period free-periodic grant is a quota entitlement that is active
//         and immediately available (effective_from <= now).
#[test_context(TestContext)]
#[tokio::test]
async fn test_free_periodic_first_period_immediately_available(ctx: &mut TestContext) {
    let pool = &ctx._app_state.pool;
    let realm_id = ctx._realm_id.clone();
    let points_per_period: i64 = 50;

    crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm(pool, &realm_id).await;

    // Configure a realm where free periodic is enabled and registration bonus
    // is zero, so the only grant is the free-periodic quota entitlement.
    sqlx::query(
        r#"
        INSERT INTO realm_default_configs
            (realm_id, registration_bonus_points, free_periodic_points_amount,
             free_periodic_validity_days, free_periodic_grant_period_type, free_periodic_quota_windows)
        VALUES ($1, 0, $2, 1, 'daily', $3::jsonb)
        ON CONFLICT (realm_id) DO UPDATE SET
            registration_bonus_points = EXCLUDED.registration_bonus_points,
            free_periodic_points_amount = EXCLUDED.free_periodic_points_amount,
            free_periodic_validity_days = EXCLUDED.free_periodic_validity_days,
            free_periodic_grant_period_type = EXCLUDED.free_periodic_grant_period_type,
            free_periodic_quota_windows = EXCLUDED.free_periodic_quota_windows
        "#,
    )
    .bind(&realm_id)
    .bind(points_per_period)
    .bind(json!([{"windowSeconds": 86400, "limit": points_per_period, "key": "day"}]))
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

    // Register a new user via the real HTTP path — this triggers
    // registration_service which writes the free-periodic quota entitlement.
    let app = ctx.create_unified_test_router();
    let email = format!("free-periodic-first-{}@test.com", uuid::Uuid::now_v7());
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

    // Exactly one active FreePeriodicCredit entitlement exists.
    let entitlements = crate::tests::helpers::points_helpers::get_user_quota_entitlements(
        ctx,
        user_id,
        CreditType::FreePeriodicCredit,
    )
    .await;
    assert_eq!(
        entitlements.len(),
        1,
        "free_periodic_credit must be granted as a single quota entitlement"
    );
    let entitlement = &entitlements[0];
    assert_eq!(entitlement.status, QuotaEntitlementStatus::Active);
    assert_eq!(entitlement.source_type, QuotaSourceType::FreePeriodicGrant);
    assert!(
        entitlement.effective_from <= chrono::Utc::now(),
        "first-period entitlement must be effective immediately"
    );

    // No free_periodic_credit ledger row should exist under the window model.
    let free_periodic_ledgers =
        crate::tests::helpers::points_helpers::get_user_ledgers_by_credit_type(
            ctx,
            user_id,
            CreditType::FreePeriodicCredit,
        )
        .await;
    assert!(
        free_periodic_ledgers.is_empty(),
        "free_periodic_credit must NOT be written to points_credit_ledger"
    );

    // Window availability equals the configured per-period amount.
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
/// Scenario 2: Registration credit and free_periodic first period are
/// two distinct sources
/// ============================================================================
// User Story: docs/user-stories/points-free-user.md#US-FU-001 + US-FU-002
// Covers: "注册初始积分与 free_periodic 首期作为两笔不同来源".
// WHY this test exists: registration initial bonus points and the free_periodic
// grant are TWO DIFFERENT entitlement sources. Registration remains a ledger
// row; free_periodic is a quota entitlement. They must not substitute for each
// other.
#[test_context(TestContext)]
#[tokio::test]
async fn test_registration_credit_and_free_periodic_two_distinct_sources(ctx: &mut TestContext) {
    let pool = &ctx._app_state.pool;
    let realm_id = ctx._realm_id.clone();
    let registration_bonus: i64 = 1000;
    let free_periodic_amount: i64 = 50;

    // Configure the realm with BOTH a registration bonus AND a non-zero
    // free-periodic quota window.
    sqlx::query(
        r#"
        INSERT INTO realm_default_configs
            (realm_id, registration_bonus_points, free_periodic_points_amount,
             free_periodic_validity_days, free_periodic_grant_period_type, free_periodic_quota_windows)
        VALUES ($1, $2, $3, 1, 'daily', $4::jsonb)
        ON CONFLICT (realm_id) DO UPDATE SET
            registration_bonus_points = EXCLUDED.registration_bonus_points,
            free_periodic_points_amount = EXCLUDED.free_periodic_points_amount,
            free_periodic_validity_days = EXCLUDED.free_periodic_validity_days,
            free_periodic_grant_period_type = EXCLUDED.free_periodic_grant_period_type,
            free_periodic_quota_windows = EXCLUDED.free_periodic_quota_windows
        "#,
    )
    .bind(&realm_id)
    .bind(registration_bonus)
    .bind(free_periodic_amount)
    .bind(json!([{"windowSeconds": 86400, "limit": free_periodic_amount, "key": "day"}]))
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

    // Register a new user via the real HTTP path.
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

    // Invariant: one registration ledger row and one free-periodic entitlement.
    let registration_ledgers =
        crate::tests::helpers::points_helpers::get_user_ledgers_by_credit_type(
            ctx,
            user_id,
            CreditType::RegistrationCredit,
        )
        .await;
    let free_periodic_entitlements =
        crate::tests::helpers::points_helpers::get_user_quota_entitlements(
            ctx,
            user_id,
            CreditType::FreePeriodicCredit,
        )
        .await;

    assert!(
        !registration_ledgers.is_empty(),
        "A10 violation: registration_credit ledger row missing"
    );
    assert!(
        !free_periodic_entitlements.is_empty(),
        "A10 violation: free_periodic_credit quota entitlement missing"
    );

    let reg_row = &registration_ledgers[0];
    let fp_row = &free_periodic_entitlements[0];
    assert_eq!(
        reg_row.source_type,
        CreditSourceType::Registration,
        "registration_credit row must carry source_type=Registration"
    );
    assert_eq!(
        fp_row.source_type,
        QuotaSourceType::FreePeriodicGrant,
        "free_periodic entitlement must carry source_type=FreePeriodicGrant"
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
        fp_row.quota_windows.first().map(|w| w.limit).unwrap_or(0),
        free_periodic_amount,
        "free_periodic amount must be the configured per-period amount"
    );

    // Derived balances: each pool holds its own amount independently.
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
        "derived total balance must be the SUM of the two distinct sources"
    );
}

/// ============================================================================
/// Scenario 3: Future-effective entitlement is active but unavailable until
/// its effective_from boundary is reached
/// ============================================================================
// User Story: docs/user-stories/points-free-user.md#US-FU-004
// Covers: an active entitlement with effective_from in the future contributes
// zero window availability; moving effective_to the past makes it available
// immediately.
#[test_context(TestContext)]
#[tokio::test]
async fn test_free_periodic_pre_grant_lead_time_effective_at_future(ctx: &mut TestContext) {
    let pool = &ctx._app_state.pool;
    let realm_id = ctx._realm_id.clone();
    let points_per_period: i64 = 30;

    crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm(pool, &realm_id).await;

    sqlx::query(
        r#"
        INSERT INTO realm_default_configs
            (realm_id, registration_bonus_points, free_periodic_points_amount,
             free_periodic_validity_days, free_periodic_grant_period_type, free_periodic_quota_windows)
        VALUES ($1, 0, $2, 1, 'monthly', $3::jsonb)
        ON CONFLICT (realm_id) DO UPDATE SET
            registration_bonus_points = EXCLUDED.registration_bonus_points,
            free_periodic_points_amount = EXCLUDED.free_periodic_points_amount,
            free_periodic_validity_days = EXCLUDED.free_periodic_validity_days,
            free_periodic_grant_period_type = EXCLUDED.free_periodic_grant_period_type,
            free_periodic_quota_windows = EXCLUDED.free_periodic_quota_windows
        "#,
    )
    .bind(&realm_id)
    .bind(points_per_period)
    .bind(json!([{"windowSeconds": 2592000, "limit": points_per_period, "key": "month"}]))
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

    // Register to create the entitlement (effective_from = registration time).
    let app = ctx.create_unified_test_router();
    let email = format!("pre-grant-{}@test.com", uuid::Uuid::now_v7());
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

    let entitlements = crate::tests::helpers::points_helpers::get_user_quota_entitlements(
        ctx,
        user_id,
        CreditType::FreePeriodicCredit,
    )
    .await;
    assert_eq!(entitlements.len(), 1);
    let entitlement_id = entitlements[0].id;
    let bucket_id =
        crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm(pool, &realm_id).await;

    // Simulate a future period: keep the row active but move effective_from ahead.
    let now = chrono::Utc::now();
    let future = now + chrono::Duration::hours(2);
    sqlx::query(
        "UPDATE points_quota_entitlements SET effective_from = $1, updated_at = NOW() WHERE id = $2",
    )
    .bind(future)
    .bind(entitlement_id)
    .execute(pool)
    .await
    .expect("Failed to move entitlement effective_from to the future");

    // The row is still active, but it is not yet within its effective interval.
    let entitlements_after = crate::tests::helpers::points_helpers::get_user_quota_entitlements(
        ctx,
        user_id,
        CreditType::FreePeriodicCredit,
    )
    .await;
    assert_eq!(entitlements_after[0].status, QuotaEntitlementStatus::Active);
    assert!(entitlements_after[0].effective_from > now);

    assert_eq!(
        crate::tests::helpers::points_helpers::count_all_quota_entitlements(
            ctx,
            &realm_id,
            user_id,
            bucket_id,
            CreditType::FreePeriodicCredit,
        )
        .await,
        1
    );
    assert_eq!(
        crate::tests::helpers::points_helpers::count_active_quota_entitlements(
            ctx,
            &realm_id,
            user_id,
            bucket_id,
            CreditType::FreePeriodicCredit,
        )
        .await,
        0,
        "future-effective entitlement must not count as active"
    );

    crate::tests::helpers::points_helpers::assert_derived_balance(
        ctx,
        user_id,
        &realm_id,
        CreditType::FreePeriodicCredit,
        0,
    )
    .await;

    // Simulate the clock catching up: move effective_from into the past.
    sqlx::query(
        "UPDATE points_quota_entitlements SET effective_from = $1, updated_at = NOW() WHERE id = $2",
    )
    .bind(now - chrono::Duration::seconds(1))
    .bind(entitlement_id)
    .execute(pool)
    .await
    .expect("Failed to move entitlement effective_from to the past");

    assert_eq!(
        crate::tests::helpers::points_helpers::count_active_quota_entitlements(
            ctx,
            &realm_id,
            user_id,
            bucket_id,
            CreditType::FreePeriodicCredit,
        )
        .await,
        1
    );
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
/// Scenario 4: Quota entitlement expiry is swept by the scheduler when
/// effective_until lapses
/// ============================================================================
// User Story: docs/user-stories/points-free-user.md#US-FU-004
// Covers: under the window-quota model the free-periodic entitlement is ongoing
// (effective_until = NULL). The scheduler's job is to sweep any entitlement
// whose effective_until has passed and mark it expired.
#[test_context(TestContext)]
#[tokio::test]
async fn test_free_periodic_expires_anchored_to_grant_time(ctx: &mut TestContext) {
    let pool = &ctx._app_state.pool;
    let realm_id = ctx._realm_id.clone();
    let points_per_period: i64 = 40;

    crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm(pool, &realm_id).await;

    sqlx::query(
        r#"
        INSERT INTO realm_default_configs
            (realm_id, registration_bonus_points, free_periodic_points_amount,
             free_periodic_validity_days, free_periodic_grant_period_type, free_periodic_quota_windows)
        VALUES ($1, 0, $2, 1, 'daily', $3::jsonb)
        ON CONFLICT (realm_id) DO UPDATE SET
            registration_bonus_points = EXCLUDED.registration_bonus_points,
            free_periodic_points_amount = EXCLUDED.free_periodic_points_amount,
            free_periodic_validity_days = EXCLUDED.free_periodic_validity_days,
            free_periodic_grant_period_type = EXCLUDED.free_periodic_grant_period_type,
            free_periodic_quota_windows = EXCLUDED.free_periodic_quota_windows
        "#,
    )
    .bind(&realm_id)
    .bind(points_per_period)
    .bind(json!([{"windowSeconds": 86400, "limit": points_per_period, "key": "day"}]))
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

    // Register to create the free-periodic entitlement.
    let app = ctx.create_unified_test_router();
    let email = format!("expires-anchor-{}@test.com", uuid::Uuid::now_v7());
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

    // The free-periodic entitlement created at registration is ongoing.
    let free_entitlements = crate::tests::helpers::points_helpers::get_user_quota_entitlements(
        ctx,
        user_id,
        CreditType::FreePeriodicCredit,
    )
    .await;
    assert_eq!(free_entitlements.len(), 1);
    assert!(
        free_entitlements[0].effective_until.is_none(),
        "free_periodic entitlement must be ongoing (effective_until = NULL)"
    );

    // Create a separate entitlement with effective_until in the past to verify
    // the scheduler sweeps lapsed rows.
    let bucket_id =
        crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm(pool, &realm_id).await;
    let past = chrono::Utc::now() - chrono::Duration::hours(1);
    let expired_id = crate::tests::helpers::points_helpers::grant_quota_entitlement_for_test(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::FreePeriodicCredit,
        QuotaSourceType::FreePeriodicGrant,
        "future-expired",
        &[(86400, 10, "day")],
        past,
        Some(past),
    )
    .await;

    let scheduler = GrantScheduler::new(
        Arc::clone(&ctx._app_state.points_repository),
        Arc::clone(&ctx._app_state.points_service),
    );
    let summary = scheduler
        .process_due_schedules()
        .await
        .expect("GrantScheduler::process_due_schedules failed");
    assert!(
        summary.processed >= 1,
        "expected the scheduler to expire the lapsed entitlement; got summary {:?}",
        summary
    );

    let expired = crate::tests::helpers::points_helpers::get_user_quota_entitlements(
        ctx,
        user_id,
        CreditType::FreePeriodicCredit,
    )
    .await
    .into_iter()
    .find(|e| e.id == expired_id)
    .expect("expired entitlement row not found");
    assert_eq!(
        expired.status,
        QuotaEntitlementStatus::Expired,
        "scheduler must mark lapsed effective_until rows as expired"
    );

    // The original ongoing entitlement remains active.
    let ongoing = crate::tests::helpers::points_helpers::get_user_quota_entitlements(
        ctx,
        user_id,
        CreditType::FreePeriodicCredit,
    )
    .await
    .into_iter()
    .find(|e| e.id == free_entitlements[0].id)
    .expect("ongoing entitlement row not found");
    assert_eq!(ongoing.status, QuotaEntitlementStatus::Active);
}

/// Helper Functions
/// ============================================================================
async fn extract_error_body(response: axum::response::Response) -> String {
    let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    String::from_utf8(body_bytes.to_vec())
        .unwrap_or_else(|_| "Unable to extract error body".to_string())
}
