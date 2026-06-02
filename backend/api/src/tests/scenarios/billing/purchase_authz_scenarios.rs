use crate::tests::helpers::auth_helpers::create_admin_session_with_user;
use crate::tests::helpers::billing_helpers::create_test_plan;
use crate::tests::helpers::points_helpers::create_points_wallet;
use crate::tests::helpers::points_package_helpers::{
    create_payment_attempt, create_points_package,
};
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::{
    Json,
    body::{Body, to_bytes},
    extract::{Path, State},
    http::{Request, StatusCode},
};
use serde_json::json;
use test_context::test_context;
use tower::ServiceExt;
use uuid::Uuid;

#[cfg(test)]
mod tests {
    use super::*;

    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_payment_attempt_create_requires_same_realm_user(ctx: &mut TestContext) {
        let app = ctx.create_unified_test_router();
        let realm_a_id = ctx._realm_id.clone();
        let plan_id = create_test_plan(ctx, &realm_a_id, "purchase-authz-plan").await;

        let realm_b_id = Uuid::now_v7().to_string();
        sqlx::query(
            "INSERT INTO realm (id, name, created_at, updated_at)
             VALUES ($1, 'purchase-authz-realm-b', NOW(), NOW())",
        )
        .bind(&realm_b_id)
        .execute(&ctx._app_state.pool)
        .await
        .unwrap();

        let (token, _user_id) =
            create_admin_session_with_user(ctx, "purchase-user@test.com", 1800).await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/bill/{}/purchase/payment-attempts",
                        realm_b_id
                    ))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::from(
                        json!({
                            "targetType": "subscription_plan",
                            "targetId": plan_id.to_string(),
                            "paymentProvider": "stripe"
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
            body_text.contains("different realm"),
            "Forbidden response should mention cross-realm denial, got: {body_text}"
        );
    }

    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_purchase_history_requires_same_realm_user(ctx: &mut TestContext) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();
        let user_id = Uuid::now_v7();
        let package_id = create_points_package(
            ctx,
            &realm_id,
            "history-authz-pack",
            "History AuthZ Pack",
            600,
            1299,
            "USD",
            true,
        )
        .await;

        create_points_wallet(ctx, user_id, &realm_id).await;

        let attempt_id = create_payment_attempt(
            ctx,
            &realm_id,
            user_id,
            "points_package",
            package_id,
            "wechat",
            1299,
            "USD",
        )
        .await;

        let payload = json!({
            "providerStatus": "success",
            "providerTransactionId": format!("wx_purchase_history_{attempt_id}"),
            "completedAt": chrono::Utc::now().to_rfc3339(),
        });

        let response = crate::application::http::billing::purchase_handlers::fulfill_payment(
            State((*ctx.app_state).clone()),
            Path(attempt_id),
            Json(serde_json::from_value(payload).unwrap()),
        )
        .await;
        assert!(response.is_ok(), "Purchase fulfillment should succeed");

        let realm_b_id = Uuid::now_v7().to_string();
        sqlx::query(
            "INSERT INTO realm (id, name, created_at, updated_at)
             VALUES ($1, 'purchase-history-realm-b', NOW(), NOW())",
        )
        .bind(&realm_b_id)
        .execute(&ctx._app_state.pool)
        .await
        .unwrap();

        let (token, _session_user_id) =
            create_admin_session_with_user(ctx, "purchase-history-user@test.com", 1800).await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!(
                        "/api/bill/{}/purchase/points-packages/history",
                        realm_b_id
                    ))
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_text = String::from_utf8(body.to_vec()).unwrap();
        assert!(
            body_text.contains("different realm"),
            "Forbidden response should mention cross-realm denial, got: {body_text}"
        );
    }
}
