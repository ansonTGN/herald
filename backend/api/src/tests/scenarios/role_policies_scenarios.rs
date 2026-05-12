// =============================================================================
// Role Policies Scenario Tests (GWT Format)
// =============================================================================
//
// Tests for role policy management API
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
use herald_core::domain::authorization::permission_service::PermissionService;
use serde_json::json;
use test_context::test_context;
use tower::ServiceExt;

// ============================================================================
// Scenario 1: Add Policy to Role
// ============================================================================

/// **Given**: 角色 role-a 没有任何策略
/// **When**: POST /api/admin/roles/role-a/policies, body: { resource: "users", action: "view" }
/// **Then**: HTTP 201 Created
/// **And**: GET /api/admin/roles/role-a/policies 返回包含新策略
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_add_policy_to_role(ctx: &mut SchemaTestContext) {
    let (token, user_id) = create_admin_session_with_user(ctx, "test-admin", 1800).await;
    grant_realm_admin_role(ctx, &user_id).await;

    // Given: Create role without policies
    let role_id = create_role(ctx, &ctx._realm_id, &token, "role-a", "Role A").await;

    // When: Add policy to role
    let app = ctx.create_unified_test_router();
    let req_body = json!({
        "resource": "users",
        "action": "view"
    })
    .to_string();

    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/permission/roles/{}/policies", role_id))
        .header("content-type", "application/json")
        .header(header::COOKIE, format!("X-Auth={}", token))
        .body(Body::from(req_body))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();

    // Then: HTTP 201 Created
    assert_eq!(resp.status(), StatusCode::CREATED);
    let resp_json: serde_json::Value = response_json(resp).await;
    assert_eq!(resp_json["resource"], "users");
    assert_eq!(resp_json["action"], "view");
    assert!(resp_json["id"].is_string());
    assert!(resp_json["meta"].is_null());

    // And: Verify policy exists in database
    let policy_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM role_policies
         WHERE role_id = $1 AND resource = $2 AND action = $3",
    )
    .bind(role_id)
    .bind("users")
    .bind("view")
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to query role_policies");

    assert_eq!(policy_count, 1, "Policy should exist in database");
}

// ============================================================================
// Scenario 2: Delete Policy from Role
// ============================================================================

/// **Given**: 角色 role-a 有 users.view 策略
/// **When**: DELETE /api/admin/roles/role-a/policies/{policy_id}
/// **Then**: HTTP 204 No Content
/// **And**: GET /api/admin/roles/role-a/policies 不包含该策略
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_delete_policy_from_role(ctx: &mut SchemaTestContext) {
    let (token, user_id) = create_admin_session_with_user(ctx, "test-admin", 1800).await;
    grant_realm_admin_role(ctx, &user_id).await;

    // Given: Create role with policy
    let role_id = create_role(ctx, &ctx._realm_id, &token, "role-a", "Role A").await;

    // Insert policy directly
    let policy_id = uuid::Uuid::now_v7();
    sqlx::query(
        "INSERT INTO role_policies (id, realm_id, role_id, resource, action)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(policy_id)
    .bind(&ctx._realm_id)
    .bind(role_id)
    .bind("users")
    .bind("view")
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to insert policy");

    // Invalidate cache
    let _ = ctx
        ._app_state
        .permission_checker
        .invalidate_role_policy_cache(&ctx._realm_id, &role_id.to_string())
        .await;

    // When: Delete policy
    let app = ctx.create_unified_test_router();
    let req = Request::builder()
        .method("DELETE")
        .uri(format!(
            "/api/permission/roles/{}/policies/{}",
            role_id, policy_id
        ))
        .header(header::COOKIE, format!("X-Auth={}", token))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();

    // Then: HTTP 204 No Content
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // And: Verify policy is deleted
    let policy_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM role_policies WHERE id = $1")
        .bind(policy_id)
        .fetch_one(&ctx._app_state.pool)
        .await
        .expect("Failed to query role_policies");

    assert_eq!(policy_count, 0, "Policy should be deleted");
}

// ============================================================================
// Scenario 3: Policy Uniqueness Constraint
// ============================================================================

/// **Given**: 角色 role-a 已有 users.view 策略
/// **When**: POST /api/admin/roles/role-a/policies, body: { resource: "users", action: "view" }
/// **Then**: HTTP 409 Conflict (唯一性约束)
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_policy_uniqueness_constraint(ctx: &mut SchemaTestContext) {
    let (token, user_id) = create_admin_session_with_user(ctx, "test-admin", 1800).await;
    grant_realm_admin_role(ctx, &user_id).await;

    // Given: Create role with policy
    let role_id = create_role(ctx, &ctx._realm_id, &token, "role-a", "Role A").await;

    // Insert first policy
    let policy_id = uuid::Uuid::now_v7();
    sqlx::query(
        "INSERT INTO role_policies (id, realm_id, role_id, resource, action)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(policy_id)
    .bind(&ctx._realm_id)
    .bind(role_id)
    .bind("users")
    .bind("view")
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to insert policy");

    // Invalidate cache
    let _ = ctx
        ._app_state
        .permission_checker
        .invalidate_role_policy_cache(&ctx._realm_id, &role_id.to_string())
        .await;

    // When: Try to add duplicate policy
    let app = ctx.create_unified_test_router();
    let req_body = json!({
        "resource": "users",
        "action": "view"
    })
    .to_string();

    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/permission/roles/{}/policies", role_id))
        .header("content-type", "application/json")
        .header(header::COOKIE, format!("X-Auth={}", token))
        .body(Body::from(req_body))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();

    // Then: HTTP 409 Conflict
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

// ============================================================================
// Scenario 4: Get Role Policies (Empty List)
// ============================================================================

/// **Given**: 角色 role-a 没有任何策略
/// **When**: GET /api/admin/roles/role-a/policies
/// **Then**: HTTP 200 OK
/// **And**: 返回空列表 []
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_get_role_policies_empty(ctx: &mut SchemaTestContext) {
    let (token, user_id) = create_admin_session_with_user(ctx, "test-admin", 1800).await;
    grant_realm_admin_role(ctx, &user_id).await;

    // Given: Role without policies
    let role_id = create_role(ctx, &ctx._realm_id, &token, "role-a", "Role A").await;

    // When: Get role policies
    let app = ctx.create_unified_test_router();
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/permission/roles/{}/policies", role_id))
        .header(header::COOKIE, format!("X-Auth={}", token))
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();

    // Then: HTTP 200 OK with empty list
    assert_eq!(resp.status(), StatusCode::OK);
    let resp_json: serde_json::Value = response_json(resp).await;
    assert_eq!(resp_json["policies"].as_array().unwrap().len(), 0);
    assert!(resp_json["meta"].is_null());
}

// ============================================================================
// Scenario 5: Super Admin "All" Policy
// ============================================================================

/// **Given**: 角色 super-admin
/// **When**: POST /api/admin/roles/super-admin/policies, body: { resource: "All", action: "allow" }
/// **Then**: HTTP 201 Created
/// **And**: 角色拥有 "All" 资源的 "allow" 策略
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_super_admin_all_policy(ctx: &mut SchemaTestContext) {
    let (token, user_id) = create_admin_session_with_user(ctx, "test-admin", 1800).await;
    grant_realm_admin_role(ctx, &user_id).await;

    // Given: Create super-admin role
    let role_id = create_role(
        ctx,
        &ctx._realm_id,
        &token,
        "super-admin",
        "Super Administrator",
    )
    .await;

    // When: Add "All" policy
    let app = ctx.create_unified_test_router();
    let req_body = json!({
        "resource": "All",
        "action": "allow"
    })
    .to_string();

    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/permission/roles/{}/policies", role_id))
        .header("content-type", "application/json")
        .header(header::COOKIE, format!("X-Auth={}", token))
        .body(Body::from(req_body))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();

    // Then: HTTP 201 Created
    assert_eq!(resp.status(), StatusCode::CREATED);
    let resp_json: serde_json::Value = response_json(resp).await;
    assert_eq!(resp_json["resource"], "All");
    assert_eq!(resp_json["action"], "allow");
    assert!(resp_json["meta"].is_null());

    // And: Verify "All" policy exists
    let policy_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM role_policies
         WHERE role_id = $1 AND resource = $2 AND action = $3",
    )
    .bind(role_id)
    .bind("All")
    .bind("allow")
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to query role_policies");

    assert_eq!(policy_count, 1, "Super Admin should have All policy");
}
