use crate::tests::helpers::auth_helpers::{create_admin_session_with_user, grant_realm_admin_role};
use crate::tests::helpers::wechat_helpers::{
    send_create_wechat_config, setup_wechat_config, valid_wechat_config_payload,
};
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::{body::to_bytes, http::StatusCode};

#[cfg(test)]
mod tests {
    use super::*;
    use test_context::test_context;

    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_wechat_config_requires_realm_admin(ctx: &mut TestContext) {
        let app = ctx.create_unified_test_router();
        let (token, _user_id) =
            create_admin_session_with_user(ctx, "wechat-user@test.com", 1800).await;

        let response =
            send_create_wechat_config(&app, &ctx._realm_id, &token, &valid_wechat_config_payload())
                .await;

        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_text = String::from_utf8(body.to_vec()).unwrap();
        assert!(
            body_text.contains("billing.manage"),
            "Forbidden response should mention billing.manage requirement, got: {body_text}"
        );
    }

    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_wechat_config_realm_admin_can_view_existing_config(
        ctx: &mut TestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();
        let (token, user_id) =
            create_admin_session_with_user(ctx, "wechat-admin@test.com", 1800).await;
        grant_realm_admin_role(ctx, &user_id).await;
        setup_wechat_config(ctx, &realm_id).await;

        let response =
            crate::tests::helpers::wechat_helpers::send_get_wechat_config(&app, &realm_id, &token)
                .await;

        assert_eq!(response.status(), StatusCode::OK);
    }
}
