use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use serde_json::json;
use sqlx::PgPool;
use test_context::test_context;
use tower::ServiceExt;

// Use SchemaTestContext
use crate::tests::schema_test_context::SchemaTestContext;

// Import test helper from main.rs
use crate::tests::extract_set_cookie_token;

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_get_profile_success(ctx: &mut SchemaTestContext) {
    // Enable registration for this realm
    sqlx::query(
        "INSERT INTO realm_config (realm_id, config_type, config_key, config_value, enabled)
         VALUES ($1, 'registration', 'allowed', 'true', true)",
    )
    .bind(&ctx._realm_id)
    .execute(&ctx._app_state.pool as &sqlx::PgPool)
    .await
    .unwrap();

    // Enable email verification requirement
    sqlx::query(
        "INSERT INTO realm_config (realm_id, config_type, config_key, config_value, enabled)
         VALUES ($1, 'registration', 'require_email_verification', 'true', true)",
    )
    .bind(&ctx._realm_id)
    .execute(&ctx._app_state.pool as &sqlx::PgPool)
    .await
    .unwrap();

    let app = ctx.create_unified_test_router();

    // 1) Register and login
    let payload = json!({
        "email": "profile@example.com",
        "password": "password123",
        "turnstile_token": "dummy"
    });
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/auth/{}/register", ctx._realm_id))
        .header("content-type", "application/json")
        .header("x-forwarded-for", "3.3.3.3")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 2) Confirm email
    let code: String = sqlx::query_scalar(
        "select verification_code from email_verification_code where email = $1 order by id desc limit 1",
    )
    .bind("profile@example.com")
    .fetch_one(&ctx._app_state.pool as &sqlx::PgPool)
    .await
    .unwrap();

    let req = Request::builder()
        .method("GET")
        .uri(format!(
            "/api/auth/{}/verify_email/confirm/{code}",
            ctx._realm_id
        ))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 3) Login
    let payload = json!({
        "clientId": ctx._client_id,
        "email": "profile@example.com",
        "password": "password123",
        "turnstileToken": "dummy"
    });
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/auth/{}/login", ctx._realm_id))
        .header("content-type", "application/json")
        .header("x-forwarded-for", "3.3.3.3")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let set_cookie = resp
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|v| v.to_str().ok())
        .unwrap()
        .to_string();
    let token = extract_set_cookie_token(&set_cookie, "X-Auth").unwrap();

    // 4) Get profile
    let req = Request::builder()
        .method("GET")
        .uri("/api/user/profile")
        .header(header::COOKIE, format!("X-Auth={}", token))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify response
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["email"], "profile@example.com");
    assert_eq!(json["status"], 1);
}

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_update_profile_nickname(ctx: &mut SchemaTestContext) {
    // Enable registration
    sqlx::query(
        "INSERT INTO realm_config (realm_id, config_type, config_key, config_value, enabled)
         VALUES ($1, 'registration', 'allowed', 'true', true)",
    )
    .bind(&ctx._realm_id)
    .execute(&ctx._app_state.pool as &PgPool)
    .await
    .unwrap();

    // Enable email verification requirement
    sqlx::query(
        "INSERT INTO realm_config (realm_id, config_type, config_key, config_value, enabled)
         VALUES ($1, 'registration', 'require_email_verification', 'true', true)",
    )
    .bind(&ctx._realm_id)
    .execute(&ctx._app_state.pool as &PgPool)
    .await
    .unwrap();

    let app = ctx.create_unified_test_router();

    // 1) Register and login
    let payload = json!({
        "email": "nickname@example.com",
        "password": "password123",
        "turnstile_token": "dummy"
    });
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/auth/{}/register", ctx._realm_id))
        .header("content-type", "application/json")
        .header("x-forwarded-for", "3.3.3.3")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 2) Confirm email
    let code: String = sqlx::query_scalar(
        "select verification_code from email_verification_code where email = $1 order by id desc limit 1",
    )
    .bind("nickname@example.com")
    .fetch_one(&ctx._app_state.pool as &PgPool)
    .await
    .unwrap();

    let req = Request::builder()
        .method("GET")
        .uri(format!(
            "/api/auth/{}/verify_email/confirm/{code}",
            ctx._realm_id
        ))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 3) Login
    let payload = json!({
        "clientId": ctx._client_id,
        "email": "nickname@example.com",
        "password": "password123",
        "turnstileToken": "dummy"
    });
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/auth/{}/login", ctx._realm_id))
        .header("content-type", "application/json")
        .header("x-forwarded-for", "3.3.3.3")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let set_cookie = resp
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|v| v.to_str().ok())
        .unwrap()
        .to_string();
    let token = extract_set_cookie_token(&set_cookie, "X-Auth").unwrap();

    // 4) Update nickname
    let payload = json!({
        "nickname": "TestNickname"
    });
    let req = Request::builder()
        .method("PUT")
        .uri("/api/user/profile")
        .header("content-type", "application/json")
        .header(header::COOKIE, format!("X-Auth={}", token))
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify response
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["nickname"], "TestNickname");
}

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_change_password_success(ctx: &mut SchemaTestContext) {
    // Enable registration
    sqlx::query(
        "INSERT INTO realm_config (realm_id, config_type, config_key, config_value, enabled)
         VALUES ($1, 'registration', 'allowed', 'true', true)",
    )
    .bind(&ctx._realm_id)
    .execute(&ctx._app_state.pool as &PgPool)
    .await
    .unwrap();

    // Enable email verification requirement
    sqlx::query(
        "INSERT INTO realm_config (realm_id, config_type, config_key, config_value, enabled)
         VALUES ($1, 'registration', 'require_email_verification', 'true', true)",
    )
    .bind(&ctx._realm_id)
    .execute(&ctx._app_state.pool as &PgPool)
    .await
    .unwrap();

    let app = ctx.create_unified_test_router();

    // 1) Register and login
    let payload = json!({
        "email": "password@example.com",
        "password": "oldPassword123",
        "turnstile_token": "dummy"
    });
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/auth/{}/register", ctx._realm_id))
        .header("content-type", "application/json")
        .header("x-forwarded-for", "3.3.3.3")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 2) Confirm email
    let code: String = sqlx::query_scalar(
        "select verification_code from email_verification_code where email = $1 order by id desc limit 1",
    )
    .bind("password@example.com")
    .fetch_one(&ctx._app_state.pool as &PgPool)
    .await
    .unwrap();

    let req = Request::builder()
        .method("GET")
        .uri(format!(
            "/api/auth/{}/verify_email/confirm/{code}",
            ctx._realm_id
        ))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 3) Login
    let payload = json!({
        "clientId": ctx._client_id,
        "email": "password@example.com",
        "password": "oldPassword123",
        "turnstileToken": "dummy"
    });
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/auth/{}/login", ctx._realm_id))
        .header("content-type", "application/json")
        .header("x-forwarded-for", "3.3.3.3")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let set_cookie = resp
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|v| v.to_str().ok())
        .unwrap()
        .to_string();
    let token = extract_set_cookie_token(&set_cookie, "X-Auth").unwrap();

    // 4) Change password
    let payload = json!({
        "oldPass": "oldPassword123",
        "newPass": "newPassword123"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/user/change-password")
        .header("content-type", "application/json")
        .header(header::COOKIE, format!("X-Auth={}", token))
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 5) Verify new password works
    let payload = json!({
        "clientId": ctx._client_id,
        "email": "password@example.com",
        "password": "newPassword123",
        "turnstileToken": "dummy"
    });
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/auth/{}/login", ctx._realm_id))
        .header("content-type", "application/json")
        .header("x-forwarded-for", "3.3.3.3")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
