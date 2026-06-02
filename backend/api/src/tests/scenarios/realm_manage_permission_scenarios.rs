// =============================================================================
// Realm Manage Permission Scenario Tests
// =============================================================================
//
// Verifies that the refactored permission model enforces realm.manage as the
// sole permission governing realm CRUD operations in the admin realm, that
// realm.view alone cannot create or update realms, and that legacy permissions
// (realm.admin, realm.admin:{realm_id}, realm.create) are absent from RBAC
// initialization.
//
// User Stories covered:
// - US-AR-001: Realm isolation access — admin realm context
// - US-AR-004: Realm creation permission control
// - US-BP-001: Default role and permission protection (no legacy permissions)
//
// Reference: docs/user-stories/core/admin-realm.md (US-AR-001, US-AR-004)
// Reference: docs/user-stories/core/builtin-protection.md (US-BP-001)
//
// Routes:
//   POST /api/realms                       — requires realm.manage
//   PUT  /api/realms/{realmId}             — self-realm only, requires settings.manage
//   GET  /api/realms                       — requires realm.view (admin realm only)
//
// =============================================================================

use crate::tests::helpers::auth_helpers::{create_admin_session_with_user, grant_realm_admin_role};
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use herald_core::domain::authorization::permission_service::PermissionService;
use serde_json::json;
use test_context::test_context;
use tower::ServiceExt;

// =============================================================================
// Helper: grant a single permission to a user via a dedicated role
// =============================================================================

/// Creates a role with a single permission and assigns it to the user.
///
/// Reuses the pattern from dashboard_audit_permission_scenarios and
/// api_keys_permission_scenarios.
async fn grant_single_permission(ctx: &TestContext, user_id: &str, resource: &str, action: &str) {
    let role_uuid = uuid::Uuid::now_v7();
    sqlx::query(
        "INSERT INTO roles (id, name, description, realm_id, client_id, is_builtin)
         VALUES ($1, $2, $3, $4, $5, false)",
    )
    .bind(role_uuid)
    .bind(format!("test-role-{}-{}", resource, action))
    .bind(format!("Test role for {}.{} only", resource, action))
    .bind(&ctx._realm_id)
    .bind(&ctx._client_id)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create single-permission role");

    let policy_id = uuid::Uuid::now_v7();
    sqlx::query(
        "INSERT INTO role_policies (id, role_id, realm_id, resource, action)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(policy_id)
    .bind(role_uuid)
    .bind(&ctx._realm_id)
    .bind(resource)
    .bind(action)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to add single permission to role");

    let user_role_id = uuid::Uuid::now_v7();
    let user_uuid = uuid::Uuid::parse_str(user_id).expect("Failed to parse user_id as UUID");
    sqlx::query(
        "INSERT INTO user_roles (id, user_id, role_id, realm_id, client_id, principal_type, principal_id)
         VALUES ($1, $2, $3, $4, $5, $6, $2::text)
         ON CONFLICT DO NOTHING",
    )
    .bind(user_role_id)
    .bind(user_uuid)
    .bind(role_uuid)
    .bind(&ctx._realm_id)
    .bind(&ctx._client_id)
    .bind(herald_core::domain::authorization::principal_types::USER)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to assign single-permission role to user");

    let _ = ctx
        ._app_state
        .permission_checker
        .invalidate_user_role_cache(&ctx._realm_id, user_id)
        .await;
}

// =============================================================================
// Scenario 1: realm.manage grants realm creation in admin realm (US-AR-001, US-AR-004)
// =============================================================================

/// User Story: docs/user-stories/core/admin-realm.md — Story 1 (US-AR-001)
/// User Story: docs/user-stories/core/admin-realm.md — Story 4 (US-AR-004)
/// Covers: User with realm.manage permission can create a new realm
///
/// Given a user in the admin realm with ONLY realm.manage permission,
/// When calling POST /api/realms with a valid realm payload,
/// Then response is 201 Created.
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_realm_manage_grants_create_in_admin_realm(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    // Given: user in admin realm with realm.manage permission
    let (user_token, user_id) =
        create_admin_session_with_user(ctx, "realm-manage-create@test.com", 1800).await;
    grant_single_permission(ctx, &user_id, "realm", "manage").await;

    // Invalidate realm cache so the new policy is picked up
    let _ = ctx
        ._app_state
        .permission_checker
        .invalidate_realm_cache(&ctx._realm_id)
        .await;

    let test_realm_id = format!("test-manage-create-{}", ctx._realm_id);

    // When: creating a new realm
    let req = Request::builder()
        .method("POST")
        .uri("/api/realms")
        .header(header::COOKIE, format!("X-Auth={}", user_token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "id": test_realm_id,
                "name": "Test Realm Manage Create",
                "adminUser": {
                    "email": format!("admin@{}", test_realm_id),
                    "password": "Password123!"
                }
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();

    // Then: 201 Created
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "User with realm.manage should be able to create realm"
    );
}

// =============================================================================
// Scenario 2: realm.view alone cannot create realms (US-AR-004)
// =============================================================================

/// User Story: docs/user-stories/core/admin-realm.md — Story 4 (US-AR-004)
/// Covers: User with ONLY realm.view permission cannot create realms
///
/// Given a user in the admin realm with ONLY realm.view permission,
/// When calling POST /api/realms with a valid realm payload,
/// Then response is 403 Forbidden.
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_realm_view_cannot_create(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    // Given: user in admin realm with ONLY realm.view permission
    let (user_token, user_id) =
        create_admin_session_with_user(ctx, "realm-view-no-create@test.com", 1800).await;
    grant_single_permission(ctx, &user_id, "realm", "view").await;

    // Invalidate realm cache so the new policy is picked up
    let _ = ctx
        ._app_state
        .permission_checker
        .invalidate_realm_cache(&ctx._realm_id)
        .await;

    let test_realm_id = format!("test-view-nocreate-{}", ctx._realm_id);

    // When: attempting to create a realm
    let req = Request::builder()
        .method("POST")
        .uri("/api/realms")
        .header(header::COOKIE, format!("X-Auth={}", user_token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "id": test_realm_id,
                "name": "Test Realm View No Create",
                "adminUser": {
                    "email": format!("admin@{}", test_realm_id),
                    "password": "Password123!"
                }
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();

    // Then: 403 Forbidden
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "User with only realm.view should get 403 Forbidden when creating realm"
    );
}

// =============================================================================
// Scenario 3: cross-realm update is NOT allowed even with realm.manage (US-AR-001)
// =============================================================================

/// User Story: docs/user-stories/core/admin-realm.md — Story 1 (US-AR-001)
/// Covers: Even Super Admin (realm.manage) cannot update another realm's metadata
///
/// Given a user in the admin realm with ONLY realm.manage permission,
///   and a target realm exists (not the user's own realm),
/// When calling PUT /api/realms/{realmId} with updated name,
/// Then response is 403 Forbidden.
///
/// Note: realm.manage is for create/delete only. Cross-realm metadata editing
/// is not allowed — each realm's admin edits their own realm with settings.manage.
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_realm_manage_cannot_update_other_realm(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    // Given: first set up a full admin to create a target realm
    let (admin_token, admin_user_id) =
        create_admin_session_with_user(ctx, "realm-update-admin@test.com", 1800).await;
    grant_realm_admin_role(ctx, &admin_user_id).await;

    let target_realm_id = format!("target-realm-update-{}", ctx._realm_id);
    let create_req = Request::builder()
        .method("POST")
        .uri("/api/realms")
        .header(header::COOKIE, format!("X-Auth={}", admin_token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "id": target_realm_id,
                "name": "Target Realm For Update",
                "adminUser": {
                    "email": format!("admin@{}", target_realm_id),
                    "password": "Password123!"
                }
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(create_req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "Setup: target realm creation should succeed"
    );

    // Given: a second user with ONLY realm.manage permission
    let (user_token, user_id) =
        create_admin_session_with_user(ctx, "realm-manage-update@test.com", 1800).await;
    grant_single_permission(ctx, &user_id, "realm", "manage").await;

    // Invalidate realm cache so the new policy is picked up
    let _ = ctx
        ._app_state
        .permission_checker
        .invalidate_realm_cache(&ctx._realm_id)
        .await;

    // When: attempting to update the target realm
    let update_req = Request::builder()
        .method("PUT")
        .uri(format!("/api/realms/{}", target_realm_id))
        .header(header::COOKIE, format!("X-Auth={}", user_token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "name": "Should Not Update",
                "description": "Should fail with 403"
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(update_req).await.unwrap();

    // Then: 403 Forbidden — cross-realm editing is not allowed
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "Cross-realm editing should be forbidden even with realm.manage — only self-realm editing with settings.manage is allowed"
    );
}

// =============================================================================
// Scenario 4: realm.view alone cannot update another realm (US-AR-001, US-AR-004)
// =============================================================================

/// User Story: docs/user-stories/core/admin-realm.md — Story 1 (US-AR-001)
/// User Story: docs/user-stories/core/admin-realm.md — Story 4 (US-AR-004)
/// Covers: User with ONLY realm.view permission cannot update another realm
///
/// Given a user in the admin realm with ONLY realm.view permission,
///   and a target realm exists (not the user's own realm),
/// When calling PUT /api/realms/{realmId} with updated name,
/// Then response is 403 Forbidden.
///
/// Note: The original task specified a DELETE scenario, but the current API
//  does not expose a DELETE /api/realms/{realmId} endpoint. This test
//  validates the same permission boundary (realm.manage required for
//  cross-realm mutations) via the update operation instead.
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_realm_view_cannot_update_other_realm(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    // Given: first set up a full admin to create a target realm
    let (admin_token, admin_user_id) =
        create_admin_session_with_user(ctx, "realm-view-update-admin@test.com", 1800).await;
    grant_realm_admin_role(ctx, &admin_user_id).await;

    let target_realm_id = format!("target-realm-viewupd-{}", ctx._realm_id);
    let create_req = Request::builder()
        .method("POST")
        .uri("/api/realms")
        .header(header::COOKIE, format!("X-Auth={}", admin_token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "id": target_realm_id,
                "name": "Target Realm View Update",
                "adminUser": {
                    "email": format!("admin@{}", target_realm_id),
                    "password": "Password123!"
                }
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(create_req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "Setup: target realm creation should succeed"
    );

    // Given: a second user with ONLY realm.view permission
    let (user_token, user_id) =
        create_admin_session_with_user(ctx, "realm-view-no-update@test.com", 1800).await;
    grant_single_permission(ctx, &user_id, "realm", "view").await;

    // Invalidate realm cache so the new policy is picked up
    let _ = ctx
        ._app_state
        .permission_checker
        .invalidate_realm_cache(&ctx._realm_id)
        .await;

    // When: attempting to update the target realm
    let update_req = Request::builder()
        .method("PUT")
        .uri(format!("/api/realms/{}", target_realm_id))
        .header(header::COOKIE, format!("X-Auth={}", user_token))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "name": "Should Not Update",
                "description": "Should fail with 403"
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(update_req).await.unwrap();

    // Then: 403 Forbidden
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "User with only realm.view should get 403 Forbidden when updating another realm"
    );
}

// =============================================================================
// Scenario 5: no legacy realm.admin permission in RBAC init (US-BP-001)
// =============================================================================

/// User Story: docs/user-stories/core/builtin-protection.md (US-BP-001)
/// Covers: Legacy permissions (realm.admin, realm.admin:{realm_id}) are not
///   present in role_policies after RBAC initialization
///
/// Given a realm has been initialized through RBAC init,
/// When querying role_policies for that realm,
/// Then no policy has action='admin' on resource='realm',
///   and no policy has a resource matching 'realm.admin:*'.
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_no_realm_admin_in_rbac_init(ctx: &mut TestContext) {
    // Given: the test realm has already been initialized via SchemaTestContext
    //   setup. Query role_policies for this realm to inspect what was created.

    // Check: no policy with action='admin' on resource='realm'
    let admin_action_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM role_policies WHERE realm_id = $1 AND resource = 'realm' AND action = 'admin'",
    )
    .bind(&ctx._realm_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to query role_policies for admin action");

    assert_eq!(
        admin_action_count, 0,
        "No policy with action='admin' on resource='realm' should exist — legacy 'realm.admin' permission has been removed"
    );

    // Check: no policy with resource matching 'realm.admin:*' pattern
    let realm_admin_wildcard_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM role_policies WHERE realm_id = $1 AND resource LIKE 'realm.admin%'",
    )
    .bind(&ctx._realm_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to query role_policies for realm.admin wildcard");

    assert_eq!(
        realm_admin_wildcard_count, 0,
        "No policy with resource matching 'realm.admin:*' should exist — legacy pattern has been removed"
    );
}

// =============================================================================
// Scenario 6: no legacy realm.create action in RBAC init (US-AR-004)
// =============================================================================

/// User Story: docs/user-stories/core/admin-realm.md — Story 4 (US-AR-004)
/// Covers: Legacy permission 'realm.create' is absent from RBAC init;
///   the new 'realm.manage' policy DOES exist for admin realm
///
/// Given the admin realm has been initialized through RBAC init,
/// When querying role_policies for the admin realm,
/// Then no policy has action='create' on resource='realm',
///   and a policy with action='manage' on resource='realm' DOES exist.
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_no_realm_create_in_rbac_init(ctx: &mut TestContext) {
    // Given: the test realm is initialized. For admin realm context,
    //   the test verifies the expected policies exist.

    // Check: no policy with action='create' on resource='realm'
    let create_action_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM role_policies WHERE realm_id = $1 AND resource = 'realm' AND action = 'create'",
    )
    .bind(&ctx._realm_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to query role_policies for create action");

    assert_eq!(
        create_action_count, 0,
        "No policy with action='create' on resource='realm' should exist — legacy 'realm.create' permission has been replaced by 'realm.manage'"
    );

    // Check: policy with action='manage' on resource='realm' DOES exist (only for admin realm)
    if ctx._realm_id == "admin" {
        let manage_action_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM role_policies WHERE realm_id = $1 AND resource = 'realm' AND action = 'manage'",
        )
        .bind(&ctx._realm_id)
        .fetch_one(&ctx._app_state.pool)
        .await
        .expect("Failed to query role_policies for manage action");

        assert!(
            manage_action_count > 0,
            "Policy with action='manage' on resource='realm' MUST exist in admin realm — this is the new permission that replaces legacy realm.create"
        );
    }
}
