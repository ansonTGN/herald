use axum::{
    Router,
    extract::Request,
    http::StatusCode,
    middleware::{Next, from_fn},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};

use crate::credit_bucket_handlers::{
    create_credit_bucket_handler, delete_credit_bucket_handler, get_bucket_overview_handler,
    get_credit_bucket_handler, list_credit_buckets_handler, update_credit_bucket_handler,
};
use crate::entitlement_mapping_handlers::{
    batch_update_entitlement_mappings, get_entitlement_mapping, list_entitlement_mappings,
    list_one_time_mappings, sync_provider_products, update_entitlement_mapping,
};
use crate::feature_availability::get_feature_availability;
use crate::handlers::{
    cancel_subscription_for_client_app, create_checkout_session, get_subscription,
    get_subscription_for_client_app, list_purchase_options, list_subscriptions,
};
use crate::handlers_history::{
    get_my_subscription_history, get_subscription_history, list_my_subscription_history,
    list_subscription_history,
};
use crate::invoice_handlers::{
    apply_invoice, create_credit_note, create_invoice, download_invoice_pdf,
    download_my_invoice_pdf, get_invoice, get_invoice_apply_eligibility, get_my_invoice,
    get_seller_config, issue_invoice, list_invoices, list_my_invoices, mark_paid, update_invoice,
    upsert_seller_config, void_invoice,
};
use crate::provider_handlers::list_payment_providers;
use crate::purchase_handlers::{
    cancel_payment_attempt, create_payment_attempt, fulfill_payment, get_payment_attempt_status,
    get_purchase_history,
};
use crate::stripe_webhook_handlers::handle_stripe_webhook;
use crate::webhook_handlers::handle_creem_webhook;
use herald_api_base::application::http::state::AppState;
use herald_infra_shopify::constant_time_compare;

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
        // ===== Credit Bucket Directory =====
        // NOTE: `/overview` is registered BEFORE `/{bucketId}` so the static
        // segment is matched unambiguously (Axum matchit prefers static over
        // dynamic, but explicit ordering keeps the intent legible).
        .route(
            "/api/realms/{realmId}/billing/credit-buckets",
            get(list_credit_buckets_handler).post(create_credit_bucket_handler),
        )
        .route(
            "/api/realms/{realmId}/billing/credit-buckets/overview",
            get(get_bucket_overview_handler),
        )
        .route(
            "/api/realms/{realmId}/billing/credit-buckets/{bucketId}",
            get(get_credit_bucket_handler)
                .put(update_credit_bucket_handler)
                .delete(delete_credit_bucket_handler),
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
        // Static `batch` segment is registered BEFORE `/{mappingId}` so it is
        // matched unambiguously (same convention as `/overview` above).
        .route(
            "/api/bill/{realmId}/entitlement-mappings/batch",
            put(batch_update_entitlement_mappings),
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
            "/api/bill/{realmId}/client/{clientAppId}/purchase-options",
            get(list_purchase_options),
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
        .route(
            "/api/bill/{realmId}/invoices/{invoiceId}/credit-notes",
            post(create_credit_note),
        )
        // ===== User Invoice (My Invoices) =====
        .route(
            "/api/bill/{realmId}/my/invoices/apply-eligibility",
            get(get_invoice_apply_eligibility),
        )
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
}
