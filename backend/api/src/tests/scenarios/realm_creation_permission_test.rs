// =============================================================================
// Realm Creation Permission Scenario Tests
// =============================================================================
//
// Test realm creation with permission-based access control:
// - Users with "realm.manage" permission can create realms
// - Users without "realm.manage" permission receive 403 Forbidden
//
// =============================================================================

use crate::tests::helpers::auth_helpers::{create_admin_session, grant_realm_admin_role};
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use herald_core::domain::authorization::permission_service::PermissionService;
use serde_json::json;
use test_context::test_context;
use tower::ServiceExt;

/// ============================================================================
/// Scenario Tests: Realm Creation with Permission Check
/// ============================================================================

#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_realm_creation_with_permission(ctx: &mut TestContext) {
    // Given: User has realm.manage permission (via realm-admin role in admin realm)
    let email = format!("admin-with-perm-{}", ctx._realm_id);

    // Create user and grant realm-admin role
    let (admin_token, user_id) = create_admin_session_with_user(ctx, &email, 1800).await;
    grant_realm_admin_role(ctx, &user_id).await;

    // IMPORTANT: Add realm.manage permission for admin realm
    // This is the permission that controls who can create new realms
    if ctx._realm_id == "admin" {
        // Get the realm-admin role UUID
        let role_uuid: uuid::Uuid =
            sqlx::query_scalar("SELECT id FROM roles WHERE name = 'realm-admin' AND realm_id = $1")
                .bind(&ctx._realm_id)
                .fetch_one(&ctx._app_state.pool)
                .await
                .unwrap();

        // Add realm.manage permission to the role
        let policy_id = uuid::Uuid::now_v7();
        sqlx::query(
            "INSERT INTO role_policies (id, role_id, realm_id, resource, action)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (role_id, resource, action) DO NOTHING",
        )
        .bind(policy_id)
        .bind(role_uuid)
        .bind(&ctx._realm_id)
        .bind("realm")
        .bind("manage")
        .execute(&ctx._app_state.pool)
        .await
        .expect("Failed to add realm.manage permission");

        // Invalidate permission cache
        let _ = ctx
            ._app_state
            .permission_checker
            .invalidate_realm_cache(&ctx._realm_id)
            .await;

        tracing::info!("Added realm.manage permission to realm-admin role");
    }

    let test_realm_id = format!("test-realm-with-perm-{}", ctx._realm_id);

    // When: Create realm
    let request = Request::builder()
        .method("POST")
        .uri("/api/realms")
        .header("authorization", format!("Bearer {}", admin_token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "id": test_realm_id,
                "name": "Test Realm With Permission",
                "adminUser": {
                    "email": format!("admin@{}", test_realm_id),
                    "password": "Password123!"
                }
            })
            .to_string(),
        ))
        .unwrap();

    // Then: Realm creation succeeds (201 Created)
    let app = ctx.create_unified_test_router();
    let response = app.clone().oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::CREATED,
        "User with realm.manage permission should be able to create realm"
    );
}

#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_realm_creation_without_permission_returns_403(ctx: &mut TestContext) {
    // Given: User does NOT have realm.manage permission
    let email = format!("admin-no-perm-{}", ctx._realm_id);
    let admin_token = create_admin_session(ctx, &email, 1800).await;
    // Note: NOT granting realm-admin role, so no realm.manage permission

    let test_realm_id = format!("test-realm-no-perm-{}", ctx._realm_id);

    // When: Attempt to create realm
    let request = Request::builder()
        .method("POST")
        .uri("/api/realms")
        .header("authorization", format!("Bearer {}", admin_token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "id": test_realm_id,
                "name": "Test Realm No Permission",
                "adminUser": {
                    "email": format!("admin@{}", test_realm_id),
                    "password": "Password123!"
                }
            })
            .to_string(),
        ))
        .unwrap();

    // Then: Returns 403 Forbidden
    let app = ctx.create_unified_test_router();
    let response = app.clone().oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "User without realm.manage permission should receive 403"
    );
}

// ============================================================================
// Helper Functions
// ============================================================================

use crate::tests::helpers::auth_helpers::create_admin_session_with_user;
use sqlx;
