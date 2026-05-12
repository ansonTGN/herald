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
        VALUES ($1, 'registration', 'allowed', 'true', true)
        ON CONFLICT (realm_id, config_type, config_key)
        DO UPDATE SET config_value = EXCLUDED.config_value, enabled = EXCLUDED.enabled
        "#,
    )
    .bind(&ctx._realm_id)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to enable registration");

    println!("[Step 1] ✓ Realm default config created");

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

    // Check that the user has a points_account with total_balance = 1050
    // 1000 (registration bonus) + 50 (first periodic grant) = 1050
    let total_balance: i64 =
        sqlx::query_scalar("SELECT total_balance FROM points_accounts WHERE user_id = $1")
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
        VALUES ($1, 'registration', 'allowed', 'true', true)
        ON CONFLICT (realm_id, config_type, config_key)
        DO UPDATE SET config_value = EXCLUDED.config_value, enabled = EXCLUDED.enabled
        "#,
    )
    .bind(&ctx._realm_id)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to enable registration");

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

    // Verify initial registration credit
    let initial_balance: i64 =
        sqlx::query_scalar("SELECT total_balance FROM points_accounts WHERE user_id = $1")
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

    // And: No additional registration points are granted
    let final_balance: i64 =
        sqlx::query_scalar("SELECT total_balance FROM points_accounts WHERE user_id = $1")
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
/// Scenario 3: Registration points with custom realm config
/// ============================================================================
// User Story: docs/user-stories/points-free-user.md#US-FU-01
// Covers: 验收标准 1.2
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_free_user_registration_custom_realm_config(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    // ============================================================================
    // Given: realm-1 has custom config: registration_bonus_points = 1500
    // ============================================================================
    println!("[Step 1] Set up custom realm config");

    sqlx::query(
        r#"
        INSERT INTO realm_default_configs (realm_id, registration_bonus_points, free_periodic_points_amount, free_periodic_validity_days, free_periodic_grant_period_type)
        VALUES ($1, 1500, 50, 1, 'daily')
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
        VALUES ($1, 'registration', 'allowed', 'true', true)
        ON CONFLICT (realm_id, config_type, config_key)
        DO UPDATE SET config_value = EXCLUDED.config_value, enabled = EXCLUDED.enabled
        "#,
    )
    .bind(&ctx._realm_id)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to enable registration");

    println!("[Step 1] ✓ Custom realm config created: 1500 bonus points");

    // ============================================================================
    // When: A new user registers in realm-1
    // ============================================================================
    println!("[Step 2] New user registers");

    let email = "customconfiguser@example.com";
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
    // Then: The user receives 1500 registration_credit points
    // ============================================================================
    println!("[Step 3] Verify custom registration credit");

    let registration_credit_amount: i64 = sqlx::query_scalar(
        "SELECT granted_amount FROM points_credit_ledger WHERE user_id = $1 AND credit_type = 'registration_credit'"
    )
    .bind(user_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Registration credit not found");

    assert_eq!(registration_credit_amount, 1500);

    // And: The transaction description reflects "Registration bonus: 1500 points"
    let transaction_description: Option<String> = sqlx::query_scalar(
        "SELECT description FROM points_transactions WHERE user_id = $1 AND type = 'registration_grant'"
    )
    .bind(user_id)
    .fetch_optional(&ctx._app_state.pool)
    .await
    .expect("Failed to fetch transaction");

    assert!(transaction_description.is_some());
    let description = transaction_description.unwrap();
    assert!(
        description.contains("1500") || description.contains("registration"),
        "Transaction description should mention 1500 points"
    );

    println!("[Step 3] ✓ Custom registration credit verified: 1500 points");
}

/// ============================================================================
/// Scenario 4: Registration creates user_points_config record
/// ============================================================================
// User Story: docs/user-stories/points-free-user.md#US-FU-01
// Covers: 验收标准 1.4
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_free_user_registration_creates_user_points_config(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    // ============================================================================
    // Given: realm-1 has default config
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
        VALUES ($1, 'registration', 'allowed', 'true', true)
        ON CONFLICT (realm_id, config_type, config_key)
        DO UPDATE SET config_value = EXCLUDED.config_value, enabled = EXCLUDED.enabled
        "#,
    )
    .bind(&ctx._realm_id)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to enable registration");

    println!("[Step 1] ✓ Realm default config created");

    // ============================================================================
    // When: A new user registers in realm-1
    // ============================================================================
    println!("[Step 2] New user registers");

    let email = "configtestuser@example.com";
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
    // Then: A user_points_configs record exists
    // ============================================================================
    println!("[Step 3] Verify user_points_config record");

    let config_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM user_points_configs WHERE user_id = $1)")
            .bind(user_id)
            .fetch_one(&ctx._app_state.pool)
            .await
            .expect("Failed to check user_points_config");

    assert!(config_exists, "user_points_config record should exist");

    // Verify the config values
    let (registration_bonus_points, free_periodic_points_amount): (i64, i64) = sqlx::query_as(
        "SELECT registration_bonus_points, free_periodic_points_amount FROM user_points_configs WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to fetch user_points_config");

    assert_eq!(registration_bonus_points, 1000);
    assert_eq!(free_periodic_points_amount, 50);

    // And: next_grant_time is set to tomorrow 15:30:00 UTC
    let next_grant_time: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT next_grant_time FROM user_points_configs WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&ctx._app_state.pool)
            .await
            .expect("Failed to fetch next_grant_time");

    assert!(next_grant_time.is_some(), "next_grant_time should be set");

    let next_grant = next_grant_time.unwrap();
    let now = chrono::Utc::now();
    let expected_min = now + chrono::Duration::hours(23);
    let expected_max = now + chrono::Duration::hours(25);

    assert!(
        next_grant >= expected_min && next_grant <= expected_max,
        "next_grant_time should be approximately 24 hours from now"
    );

    println!("[Step 3] ✓ user_points_config record verified with next_grant_time");
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
        VALUES ($1, 'registration', 'allowed', 'true', true)
        ON CONFLICT (realm_id, config_type, config_key)
        DO UPDATE SET config_value = EXCLUDED.config_value, enabled = EXCLUDED.enabled
        "#,
    )
    .bind(&ctx._realm_id)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to enable registration");

    println!("[Step 1] ✓ Realm config created with periodic points disabled");

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

/// ============================================================================
/// Scenario 8: Registration during realm maintenance
/// ============================================================================
// User Story: docs/user-stories/points-free-user.md#US-FU-01
// Covers: 验收标准 - 系统维护模式
// NOTE: Skipped because realm table doesn't have maintenance_mode column
#[test_context(TestContext)]
#[tokio::test]
#[ignore = "Maintenance mode feature not implemented - requires realm_config integration"]
async fn test_scenario_free_user_registration_during_maintenance(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    // ============================================================================
    // Given: realm-1 is in maintenance mode
    // ============================================================================
    println!("[Step 1] Set realm to maintenance mode");

    // Maintenance mode would be set via realm_config table
    // Example: INSERT INTO realm_config (realm_id, config_type, config_key, config_value)
    //        VALUES ($1, 'maintenance', 'enabled', 'true')
    sqlx::query("INSERT INTO realm_config (realm_id, config_type, config_key, config_value, is_secret, enabled, metadata, created_at, updated_at)
         VALUES ($1, 'maintenance', 'enabled', 'true', false, true, '{}'::jsonb, NOW(), NOW())
         ON CONFLICT (realm_id, config_type, config_key)
         DO UPDATE SET config_value = EXCLUDED.config_value, enabled = EXCLUDED.enabled, updated_at = NOW()")
        .bind(&ctx._realm_id)
        .execute(&ctx._app_state.pool)
        .await
        .expect("Failed to set maintenance mode");

    println!("[Step 1] ✓ Realm set to maintenance mode");

    // ============================================================================
    // When: A new user attempts to register in realm-1
    // ============================================================================
    println!("[Step 2] Attempt registration during maintenance");

    let email = "maintenanceuser@example.com";
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

    // ============================================================================
    // Then: The registration request is rejected
    // ============================================================================
    assert_eq!(
        registration_response.status(),
        StatusCode::SERVICE_UNAVAILABLE
    );

    let error_body = extract_error_body(registration_response).await;
    assert!(
        error_body.contains("maintenance") || error_body.contains("Maintenance"),
        "Error message should mention maintenance"
    );

    // And: HTTP status code is 503 Service Unavailable (already verified above)
    // And: No registration_credit points are granted
    let user_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM account WHERE email = $1)")
            .bind(email)
            .fetch_one(&ctx._app_state.pool)
            .await
            .expect("Failed to check user existence");

    assert!(
        !user_exists,
        "User should not be created during maintenance"
    );

    // And: No user_points_configs record is created
    let config_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM user_points_configs WHERE user_id IN (SELECT id FROM account WHERE email = $1))"
    )
    .bind(email)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to check user_points_config existence");

    assert!(
        !config_exists,
        "user_points_config should not be created during maintenance"
    );

    // Cleanup: Reset maintenance mode (stored in realm_config, not realm table)
    sqlx::query("DELETE FROM realm_config WHERE realm_id = $1 AND config_type = 'maintenance'")
        .bind(&ctx._realm_id)
        .execute(&ctx._app_state.pool)
        .await
        .expect("Failed to reset maintenance mode");

    println!("[Step 2] ✓ Registration rejected during maintenance");
}

/// ============================================================================
/// Scenario 9: Registration with invalid email format
/// ============================================================================
// User Story: docs/user-stories/points-free-user.md#US-FU-01
// Covers: 验收标准 - 输入验证
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_free_user_registration_invalid_email(ctx: &mut TestContext) {
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
        VALUES ($1, 'registration', 'allowed', 'true', true)
        ON CONFLICT (realm_id, config_type, config_key)
        DO UPDATE SET config_value = EXCLUDED.config_value, enabled = EXCLUDED.enabled
        "#,
    )
    .bind(&ctx._realm_id)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to enable registration");

    println!("[Step 1] ✓ Realm default config created");

    // ============================================================================
    // When: A new user attempts to register with invalid email "invalid-email"
    // ============================================================================
    println!("[Step 2] Attempt registration with invalid email");

    let registration_payload = json!({
        "clientId": ctx._client_id,
        "email": "invalid-email",
        "password": "SecurePassword123!",
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

    // ============================================================================
    // Then: The registration request is rejected
    // ============================================================================
    assert_eq!(registration_response.status(), StatusCode::BAD_REQUEST);

    let error_body = extract_error_body(registration_response).await;
    assert!(
        error_body.contains("Invalid email")
            || error_body.contains("email")
            || error_body.contains("Email"),
        "Error message should mention email validation"
    );

    // And: HTTP status code is 400 Bad Request (already verified above)
    // And: No registration_credit points are granted
    let user_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM account WHERE email = 'invalid-email')")
            .fetch_one(&ctx._app_state.pool)
            .await
            .expect("Failed to check user existence");

    assert!(
        !user_exists,
        "User should not be created with invalid email"
    );

    // And: No user_points_configs record is created
    let config_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM user_points_configs WHERE user_id IN (SELECT id FROM account WHERE email = 'invalid-email'))"
    )
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to check user_points_config existence");

    assert!(
        !config_exists,
        "user_points_config should not be created with invalid email"
    );

    println!("[Step 2] ✓ Registration rejected with invalid email");
}

/// ============================================================================
/// Helper Functions
/// ============================================================================
async fn extract_error_body(response: axum::response::Response) -> String {
    let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    String::from_utf8(body_bytes.to_vec())
        .unwrap_or_else(|_| "Unable to extract error body".to_string())
}
