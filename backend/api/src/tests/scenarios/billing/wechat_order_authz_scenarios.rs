use crate::tests::helpers::auth_helpers::create_admin_session_with_user;
use crate::tests::helpers::billing_helpers::create_test_plan;
use crate::tests::helpers::wechat_helpers::{send_create_wechat_order, setup_wechat_config};
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::{body::to_bytes, http::StatusCode};
use serde_json::json;
use test_context::test_context;
use uuid::Uuid;

#[cfg(test)]
mod tests {
    use super::*;

    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_wechat_order_create_requires_same_realm_user(ctx: &mut TestContext) {
        let app = ctx.create_unified_test_router();
        let realm_a_id = ctx._realm_id.clone();
        let plan_id = create_test_plan(ctx, &realm_a_id, "wechat-order-plan").await;
        setup_wechat_config(ctx, &realm_a_id).await;

        let realm_b_id = Uuid::now_v7().to_string();
        sqlx::query(
            "INSERT INTO realm (id, name, created_at, updated_at)
             VALUES ($1, 'wechat-order-realm-b', NOW(), NOW())",
        )
        .bind(&realm_b_id)
        .execute(&ctx._app_state.pool)
        .await
        .unwrap();

        let (token, _user_id) =
            create_admin_session_with_user(ctx, "wechat-order-user@test.com", 1800).await;

        let response = send_create_wechat_order(
            &app,
            &realm_b_id,
            &token,
            &json!({
                "planId": plan_id.to_string()
            }),
        )
        .await;

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
    async fn test_scenario_wechat_order_create_same_realm_user_reaches_business_validation(
        ctx: &mut TestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();
        let plan_id = create_test_plan(ctx, &realm_id, "wechat-order-plan").await;
        let (token, _user_id) =
            create_admin_session_with_user(ctx, "wechat-order-owner@test.com", 1800).await;

        let response = send_create_wechat_order(
            &app,
            &realm_id,
            &token,
            &json!({
                "planId": plan_id.to_string()
            }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
