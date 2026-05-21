// Herald API Billing Module
// Handles billing, subscriptions, payments, webhooks

pub(crate) mod webhook_common;
pub(crate) mod webhook_subscription_helpers;
mod webhooks;

pub mod feature_availability;
pub mod handlers;
pub mod handlers_history;
pub mod invoice_handlers;
pub mod invoice_types;
mod payment_email;
pub mod points_package_handlers;
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

#[cfg(test)]
mod handlers_test;

/// OpenAPI specification for billing module
#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
        crate::handlers::list_plans,
        crate::handlers::create_plan,
        crate::handlers::get_plan,
        crate::handlers::update_plan,
        crate::handlers::delete_plan,
        crate::handlers::list_products,
        crate::handlers::create_product,
        crate::handlers::get_product,
        crate::handlers::update_product,
        crate::handlers::delete_product,
        crate::handlers::get_product_plans,
        crate::handlers::assign_plan_to_client_app,
        crate::handlers::list_plan_assignments,
        crate::handlers::list_plan_assignments_batch,
        crate::handlers::toggle_plan_assignment,
        crate::handlers::remove_plan_assignment,
        crate::handlers::get_subscription_for_client_app,
        crate::handlers::cancel_subscription_for_client_app,
        crate::handlers::create_checkout_session,
        crate::handlers::list_plan_payment_providers,
        crate::handlers::add_payment_provider_to_plan,
        crate::handlers::update_plan_payment_provider,
        crate::handlers::toggle_plan_payment_provider,
        crate::handlers::remove_payment_provider_from_plan,
        crate::handlers::webhook_handler,
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
        crate::points_package_handlers::create_points_package,
        crate::points_package_handlers::list_points_packages,
        crate::points_package_handlers::get_points_package,
        crate::points_package_handlers::update_points_package,
        crate::points_package_handlers::delete_points_package,
        crate::points_package_handlers::list_payment_provider_mappings,
        crate::points_package_handlers::create_payment_provider_mapping,
        crate::points_package_handlers::update_payment_provider_mapping,
        crate::points_package_handlers::delete_payment_provider_mapping,
        crate::purchase_handlers::create_payment_attempt,
        crate::purchase_handlers::get_payment_attempt_status,
        crate::purchase_handlers::cancel_payment_attempt,
        crate::purchase_handlers::fulfill_payment,
        crate::purchase_handlers::get_points_package_purchase_history,
        crate::purchase_handlers::get_points_package_purchase_details,
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
        crate::types::CreateSubscriptionPlanRequest,
        crate::types::UpdateSubscriptionPlanRequest,
        crate::types::SubscriptionPlanResponse,
        crate::types::ListSubscriptionPlansResponse,
        crate::types::SubscriptionPlanAssignmentRequest,
        crate::types::SubscriptionPlanAssignmentResponse,
        crate::types::ListSubscriptionPlanAssignmentsResponse,
        crate::types::ToggleSubscriptionPlanAssignmentRequest,
        crate::types::SubscriptionDetailResponse,
        crate::types::CancelSubscriptionRequest,
        crate::types::CancelSubscriptionResponse,
        crate::types::CreateCheckoutSessionRequest,
        crate::types::CreateCheckoutResponse,
        crate::shopify_claim_types::ShopifyClaimRequest,
        crate::shopify_claim_types::ShopifyClaimResponse,
        crate::types::CreateProductRequest,
        crate::types::UpdateProductRequest,
        crate::types::ProductResponse,
        crate::types::ListProductsResponse,
        crate::types::ProductDetailResponse,
        crate::types::SubscriptionPlanSummaryForProduct,
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
        crate::types::CreateSubscriptionPlanPaymentProviderRequest,
        crate::feature_availability::FeatureAvailabilityResponse,
        crate::feature_availability::AdminFeatureAvailability,
        crate::feature_availability::UserFeatureAvailability,
        crate::feature_availability::FeatureAvailabilityFacts,
        crate::types::SubscriptionPlanPaymentProviderResponse,
        crate::types::UpdateSubscriptionPlanPaymentProviderRequest,
        crate::types::ToggleSubscriptionPlanPaymentProviderRequest,
        crate::points_package_handlers::CreatePointsPackageRequest,
        crate::points_package_handlers::PointsPackageResponse,
        crate::points_package_handlers::ListPointsPackagesResponse,
        crate::points_package_handlers::UpdatePointsPackageRequest,
        crate::points_package_handlers::CreatePaymentProviderMappingRequest,
        crate::points_package_handlers::PaymentProviderMappingResponse,
        crate::points_package_handlers::ListPaymentProviderMappingsResponse,
        crate::points_package_handlers::UpdatePaymentProviderMappingRequest,
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
pub use handlers::subscription_plan_to_response;
pub use routes::{billing_public_routes, billing_routes};

pub use routes::billing_test_routes;
