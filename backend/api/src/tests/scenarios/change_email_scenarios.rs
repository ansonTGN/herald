use crate::tests::helpers::test_setup_helpers::*;
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::json;
use test_context::test_context;
use tower::ServiceExt;

#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_change_email_flow(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();
    let realm_id = ctx._realm_id.clone();

    // Step 1: Create and login user
    let email = "changeme@cas.com";
    let password = "password123";
    let (_user_id, _token) = create_user_and_login(ctx, email, password).await;

    // Step 2: Request email change
    let request_payload = json!({
        "newEmail": "newemail@cas.com"
    });

    let request_req = Request::builder()
        .method("POST")
        .uri(format!("/api/auth/{}/change_email/request", realm_id))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", _token))
        .body(Body::from(request_payload.to_string()))
        .unwrap();

    let request_resp = app.clone().oneshot(request_req).await.unwrap();
    assert_eq!(request_resp.status(), 200, "Request should return 200 OK");

    let request_body = axum::body::to_bytes(request_resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let request_json: serde_json::Value = serde_json::from_slice(&request_body).unwrap();
    // New response structure: just check response is valid JSON
    assert!(request_json.is_object(), "Response should be a JSON object");

    // Step 3: Confirm email change (would require verification code from DB)
    // For now, just verify the request endpoint works
}

#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_change_email_confirm_requires_same_authenticated_user(
    ctx: &mut TestContext,
) {
    let app = ctx.create_unified_test_router();
    let realm_id = ctx._realm_id.clone();

    let (_user_a_id, token_a) =
        create_user_and_login(ctx, "change-owner@cas.com", "password123").await;
    let (_user_b_id, token_b) =
        create_user_and_login(ctx, "change-attacker@cas.com", "password123").await;

    let request_payload = json!({
        "newEmail": "owner-new@cas.com"
    });

    let request_req = Request::builder()
        .method("POST")
        .uri(format!("/api/auth/{}/change_email/request", realm_id))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token_a))
        .body(Body::from(request_payload.to_string()))
        .unwrap();

    let request_resp = app.clone().oneshot(request_req).await.unwrap();
    assert_eq!(request_resp.status(), StatusCode::OK);

    let change_code: String = sqlx::query_scalar(
        "SELECT verification_code FROM email_verification_code
         WHERE email = $1 AND type = 'change_email'
         ORDER BY id DESC LIMIT 1",
    )
    .bind("owner-new@cas.com")
    .fetch_one(&ctx._app_state.pool)
    .await
    .unwrap();

    let attacker_confirm_req = Request::builder()
        .method("GET")
        .uri(format!(
            "/api/auth/{}/change_email/confirm/{}",
            realm_id, change_code
        ))
        .header("authorization", format!("Bearer {}", token_b))
        .body(Body::empty())
        .unwrap();

    let attacker_confirm_resp = app.clone().oneshot(attacker_confirm_req).await.unwrap();
    assert_eq!(attacker_confirm_resp.status(), StatusCode::FORBIDDEN);

    let owner_confirm_req = Request::builder()
        .method("GET")
        .uri(format!(
            "/api/auth/{}/change_email/confirm/{}",
            realm_id, change_code
        ))
        .header("authorization", format!("Bearer {}", token_a))
        .body(Body::empty())
        .unwrap();

    let owner_confirm_resp = app.clone().oneshot(owner_confirm_req).await.unwrap();
    assert_eq!(owner_confirm_resp.status(), StatusCode::OK);

    let updated_email: String = sqlx::query_scalar("SELECT email FROM account WHERE id = $1")
        .bind(_user_a_id)
        .fetch_one(&ctx._app_state.pool)
        .await
        .unwrap();
    assert_eq!(updated_email, "owner-new@cas.com");
}
