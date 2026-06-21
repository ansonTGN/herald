// =============================================================================
// Billing & Points Permission Scenario Tests
// =============================================================================
//
// Verifies that billing.view/manage are enforced as distinct permissions for
// billing endpoints.
//
// User Stories covered:
// - US-RA-009: Permission hierarchy -- billing.view/manage enforcement
//
// Reference: docs/user-stories/core/realm-admin.md (US-RA-009)
//
// Routes:
//   GET  /api/bill/{realmId}/subscriptions/history     -> requires billing.view
//   GET  /api/third/pay/{realmId}/providers            -> any authenticated realm member; returns all configured providers
//
// =============================================================================

use crate::tests::helpers::auth_helpers::create_admin_session_with_user;
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use herald_core::domain::authorization::permission_service::PermissionService;
use test_context::test_context;
use tower::ServiceExt;

// =============================================================================
// Helper: grant a single permission to a user via a dedicated role
// =============================================================================

/// Creates a role with a single permission and assigns it to the user.
///
/// This avoids granting the full realm-admin role when we need to test
/// access with exactly one specific permission.
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
// Scenario 1: billing.view grants subscription history access (US-RA-009)
// =============================================================================

// User Story: docs/user-stories/core/realm-admin.md - Story 9 (US-RA-009)
// Covers: User with billing.view can access subscription history endpoint
//
// Given a user with ONLY billing.view permission,
// When calling GET /api/bill/{realmId}/subscriptions/history,
// Then response is 200 OK.
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_billing_view_grants_history_access(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    // Given: user with billing.view permission
    let (user_token, user_id) =
        create_admin_session_with_user(ctx, "billing-view-history@test.com", 1800).await;
    grant_single_permission(ctx, &user_id, "billing", "view").await;

    // When: calling subscription history endpoint
    let req = Request::builder()
        .method("GET")
        .uri(format!(
            "/api/bill/{}/subscriptions/history?page=1&pageSize=20",
            ctx._realm_id
        ))
        .header(header::COOKIE, format!("X-Auth={}", user_token))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();

    // Then: 200 OK
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "User with billing.view should get 200 OK for subscription history"
    );
}

// =============================================================================
// Scenario 2: payment provider list is accessible to realm members (US-RA-009)
// =============================================================================

// User Story: docs/user-stories/core/realm-admin.md - Story 9 (US-RA-009)
// Covers: Authenticated realm member can list configured payment providers
//
// Given an authenticated realm member,
// When calling GET /api/third/pay/{realmId}/providers,
// Then response is 200 OK (returns all configured providers).
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_can_list_payment_providers(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    // Given: authenticated realm member
    let (user_token, user_id) =
        create_admin_session_with_user(ctx, "billing-view-providers@test.com", 1800).await;
    grant_single_permission(ctx, &user_id, "billing", "view").await;

    // When: calling payment providers list endpoint
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/third/pay/{}/providers", ctx._realm_id))
        .header(header::COOKIE, format!("X-Auth={}", user_token))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();

    // Then: 200 OK
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Authenticated realm member should get 200 OK for payment provider list"
    );
}
