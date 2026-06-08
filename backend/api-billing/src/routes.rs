use axum::{
    Router,
    extract::Request,
    http::StatusCode,
    middleware::{Next, from_fn},
    response::{IntoResponse, Response},
    routing::{get, post},
};

use crate::entitlement_mapping_handlers::{
    get_entitlement_mapping, list_entitlement_mappings, list_one_time_mappings,
    sync_provider_products, update_entitlement_mapping,
};
use crate::feature_availability::get_feature_availability;
use crate::handlers::{
    cancel_subscription_for_client_app, create_checkout_session, get_subscription,
    get_subscription_for_client_app, list_subscriptions,
};
use crate::handlers_history::{
    get_my_subscription_history, get_subscription_history, list_my_subscription_history,
    list_subscription_history,
};
use crate::invoice_handlers::{
    apply_invoice, create_invoice, download_invoice_pdf, download_my_invoice_pdf, get_invoice,
    get_my_invoice, get_seller_config, issue_invoice, list_invoices, list_my_invoices, mark_paid,
    update_invoice, upsert_seller_config, void_invoice,
};
use crate::purchase_handlers::{
    cancel_payment_attempt, create_payment_attempt, fulfill_payment, get_payment_attempt_status,
    get_purchase_history,
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
        // ===== Feature Availability =====
        .route(
            "/api/realms/{realmId}/feature-availability",
            get(get_feature_availability),
        )
        // ===== Entitlement Mapping =====
        .route(
            "/api/bill/{realmId}/entitlement-mappings",
            get(list_entitlement_mappings),
        )
        .route(
            "/api/bill/{realmId}/one-time-mappings",
            get(list_one_time_mappings),
        )
        .route(
            "/api/bill/{realmId}/entitlement-mappings/sync",
            post(sync_provider_products),
        )
        .route(
            "/api/bill/{realmId}/entitlement-mappings/{mappingId}",
            get(get_entitlement_mapping).patch(update_entitlement_mapping),
        )
        // ===== Subscription List/Detail =====
        .route("/api/bill/{realmId}/subscriptions", get(list_subscriptions))
        .route(
            "/api/bill/{realmId}/subscriptions/{subscriptionId}",
            get(get_subscription),
        )
        // ===== Subscription (Client App) =====
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
        .route(
            "/api/bill/{realmId}/my/subscriptions/history",
            get(list_my_subscription_history),
        )
        .route(
            "/api/bill/{realmId}/my/subscriptions/{subscriptionId}/history",
            get(get_my_subscription_history),
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
            "/api/bill/{realmId}/purchase/history",
            get(get_purchase_history),
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
