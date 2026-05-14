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
    http::{Method, Request, StatusCode, header},
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

    user_uuid
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

    let set_cookie = login_response
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|v| v.to_str().ok())
        .expect("Should return Set-Cookie header");

    crate::tests::extract_set_cookie_token(set_cookie, "X-Auth")
        .expect("Should extract X-Auth token")
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
