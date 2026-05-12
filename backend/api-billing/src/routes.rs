use axum::{
    Router,
    extract::Request,
    http::StatusCode,
    middleware::{Next, from_fn},
    response::{IntoResponse, Response},
    routing::{get, patch, post},
};

use crate::handlers::{
    add_payment_provider_to_plan, assign_plan_to_client_app, cancel_subscription_for_client_app,
    create_checkout_session, create_plan, create_product, delete_plan, delete_product, get_plan,
    get_product, get_product_plans, get_subscription_for_client_app, list_plan_assignments,
    list_plan_payment_providers, list_plans, list_products, remove_payment_provider_from_plan,
    remove_plan_assignment, toggle_plan_assignment, toggle_plan_payment_provider, update_plan,
    update_plan_payment_provider, update_product,
};
use crate::handlers_history::{get_subscription_history, list_subscription_history};
use crate::invoice_handlers::{
    apply_invoice, create_invoice, download_invoice_pdf, download_my_invoice_pdf, get_invoice,
    get_my_invoice, get_seller_config, issue_invoice, list_invoices, list_my_invoices, mark_paid,
    update_invoice, upsert_seller_config, void_invoice,
};
use crate::points_package_handlers::{
    create_payment_provider_mapping, create_points_package, delete_payment_provider_mapping,
    delete_points_package, get_points_package, list_payment_provider_mappings,
    list_points_packages, update_payment_provider_mapping, update_points_package,
};
use crate::purchase_handlers::{
    cancel_payment_attempt, create_payment_attempt, fulfill_payment, get_payment_attempt_status,
    get_points_package_purchase_details, get_points_package_purchase_history,
};
use crate::shopify_claim_handlers::claim_shopify_subscriptions;
use crate::shopify_config_handlers::{
    create_shopify_config, delete_shopify_config, get_shopify_config, list_payment_providers,
    test_shopify_connection_endpoint, update_shopify_config,
};
use crate::shopify_webhook_handlers::shopify_webhook_handler;
use crate::shopify_webhook_utils::constant_time_compare;
use crate::stripe_webhook_handlers::handle_stripe_webhook;
use crate::webhook_handlers::handle_creem_webhook;
use crate::wechat_config_handlers::{
    create_wechat_config, delete_wechat_config, get_wechat_config, update_wechat_config,
};
use crate::wechat_order_handlers::{
    close_wechat_order, create_wechat_order, get_wechat_order_status,
};
use crate::wechat_webhook_handlers::wechat_webhook_handler;
use herald_api_base::application::http::state::AppState;

use crate::shopify_test_handlers::create_unclaimed_subscription;

async fn internal_api_key_middleware(req: Request, next: Next) -> Response {
    let provided_key = req
        .headers()
        .get("X-Internal-API-Key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim);

    let expected_key = std::env::var("INTERNAL_API_KEY")
        .ok()
        .filter(|key| !key.trim().is_empty());

    match (provided_key, expected_key) {
        (Some(provided), Some(expected)) if constant_time_compare(provided, &expected) => {
            next.run(req).await
        }
        _ => StatusCode::UNAUTHORIZED.into_response(),
    }
}

pub fn billing_public_routes() -> Router<AppState> {
    Router::new()
        // ===== Webhooks =====
        .route(
            "/api/third/pay/{realmId}/creem/webhooks",
            post(handle_creem_webhook),
        )
        .route(
            "/api/third/pay/{realmId}/stripe/webhooks",
            post(handle_stripe_webhook),
        )
        .route(
            "/api/third/pay/{realmId}/shopify/webhooks",
            post(shopify_webhook_handler),
        )
        .route(
            "/api/third/pay/{realmId}/wechat/webhooks",
            post(wechat_webhook_handler),
        )
        // ===== Internal Fulfillment Webhook =====
        .route(
            "/api/internal/bill/purchase/payment-attempts/{attemptId}/fulfill",
            post(fulfill_payment).layer(from_fn(internal_api_key_middleware)),
        )
}

pub fn billing_routes() -> Router<AppState> {
    Router::new()
        // ===== Plan Management =====
        .route(
            "/api/bill/{realmId}/plans",
            get(list_plans).post(create_plan),
        )
        .route(
            "/api/bill/{realmId}/plans/{planId}",
            get(get_plan).patch(update_plan).delete(delete_plan),
        )
        // ===== Plan Payment Provider Mapping =====
        .route(
            "/api/bill/{realmId}/plans/{planId}/providers",
            get(list_plan_payment_providers).post(add_payment_provider_to_plan),
        )
        .route(
            "/api/bill/{realmId}/plans/{planId}/providers/{mappingId}",
            patch(update_plan_payment_provider).delete(remove_payment_provider_from_plan),
        )
        .route(
            "/api/bill/{realmId}/plans/{planId}/providers/{mappingId}/toggle",
            patch(toggle_plan_payment_provider),
        )
        // Product Management
        .route(
            "/api/bill/{realmId}/products",
            get(list_products).post(create_product),
        )
        .route(
            "/api/bill/{realmId}/products/{productId}",
            get(get_product)
                .patch(update_product)
                .delete(delete_product),
        )
        .route(
            "/api/bill/{realmId}/products/{productId}/plans",
            get(get_product_plans),
        )
        // ===== Plan Assignment =====
        .route(
            "/api/bill/{realmId}/client/{clientAppId}/plans",
            get(list_plan_assignments).post(assign_plan_to_client_app),
        )
        .route(
            "/api/bill/{realmId}/client/{clientAppId}/plans/{assignmentId}",
            patch(toggle_plan_assignment).delete(remove_plan_assignment),
        )
        // ===== Subscription (Simplified) =====
        .route(
            "/api/bill/{realmId}/client/{clientAppId}/subscription",
            get(get_subscription_for_client_app),
        )
        .route(
            "/api/bill/{realmId}/client/{clientAppId}/subscription/cancel",
            post(cancel_subscription_for_client_app),
        )
        .route(
            "/api/bill/{realmId}/client/{clientAppId}/checkout",
            post(create_checkout_session),
        )
        .route(
            "/api/bill/{realmId}/shopify/claim",
            post(claim_shopify_subscriptions),
        )
        // ===== Subscription History =====
        .route(
            "/api/bill/{realmId}/subscriptions/{subscriptionId}/history",
            get(get_subscription_history),
        )
        .route(
            "/api/bill/{realmId}/subscriptions/history",
            get(list_subscription_history),
        )
        // ===== Points Package Management =====
        .route(
            "/api/bill/{realmId}/points-packages",
            get(list_points_packages).post(create_points_package),
        )
        .route(
            "/api/bill/{realmId}/points-packages/{packageId}",
            get(get_points_package)
                .patch(update_points_package)
                .delete(delete_points_package),
        )
        // ===== Points Package Payment Provider Mappings =====
        .route(
            "/api/bill/{realmId}/points-packages/{packageId}/providers",
            get(list_payment_provider_mappings).post(create_payment_provider_mapping),
        )
        .route(
            "/api/bill/{realmId}/points-packages/{packageId}/providers/{mappingId}",
            patch(update_payment_provider_mapping).delete(delete_payment_provider_mapping),
        )
        // ===== Purchase Flow =====
        .route(
            "/api/bill/{realmId}/purchase/payment-attempts",
            post(create_payment_attempt),
        )
        .route(
            "/api/bill/{realmId}/purchase/payment-attempts/{attemptId}",
            get(get_payment_attempt_status),
        )
        .route(
            "/api/bill/{realmId}/purchase/payment-attempts/{attemptId}/cancel",
            post(cancel_payment_attempt),
        )
        // ===== Purchase History =====
        .route(
            "/api/bill/{realmId}/purchase/points-packages/history",
            get(get_points_package_purchase_history),
        )
        .route(
            "/api/bill/{realmId}/purchase/points-packages/history/{purchaseId}",
            get(get_points_package_purchase_details),
        )
        .route(
            "/api/realms/{realmId}/billing/purchase/points-packages/history",
            get(get_points_package_purchase_history),
        )
        .route(
            "/api/realms/{realmId}/billing/purchase/points-packages/history/{purchaseId}",
            get(get_points_package_purchase_details),
        )
        // ===== Payment Provider Configuration =====
        .route(
            "/api/third/pay/{realmId}/providers",
            get(list_payment_providers),
        )
        .route(
            "/api/third/pay/{realmId}/providers/shopify",
            post(create_shopify_config)
                .get(get_shopify_config)
                .put(update_shopify_config)
                .delete(delete_shopify_config),
        )
        .route(
            "/api/third/pay/{realmId}/providers/shopify/test",
            post(test_shopify_connection_endpoint),
        )
        // ===== WeChat Pay Configuration =====
        .route(
            "/api/third/pay/{realmId}/providers/wechat",
            post(create_wechat_config)
                .get(get_wechat_config)
                .put(update_wechat_config)
                .delete(delete_wechat_config),
        )
        // ===== WeChat Pay Orders =====
        .route(
            "/api/third/pay/{realmId}/wechat/create-order",
            post(create_wechat_order),
        )
        .route(
            "/api/third/pay/{realmId}/wechat/order-status/{orderId}",
            get(get_wechat_order_status),
        )
        .route(
            "/api/third/pay/{realmId}/wechat/close-order/{orderId}",
            post(close_wechat_order),
        )
        // ===== Invoice Management =====
        .route(
            "/api/bill/{realmId}/invoice-seller-config",
            get(get_seller_config).put(upsert_seller_config),
        )
        .route(
            "/api/bill/{realmId}/invoices",
            get(list_invoices).post(create_invoice),
        )
        .route(
            "/api/bill/{realmId}/invoices/{invoiceId}",
            get(get_invoice).patch(update_invoice),
        )
        .route(
            "/api/bill/{realmId}/invoices/{invoiceId}/issue",
            post(issue_invoice),
        )
        .route(
            "/api/bill/{realmId}/invoices/{invoiceId}/void",
            post(void_invoice),
        )
        .route(
            "/api/bill/{realmId}/invoices/{invoiceId}/mark-paid",
            post(mark_paid),
        )
        .route(
            "/api/bill/{realmId}/invoices/{invoiceId}/pdf",
            get(download_invoice_pdf),
        )
        // ===== User Invoice (My Invoices) =====
        .route(
            "/api/bill/{realmId}/my/invoices",
            get(list_my_invoices).post(apply_invoice),
        )
        .route(
            "/api/bill/{realmId}/my/invoices/{invoiceId}",
            get(get_my_invoice),
        )
        .route(
            "/api/bill/{realmId}/my/invoices/{invoiceId}/pdf",
            get(download_my_invoice_pdf),
        )
}

/// Test routes for billing integration tests
/// Always compiled but only used in test builds
pub fn billing_test_routes() -> Router<AppState> {
    Router::new()
        // ===== Test Data Creation =====
        .route(
            "/api/test/{realmId}/shopify/unclaimed-subscriptions",
            post(create_unclaimed_subscription),
        )
}
