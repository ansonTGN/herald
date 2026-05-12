//! 简化的Shopify webhook测试
//!
//! 基本的HMAC验证和webhook处理测试

#[cfg(test)]
mod tests {
    use crate::tests::helpers::shopify_helpers::*;
    use crate::tests::helpers::test_setup_helpers::*;
    use crate::tests::helpers::*;

    use crate::tests::schema_test_context::SchemaTestContext;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use serde_json::json;
    use test_context::test_context;
    use tower::ServiceExt;
    use uuid::Uuid;

    use SchemaTestContext as ShopifyTestContext;

    /// 基本的HMAC验证测试
    #[test_context(ShopifyTestContext)]
    #[tokio::test]
    async fn test_shopify_hmac_basic_verification(ctx: &mut ShopifyTestContext) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();
        let client_secret = "test_client_secret";

        // Setup Shopify config
        setup_shopify_config(
            ctx,
            &realm_id,
            "test-shop.myshopify.com",
            "shpat_test_token",
            "shp_test_token",
            client_secret,
            "2024-01",
        )
        .await;

        // Create simple webhook payload
        let contract_id = generate_shopify_contract_id();
        let customer_id = generate_shopify_customer_id();
        let plan_id = generate_test_plan_id();
        setup_test_plan_config_with_points(ctx, &realm_id, plan_id, 1000).await;
        let email = format!("test-{}@example.com", Uuid::now_v7());
        let user_id = create_test_user(ctx, &email, "password123").await;

        let payload = json!({
            "id": contract_id,
            "adminGraphqlApiId": contract_id,
            "customerId": customer_id,
            "originOrderId": format!("gid://shopify/Order/{}", Uuid::now_v7()),
            "sellingPlanId": format!("gid://shopify/SellingPlan/{}", Uuid::now_v7()),
            "currentPeriodEnd": (chrono::Utc::now() + chrono::Duration::days(30)).to_rfc3339(),
            "status": "ACTIVE",
            "casRealmId": realm_id,
            "casUserId": user_id.to_string(),
            "casPlanId": plan_id.to_string()
        });

        // Calculate valid HMAC
        let payload_str = serde_json::to_string(&payload).unwrap();
        let signature = calculate_shopify_hmac(&payload_str, client_secret);

        // Send webhook with valid signature
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/third/pay/{}/shopify/webhooks", realm_id))
                    .header("x-shopify-topic", "subscription_contracts/create")
                    .header("x-shopify-event-id", Uuid::now_v7().to_string())
                    .header("x-shopify-hmac-sha256", signature)
                    .header("Content-Type", "application/json")
                    .body(Body::from(payload_str))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should get 202 Accepted
        assert_eq!(response.status(), StatusCode::ACCEPTED);
    }

    /// HMAC验证失败测试
    #[test_context(ShopifyTestContext)]
    #[tokio::test]
    async fn test_shopify_hmac_invalid_signature(ctx: &mut ShopifyTestContext) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();
        let client_secret = "test_client_secret";

        // Setup Shopify config
        setup_shopify_config(
            ctx,
            &realm_id,
            "test-shop.myshopify.com",
            "shpat_test_token",
            "shp_test_token",
            client_secret,
            "2024-01",
        )
        .await;

        // Create webhook payload
        let payload = json!({
            "id": "test_contract",
            "status": "ACTIVE"
        });

        let payload_str = serde_json::to_string(&payload).unwrap();

        // Send with invalid signature
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/third/pay/{}/shopify/webhooks", realm_id))
                    .header("x-shopify-topic", "subscription_contracts/create")
                    .header("x-shopify-event-id", Uuid::now_v7().to_string())
                    .header("x-shopify-hmac-sha256", "invalid_signature")
                    .header("Content-Type", "application/json")
                    .body(Body::from(payload_str))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should get 401 Unauthorized
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    /// 幂等性测试
    #[test_context(ShopifyTestContext)]
    #[tokio::test]
    async fn test_shopify_webhook_idempotency(ctx: &mut ShopifyTestContext) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();
        let client_secret = "test_client_secret";

        // Setup Shopify config
        setup_shopify_config(
            ctx,
            &realm_id,
            "test-shop.myshopify.com",
            "shpat_test_token",
            "shp_test_token",
            client_secret,
            "2024-01",
        )
        .await;

        // Create webhook payload
        let event_id = Uuid::now_v7().to_string();
        let contract_id = generate_shopify_contract_id();
        let customer_id = generate_shopify_customer_id();
        let plan_id = generate_test_plan_id();
        setup_test_plan_config_with_points(ctx, &realm_id, plan_id, 1000).await;
        let email = format!("test-{}@example.com", Uuid::now_v7());
        let user_id = create_test_user(ctx, &email, "password123").await;

        let payload = build_shopify_subscription_contracts_create_event(
            event_id.clone(),
            contract_id,
            customer_id,
            user_id,
            plan_id,
            &realm_id,
            None,
        );

        let payload_str = serde_json::to_string(&payload).unwrap();
        let signature = calculate_shopify_hmac(&payload_str, client_secret);

        // Send first webhook
        let response1 = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/third/pay/{}/shopify/webhooks", realm_id))
                    .header("x-shopify-topic", "subscription_contracts/create")
                    .header("x-shopify-event-id", event_id.clone())
                    .header("x-shopify-hmac-sha256", signature.clone())
                    .header("Content-Type", "application/json")
                    .body(Body::from(payload_str.clone()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response1.status(), StatusCode::ACCEPTED);

        // Send second webhook with same event ID
        let response2 = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/third/pay/{}/shopify/webhooks", realm_id))
                    .header("x-shopify-topic", "subscription_contracts/create")
                    .header("x-shopify-event-id", event_id)
                    .header("x-shopify-hmac-sha256", signature)
                    .header("Content-Type", "application/json")
                    .body(Body::from(payload_str))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should also get 202 (idempotent)
        assert_eq!(response2.status(), StatusCode::ACCEPTED);
    }
}
