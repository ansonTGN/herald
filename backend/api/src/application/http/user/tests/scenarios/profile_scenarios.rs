// =============================================================================
// 个人资料管理场景测试
// =============================================================================
//
// 运行方式：
// cd api
// cargo nextest run test_scenario_view_profile
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
/// User Story 1: 用户查看个人资料
///
/// **场景描述**：
/// 已登录用户访问个人资料页面，查看账户详情。
///
/// **测试步骤**：
/// 1. 创建并登录用户
/// 2. 用户访问个人资料 API
///
/// **验收标准**：
/// - 成功返回用户信息（ID、邮箱、状态等）
/// - 返回 200 状态码
///
/// **运行方式**：
/// ```bash
/// cargo nextest run test_scenario_view_profile
/// ```
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_view_profile(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    // ============================================================================
    // Setup: 创建并登录用户
    // ============================================================================
    println!("[Setup] 创建测试用户");
    let email = "view_profile@example.com";
    let password = "SecurePassword123!";
    let user_id = register_and_verify_user(ctx, &ctx._realm_id, email, password).await;
    println!("[Setup] ✓ 用户创建成功: {}", user_id);

    println!("[Setup] 用户登录");
    let token = login_user(ctx, &ctx._realm_id, &ctx._client_id, email, password).await;
    println!("[Setup] ✓ 用户登录成功");

    // ============================================================================
    // Step 1: 访问个人资料 API
    // ============================================================================
    println!("[Step 1] 访问个人资料 API");
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/users/{}/profile", ctx._realm_id))
        .header(header::COOKIE, format!("X-Auth={}", token))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();

    assert_eq!(resp.status(), StatusCode::OK, "访问资料应返回 200");

    let body: serde_json::Value = crate::tests::response_json(resp).await;
    assert_eq!(body["id"], user_id, "用户 ID 应匹配");
    assert_eq!(body["email"], email, "邮箱应匹配");
    assert_eq!(body["status"], 1, "用户状态应为 active (1)");
    println!("[Step 1] ✓ 个人资料获取成功");

    println!("\n✅ User Story 1 完成：用户查看个人资料成功");
}

/// ============================================================================
/// User Story 2: 用户更新个人资料
///
/// **场景描述**：
/// 已登录用户更新个人资料昵称，系统保存更新。
///
/// **测试步骤**：
/// 1. 创建并登录用户
/// 2. 用户提交更新的昵称
/// 3. 系统保存更新
/// 4. 再次查询验证更新成功
///
/// **验收标准**：
/// - 资料更新成功
/// - 再次查询可以看到更新后的信息
///
/// **运行方式**：
/// ```bash
/// cargo nextest run test_scenario_update_profile
/// ```
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_update_profile(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    // ============================================================================
    // Setup: 创建并登录用户
    // ============================================================================
    println!("[Setup] 创建测试用户");
    let email = "update_profile@example.com";
    let password = "SecurePassword123!";
    register_and_verify_user(ctx, &ctx._realm_id, email, password).await;
    println!("[Setup] ✓ 用户创建成功");

    println!("[Setup] 用户登录");
    let token = login_user(ctx, &ctx._realm_id, &ctx._client_id, email, password).await;
    println!("[Setup] ✓ 用户登录成功");

    // ============================================================================
    // Step 1: 更新个人资料（设置昵称）
    // ============================================================================
    println!("[Step 1] 更新个人资料（设置昵称）");
    let new_nickname = "Test User";
    let req_body = json!({
        "nickname": new_nickname
    })
    .to_string();

    let req = Request::builder()
        .method("PUT")
        .uri(format!("/api/users/{}/profile", ctx._realm_id))
        .header("content-type", "application/json")
        .header(header::COOKIE, format!("X-Auth={}", token))
        .body(Body::from(req_body))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();

    assert_eq!(resp.status(), StatusCode::OK, "更新资料应返回 200");
    println!("[Step 1] ✓ 个人资料更新成功");

    // ============================================================================
    // Step 2: 验证更新后的资料
    // ============================================================================
    println!("[Step 2] 验证更新后的资料");
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/users/{}/profile", ctx._realm_id))
        .header(header::COOKIE, format!("X-Auth={}", token))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();

    let body: serde_json::Value = crate::tests::response_json(resp).await;
    assert_eq!(body["nickname"], new_nickname, "昵称应已更新");
    println!("[Step 2] ✓ 资料验证成功，昵称已更新");

    println!("\n✅ User Story 2 完成：用户更新个人资料成功");
}

/// ============================================================================
/// User Story 3: 用户修改密码
///
/// **场景描述**：
/// 已登录用户修改密码，系统验证旧密码后保存新密码。
///
/// **测试步骤**：
/// 1. 创建并登录用户
/// 2. 用户输入当前密码和新密码
/// 3. 系统验证当前密码
/// 4. 系统保存新密码
/// 5. 旧密码无法登录
/// 6. 新密码可以成功登录
///
/// **验收标准**：
/// - 密码修改成功
/// - 旧密码无法登录
/// - 新密码可以成功登录
///
/// **运行方式**：
/// ```bash
/// cargo nextest run test_scenario_change_password
/// ```
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_change_password(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();

    // ============================================================================
    // Setup: 创建并登录用户
    // ============================================================================
    println!("[Setup] 创建测试用户");
    let email = "change_password@example.com";
    let old_password = "OldPassword123!";
    register_and_verify_user(ctx, &ctx._realm_id, email, old_password).await;
    println!("[Setup] ✓ 用户创建成功");

    println!("[Setup] 用户登录");
    let token = login_user(ctx, &ctx._realm_id, &ctx._client_id, email, old_password).await;
    println!("[Setup] ✓ 用户登录成功");

    // ============================================================================
    // Step 1: 修改密码
    // ============================================================================
    println!("[Step 1] 修改密码");
    let new_password = "NewPassword456!";
    let req_body = json!({
        "oldPass": old_password,
        "newPass": new_password
    })
    .to_string();

    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/users/{}/change-password", ctx._realm_id))
        .header("content-type", "application/json")
        .header(header::COOKIE, format!("X-Auth={}", token))
        .body(Body::from(req_body))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();

    assert_eq!(resp.status(), StatusCode::OK, "修改密码应返回 200");
    println!("[Step 1] ✓ 密码修改成功");

    // ============================================================================
    // Step 2: 旧密码无法登录
    // ============================================================================
    println!("[Step 2] 旧密码无法登录");
    let req_body = json!({
        "client_id": ctx._client_id,
        "email": email,
        "password": old_password,
        "turnstile_token": "dummy"
    })
    .to_string();

    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/auth/{}/login", ctx._realm_id))
        .header("content-type", "application/json")
        .header("x-forwarded-for", "3.3.3.3")
        .body(Body::from(req_body))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "旧密码应无法登录");
    println!("[Step 2] ✓ 旧密码无法登录");

    // ============================================================================
    // Step 3: 新密码可以成功登录
    // ============================================================================
    println!("[Step 3] 新密码登录");
    let _token = login_user(ctx, &ctx._realm_id, &ctx._client_id, email, new_password).await;
    println!("[Step 3] ✓ 新密码登录成功");

    println!("\n✅ User Story 3 完成：用户修改密码成功");
}
