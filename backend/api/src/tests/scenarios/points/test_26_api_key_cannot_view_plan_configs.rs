use crate::tests::helpers::client_helpers::create_test_api_key;
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use test_context::test_context;
use tower::ServiceExt;

#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_api_key_cannot_view_points_plan_configs(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();
    let (api_key, _entity) = create_test_api_key(ctx, "points-config-reader", true, None).await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/points/{}/plan-configs", ctx._realm_id))
                .header("X-API-Key", api_key)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_text = String::from_utf8(body.to_vec()).unwrap();
    assert!(
        body_text.contains("authenticated user session required"),
        "Forbidden response should mention session-only access, got: {body_text}"
    );
}
