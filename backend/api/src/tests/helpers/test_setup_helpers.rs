// =============================================================================
// Test Setup Helper Functions
// =============================================================================
//
// **Purpose**: Eliminate common code duplication across all scenario tests by
// providing shared utilities for authentication, request building, and response validation.
//
// **Benefits**:
// - Reduces test code by 40-50%
// - Provides consistent test patterns
// - Easier to maintain and extend
// - Centralized authentication logic
//
// **Usage**:
// ```rust
// use crate::tests::helpers::test_setup_helpers::*;
//
// // Create admin user and login
// let token = create_admin_and_login(ctx, "admin@test.com", "admin123").await;
//
// // Make authenticated request
// let response = make_authenticated_get_request(ctx, &token, "/api/users", None).await;
//
// // Validate response
// assert_response_success(&response);
// let data = assert_response_data(&response);
// ```
//
// =============================================================================

use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

// =============================================================================
// Authentication Helpers
// =============================================================================

/// Create regular user and login, returning (user_id, token)
///
/// # Arguments
/// * `ctx` - Test context
/// * `email` - User email
/// * `password` - User password
///
/// # Returns
/// (user_id, token) tuple
///
/// # Example
/// ```rust
/// let (user_id, token) = create_user_and_login(ctx, "user@test.com", "password123").await;
/// ```
pub async fn create_user_and_login(
    ctx: &mut TestContext,
    email: &str,
    password: &str,
) -> (Uuid, String) {
    println!("[Auth] Creating user: {}", email);
    let user_id = create_test_user(ctx, email, password).await;

    println!("[Auth] User logging in: {}", email);
    let token = login_user(ctx, email, password).await;

    println!("[Auth] ✓ User logged in successfully");
    (user_id, token)
}

/// Create test user with password hashing (without authentication/roles)
///
/// # Arguments
/// * `ctx` - Test context
/// * `email` - User email
/// * `password` - User password
///
/// # Returns
/// User UUID
///
/// # Example
/// ```rust
/// let user_id = create_test_user(ctx, "user@test.com", "password123").await;
/// ```
pub async fn create_test_user(ctx: &mut TestContext, email: &str, password: &str) -> Uuid {
    let user_uuid = Uuid::now_v7();
    let password_hash =
        bcrypt::hash(password, bcrypt::DEFAULT_COST).expect("Failed to hash password");

    sqlx::query(
        "INSERT INTO account (id, realm_id, email, password, status)
         VALUES ($1, $2, $3, $4, 1)",
    )
    .bind(user_uuid)
    .bind(&ctx._realm_id)
    .bind(email)
    .bind(&password_hash)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create test user");

    // Register-time consent is recorded by the real register handler. Test users
    // created via SQL bypass that path, so mirror it here to prevent the legal-
    // consent gate (BE-D08) from blocking their logins.
    record_test_user_consent(&ctx._app_state.pool, user_uuid, &ctx._realm_id).await;

    user_uuid
}

/// Record consent to the current effective ToS and Privacy Policy for a test user.
/// Mirrors the registration-time consent recording so that subsequent logins do
/// not get blocked by the legal-consent gate (BE-D08).
pub async fn record_test_user_consent(pool: &sqlx::PgPool, user_id: Uuid, realm_id: &str) {
    for agreement_type in ["terms_of_service", "privacy_policy"] {
        let version_id: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM legal_agreement_version
             WHERE agreement_type = $1
               AND (realm_id = $2 OR realm_id IS NULL)
             ORDER BY CASE WHEN realm_id = $2 THEN 0 ELSE 1 END, version_no DESC
             LIMIT 1",
        )
        .bind(agreement_type)
        .bind(realm_id)
        .fetch_optional(pool)
        .await
        .expect("Failed to look up effective agreement version");

        if let Some(version_id) = version_id {
            sqlx::query(
                "INSERT INTO user_agreement_consent (id, user_id, realm_id, agreement_type, consented_version_id)
                 VALUES ($1, $2, $3, $4, $5)
                 ON CONFLICT (user_id, agreement_type)
                 DO UPDATE SET consented_version_id = EXCLUDED.consented_version_id, consented_at = NOW()",
            )
            .bind(Uuid::now_v7())
            .bind(user_id)
            .bind(realm_id)
            .bind(agreement_type)
            .bind(version_id)
            .execute(pool)
            .await
            .expect("Failed to record test user consent");
        }
    }
}

/// Perform login and return authentication token
///
/// # Arguments
/// * `ctx` - Test context
/// * `email` - User email
/// * `password` - User password
///
/// # Returns
/// Authentication token string
///
/// # Example
/// ```rust
/// let token = login_user(ctx, "admin@test.com", "password123").await;
/// ```
pub async fn login_user(ctx: &mut TestContext, email: &str, password: &str) -> String {
    let login_payload = json!({
        "clientId": ctx._client_id,
        "email": email,
        "password": password,
        "turnstileToken": "dummy"
    });

    let login_request = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/auth/{}/login", ctx._realm_id))
        .header("content-type", "application/json")
        .header("x-forwarded-for", "3.3.3.3")
        .body(Body::from(login_payload.to_string()))
        .unwrap();

    let login_response = ctx
        .create_unified_test_router()
        .oneshot(login_request)
        .await
        .unwrap();
    assert_eq!(login_response.status(), StatusCode::OK);

    let (_response, token) = crate::tests::extract_bearer_token(login_response).await;
    token.expect("Login should return accessToken")
}

// =============================================================================
// HTTP Request Helpers
// =============================================================================

// =============================================================================
// Response Validation Helpers
// =============================================================================

/// Assert response contains error message
///
/// # Arguments
/// * `response` - HTTP response to validate
/// * `expected_message` - Expected error message (optional)
///
/// # Example
/// ```rust
/// let response = make_authenticated_get_request(ctx, &token, "/users", None).await;
/// assert_response_error(&response, Some("User not found"));
/// ```
pub fn assert_response_error(response: &serde_json::Value, expected_message: Option<&str>) {
    assert!(
        response.get("error").is_some() || response.get("message").is_some(),
        "Response should contain error field"
    );

    if let Some(expected) = expected_message {
        let actual = response
            .get("error")
            .and_then(|e| e.as_str())
            .or_else(|| response.get("message").and_then(|m| m.as_str()))
            .expect("Response should have error message");

        assert_eq!(
            actual, expected,
            "Error message should match expected: {}",
            expected
        );
    }
}
