// =============================================================================
// Permission System Basic Scenario Tests (GWT Format)
// =============================================================================
//
// Basic scenario tests for the self-developed permission system
// Tests focus on API-level functionality without deep cache testing
//
// =============================================================================

use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use serde_json::json;
use tower::ServiceExt;

// =============================================================================
// Scenario 1: Basic Permission Check
// =============================================================================

/// **User Story**: Basic permission checking with role-based access control
///
/// **Given**: User has realm-admin role
/// **And**: realm-admin role has users.manage permission
/// **When**: Check if user has users.manage permission
/// **Then**: Return true
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_basic_permission_check(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    // Given: User has realm-admin role
    let user_id = "test-user-basic-perm";
    let role_id = "realm-admin";

    // Create user role entry
    sqlx::query(
        r#"
        INSERT INTO user_roles (id, realm_id, user_id, role_id, client_id, created_at)
        VALUES ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP)
        "#,
    )
    .bind(uuid::Uuid::now_v7())
    .bind(&ctx._realm_id)
    .bind(user_id)
    .bind(role_id)
    .bind(&ctx._client_id)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create user role");

    // And: realm-admin role has users.manage permission
    sqlx::query(
        r#"
        INSERT INTO role_policies (id, realm_id, role_id, resource, action, created_at)
        VALUES ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP)
        "#,
    )
    .bind(uuid::Uuid::now_v7())
    .bind(&ctx._realm_id)
    .bind(role_id)
    .bind("users")
    .bind("manage")
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create role policy");

    // When: Check permission via API
    let req_body = json!({
        "user_id": user_id,
        "resource": "users",
        "action": "manage"
    })
    .to_string();

    let req = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/roles/{}/permission/check",
            ctx._realm_id
        ))
        .header("content-type", "application/json")
        .body(Body::from(req_body))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();

    // Then: Should return 200 OK with allowed: true
    assert_eq!(resp.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(resp.into_body())
        .await
        .unwrap();
    let result: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(result["allowed"], true);
}

// =============================================================================
// Scenario 2: Cross-Realm Isolation
// =============================================================================

/// **User Story**: Multi-tenant realm isolation
///
/// **Given**: User in realm-1 has admin role
/// **And**: realm-2 also exists
/// **When**: Check permission in realm-2
/// **Then**: Return false (permissions are realm-isolated)
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_cross_realm_isolation(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    // Given: User in realm-1 has admin role
    let user_id = "test-user-cross-realm";
    let role_id = "admin";

    sqlx::query(
        r#"
        INSERT INTO user_roles (id, realm_id, user_id, role_id, client_id, created_at)
        VALUES ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP)
        "#,
    )
    .bind(uuid::Uuid::now_v7())
    .bind(&ctx._realm_id) // realm-1
    .bind(user_id)
    .bind(role_id)
    .bind(&ctx._client_id)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create user role");

    sqlx::query(
        r#"
        INSERT INTO role_policies (id, realm_id, role_id, resource, action, created_at)
        VALUES ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP)
        "#,
    )
    .bind(uuid::Uuid::now_v7())
    .bind(&ctx._realm_id) // realm-1
    .bind(role_id)
    .bind("users")
    .bind("manage")
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create role policy");

    // When: Check permission in realm-1 (should succeed)
    let req_body = json!({
        "user_id": user_id,
        "resource": "users",
        "action": "manage"
    })
    .to_string();

    let req = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/roles/{}/permission/check",
            ctx._realm_id // realm-1
        ))
        .header("content-type", "application/json")
        .body(Body::from(req_body.clone()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(resp.into_body())
        .await
        .unwrap();
    let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(result["allowed"], true);

    // When: Check permission in realm-2 (different realm)
    let realm2_id = "realm-2-different";

    let req = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/roles/{}/permission/check",
            realm2_id // realm-2
        ))
        .header("content-type", "application/json")
        .body(Body::from(req_body))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();

    // Then: Should return false (realm isolation)
    assert_eq!(resp.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(resp.into_body())
        .await
        .unwrap();
    let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(result["allowed"], false);
}

// =============================================================================
// Scenario 3: Super Admin "All" Policy
// =============================================================================

/// **User Story**: Super Admin with unrestricted access
///
/// **Given**: User has super-admin role
/// **And**: super-admin has "All" resource with "allow" action
/// **When**: Check any permission
/// **Then**: Return true (Super Admin can access everything)
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_super_admin_all_policy(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    // Given: User has super-admin role
    let user_id = "test-user-super-admin";
    let role_id = "super-admin";

    sqlx::query(
        r#"
        INSERT INTO user_roles (id, realm_id, user_id, role_id, client_id, created_at)
        VALUES ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP)
        "#,
    )
    .bind(uuid::Uuid::now_v7())
    .bind(&ctx._realm_id)
    .bind(user_id)
    .bind(role_id)
    .bind(&ctx._client_id)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create user role");

    // And: super-admin has "All" resource with "allow" action
    sqlx::query(
        r#"
        INSERT INTO role_policies (id, realm_id, role_id, resource, action, created_at)
        VALUES ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP)
        "#,
    )
    .bind(uuid::Uuid::now_v7())
    .bind(&ctx._realm_id)
    .bind(role_id)
    .bind("All")
    .bind("allow")
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create role policy");

    // When: Check various permissions
    let test_cases = vec![
        ("users", "view"),
        ("users", "manage"),
        ("clients", "delete"),
        ("any_resource", "any_action"),
    ];

    for (resource, action) in test_cases {
        let req_body = json!({
            "user_id": user_id,
            "resource": resource,
            "action": action
        })
        .to_string();

        let req = Request::builder()
            .method("POST")
            .uri(format!(
                "/api/roles/{}/permission/check",
                ctx._realm_id
            ))
            .header("content-type", "application/json")
            .body(Body::from(req_body))
            .unwrap();

        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(resp.into_body())
            .await
            .unwrap();
        let result: serde_json::Value = serde_json::from_slice(&body).unwrap();

        // Then: All permissions should be granted
        assert_eq!(
            result["allowed"], true,
            "Super Admin should have access to {}:{}",
            resource, action
        );
    }
}

// =============================================================================
// Scenario 4: Multi-Role Permission Merge
// =============================================================================

/// **User Story**: User with multiple roles
///
/// **Given**: User has both role-a and role-b
/// **And**: role-a has users.view permission
/// **And**: role-b has users.manage permission
/// **When**: Check users.manage permission
/// **Then**: Return true (any role with permission grants access)
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_multi_role_permission_merge(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    // Given: User has both role-a and role-b
    let user_id = "test-user-multi-role";
    let role_a = "role-a";
    let role_b = "role-b";

    // Create user roles
    for role_id in &[role_a, role_b] {
        sqlx::query(
            r#"
            INSERT INTO user_roles (id, realm_id, user_id, role_id, client_id, created_at)
            VALUES ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP)
            "#,
        )
        .bind(uuid::Uuid::now_v7())
        .bind(&ctx._realm_id)
        .bind(user_id)
        .bind(role_id)
        .bind(&ctx._client_id)
        .execute(&ctx._app_state.pool)
        .await
        .expect("Failed to create user role");
    }

    // And: role-a has users.view permission
    sqlx::query(
        r#"
        INSERT INTO role_policies (id, realm_id, role_id, resource, action, created_at)
        VALUES ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP)
        "#,
    )
    .bind(uuid::Uuid::now_v7())
    .bind(&ctx._realm_id)
    .bind(role_a)
    .bind("users")
    .bind("view")
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create role policy");

    // And: role-b has users.manage permission
    sqlx::query(
        r#"
        INSERT INTO role_policies (id, realm_id, role_id, resource, action, created_at)
        VALUES ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP)
        "#,
    )
    .bind(uuid::Uuid::now_v7())
    .bind(&ctx._realm_id)
    .bind(role_b)
    .bind("users")
    .bind("manage")
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create role policy");

    // When: Check users.view permission
    let req_body = json!({
        "user_id": user_id,
        "resource": "users",
        "action": "view"
    })
    .to_string();

    let req = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/roles/{}/permission/check",
            ctx._realm_id
        ))
        .header("content-type", "application/json")
        .body(Body::from(req_body.clone()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(resp.into_body())
        .await
        .unwrap();
    let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(result["allowed"], true);

    // When: Check users.manage permission
    let req_body = json!({
        "user_id": user_id,
        "resource": "users",
        "action": "manage"
    })
    .to_string();

    let req = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/roles/{}/permission/check",
            ctx._realm_id
        ))
        .header("content-type", "application/json")
        .body(Body::from(req_body))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(resp.into_body())
        .await
        .unwrap();
    let result: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Then: Both permissions should be granted
    assert_eq!(result["allowed"], true);
}

// =============================================================================
// Scenario 5: Default Deny (No Roles)
// =============================================================================

/// **User Story**: Default deny for users without roles
///
/// **Given**: User has no roles
/// **When**: Check any permission
/// **Then**: Return false (default deny)
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_default_deny_no_roles(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    // Given: User has no roles
    let user_id = "test-user-no-roles";

    // When: Check any permission
    let req_body = json!({
        "user_id": user_id,
        "resource": "users",
        "action": "view"
    })
    .to_string();

    let req = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/roles/{}/permission/check",
            ctx._realm_id
        ))
        .header("content-type", "application/json")
        .body(Body::from(req_body))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(resp.into_body())
        .await
        .unwrap();
    let result: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Then: Should return false (default deny)
    assert_eq!(result["allowed"], false);
}

// =============================================================================
// Scenario 6: Default Deny (Role Without Policies)
// =============================================================================

/// **User Story**: Default deny for roles without policies
///
/// **Given**: User has role-a
/// **And**: role-a has no permission policies
/// **When**: Check any permission
/// **Then**: Return false (default deny)
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_default_deny_no_policies(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    // Given: User has role-a
    let user_id = "test-user-no-policies";
    let role_id = "role-a";

    sqlx::query(
        r#"
        INSERT INTO user_roles (id, realm_id, user_id, role_id, client_id, created_at)
        VALUES ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP)
        "#,
    )
    .bind(uuid::Uuid::now_v7())
    .bind(&ctx._realm_id)
    .bind(user_id)
    .bind(role_id)
    .bind(&ctx._client_id)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create user role");

    // And: role-a has NO permission policies (intentionally not creating any)

    // When: Check any permission
    let req_body = json!({
        "user_id": user_id,
        "resource": "users",
        "action": "manage"
    })
    .to_string();

    let req = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/roles/{}/permission/check",
            ctx._realm_id
        ))
        .header("content-type", "application/json")
        .body(Body::from(req_body))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(resp.into_body())
        .await
        .unwrap();
    let result: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Then: Should return false (default deny)
    assert_eq!(result["allowed"], false);
}
