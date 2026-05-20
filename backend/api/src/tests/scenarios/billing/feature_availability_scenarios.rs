// =============================================================================
// Feature Availability Scenario Tests
// =============================================================================
//
// User Story: .ai/future/fix_visible.md
// Covers:
// - US-BILL-VIS-001: 管理员可进入 billing 配置起点
// - US-BILL-VIS-002: 产品创建后显示计划配置入口
// - US-POINTS-VIS-001/002: 积分基础能力和购买能力按配置显示
// - US-INVOICE-VIS-001: 用户端发票入口按配置显示
//
// =============================================================================

#[cfg(test)]
mod tests {
    use crate::tests::helpers::points_package_helpers::{
        create_payment_provider_mapping, create_points_package,
    };
    use crate::tests::helpers::*;
    use crate::tests::schema_test_context::SchemaTestContext;
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use herald_core::domain::authorization::PermissionService;
    use serde_json::Value;
    use test_context::test_context;
    use tower::ServiceExt;
    use uuid::Uuid;

    async fn grant_permission(
        ctx: &SchemaTestContext,
        user_id: &str,
        resource: &str,
        action: &str,
    ) {
        let role_uuid = Uuid::now_v7();
        let user_uuid = Uuid::parse_str(user_id).expect("user_id should be a UUID");
        let role_name = format!("feature-availability-{resource}-{action}-{role_uuid}");

        sqlx::query(
            "INSERT INTO roles (id, name, description, realm_id, client_id, is_builtin)
             VALUES ($1, $2, 'Feature availability test role', $3, $4, false)",
        )
        .bind(role_uuid)
        .bind(role_name)
        .bind(&ctx._realm_id)
        .bind(&ctx._client_id)
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to insert feature availability test role");

        sqlx::query(
            "INSERT INTO role_policies (id, role_id, realm_id, resource, action)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(Uuid::now_v7())
        .bind(role_uuid)
        .bind(&ctx._realm_id)
        .bind(resource)
        .bind(action)
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to insert feature availability test policy");

        sqlx::query(
            "INSERT INTO user_roles (id, user_id, role_id, realm_id, client_id)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(Uuid::now_v7())
        .bind(user_uuid)
        .bind(role_uuid)
        .bind(&ctx._realm_id)
        .bind(&ctx._client_id)
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to assign feature availability test role");

        let _ = ctx
            .app_state
            .permission_checker
            .invalidate_user_role_cache(&ctx._realm_id, user_id)
            .await;
    }

    async fn reset_feature_data(ctx: &SchemaTestContext) {
        let realm_id = &ctx._realm_id;

        sqlx::query(
            "DELETE FROM invoice_history
             USING invoice
             WHERE invoice_history.invoice_id = invoice.id AND invoice.realm_id = $1",
        )
        .bind(realm_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        sqlx::query(
            "DELETE FROM invoice_line_item
             USING invoice
             WHERE invoice_line_item.invoice_id = invoice.id AND invoice.realm_id = $1",
        )
        .bind(realm_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        for sql in [
            "DELETE FROM invoice WHERE realm_id = $1",
            "DELETE FROM invoice_seller_config WHERE realm_id = $1",
            "DELETE FROM points_package_purchases WHERE realm_id = $1",
            "DELETE FROM payment_attempts WHERE realm_id = $1",
            "DELETE FROM subscription_history WHERE realm_id = $1",
            "DELETE FROM subscription WHERE realm_id = $1",
            "DELETE FROM client_app_subscription_plan
             USING client_app
             WHERE client_app_subscription_plan.client_app_id = client_app.id
               AND client_app.realm_id = $1",
            "DELETE FROM subscription_plan_payment_provider
             USING subscription_plan
             WHERE subscription_plan_payment_provider.plan_id = subscription_plan.id
               AND subscription_plan.realm_id = $1",
            "DELETE FROM points_plan_configs WHERE realm_id = $1",
            "DELETE FROM subscription_plan WHERE realm_id = $1",
            "DELETE FROM products WHERE realm_id = $1",
            "DELETE FROM points_package_payment_providers
             USING points_packages
             WHERE points_package_payment_providers.points_package_id = points_packages.id
               AND points_packages.realm_id = $1",
            "DELETE FROM points_packages WHERE realm_id = $1",
            "DELETE FROM realm_config
             WHERE realm_id = $1 AND config_type IN ('wechat', 'shopify', 'stripe', 'creem')",
        ] {
            sqlx::query(sql)
                .bind(realm_id)
                .execute(&ctx.app_state.pool)
                .await
                .unwrap();
        }
    }

    async fn get_feature_availability(ctx: &SchemaTestContext, token: &str) -> (StatusCode, Value) {
        let app = ctx.create_unified_test_router();
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!(
                        "/api/realms/{}/feature-availability",
                        ctx._realm_id
                    ))
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json = serde_json::from_slice(&body).unwrap_or(Value::Null);
        (status, json)
    }

    async fn create_stripe_api_key(ctx: &SchemaTestContext) {
        sqlx::query(
            "INSERT INTO realm_config
             (realm_id, config_type, config_key, config_value, is_secret, enabled)
             VALUES ($1, 'stripe', 'api_key', 'sk_test_feature_visibility', true, true)
             ON CONFLICT (realm_id, config_type, config_key)
             DO UPDATE SET config_value = EXCLUDED.config_value, enabled = true, updated_at = NOW()",
        )
        .bind(&ctx._realm_id)
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to create Stripe API key config");
    }

    async fn create_plan_payment_mapping(ctx: &SchemaTestContext, plan_id: Uuid) {
        sqlx::query(
            "INSERT INTO subscription_plan_payment_provider
             (id, plan_id, payment_provider, external_product_id, external_price_id, enabled)
             VALUES ($1, $2, 'stripe', 'prod_feature_visibility', 'price_feature_visibility', true)",
        )
        .bind(Uuid::now_v7())
        .bind(plan_id)
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to create plan payment provider mapping");
    }

    async fn assign_plan_to_default_client_app(ctx: &SchemaTestContext, plan_id: Uuid) {
        let client_app_id = Uuid::parse_str(&ctx._client_app_id).expect("client app id is a UUID");
        sqlx::query(
            "INSERT INTO client_app_subscription_plan (id, client_app_id, plan_id, enabled)
             VALUES ($1, $2, $3, true)",
        )
        .bind(Uuid::now_v7())
        .bind(client_app_id)
        .bind(plan_id)
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to assign plan to default client app");
    }

    #[test_context(SchemaTestContext)]
    #[tokio::test]
    async fn test_feature_availability_unconfigured_realm_returns_safe_defaults(
        ctx: &mut SchemaTestContext,
    ) {
        reset_feature_data(ctx).await;
        let (token, user_id) =
            create_admin_session_with_user(ctx, "feature-defaults@test.com", 1800).await;
        grant_permission(ctx, &user_id, "billing", "view").await;
        grant_permission(ctx, &user_id, "points", "view").await;

        let (status, json) = get_feature_availability(ctx, &token).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["admin"]["billingVisible"], true);
        assert_eq!(json["admin"]["billingConfigVisible"], true);
        assert_eq!(json["admin"]["productsVisible"], true);
        assert_eq!(json["admin"]["plansVisible"], false);
        assert_eq!(json["admin"]["subscriptionHistoryVisible"], false);
        assert_eq!(json["admin"]["pointsVisible"], true);
        assert_eq!(json["admin"]["pointsPackagesVisible"], true);
        assert_eq!(json["user"]["pointsVisible"], true);
        assert_eq!(json["user"]["pointsPurchaseVisible"], false);
        assert_eq!(json["user"]["subscriptionVisible"], false);
        assert_eq!(json["user"]["invoicesVisible"], false);
        assert_eq!(json["facts"]["hasPaymentProviders"], false);
        assert_eq!(json["facts"]["hasProducts"], false);
        assert_eq!(json["facts"]["hasPlans"], false);
        assert_eq!(json["facts"]["hasInvoiceSellerConfig"], false);
    }

    #[test_context(SchemaTestContext)]
    #[tokio::test]
    async fn test_feature_availability_product_and_plan_setup_controls_subscription_visibility(
        ctx: &mut SchemaTestContext,
    ) {
        reset_feature_data(ctx).await;
        let (token, user_id) =
            create_admin_session_with_user(ctx, "feature-subscription@test.com", 1800).await;
        grant_permission(ctx, &user_id, "billing", "view").await;

        let product_id = ensure_default_product(ctx, &ctx._realm_id.clone()).await;

        let (status_after_product, after_product) = get_feature_availability(ctx, &token).await;
        assert_eq!(status_after_product, StatusCode::OK);
        assert_eq!(after_product["facts"]["hasProducts"], true);
        assert_eq!(after_product["admin"]["plansVisible"], true);
        assert_eq!(after_product["user"]["subscriptionVisible"], false);

        let plan_id = create_test_plan(ctx, &ctx._realm_id.clone(), "feature-visible-plan").await;
        create_stripe_api_key(ctx).await;
        create_plan_payment_mapping(ctx, plan_id).await;
        assign_plan_to_default_client_app(ctx, plan_id).await;

        let (status_after_plan, after_plan) = get_feature_availability(ctx, &token).await;
        assert_eq!(status_after_plan, StatusCode::OK);
        assert_eq!(after_plan["facts"]["hasPlans"], true);
        assert_eq!(after_plan["facts"]["hasPlanPaymentMappings"], true);
        assert_eq!(after_plan["user"]["subscriptionVisible"], true);

        assert_ne!(product_id, Uuid::nil());
    }

    #[test_context(SchemaTestContext)]
    #[tokio::test]
    async fn test_feature_availability_points_purchase_requires_configured_provider_mapping(
        ctx: &mut SchemaTestContext,
    ) {
        reset_feature_data(ctx).await;
        let (token, user_id) =
            create_admin_session_with_user(ctx, "feature-points@test.com", 1800).await;
        grant_permission(ctx, &user_id, "points", "view").await;

        let package_id = create_points_package(
            ctx,
            &ctx._realm_id.clone(),
            "feature-points-package",
            "Feature Points Package",
            1000,
            1000,
            "USD",
            true,
        )
        .await;

        let (status_without_mapping, without_mapping) = get_feature_availability(ctx, &token).await;
        assert_eq!(status_without_mapping, StatusCode::OK);
        assert_eq!(without_mapping["facts"]["hasPointsPackages"], true);
        assert_eq!(without_mapping["user"]["pointsPurchaseVisible"], false);

        create_stripe_api_key(ctx).await;
        create_payment_provider_mapping(ctx, package_id, "stripe", Some("prod_points"), true).await;

        let (status_with_mapping, with_mapping) = get_feature_availability(ctx, &token).await;
        assert_eq!(status_with_mapping, StatusCode::OK);
        assert_eq!(
            with_mapping["facts"]["hasPointsPackagePaymentMappings"],
            true
        );
        assert_eq!(with_mapping["user"]["pointsPurchaseVisible"], true);
    }

    #[test_context(SchemaTestContext)]
    #[tokio::test]
    async fn test_feature_availability_invoice_requires_seller_config_and_user_source(
        ctx: &mut SchemaTestContext,
    ) {
        reset_feature_data(ctx).await;
        let (token, user_id) =
            create_admin_session_with_user(ctx, "feature-invoice@test.com", 1800).await;
        grant_permission(ctx, &user_id, "billing", "view").await;
        let user_uuid = Uuid::parse_str(&user_id).expect("user_id should be a UUID");

        let (status_without_config, without_config) = get_feature_availability(ctx, &token).await;
        assert_eq!(status_without_config, StatusCode::OK);
        assert_eq!(without_config["user"]["invoicesVisible"], false);

        sqlx::query(
            "INSERT INTO invoice_seller_config
             (realm_id, seller_name, seller_address, seller_tax_id)
             VALUES ($1, 'Feature Seller', 'Feature Address', 'TAX-FEATURE')",
        )
        .bind(&ctx._realm_id)
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to create invoice seller config");

        let (status_config_only, config_only) = get_feature_availability(ctx, &token).await;
        assert_eq!(status_config_only, StatusCode::OK);
        assert_eq!(config_only["facts"]["hasInvoiceSellerConfig"], true);
        assert_eq!(config_only["user"]["invoicesVisible"], false);

        let package_id = create_points_package(
            ctx,
            &ctx._realm_id.clone(),
            "feature-invoice-package",
            "Feature Invoice Package",
            1000,
            1000,
            "USD",
            true,
        )
        .await;
        sqlx::query(
            "INSERT INTO payment_attempts
             (id, realm_id, user_id, payment_provider, target_type, target_id, amount, currency, status, expires_at)
             VALUES ($1, $2, $3, 'stripe', 'points_package', $4, 1000, 'USD', 'Succeeded', NOW() + INTERVAL '30 minutes')",
        )
        .bind(Uuid::now_v7())
        .bind(&ctx._realm_id)
        .bind(user_uuid)
        .bind(package_id)
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to create succeeded payment attempt");

        let (status_with_source, with_source) = get_feature_availability(ctx, &token).await;
        assert_eq!(status_with_source, StatusCode::OK);
        assert_eq!(with_source["user"]["invoicesVisible"], true);
    }
}
