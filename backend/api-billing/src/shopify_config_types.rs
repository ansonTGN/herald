//! Shopify configuration management types (copied from herald-api for api-billing crate)
//! No changes needed - no crate:: imports

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// Shopify configuration request (create/update)
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ShopifyConfigRequest {
    /// Shop domain (e.g., demo-store.myshopify.com)
    #[validate(length(min = 1, message = "Shop domain is required"))]
    #[schema(example = "demo-store.myshopify.com")]
    pub shop_domain: String,

    /// Admin API access token (shpat_*)
    #[validate(length(min = 1, message = "Admin access token is required"))]
    #[schema(example = "shpat_xxxxxxxxxxxxxxxxxxxxxxxxxxxx")]
    pub admin_access_token: String,

    /// Storefront API access token (shp_*)
    #[validate(length(min = 1, message = "Storefront access token is required"))]
    #[schema(example = "shp_xxxxxxxxxxxxxxxxxxxxxxxxxxxx")]
    pub storefront_access_token: String,

    /// App client secret for webhook HMAC verification
    #[validate(length(min = 32, message = "Client secret must be at least 32 characters"))]
    #[schema(example = "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx")]
    pub app_client_secret: String,

    /// Shopify API version (optional, defaults to 2024-01)
    #[serde(default = "default_api_version")]
    #[schema(example = "2024-01")]
    pub api_version: String,

    /// Webhook subscription mode (optional, defaults to admin_api)
    #[serde(default = "default_webhook_mode")]
    #[schema(example = "admin_api")]
    pub webhook_subscription_mode: String,

    /// HTTP timeout in seconds (optional, defaults to 30)
    #[serde(default = "default_timeout")]
    #[validate(range(
        min = 5,
        max = 120,
        message = "Timeout must be between 5 and 120 seconds"
    ))]
    #[schema(example = 30)]
    pub timeout: u32,

    /// Skip connection test (for demo/test environments)
    #[serde(default = "default_skip_connection_test")]
    #[schema(example = false)]
    pub skip_connection_test: bool,
}

/// Shopify configuration update request (secret fields optional — omit to keep existing values)
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ShopifyConfigUpdateRequest {
    /// Shop domain (e.g., demo-store.myshopify.com)
    #[validate(length(min = 1, message = "Shop domain is required"))]
    #[schema(example = "demo-store.myshopify.com")]
    pub shop_domain: Option<String>,

    /// Admin API access token (shpat_*) — omit to keep existing
    pub admin_access_token: Option<String>,

    /// Storefront API access token (shp_*) — omit to keep existing
    pub storefront_access_token: Option<String>,

    /// App client secret for webhook HMAC verification — omit to keep existing
    pub app_client_secret: Option<String>,

    /// Shopify API version (optional, defaults to 2024-01)
    pub api_version: Option<String>,

    /// Webhook subscription mode (optional, defaults to admin_api)
    pub webhook_subscription_mode: Option<String>,

    /// HTTP timeout in seconds (optional, defaults to 30)
    pub timeout: Option<u32>,

    /// Skip connection test (for demo/test environments)
    pub skip_connection_test: Option<bool>,
}

fn default_api_version() -> String {
    "2024-01".to_string()
}

fn default_webhook_mode() -> String {
    "admin_api".to_string()
}

fn default_timeout() -> u32 {
    30
}

fn default_skip_connection_test() -> bool {
    false
}

/// Shopify configuration response
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ShopifyConfigResponse {
    pub platform: String,
    pub shop_domain: String,
    pub admin_access_token: String,
    pub storefront_access_token: String,
    pub app_client_secret: String,
    pub api_version: String,
    pub webhook_subscription_mode: String,
    pub timeout: u32,
    pub webhook_endpoint: String,
    pub created_at: String,
    pub last_updated: String,
    pub enabled: bool,
}

/// Payment provider list response
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct PaymentProvidersResponse {
    pub providers: Vec<PaymentProviderInfo>,
}

/// Payment provider information
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaymentProviderInfo {
    pub platform: String,
    pub shop_domain: Option<String>,
    pub api_version: Option<String>,
    pub webhook_endpoint: Option<String>,
    pub last_updated: Option<String>,
    pub enabled: bool,
}

/// Test connection request
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct TestConnectionRequest {
    #[validate(length(min = 1, message = "Shop domain is required"))]
    pub shop_domain: String,
    #[validate(length(min = 1, message = "Admin access token is required"))]
    pub admin_access_token: String,
    #[validate(length(min = 1, message = "Storefront access token is required"))]
    pub storefront_access_token: String,
}

/// Test connection response
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct TestConnectionResponse {
    pub success: bool,
    #[serde(default)]
    pub results: TestConnectionResults,
    #[serde(default)]
    pub errors: Vec<String>,
}

/// Detailed test connection results
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct TestConnectionResults {
    #[serde(default)]
    pub admin_api_connection: String,
    #[serde(default)]
    pub storefront_api_connection: String,
    #[serde(default)]
    pub shop_access: String,
}

/// Error response for validation failures
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ValidationErrorResponse {
    pub error: String,
    pub details: Vec<ValidationErrorDetail>,
}

/// Individual validation error
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ValidationErrorDetail {
    pub field: String,
    pub message: String,
}

/// Generic error response
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct GenericErrorResponse {
    pub error: String,
    pub message: String,
}

pub fn validate_request<T: Validate>(req: &T) -> Result<(), ValidationErrorResponse> {
    if let Err(errors) = req.validate() {
        let details: Vec<ValidationErrorDetail> = errors
            .field_errors()
            .iter()
            .flat_map(|(field, errs)| {
                errs.iter().map(move |error| ValidationErrorDetail {
                    field: field.to_string(),
                    message: error.code.to_string(),
                })
            })
            .collect();
        Err(ValidationErrorResponse {
            error: "validation_failed".to_string(),
            details,
        })
    } else {
        Ok(())
    }
}
