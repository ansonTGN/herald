use crate::tests::helpers::*;
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use serde_json::json;
use test_context::test_context;
use tower::ServiceExt;

/// Setup Realm Passkey configuration via direct SQL upsert.
async fn setup_realm_passkey_config(
    ctx: &TestContext,
    enabled: bool,
    force_enabled: bool,
    user_verification: &str,
    cross_platform_authenticator: bool,
) {
    let config_value = json!({
        "enabled": enabled,
        "force_enabled": force_enabled,
        "user_verification": user_verification,
        "cross_platform_authenticator": cross_platform_authenticator,
    });
    let metadata = json!({"force_enabled": force_enabled});

    sqlx::query(
        "INSERT INTO realm_config (realm_id, config_type, config_key, config_value, is_secret, enabled, metadata, created_at, updated_at)
         VALUES ($1, 'passkey', 'settings', $2, false, $3, $4::jsonb, NOW(), NOW())
         ON CONFLICT (realm_id, config_type, config_key)
         DO UPDATE SET config_value = EXCLUDED.config_value, enabled = EXCLUDED.enabled, metadata = EXCLUDED.metadata, updated_at = NOW()",
    )
    .bind(&ctx._realm_id)
    .bind(config_value.to_string())
    .bind(enabled)
    .bind(metadata.to_string())
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to upsert realm Passkey config");
}

/// User Story: US-PK-001 / US-PK-003 — Realm 管理员更新 Passkey 配置
/// Covers: design §4.2.1 (PUT config/passkey), §5.1 (ConfigType::Passkey + config_value JSON shape)
#[test_context(TestContext)]
#[tokio::test]
async fn test_put_realm_passkey_config_returns_wrapped_response(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();
    let (token, user_id) =
        create_admin_session_with_user(ctx, "passkey-config-admin@test.com", 1800).await;
    grant_realm_admin_role(ctx, &user_id).await;

    let req = Request::builder()
        .method("PUT")
        .uri(format!("/api/realms/{}/config/passkey", ctx._realm_id))
        .header("content-type", "application/json")
        .header(header::COOKIE, format!("X-Auth={token}"))
        .body(Body::from(
            json!({
                "enabled": true,
                "forceEnabled": true,
                "userVerification": "required",
                "crossPlatformAuthenticator": true
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = crate::tests::response_json(resp).await;
    assert_eq!(body["message"], "Realm Passkey configuration updated");
    assert_eq!(body["enabled"], true);
    assert_eq!(body["forceEnabled"], true);
    assert!(body["updatedAt"].is_string());

    // Verify persisted config_value shape (snake_case in DB JSON)
    let config_value: String = sqlx::query_scalar(
        "SELECT config_value FROM realm_config
         WHERE realm_id = $1 AND config_type = 'passkey' AND config_key = 'settings'",
    )
    .bind(&ctx._realm_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to fetch Passkey config");

    assert!(config_value.contains(r#""enabled":true"#));
    assert!(config_value.contains(r#""force_enabled":true"#));
    assert!(config_value.contains(r#""user_verification":"required""#));
    assert!(config_value.contains(r#""cross_platform_authenticator":true"#));
}

/// User Story: US-PK-001 / US-PK-003 — Realm 管理员读取 Passkey 配置
/// Covers: design §4.2.1 (GET config/passkey), §5.1 (ConfigType::Passkey)
#[test_context(TestContext)]
#[tokio::test]
async fn test_get_realm_passkey_config_returns_wrapped_response(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();
    let (token, user_id) =
        create_admin_session_with_user(ctx, "passkey-config-viewer@test.com", 1800).await;
    grant_realm_admin_role(ctx, &user_id).await;

    setup_realm_passkey_config(ctx, true, false, "preferred", true).await;

    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/realms/{}/config/passkey", ctx._realm_id))
        .header(header::COOKIE, format!("X-Auth={token}"))
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = crate::tests::response_json(resp).await;
    assert_eq!(body["enabled"], true);
    assert_eq!(body["forceEnabled"], false);
    assert_eq!(body["userVerification"], "preferred");
    assert_eq!(body["crossPlatformAuthenticator"], true);
}

/// User Story: US-PK-001 — 无 settings.manage 权限的用户无法更新 Passkey 配置
/// Covers: design §4.2.3 (权限：无权限 403)
#[test_context(TestContext)]
#[tokio::test]
async fn test_put_realm_passkey_config_requires_settings_manage(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();
    // Create session but do NOT grant realm-admin role
    let (token, _user_id) =
        create_admin_session_with_user(ctx, "passkey-config-plain-user@test.com", 1800).await;

    let req = Request::builder()
        .method("PUT")
        .uri(format!("/api/realms/{}/config/passkey", ctx._realm_id))
        .header("content-type", "application/json")
        .header(header::COOKIE, format!("X-Auth={token}"))
        .body(Body::from(
            json!({
                "enabled": true,
                "forceEnabled": false
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "PUT config/passkey without settings.manage should return 403"
    );
}

/// User Story: US-PK-001 — 无 settings.view 权限的用户无法查看 Passkey 配置
/// Covers: design §4.2.3 (权限：无权限 403)
#[test_context(TestContext)]
#[tokio::test]
async fn test_get_realm_passkey_config_requires_settings_view(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();
    // Create session but do NOT grant realm-admin role
    let (token, _user_id) =
        create_admin_session_with_user(ctx, "passkey-config-plain-viewer@test.com", 1800).await;

    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/realms/{}/config/passkey", ctx._realm_id))
        .header(header::COOKIE, format!("X-Auth={token}"))
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "GET config/passkey without settings.view should return 403"
    );
}

/// User Story: US-PK-001 — 跨 Realm 访问 Passkey 配置被拒绝
/// Covers: design §4.2.3 (跨 realm 403), §4.5
#[test_context(TestContext)]
#[tokio::test]
async fn test_realm_passkey_config_cross_realm_forbidden(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();
    let (token, user_id) =
        create_admin_session_with_user(ctx, "passkey-config-cross-realm-admin@test.com", 1800)
            .await;
    grant_realm_admin_role(ctx, &user_id).await;

    let other_realm_id = uuid::Uuid::now_v7().to_string();

    // PUT to a different realm should be forbidden
    let put_req = Request::builder()
        .method("PUT")
        .uri(format!("/api/realms/{}/config/passkey", other_realm_id))
        .header("content-type", "application/json")
        .header(header::COOKIE, format!("X-Auth={token}"))
        .body(Body::from(
            json!({
                "enabled": true,
                "forceEnabled": false
            })
            .to_string(),
        ))
        .unwrap();

    let put_resp = app.clone().oneshot(put_req).await.unwrap();
    assert_eq!(
        put_resp.status(),
        StatusCode::FORBIDDEN,
        "PUT config/passkey for a different realm should return 403"
    );

    // GET from a different realm should also be forbidden
    let get_req = Request::builder()
        .method("GET")
        .uri(format!("/api/realms/{}/config/passkey", other_realm_id))
        .header(header::COOKIE, format!("X-Auth={token}"))
        .body(Body::empty())
        .unwrap();

    let get_resp = app.oneshot(get_req).await.unwrap();
    assert_eq!(
        get_resp.status(),
        StatusCode::FORBIDDEN,
        "GET config/passkey for a different realm should return 403"
    );
}

/// User Story: US-PK-002 — 强制启用 Passkey 后 force_enabled 持久化为 true
/// Covers: design §4.2.1 (forceEnabled 读写), §5.1
#[test_context(TestContext)]
#[tokio::test]
async fn test_force_enable_passkey_persists_flag(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();
    let (admin_token, user_id) =
        create_admin_session_with_user(ctx, "passkey-config-force-admin@test.com", 1800).await;
    grant_realm_admin_role(ctx, &user_id).await;

    // Initial state: enabled but not forced
    setup_realm_passkey_config(ctx, true, false, "preferred", true).await;

    let req = Request::builder()
        .method("PUT")
        .uri(format!("/api/realms/{}/config/passkey", ctx._realm_id))
        .header("content-type", "application/json")
        .header(header::COOKIE, format!("X-Auth={admin_token}"))
        .body(Body::from(
            json!({
                "enabled": true,
                "forceEnabled": true,
                "userVerification": "required",
                "crossPlatformAuthenticator": true
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let config_value: String = sqlx::query_scalar(
        "SELECT config_value FROM realm_config
         WHERE realm_id = $1 AND config_type = 'passkey' AND config_key = 'settings'",
    )
    .bind(&ctx._realm_id)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to fetch Passkey config");

    assert!(
        config_value.contains(r#""force_enabled":true"#),
        "force_enabled should be persisted as true"
    );
}
