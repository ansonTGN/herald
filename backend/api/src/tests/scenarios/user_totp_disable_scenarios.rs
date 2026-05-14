// =============================================================================
// User TOTP Disable Scenarios
// =============================================================================
//
// **User Story**: US-TO-004 (User Disables TOTP)
// **Priority**: P0
//
// **Scenario**: User disables TOTP authentication (if not forced)
//
// **Given**:
// - A user with TOTP enabled
// - Realm does not force TOTP
//
// **When**:
// - The user requests to disable TOTP with password verification
//
// **Then**:
// - TOTP is disabled for the user
// - Backup codes are cleared
// - User can login without TOTP
//
// =============================================================================

use crate::tests::helpers::*;
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use serde_json::json;
use test_context::test_context;
use tower::ServiceExt;

/// ============================================================================
/// Scenario 1: User Disables TOTP Successfully
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_user_disables_totp_successfully(ctx: &mut TestContext) {
    // ============================================================================
    // Given: A user with TOTP enabled, realm does not force TOTP
    // ============================================================================
    println!("[Step 1] Create user with TOTP enabled");

    let app = ctx.create_unified_test_router();
    let (token, user_id) =
        create_admin_session_with_user(ctx, "user-disable-totp@test.com", 1800).await;

    // Update user password
    let password_hash = bcrypt::hash("Password123!", bcrypt::DEFAULT_COST).unwrap();
    sqlx::query("UPDATE account SET password = $1 WHERE id = $2::uuid")
        .bind(password_hash)
        .bind(&user_id)
        .execute(&ctx._app_state.pool)
        .await
        .expect("Failed to update password");

    // Enable TOTP for the user
    let secret_hash = "JBSWY3DPEHPK3PXP"; // In production this should be properly encrypted

    sqlx::query(
        "INSERT INTO user_totp_config (user_id, realm_id, secret_hash, enabled, verified_at, created_at, updated_at)
         VALUES ($1::uuid, $2, $3, $4, NOW(), NOW(), NOW())
         ON CONFLICT (user_id) DO UPDATE SET enabled = EXCLUDED.enabled, updated_at = NOW()",
    )
    .bind(&user_id)
    .bind(&ctx._realm_id)
    .bind(secret_hash)
    .bind(true)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to enable TOTP for user");

    // Verify TOTP is enabled
    let (enabled,): (bool,) =
        sqlx::query_as("SELECT enabled FROM user_totp_config WHERE user_id = $1::uuid")
            .bind(&user_id)
            .fetch_one(&ctx._app_state.pool)
            .await
            .expect("Failed to check TOTP status");

    assert!(enabled, "TOTP should be initially enabled");

    println!("[Step 1] ✓ User with TOTP enabled created");

    // ============================================================================
    // When: User disables TOTP with password verification
    // ============================================================================
    println!("[Step 2] Disable TOTP");

    let disable_request = json!({
        "password": "Password123!"
    });

    let req = Request::builder()
        .method("DELETE")
        .uri("/api/user/totp")
        .header("content-type", "application/json")
        .header(header::COOKIE, format!("X-Auth={token}"))
        .body(Body::from(disable_request.to_string()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    println!("[Step 2] ✓ TOTP disable request sent");

    // ============================================================================
    // Then: Verify TOTP is disabled
    // ============================================================================
    println!("[Step 3] Verify TOTP disabled");

    // TOTP config is deleted when disabled
    let totp_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM user_totp_config WHERE user_id = $1::uuid)",
    )
    .bind(user_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to check TOTP status after disable");

    assert!(!totp_exists, "TOTP config should be deleted when disabled");

    println!("[Step 3] ✓ TOTP disabled");
    println!("\n✅ Scenario 1 完成：用户成功禁用TOTP");
}
