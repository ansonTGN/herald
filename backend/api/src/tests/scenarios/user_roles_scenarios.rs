// =============================================================================
// User Roles Scenario Tests (GWT Format)
// =============================================================================
//
// Tests for user role management API
// Based on design document Section 5.7.2
//
// =============================================================================

use crate::tests::helpers::*;
use crate::tests::response_json;
use crate::tests::schema_test_context::SchemaTestContext;
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use serde_json::json;
use test_context::test_context;
use tower::ServiceExt;

// ============================================================================
// Scenario 1: Assign Role to User
// ============================================================================

/// **Given**: 用户 user-1 没有任何角色
/// **When**: POST /api/admin/users/user-1/roles, body: { role_ids: ["role-a"] }
/// **Then**: HTTP 200 OK
/// **And**: GET /api/admin/users/user-1/roles 返回包含 role-a
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_assign_role_to_user(ctx: &mut SchemaTestContext) {
    let (token, user_id_str) = create_admin_session_with_user(ctx, "test-admin", 1800).await;
    grant_realm_admin_role(ctx, &user_id_str).await;

    // Given: Create user without roles
    let user_id = create_simple_test_user(ctx, "user-1@example.com")
        .await
        .to_string();

    // Given: Create role
    let role_id = create_role(ctx, &ctx._realm_id, &token, "role-a", "Role A").await;

    // When: Assign role to user
    let app = ctx.create_unified_test_router();
    let req_body = json!({
        "roleIds": [role_id]
    })
    .to_string();

    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/permission/users/{}/roles", user_id))
        .header("content-type", "application/json")
        .header(header::COOKIE, format!("X-Auth={}", token))
        .body(Body::from(req_body))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();

    // Then: HTTP 201 CREATED
    let status = resp.status();
    if status != StatusCode::CREATED {
        eprintln!("Error response status: {}", status);
        let error_json: serde_json::Value = response_json(resp).await;
        eprintln!("Error response body: {}", error_json);
    }
    assert_eq!(status, StatusCode::CREATED);

    // And: Verify role is assigned
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/permission/users/{}/roles", user_id))
        .header(header::COOKIE, format!("X-Auth={}", token))
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let resp_json: serde_json::Value = response_json(resp).await;
    assert_eq!(resp_json["roles"].as_array().unwrap().len(), 1);
    assert_eq!(resp_json["roles"][0]["id"], serde_json::json!(role_id));
}

// ============================================================================
// Scenario 2: Remove Role from User
// ============================================================================

/// **Given**: 用户 user-1 有 role-a 角色
/// **When**: DELETE /api/admin/users/user-1/roles/role-a
/// **Then**: HTTP 204 No Content
/// **And**: GET /api/admin/users/user-1/roles 不包含 role-a
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_remove_role_from_user(ctx: &mut SchemaTestContext) {
    let (token, user_id_str) = create_admin_session_with_user(ctx, "test-admin", 1800).await;
    grant_realm_admin_role(ctx, &user_id_str).await;

    // Given: Create user with role
    let user_id = create_simple_test_user(ctx, "user-2@example.com").await;
    let role_id = create_role(ctx, &ctx._realm_id, &token, "role-a", "Role A").await;
    assign_role_to_user(ctx, &ctx._realm_id, &token, user_id, role_id).await;

    // When: Remove role from user
    let app = ctx.create_unified_test_router();
    let req = Request::builder()
        .method("DELETE")
        .uri(format!(
            "/api/permission/users/{}/roles/{}",
            &user_id.to_string(),
            &role_id.to_string()
        ))
        .header(header::COOKIE, format!("X-Auth={}", token))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();

    // Then: HTTP 204 No Content
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // And: Verify role is removed
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/permission/users/{}/roles", user_id))
        .header(header::COOKIE, format!("X-Auth={}", token))
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    let resp_json: serde_json::Value = response_json(resp).await;

    assert_eq!(resp_json["roles"].as_array().unwrap().len(), 0);
}
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_assign_duplicate_role_ids_in_single_request(ctx: &mut SchemaTestContext) {
    let (token, user_id_str) = create_admin_session_with_user(ctx, "test-admin", 1800).await;
    grant_realm_admin_role(ctx, &user_id_str).await;

    let user_id = create_simple_test_user(ctx, "user-duplicate-role-request@example.com").await;
    let role_id = create_role(ctx, &ctx._realm_id, &token, "role-dup", "Role Dup").await;

    let app = ctx.create_unified_test_router();
    let req_body = json!({
        "roleIds": [role_id, role_id]
    })
    .to_string();

    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/permission/users/{}/roles", user_id))
        .header("content-type", "application/json")
        .header(header::COOKIE, format!("X-Auth={}", token))
        .body(Body::from(req_body))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/permission/users/{}/roles", user_id))
        .header(header::COOKIE, format!("X-Auth={}", token))
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    let resp_json: serde_json::Value = response_json(resp).await;

    let roles = resp_json["roles"].as_array().unwrap();
    assert!(roles.len() == 1, "Expected 1 role, got {}", roles.len());
    assert_eq!(resp_json["roles"][0]["id"], serde_json::json!(role_id));
}
// ============================================================================
// Scenario 5: Assign Multiple Roles
// ============================================================================

/// **Given**: 用户 user-1 没有任何角色
/// **When**: POST /api/admin/users/user-1/roles, body: { role_ids: ["role-a", "role-b", "role-c"] }
/// **Then**: HTTP 200 OK
/// **And**: 用户拥有所有三个角色
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_assign_multiple_roles(ctx: &mut SchemaTestContext) {
    let (token, user_id_str) = create_admin_session_with_user(ctx, "test-admin", 1800).await;
    grant_realm_admin_role(ctx, &user_id_str).await;

    // Given: User without roles
    let user_id = create_simple_test_user(ctx, "multi-roles-user@example.com").await;

    // Given: Create three roles
    let role_a = create_role(ctx, &ctx._realm_id, &token, "role-a", "Role A").await;
    let role_b = create_role(ctx, &ctx._realm_id, &token, "role-b", "Role B").await;
    let role_c = create_role(ctx, &ctx._realm_id, &token, "role-c", "Role C").await;

    // When: Assign all three roles
    let app = ctx.create_unified_test_router();
    let req_body = json!({
        "roleIds": [role_a.clone(), role_b.clone(), role_c.clone()]
    })
    .to_string();

    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/permission/users/{}/roles", user_id))
        .header("content-type", "application/json")
        .header(header::COOKIE, format!("X-Auth={}", token))
        .body(Body::from(req_body))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Then: Verify user has all three roles
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/permission/users/{}/roles", user_id))
        .header(header::COOKIE, format!("X-Auth={}", token))
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    let resp_json: serde_json::Value = response_json(resp).await;

    let roles = resp_json["roles"].as_array().unwrap();
    assert_eq!(roles.len(), 3);

    let role_ids: Vec<uuid::Uuid> = roles
        .iter()
        .filter_map(|r| r["id"].as_str().and_then(|s| uuid::Uuid::parse_str(s).ok()))
        .collect();

    assert!(role_ids.contains(&role_a));
    assert!(role_ids.contains(&role_b));
    assert!(role_ids.contains(&role_c));
}

// ============================================================================
// Scenario 6: Role Assignment Requires Permission
// ============================================================================

/// **Given**: 普通用户没有 `roles.manage` 权限
/// **When**: POST /api/permission/users/{userId}/roles
/// **Then**: HTTP 403 Forbidden
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_assign_role_requires_roles_manage(ctx: &mut SchemaTestContext) {
    let (admin_token, admin_user_id) =
        create_admin_session_with_user(ctx, "setup-admin@example.com", 1800).await;
    grant_realm_admin_role(ctx, &admin_user_id).await;

    let (user_token, _user_id) =
        create_admin_session_with_user(ctx, "plain-user@example.com", 1800).await;

    let target_user_id = create_simple_test_user(ctx, "assign-target@example.com").await;
    let role_id = create_role(ctx, &ctx._realm_id, &admin_token, "limited-role", "Limited").await;

    let app = ctx.create_unified_test_router();
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/permission/users/{}/roles", target_user_id))
        .header("content-type", "application/json")
        .header(header::COOKIE, format!("X-Auth={}", user_token))
        .body(Body::from(
            json!({
                "roleIds": [role_id]
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}
