use crate::tests::helpers::auth_helpers::{create_admin_session_with_user, grant_realm_admin_role};
use crate::tests::helpers::points_package_helpers::create_points_package;
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;

#[cfg(test)]
mod tests {
    use super::*;
    use test_context::test_context;

    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_points_package_create_requires_realm_admin(ctx: &mut TestContext) {
        let app = ctx.create_unified_test_router();
        let (token, _user_id) =
            create_admin_session_with_user(ctx, "points-user@test.com", 1800).await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/bill/{}/points-packages", ctx._realm_id))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::from(
                        json!({
                            "name": "starter-pack",
                            "title": "Starter Pack",
                            "points": 500,
                            "price": 9900,
                            "currency": "CNY",
                            "enabled": true
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_text = String::from_utf8(body.to_vec()).unwrap();
        assert!(
            body_text.contains("points.manage"),
            "Forbidden response should mention points.manage requirement, got: {body_text}"
        );
    }

    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_points_package_list_only_returns_enabled_packages(
        ctx: &mut TestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();
        let (token, user_id) =
            create_admin_session_with_user(ctx, "points-viewer@test.com", 1800).await;
        grant_realm_admin_role(ctx, &user_id).await;

        create_points_package(
            ctx,
            &realm_id,
            "enabled-pack",
            "Enabled Pack",
            500,
            9900,
            "CNY",
            true,
        )
        .await;
        create_points_package(
            ctx,
            &realm_id,
            "disabled-pack",
            "Disabled Pack",
            900,
            19900,
            "CNY",
            false,
        )
        .await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/bill/{}/points-packages", realm_id))
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let packages = json["packages"].as_array().unwrap();

        assert_eq!(
            packages.len(),
            1,
            "Only enabled packages should be returned"
        );
        assert_eq!(packages[0]["name"], "enabled-pack");
    }

    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_points_package_realm_admin_can_create(ctx: &mut TestContext) {
        let app = ctx.create_unified_test_router();
        let (token, user_id) =
            create_admin_session_with_user(ctx, "points-admin@test.com", 1800).await;
        grant_realm_admin_role(ctx, &user_id).await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/bill/{}/points-packages", ctx._realm_id))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::from(
                        json!({
                            "name": "admin-pack",
                            "title": "Admin Pack",
                            "points": 800,
                            "price": 12900,
                            "currency": "CNY",
                            "enabled": true
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
    }
}
