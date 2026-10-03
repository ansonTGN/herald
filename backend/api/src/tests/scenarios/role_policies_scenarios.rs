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
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
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
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
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
// Scenario 3: Delete Policy via Mismatched Role Path
// ============================================================================

/// **Given**: 角色 role-a 与 role-b 各有策略，role-b 的策略 policy-b 仍然生效
/// **When**: DELETE /api/permission/roles/role-a/policies/{policy_b}（policy 属于 role-b）
/// **Then**: HTTP 404，policy-b 保持存在
///
/// WHY: 删除必须同时校验 policy 属于路径中的 role。否则同 realm 管理员可用
/// role-a 的路径删掉 role-b 的策略，且缓存失效会打在未校验的 roleId 上——
/// 真正受影响的 role-b 会继续从缓存拿到已被"撤销"的权限，撤销不即时生效。
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_delete_policy_with_mismatched_role_rejected(ctx: &mut SchemaTestContext) {
    let (token, user_id) = create_admin_session_with_user(ctx, "test-admin", 1800).await;
    grant_realm_admin_role(ctx, &user_id).await;

    // Given: two roles; the policy belongs to role-b
    let role_a = create_role(ctx, &ctx._realm_id, &token, "role-a", "Role A").await;
    let role_b = create_role(ctx, &ctx._realm_id, &token, "role-b", "Role B").await;

    let policy_id = uuid::Uuid::now_v7();
    sqlx::query(
        "INSERT INTO role_policies (id, realm_id, role_id, resource, action)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(policy_id)
    .bind(&ctx._realm_id)
    .bind(role_b)
    .bind("users")
    .bind("view")
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to insert policy");

    // When: address the policy through role-a's path
    let app = ctx.create_unified_test_router();
    let req = Request::builder()
        .method("DELETE")
        .uri(format!(
            "/api/permission/roles/{}/policies/{}",
            role_a, policy_id
        ))
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();

    // Then: rejected and the policy survives
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let policy_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM role_policies WHERE id = $1")
        .bind(policy_id)
        .fetch_one(&ctx._app_state.pool)
        .await
        .expect("Failed to query role_policies");
    assert_eq!(
        policy_count, 1,
        "Role-b's policy must not be deletable via role-a's path"
    );
}

// ============================================================================
// Scenario 4: Policy Uniqueness Constraint
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
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(req_body))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();

    // Then: HTTP 409 Conflict
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

// ============================================================================
// Scenario 5: Wildcard / All Policy Names Rejected (Both Segments)
// ============================================================================

/// **Given**: 角色 role-a 没有任何策略，管理员持有 realm-admin
/// **When**: POST 策略携带 action 段 `All` / `vie*`，或 resource 段 `All`（回归）
/// **Then**: 均返回 403，role_a 名下无新策略行
///
/// WHY: 通配/All 为平台保留（permissions.md §2.2）。当前 matches_policy 是
/// 精确匹配所以尚属惰性，但三个策略创建面（定义/角色策略/直接授权）必须
/// 共用同一守卫——action 段一旦漏防，未来引入通配匹配器时该面即成绕过口。
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_policy_rejects_wildcard_or_all_in_either_segment(
    ctx: &mut SchemaTestContext,
) {
    let (token, user_id) = create_admin_session_with_user(ctx, "test-admin", 1800).await;
    grant_realm_admin_role(ctx, &user_id).await;

    // Given: Create role without policies
    let role_id = create_role(ctx, &ctx._realm_id, &token, "role-a", "Role A").await;

    let app = ctx.create_unified_test_router();
    for (resource, action) in [("users", "All"), ("users", "vie*"), ("All", "manage")] {
        let req = Request::builder()
            .method("POST")
            .uri(format!("/api/permission/roles/{}/policies", role_id))
            .header("content-type", "application/json")
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .body(Body::from(
                json!({ "resource": resource, "action": action }).to_string(),
            ))
            .unwrap();

        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "Policy ({resource},{action}) must hit the shared wildcard guard"
        );
    }

    // And: no policy row was written
    let policy_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM role_policies WHERE role_id = $1")
            .bind(role_id)
            .fetch_one(&ctx._app_state.pool)
            .await
            .expect("Failed to query role_policies");
    assert_eq!(policy_count, 0, "Wildcard/All policies must not be created");
}

// ============================================================================
// Scenario 6: Cross-Realm Role Policy Endpoints Are Uniform 404
// ============================================================================

/// **Given**: realm-b 存在角色 role-b 及其策略 policy-b；管理员属于 admin realm
/// **When**: 通过 /api/permission/roles/{roleId}/policies 三端点访问 role-b /
/// policy-b（跨 realm 资源），或访问未知 roleId
/// **Then**: 一律 404——PRD permissions.md §4.2：管理端点 realm 由 admin 会话
/// token 钉定，跨 realm 资源返回 404；若对跨 realm 资源 403、对未知 id 404，
/// 状态码差异即向本 realm 管理员泄露其他 realm 的 role/policy 存在性。
/// policy-b 保持存在（不得跨 realm 误删）。
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_cross_realm_role_policy_endpoints_return_404(ctx: &mut SchemaTestContext) {
    let (token, user_id) = create_admin_session_with_user(ctx, "test-admin", 1800).await;
    grant_realm_admin_role(ctx, &user_id).await;

    // Given: a role + policy in another realm
    let realm_b = format!("realm-x-{}", uuid::Uuid::now_v7().simple());
    sqlx::query("INSERT INTO realm (id, name) VALUES ($1, 'Cross Realm')")
        .bind(&realm_b)
        .execute(&ctx._app_state.pool)
        .await
        .expect("Failed to insert realm-b");

    let role_b = uuid::Uuid::now_v7();
    sqlx::query(
        "INSERT INTO roles (id, name, description, realm_id, client_id, is_builtin)
         VALUES ($1, 'role-b', 'Foreign role', $2, $3, false)",
    )
    .bind(role_b)
    .bind(&realm_b)
    .bind(&ctx._client_id)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to insert role-b");

    let policy_b = uuid::Uuid::now_v7();
    sqlx::query(
        "INSERT INTO role_policies (id, role_id, realm_id, resource, action)
         VALUES ($1, $2, $3, 'users', 'view')",
    )
    .bind(policy_b)
    .bind(role_b)
    .bind(&realm_b)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to insert policy-b");

    let app = ctx.create_unified_test_router();

    // When/Then: GET cross-realm role policies -> 404
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/permission/roles/{}/policies", role_b))
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // When/Then: POST to cross-realm role -> 404
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/permission/roles/{}/policies", role_b))
                .header("content-type", "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .body(Body::from(
                    json!({ "resource": "users", "action": "view" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // When/Then: DELETE cross-realm policy -> 404, policy survives
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!(
                    "/api/permission/roles/{}/policies/{}",
                    role_b, policy_b
                ))
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let surviving: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM role_policies WHERE id = $1")
        .bind(policy_b)
        .fetch_one(&ctx._app_state.pool)
        .await
        .expect("Failed to query policy-b");
    assert_eq!(surviving, 1, "Cross-realm policy must not be deleted");

    // And: unknown role id is indistinguishable from a cross-realm one
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/permission/roles/{}/policies",
                    uuid::Uuid::now_v7()
                ))
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
