//! WeChat Pay configuration request/response types
//!
//! DTOs for WeChat payment provider configuration CRUD API.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// WeChat configuration create request
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct WechatConfigRequest {
    /// WeChat AppID (must start with "wx")
    #[validate(length(min = 1, message = "App ID is required"))]
    #[schema(example = "wx1234567890abcdef")]
    pub app_id: String,

    /// Merchant ID (numeric only)
    #[validate(length(min = 1, message = "Merchant ID is required"))]
    #[schema(example = "1234567890")]
    pub mch_id: String,

    /// Merchant RSA private key (PEM format)
    #[validate(length(min = 1, message = "Private key is required"))]
    #[schema(example = "-----BEGIN PRIVATE KEY-----...")]
    pub private_key: String,

    /// Merchant certificate serial number
    #[validate(length(min = 1, message = "Serial number is required"))]
    #[schema(example = "1A2B3C4D5E6F")]
    pub serial_no: String,

    /// API v3 key (exactly 32 characters)
    #[validate(length(min = 32, max = 32, message = "V3 key must be exactly 32 characters"))]
    #[schema(example = "0123456789abcdef0123456789abcdef")]
    pub v3_key: String,

    /// Webhook callback URL (must be HTTPS)
    #[validate(length(min = 1, message = "Notify URL is required"))]
    #[validate(url(message = "Notify URL must be a valid URL"))]
    #[schema(example = "https://api.example.com/wechat/webhooks")]
    pub notify_url: String,
}

/// WeChat configuration update request (all fields optional)
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct WechatConfigUpdateRequest {
    pub app_id: Option<String>,
    pub mch_id: Option<String>,
    pub private_key: Option<String>,
    pub serial_no: Option<String>,
    #[validate(length(min = 32, max = 32, message = "V3 key must be exactly 32 characters"))]
    pub v3_key: Option<String>,
    #[validate(url(message = "Notify URL must be a valid URL"))]
    pub notify_url: Option<String>,
}

/// Query parameters for retrieving WeChat configuration
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WechatConfigQuery {
    #[serde(default, alias = "reveal_secrets")]
    pub reveal_secrets: bool,
}

/// WeChat configuration response (masked sensitive fields)
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WechatConfigResponse {
    pub platform: String,
    pub app_id: String,
    pub mch_id: String,
    pub serial_no: String,
    #[schema(example = "0123*******************")]
    pub v3_key: String,
    #[schema(example = "*********** (configured)")]
    pub private_key: String,
    #[schema(example = "https://api.example.com/wechat/webhooks")]
    pub notify_url: String,
    #[schema(example = "2026-04-04T12:00:00Z")]
    pub created_at: String,
    #[schema(example = "2026-04-04T12:00:00Z")]
    pub updated_at: String,
}

/// WeChat order creation request
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct WechatOrderCreateRequest {
    #[validate(length(min = 1, message = "Plan ID is required"))]
    pub plan_id: String,
    pub client_app_id: Option<String>,
}

/// WeChat order creation response
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WechatOrderCreateResponse {
    pub order_id: String,
    pub out_trade_no: String,
    pub code_url: String,
    pub expires_at: String,
}

/// WeChat order status response
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WechatOrderStatusResponse {
    pub order_id: String,
    pub status: String,
    pub trade_state: Option<String>,
}
