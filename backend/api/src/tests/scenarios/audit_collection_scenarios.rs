// =============================================================================
// Audit Event Collection Scenario Tests
// =============================================================================
//
// Tests that verify audit events are actually recorded in the database when
// core business operations are performed. These are end-to-end collection
// tests -- perform the operation, then check the audit_events table.
//
// User Stories covered:
// - Core operations (user CRUD, realm create, login/logout) produce audit events
//
// Reference: BE-T04 -- Integration tests verifying audit events are recorded
// Design: .ai/design/audit.md
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
use uuid::Uuid;

// =============================================================================
// Helper: query audit events from the database
// =============================================================================

/// Count audit events for a realm with a given action.
async fn count_audit_events_by_action(pool: &sqlx::PgPool, realm_id: &str, action: &str) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM audit_events WHERE realm_id = $1 AND action = $2")
        .bind(realm_id)
        .bind(action)
        .fetch_one(pool)
        .await
        .expect("Failed to count audit_events")
}

/// Count all audit events for a realm.
async fn count_all_audit_events(pool: &sqlx::PgPool, realm_id: &str) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM audit_events WHERE realm_id = $1")
        .bind(realm_id)
        .fetch_one(pool)
        .await
        .expect("Failed to count audit_events")
}

/// Fetch audit events for a realm, returning raw column values for flexible assertions.
async fn fetch_audit_events_raw(
    pool: &sqlx::PgPool,
    realm_id: &str,
    action: &str,
) -> Vec<(String, String, String, String, String)> {
    sqlx::query_as::<_, (String, String, String, String, String)>(
        "SELECT category, action, actor_id, target_id, result
         FROM audit_events
         WHERE realm_id = $1 AND action = $2
         ORDER BY created_at DESC",
    )
    .bind(realm_id)
    .bind(action)
    .fetch_all(pool)
    .await
    .expect("Failed to query audit_events")
}

// =============================================================================
// Scenario 1: User create produces audit event
// =============================================================================

/// User Story: docs/user-stories/14-audit-user-stories.md
/// Covers: User create via admin API records user_create audit event
///
/// Given an admin with users.manage permission,
/// When creating a user via POST /api/users/{realmId},
/// Then an audit event with action=user_create is recorded in the database
/// and target_id matches the newly created user's ID.
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_user_create_produces_audit_event(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();
    let realm_id = ctx._realm_id.clone();

    // Setup: create admin session with realm-admin role
    let (admin_token, admin_user_id) =
        create_admin_session_with_user(ctx, "audit-user-create@test.com", 1800).await;
    grant_realm_admin_role(ctx, &admin_user_id).await;

    // Record baseline
    let baseline = count_all_audit_events(&ctx.app_state.pool, &realm_id).await;

    // Act: create a user via admin API
    // Note: role_ids requires at least 1 element due to #[validate(length(min = 1))]
    // Get the 'user' role ID which exists after RBAC initialization
    let user_role_id: Uuid =
        sqlx::query_scalar("SELECT id FROM roles WHERE realm_id = $1 AND name = 'user' LIMIT 1")
            .bind(&realm_id)
            .fetch_one(&ctx.app_state.pool)
            .await
            .expect("Should have 'user' role after RBAC init");

    let new_email = format!("created-user-{}@audit-test.com", Uuid::now_v7().as_simple());
    let create_payload = json!({
        "email": new_email,
        "password": "Password123!",
        "status": 1,
        "roleIds": [user_role_id]
    });

    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/users/{}", realm_id))
        .header("content-type", "application/json")
        .header(header::COOKIE, format!("X-Auth={}", admin_token))
        .body(Body::from(create_payload.to_string()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "User creation should return 201 Created"
    );

    // Extract the created user ID from the response
    let body: serde_json::Value = crate::tests::response_json(resp).await;
    let created_user_id = body["id"]
        .as_str()
        .expect("Response should contain user ID");

    // Assert: audit event was recorded
    let events = fetch_audit_events_raw(&ctx.app_state.pool, &realm_id, "user_create").await;

    assert!(
        !events.is_empty(),
        "Expected at least one user_create audit event, found {}",
        events.len()
    );

    let event = &events[0];
    assert_eq!(
        event.0, "user_management",
        "Category should be user_management"
    );
    assert_eq!(event.1, "user_create", "Action should be user_create");
    assert_eq!(event.4, "success", "Result should be success");

    // The target_id should match the newly created user
    assert_eq!(
        event.3, created_user_id,
        "target_id should match the created user's ID"
    );

    // Verify that new events were created (at least one more than baseline)
    let after = count_all_audit_events(&ctx.app_state.pool, &realm_id).await;
    assert!(
        after > baseline,
        "Audit event count should have increased: before={}, after={}",
        baseline,
        after
    );
}

// =============================================================================
// Scenario 2: User update produces audit event
// =============================================================================

/// User Story: docs/user-stories/14-audit-user-stories.md
/// Covers: User update via admin API records user_update audit event
///
/// Given an admin with users.manage permission and an existing user,
/// When updating the user via PUT /api/users/{realmId}/{userId},
/// Then an audit event with action=user_update is recorded.
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_user_update_produces_audit_event(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();
    let realm_id = ctx._realm_id.clone();

    // Setup: create admin session
    let (admin_token, admin_user_id) =
        create_admin_session_with_user(ctx, "audit-user-update@test.com", 1800).await;
    grant_realm_admin_role(ctx, &admin_user_id).await;

    // Create a target user to update
    let target_email = format!(
        "update-target-{}@audit-test.com",
        Uuid::now_v7().as_simple()
    );
    let target_user_id = crate::tests::helpers::test_setup_helpers::create_test_user(
        ctx,
        &target_email,
        "Password123!",
    )
    .await;

    // Act: update the user's nickname
    let update_payload = json!({
        "nickname": "Updated Nickname"
    });

    let req = Request::builder()
        .method("PUT")
        .uri(format!("/api/users/{}/{}", realm_id, target_user_id))
        .header("content-type", "application/json")
        .header(header::COOKIE, format!("X-Auth={}", admin_token))
        .body(Body::from(update_payload.to_string()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "User update should succeed");

    // Assert: audit event was recorded
    let events = fetch_audit_events_raw(&ctx.app_state.pool, &realm_id, "user_update").await;

    assert!(
        !events.is_empty(),
        "Expected at least one user_update audit event, found {}",
        events.len()
    );

    let event = &events[0];
    assert_eq!(
        event.0, "user_management",
        "Category should be user_management"
    );
    assert_eq!(event.1, "user_update", "Action should be user_update");
    assert_eq!(event.4, "success", "Result should be success");
    assert_eq!(
        event.3,
        target_user_id.to_string(),
        "target_id should match the updated user's ID"
    );
}

// =============================================================================
// Scenario 3: User delete produces audit event
// =============================================================================

/// User Story: docs/user-stories/14-audit-user-stories.md
/// Covers: User delete via admin API records user_delete audit event
///
/// Given an admin with users.manage permission and an existing user,
/// When deleting the user via DELETE /api/users/{realmId}/{userId},
/// Then an audit event with action=user_delete is recorded.
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_user_delete_produces_audit_event(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();
    let realm_id = ctx._realm_id.clone();

    // Setup: create admin session
    let (admin_token, admin_user_id) =
        create_admin_session_with_user(ctx, "audit-user-delete@test.com", 1800).await;
    grant_realm_admin_role(ctx, &admin_user_id).await;

    // Create a target user to delete
    let target_email = format!(
        "delete-target-{}@audit-test.com",
        Uuid::now_v7().as_simple()
    );
    let target_user_id = crate::tests::helpers::test_setup_helpers::create_test_user(
        ctx,
        &target_email,
        "Password123!",
    )
    .await;

    // Act: delete the user
    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/users/{}/{}", realm_id, target_user_id))
        .header(header::COOKIE, format!("X-Auth={}", admin_token))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "User deletion should return 204 No Content"
    );

    // Assert: audit event was recorded
    let events = fetch_audit_events_raw(&ctx.app_state.pool, &realm_id, "user_delete").await;

    assert!(
        !events.is_empty(),
        "Expected at least one user_delete audit event, found {}",
        events.len()
    );

    let event = &events[0];
    assert_eq!(
        event.0, "user_management",
        "Category should be user_management"
    );
    assert_eq!(event.1, "user_delete", "Action should be user_delete");
    assert_eq!(event.4, "success", "Result should be success");
    assert_eq!(
        event.3,
        target_user_id.to_string(),
        "target_id should match the deleted user's ID"
    );
}

// =============================================================================
// Scenario 4: Realm create produces audit events
// =============================================================================

/// User Story: docs/user-stories/14-audit-user-stories.md
/// Covers: Realm creation records realm_create and realm_rbac_init audit events
///
/// Given a super admin with realm.create permission,
/// When creating a new realm via POST /api/realms,
/// Then audit events for realm_create and realm_rbac_init are recorded.
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_realm_create_produces_audit_events(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    // Setup: create admin session with realm-admin role (admin realm includes realm.create)
    let (admin_token, admin_user_id) =
        create_admin_session_with_user(ctx, "audit-realm-create@test.com", 1800).await;
    grant_realm_admin_role(ctx, &admin_user_id).await;

    // Generate a unique realm ID
    let timestamp = chrono::Utc::now().timestamp_millis();
    let new_realm_id = format!("auditr{}", timestamp % 1000000000);

    // Act: create a new realm
    let create_payload = json!({
        "id": new_realm_id,
        "name": "Audit Test Realm",
        "adminUser": {
            "email": format!("admin-{}@audit-realm.com", timestamp),
            "password": "Password123!"
        }
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/realms")
        .header("content-type", "application/json")
        .header(header::COOKIE, format!("X-Auth={}", admin_token))
        .body(Body::from(create_payload.to_string()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "Realm creation should return 201 Created"
    );

    // Assert: realm_create event recorded in the new realm
    let create_count =
        count_audit_events_by_action(&ctx.app_state.pool, &new_realm_id, "realm_create").await;
    assert!(
        create_count >= 1,
        "Expected at least one realm_create audit event in realm {}, found {}",
        new_realm_id,
        create_count
    );

    // Assert: realm_rbac_init event recorded
    let rbac_init_count =
        count_audit_events_by_action(&ctx.app_state.pool, &new_realm_id, "realm_rbac_init").await;
    assert!(
        rbac_init_count >= 1,
        "Expected at least one realm_rbac_init audit event in realm {}, found {}",
        new_realm_id,
        rbac_init_count
    );
}

// =============================================================================
// Scenario 5: Login success produces audit event
// =============================================================================

/// User Story: docs/user-stories/14-audit-user-stories.md
/// Covers: Successful login records auth_login audit event
///
/// Given an active user with valid credentials,
/// When logging in via POST /api/auth/{realmId}/login,
/// Then an audit event with action=auth_login and result=success is recorded.
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_login_success_produces_audit_event(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();

    // Setup: create an active user
    let email = format!("audit-login-ok-{}@test.com", Uuid::now_v7().as_simple());
    let password = "password123";
    let user_id =
        crate::tests::helpers::test_setup_helpers::create_test_user(ctx, &email, password).await;

    // Record baseline count for auth_login events
    let baseline = count_audit_events_by_action(&ctx.app_state.pool, &realm_id, "auth_login").await;

    // Act: login
    let app = ctx.create_unified_test_router();
    let login_payload = json!({
        "clientId": ctx._client_id,
        "email": email,
        "password": password,
        "turnstileToken": "dummy"
    });

    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/auth/{}/login", realm_id))
        .header("content-type", "application/json")
        .header("x-forwarded-for", "3.3.3.3")
        .body(Body::from(login_payload.to_string()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "Login should succeed");

    // Assert: auth_login event was recorded
    let after = count_audit_events_by_action(&ctx.app_state.pool, &realm_id, "auth_login").await;
    assert!(
        after > baseline,
        "auth_login event count should have increased: before={}, after={}",
        baseline,
        after
    );

    // Verify event details
    let events = fetch_audit_events_raw(&ctx.app_state.pool, &realm_id, "auth_login").await;

    let event = &events[0];
    assert_eq!(event.0, "auth", "Category should be auth");
    assert_eq!(event.4, "success", "Result should be success");
    // The actor_id should be the user who logged in
    assert_eq!(
        event.2,
        user_id.to_string(),
        "actor_id should match the logged-in user's ID"
    );
}

// =============================================================================
// Scenario 6: Login failure produces audit event
// =============================================================================

/// User Story: docs/user-stories/14-audit-user-stories.md
/// Covers: Failed login records auth_login_failed audit event
///
/// Given a user with valid credentials,
/// When attempting login with a wrong password,
/// Then an audit event with action=auth_login_failed and result=failure is recorded.
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_login_failure_produces_audit_event(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();

    // Setup: create an active user
    let email = format!("audit-login-fail-{}@test.com", Uuid::now_v7().as_simple());
    let _ =
        crate::tests::helpers::test_setup_helpers::create_test_user(ctx, &email, "correctpassword")
            .await;

    // Record baseline count for auth_login_failed events
    let baseline =
        count_audit_events_by_action(&ctx.app_state.pool, &realm_id, "auth_login_failed").await;

    // Act: login with wrong password
    let app = ctx.create_unified_test_router();
    let login_payload = json!({
        "clientId": ctx._client_id,
        "email": email,
        "password": "wrongpassword",
        "turnstileToken": "dummy"
    });

    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/auth/{}/login", realm_id))
        .header("content-type", "application/json")
        .header("x-forwarded-for", "3.3.3.3")
        .body(Body::from(login_payload.to_string()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "Login should fail with 401"
    );

    // Assert: auth_login_failed event was recorded
    let after =
        count_audit_events_by_action(&ctx.app_state.pool, &realm_id, "auth_login_failed").await;
    assert!(
        after > baseline,
        "auth_login_failed event count should have increased: before={}, after={}",
        baseline,
        after
    );

    // Verify event details
    let events = fetch_audit_events_raw(&ctx.app_state.pool, &realm_id, "auth_login_failed").await;

    let event = &events[0];
    assert_eq!(event.0, "auth", "Category should be auth");
    assert_eq!(event.4, "failure", "Result should be failure");
}

// =============================================================================
// Scenario 7: Logout produces audit event
// =============================================================================

/// User Story: docs/user-stories/14-audit-user-stories.md
/// Covers: Logout records auth_logout audit event
///
/// Given a logged-in user with a valid session,
/// When calling the logout endpoint,
/// Then an audit event with action=auth_logout is recorded.
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_logout_produces_audit_event(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();

    // Setup: create user and login to get a session token
    let email = format!("audit-logout-{}@test.com", Uuid::now_v7().as_simple());
    let password = "password123";
    let _user_id =
        crate::tests::helpers::test_setup_helpers::create_test_user(ctx, &email, password).await;

    let app = ctx.create_unified_test_router();
    let login_payload = json!({
        "clientId": ctx._client_id,
        "email": email,
        "password": password,
        "turnstileToken": "dummy"
    });

    let login_req = Request::builder()
        .method("POST")
        .uri(format!("/api/auth/{}/login", realm_id))
        .header("content-type", "application/json")
        .header("x-forwarded-for", "3.3.3.3")
        .body(Body::from(login_payload.to_string()))
        .unwrap();

    let login_resp = app.clone().oneshot(login_req).await.unwrap();
    assert_eq!(login_resp.status(), StatusCode::OK, "Login should succeed");

    // Extract session token
    let set_cookie = login_resp
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|v| v.to_str().ok())
        .expect("Should return Set-Cookie header");
    let token =
        crate::tests::extract_set_cookie_token(set_cookie, "X-Auth").expect("Should extract token");

    // Record baseline count for auth_logout events
    let baseline =
        count_audit_events_by_action(&ctx.app_state.pool, &realm_id, "auth_logout").await;

    // Act: logout
    let logout_req = Request::builder()
        .method("GET")
        .uri(format!("/api/auth/{}/logout", realm_id))
        .header(header::COOKIE, format!("X-Auth={}", token))
        .body(Body::empty())
        .unwrap();

    let logout_resp = app.clone().oneshot(logout_req).await.unwrap();
    assert_eq!(
        logout_resp.status(),
        StatusCode::OK,
        "Logout should succeed"
    );

    // Assert: auth_logout event was recorded
    let after = count_audit_events_by_action(&ctx.app_state.pool, &realm_id, "auth_logout").await;
    assert!(
        after > baseline,
        "auth_logout event count should have increased: before={}, after={}",
        baseline,
        after
    );

    // Verify event details
    let events = fetch_audit_events_raw(&ctx.app_state.pool, &realm_id, "auth_logout").await;

    let event = &events[0];
    assert_eq!(event.0, "auth", "Category should be auth");
    assert_eq!(event.4, "success", "Result should be success");
}
