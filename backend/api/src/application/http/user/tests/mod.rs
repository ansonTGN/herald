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

use crate::tests::helpers::email_config_helpers::insert_resend_email_config_direct;

// TODO: unignore once email sending is mocked in tests — fake Resend API key causes 401
#[test_context(SchemaTestContext)]
#[tokio::test]
#[ignore]
async fn test_get_profile_success(ctx: &mut SchemaTestContext) {
    // Enable registration for this realm
    sqlx::query(
        "INSERT INTO realm_config (realm_id, config_type, config_key, config_value, enabled)
         VALUES ($1, 'registration', 'enabled', 'true', true)",
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

    // Configure email so verification actually works
    insert_resend_email_config_direct(&ctx._app_state.pool, &ctx._realm_id).await;

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

    assert!(!resp.headers().contains_key(header::SET_COOKIE));
    let login_body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let login_json: serde_json::Value = serde_json::from_slice(&login_body).unwrap();
    let token = login_json["accessToken"].as_str().unwrap().to_owned();

    // 4) Get profile
    let req = Request::builder()
        .method("GET")
        .uri("/api/user/profile")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
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

// TODO: unignore once email sending is mocked in tests — fake Resend API key causes 401
#[test_context(SchemaTestContext)]
#[tokio::test]
#[ignore]
async fn test_update_profile_nickname(ctx: &mut SchemaTestContext) {
    // Enable registration
    sqlx::query(
        "INSERT INTO realm_config (realm_id, config_type, config_key, config_value, enabled)
         VALUES ($1, 'registration', 'enabled', 'true', true)",
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

    // Configure email so verification actually works
    insert_resend_email_config_direct(&ctx._app_state.pool, &ctx._realm_id).await;

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

    assert!(!resp.headers().contains_key(header::SET_COOKIE));
    let login_body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let login_json: serde_json::Value = serde_json::from_slice(&login_body).unwrap();
    let token = login_json["accessToken"].as_str().unwrap().to_owned();

    // 4) Update nickname
    let payload = json!({
        "nickname": "TestNickname"
    });
    let req = Request::builder()
        .method("PUT")
        .uri("/api/user/profile")
        .header("content-type", "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify response
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["nickname"], "TestNickname");
}

// TODO: unignore once email sending is mocked in tests — fake Resend API key causes 401
#[test_context(SchemaTestContext)]
#[tokio::test]
#[ignore]
async fn test_change_password_success(ctx: &mut SchemaTestContext) {
    // Enable registration
    sqlx::query(
        "INSERT INTO realm_config (realm_id, config_type, config_key, config_value, enabled)
         VALUES ($1, 'registration', 'enabled', 'true', true)",
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

    // Configure email so verification actually works
    insert_resend_email_config_direct(&ctx._app_state.pool, &ctx._realm_id).await;

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

    assert!(!resp.headers().contains_key(header::SET_COOKIE));
    let login_body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let login_json: serde_json::Value = serde_json::from_slice(&login_body).unwrap();
    let token = login_json["accessToken"].as_str().unwrap().to_owned();

    // 4) Change password
    let payload = json!({
        "oldPass": "oldPassword123",
        "newPass": "newPassword123"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/user/change-password")
        .header("content-type", "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
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
