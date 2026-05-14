//! WeChat Pay data models
//!
//! Defines structures for WeChat payment order tracking,
//! order status enumeration, and order creation/query parameters.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// WeChat payment order record (mirrors `wechat_payment_order` table)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WechatPaymentOrder {
    pub id: Uuid,
    pub realm_id: String,
    pub user_id: Uuid,
    pub plan_id: Uuid,
    pub client_app_id: Option<Uuid>,
    pub out_trade_no: String,
    pub transaction_id: Option<String>,
    pub amount: i32,
    pub currency: String,
    pub code_url: String,
    pub status: WechatOrderStatus,
    pub description: Option<String>,
    pub paid_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// WeChat order status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WechatOrderStatus {
    Pending,
    Paid,
    Closed,
    Expired,
}

impl WechatOrderStatus {
    /// Returns the string representation of the order status
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Paid => "paid",
            Self::Closed => "closed",
            Self::Expired => "expired",
        }
    }
}

impl std::fmt::Display for WechatOrderStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for WechatOrderStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "paid" => Ok(Self::Paid),
            "closed" => Ok(Self::Closed),
            "expired" => Ok(Self::Expired),
            other => Err(format!("Unknown wechat order status: {}", other)),
        }
    }
}

/// Input parameters for creating a Native pay order
#[derive(Debug, Clone)]
pub struct CreateOrderParams {
    pub realm_id: String,
    pub user_id: Uuid,
    pub plan_id: Uuid,
    pub client_app_id: Option<Uuid>,
    pub amount: i32,
    pub currency: String,
    pub description: String,
    pub notify_url: String,
}

/// Output from order creation
#[derive(Debug, Clone)]
pub struct CreateOrderResult {
    pub order_id: Uuid,
    pub out_trade_no: String,
    pub code_url: String,
    pub expires_at: DateTime<Utc>,
}

/// Output from querying WeChat order status
#[derive(Debug, Clone)]
pub struct QueryOrderResult {
    pub trade_state: String,
    pub transaction_id: Option<String>,
}

/// WeChat configuration stored in realm_config
#[derive(Debug, Clone)]
pub struct WechatConfig {
    pub app_id: String,
    pub mch_id: String,
    pub private_key: String,
    pub serial_no: String,
    pub v3_key: String,
    pub notify_url: String,
    pub mock_base_url: Option<String>,
}

/// Configuration for creating WeChat Pay client
#[derive(Debug, Clone)]
pub struct WechatPayClientConfig {
    pub app_id: String,
    pub mch_id: String,
    pub private_key: String,
    pub serial_no: String,
    pub v3_key: String,
    pub notify_url: String,
    pub mock_base_url: Option<String>,
}

impl WechatPayClientConfig {
    /// Create from WechatConfig
    pub fn from_wechat_config(config: &WechatConfig) -> Self {
        Self {
            app_id: config.app_id.clone(),
            mch_id: config.mch_id.clone(),
            private_key: config.private_key.clone(),
            serial_no: config.serial_no.clone(),
            v3_key: config.v3_key.clone(),
            notify_url: config.notify_url.clone(),
            mock_base_url: config.mock_base_url.clone(),
        }
    }
}

/// Input for creating a new WeChat configuration
#[derive(Debug, Clone)]
pub struct WechatConfigCreate {
    pub app_id: String,
    pub mch_id: String,
    pub private_key: String,
    pub serial_no: String,
    pub v3_key: String,
    pub notify_url: String,
}

/// Input for updating an existing WeChat configuration (partial update)
#[derive(Debug, Clone, Default)]
pub struct WechatConfigUpdate {
    pub app_id: Option<String>,
    pub mch_id: Option<String>,
    pub private_key: Option<String>,
    pub serial_no: Option<String>,
    pub v3_key: Option<String>,
    pub notify_url: Option<String>,
}

/// Generate a unique merchant order number (out_trade_no) for WeChat Pay
///
/// Format: `CAS_{realm_prefix}_{compact_uuid}`
/// - Total length must be ≤ 32 characters per WeChat Pay requirements
/// - Uses UUID v7 (project standard)
pub fn generate_out_trade_no(realm_id: &str) -> String {
    // Take first 4 characters of realm_id to leave room for UUID
    let prefix = &realm_id[..4.min(realm_id.len())];

    let uuid_str = Uuid::now_v7().to_string().replace('-', "");

    // Calculate max characters we can use: 32 - "CAS_" (4) - prefix (4) - "_" (1) - "_" (1) = 22
    let max_uuid_chars = 22.min(uuid_str.len());
    let compact_uuid = &uuid_str[..max_uuid_chars];

    format!("CAS_{}_{}", prefix, compact_uuid)
}
