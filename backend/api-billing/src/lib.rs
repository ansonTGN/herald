// Herald API Billing Module
// Handles billing, subscriptions, payments, webhooks

pub mod compensation;
pub(crate) mod webhook_common;
pub(crate) mod webhook_subscription_helpers;
mod webhooks;

pub mod entitlement_mapping_handlers;
pub mod feature_availability;
pub mod handlers;
pub mod handlers_history;
pub mod invoice_handlers;
pub mod invoice_types;
mod payment_email;
pub mod purchase_handlers;
pub mod routes;
pub mod shopify_claim_handlers;
pub mod shopify_claim_types;
pub mod shopify_config_handlers;
pub mod shopify_config_types;
pub mod shopify_webhook_handlers;
pub mod shopify_webhook_utils;
pub mod stripe_webhook_handlers;
pub mod types;
pub mod types_history;
pub mod webhook_handlers;
pub mod wechat_config_handlers;
pub mod wechat_config_types;
pub mod wechat_order_handlers;
pub mod wechat_webhook_handlers;

pub mod shopify_test_handlers;

pub mod shopify_test_types;

/// OpenAPI specification for billing module
#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
        // Entitlement Mapping handlers
        crate::entitlement_mapping_handlers::list_entitlement_mappings,
        crate::entitlement_mapping_handlers::get_entitlement_mapping,
        crate::entitlement_mapping_handlers::update_entitlement_mapping,
        crate::entitlement_mapping_handlers::sync_provider_products,
        crate::entitlement_mapping_handlers::list_one_time_mappings,
        // Subscription handlers
        crate::handlers::list_subscriptions,
        crate::handlers::get_subscription,
        crate::handlers::get_subscription_for_client_app,
        crate::handlers::cancel_subscription_for_client_app,
        crate::handlers::create_checkout_session,
        crate::handlers_history::get_subscription_history,
        crate::handlers_history::list_subscription_history,
        crate::handlers_history::get_my_subscription_history,
        crate::handlers_history::list_my_subscription_history,
        crate::feature_availability::get_feature_availability,
        crate::shopify_claim_handlers::claim_shopify_subscriptions,
        crate::shopify_config_handlers::list_payment_providers,
        crate::shopify_config_handlers::create_shopify_config,
        crate::shopify_config_handlers::get_shopify_config,
        crate::shopify_config_handlers::update_shopify_config,
        crate::shopify_config_handlers::delete_shopify_config,
        crate::shopify_config_handlers::test_shopify_connection_endpoint,
        crate::wechat_config_handlers::create_wechat_config,
        crate::wechat_config_handlers::get_wechat_config,
        crate::wechat_config_handlers::update_wechat_config,
        crate::wechat_config_handlers::delete_wechat_config,
        crate::wechat_order_handlers::create_wechat_order,
        crate::wechat_order_handlers::get_wechat_order_status,
        crate::wechat_order_handlers::close_wechat_order,
        crate::purchase_handlers::create_payment_attempt,
        crate::purchase_handlers::get_payment_attempt_status,
        crate::purchase_handlers::cancel_payment_attempt,
        crate::purchase_handlers::fulfill_payment,
        crate::purchase_handlers::get_purchase_history,
        // Invoice handlers
        crate::invoice_handlers::get_seller_config,
        crate::invoice_handlers::upsert_seller_config,
        crate::invoice_handlers::create_invoice,
        crate::invoice_handlers::list_invoices,
        crate::invoice_handlers::get_invoice,
        crate::invoice_handlers::update_invoice,
        crate::invoice_handlers::issue_invoice,
        crate::invoice_handlers::void_invoice,
        crate::invoice_handlers::mark_paid,
        // User invoice handlers
        crate::invoice_handlers::apply_invoice,
        crate::invoice_handlers::list_my_invoices,
        crate::invoice_handlers::get_my_invoice,
        // PDF download handlers
        crate::invoice_handlers::download_invoice_pdf,
        crate::invoice_handlers::download_my_invoice_pdf,
    ),
    components(schemas(
        // Entitlement Mapping types
        crate::types::EntitlementMappingResponse,
        crate::types::EntitlementMappingListResponse,
        crate::types::EntitlementMappingQuery,
        crate::types::UpdateEntitlementMappingRequest,
        crate::types::SyncProviderRequest,
        crate::types::SyncProviderResponse,
        crate::types::PartialSyncErrorDto,
        crate::types::OneTimeMappingItem,
        crate::types::OneTimeMappingListResponse,
        // Subscription types
        crate::types::SubscriptionDetailResponse,
        crate::types::SubscriptionListItemResponse,
        crate::types::SubscriptionListResponse,
        crate::types::SubscriptionListQuery,
        crate::types::CancelSubscriptionRequest,
        crate::types::CancelSubscriptionResponse,
        crate::types::CreateCheckoutSessionRequest,
        crate::types::CreateCheckoutResponse,
        crate::types_history::SubscriptionHistoryEventResponse,
        crate::types_history::SubscriptionHistoryResponse,
        crate::types_history::SubscriptionHistoryListResponse,
        crate::types_history::SubscriptionHistoryEventWithUser,
        crate::types_history::SubscriptionSummary,
        crate::types_history::UserInfo,
        // Shopify types
        crate::shopify_claim_types::ShopifyClaimRequest,
        crate::shopify_claim_types::ShopifyClaimResponse,
        crate::shopify_config_types::ShopifyConfigRequest,
        crate::shopify_config_types::ShopifyConfigUpdateRequest,
        crate::shopify_config_types::ShopifyConfigResponse,
        crate::shopify_config_types::PaymentProvidersResponse,
        crate::shopify_config_types::PaymentProviderInfo,
        crate::shopify_config_types::TestConnectionRequest,
        crate::shopify_config_types::TestConnectionResponse,
        crate::shopify_config_types::TestConnectionResults,
        crate::shopify_config_types::ValidationErrorResponse,
        crate::shopify_config_types::ValidationErrorDetail,
        crate::shopify_config_types::GenericErrorResponse,
        crate::wechat_config_types::WechatConfigRequest,
        crate::wechat_config_types::WechatConfigUpdateRequest,
        crate::wechat_config_types::WechatConfigResponse,
        crate::wechat_config_types::WechatOrderCreateRequest,
        crate::wechat_config_types::WechatOrderCreateResponse,
        crate::wechat_config_types::WechatOrderStatusResponse,
        // Feature availability
        crate::feature_availability::FeatureAvailabilityResponse,
        crate::feature_availability::AdminFeatureAvailability,
        crate::feature_availability::UserFeatureAvailability,
        crate::feature_availability::FeatureAvailabilityFacts,
        crate::purchase_handlers::CreatePaymentAttemptRequest,
        crate::purchase_handlers::CreatePaymentAttemptResponse,
        crate::purchase_handlers::PaymentContextDto,
        crate::purchase_handlers::PaymentAttemptStatusResponse,
        crate::purchase_handlers::FulfillmentResultDto,
        crate::purchase_handlers::FulfillPaymentResponse,
        crate::purchase_handlers::PurchaseHistoryResponse,
        crate::purchase_handlers::PurchaseHistoryItemDto,
        crate::purchase_handlers::FulfillPaymentRequest,
        // Invoice types
        crate::invoice_types::SellerConfigRequest,
        crate::invoice_types::SellerConfigResponse,
        crate::invoice_types::LineItemRequest,
        crate::invoice_types::InvoiceLineItemResponse,
        crate::invoice_types::InvoiceHistoryResponse,
        crate::invoice_types::CreateInvoiceRequest,
        crate::invoice_types::UpdateInvoiceRequest,
        crate::invoice_types::IssueInvoiceRequest,
        crate::invoice_types::VoidInvoiceRequest,
        crate::invoice_types::MarkPaidRequest,
        crate::invoice_types::ApplyInvoiceRequest,
        crate::invoice_types::InvoiceListQuery,
        crate::invoice_types::InvoiceResponse,
        crate::invoice_types::InvoiceDetailResponse,
        crate::invoice_types::InvoiceListResponse,
    ))
)]
pub struct ApiDoc;

// Re-export handler functions for external use
pub use routes::{billing_public_routes, billing_routes};

pub use routes::billing_test_routes;

// Re-export compensation processor for assembly in main.rs
pub use compensation::WebhookEventProcessorImpl;
