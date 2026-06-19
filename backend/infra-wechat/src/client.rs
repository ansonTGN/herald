//! WeChat Pay client wrapper
//!
//! Wraps the `wechat-pay-rust-sdk` to provide a clean interface for
//! Native QR code payment, order querying, and order closing.

use chrono::Duration;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use wechat_pay_rust_sdk::model::NativeParams;
use wechat_pay_rust_sdk::pay::WechatPay;

use crate::models::{
    CreateOrderParams, CreateOrderResult, QueryOrderResult, WechatPayClientConfig,
    generate_out_trade_no,
};
use herald_domain::common::entities::app_errors::CoreError;

/// WeChat Pay client wrapping the SDK
///
/// Note: The wechat-pay-rust-sdk (v0.2) only provides native_pay method.
/// query_order and close_order need to be implemented via direct HTTP requests
/// to WeChat Pay API v3.
pub struct WechatPayClient {
    /// WechatPay SDK instance. Constructed inside `spawn_blocking` because
    /// `WechatPay::new()` builds its own tokio runtime (see `new`/`new_async`).
    wechat_pay: Option<WechatPay>,
    app_id: String,
    mch_id: String,
    serial_no: String,
    base_url: String,
}

impl WechatPayClient {
    /// Create a new WeChat Pay client
    ///
    /// # Arguments
    /// * `app_id` - WeChat AppID (e.g., wx...)
    /// * `mch_id` - Merchant ID (numeric string)
    /// * `private_key` - Merchant RSA private key in PEM format
    /// * `serial_no` - Merchant certificate serial number
    /// * `v3_key` - API v3 key (32 characters)
    /// * `notify_url` - Webhook callback URL (must be HTTPS)
    ///
    /// # Note
    /// This function must be called from a blocking context (e.g., via `spawn_blocking`)
    /// because `WechatPay::new()` creates a runtime internally.
    pub fn new(
        app_id: &str,
        mch_id: &str,
        private_key: &str,
        serial_no: &str,
        v3_key: &str,
        notify_url: &str,
    ) -> Result<Self, CoreError> {
        // Basic PEM format validation
        if !private_key.contains("-----BEGIN") {
            return Err(CoreError::BadRequest(
                "Invalid private key: must be in PEM format (expected -----BEGIN header)"
                    .to_string(),
            ));
        }

        let wechat_pay = WechatPay::new(app_id, mch_id, private_key, serial_no, v3_key, notify_url);

        Ok(Self {
            wechat_pay: Some(wechat_pay),
            app_id: app_id.to_string(),
            mch_id: mch_id.to_string(),
            serial_no: serial_no.to_string(),
            base_url: "https://api.mch.weixin.qq.com".to_string(),
        })
    }

    /// Create a new WeChat Pay client (async version)
    ///
    /// This is the async-safe version that creates the client in a blocking task
    /// to avoid runtime drop issues.
    pub async fn new_async(
        app_id: String,
        mch_id: String,
        private_key: String,
        serial_no: String,
        v3_key: String,
        notify_url: String,
    ) -> Result<Self, CoreError> {
        use tokio::task::spawn_blocking;

        let client = spawn_blocking(move || {
            Self::new(
                &app_id,
                &mch_id,
                &private_key,
                &serial_no,
                &v3_key,
                &notify_url,
            )
        })
        .await
        .map_err(|e| {
            CoreError::InternalServerError(format!("Failed to create WeChat Pay client: {}", e))
        })??;

        Ok(client)
    }

    /// Create a new WeChat Pay client from configuration struct (async version)
    ///
    /// This is the preferred method for creating clients from configuration
    pub async fn from_config(config: &WechatPayClientConfig) -> Result<Self, CoreError> {
        Self::new_async(
            config.app_id.clone(),
            config.mch_id.clone(),
            config.private_key.clone(),
            config.serial_no.clone(),
            config.v3_key.clone(),
            config.notify_url.clone(),
        )
        .await
    }

    /// Create a Native (QR code) payment order
    ///
    /// Calls WeChat's unified order API to create a Native payment order
    /// and returns the code_url for QR code rendering.
    ///
    /// This method is synchronous (not async) as the SDK's native_pay is synchronous.
    pub fn create_native_order(
        &self,
        params: &CreateOrderParams,
    ) -> Result<CreateOrderResult, CoreError> {
        let out_trade_no = generate_out_trade_no(&params.realm_id);
        let order_id = Uuid::now_v7();
        let expires_at = Utc::now() + Duration::hours(2);

        // SDK v0.2 native_pay takes 3 arguments: description, out_trade_no, amount
        // notify_url is already configured in WechatPay::new()
        let native_params = NativeParams::new(
            params.description.clone(),
            out_trade_no.clone(),
            params.amount.into(),
        );

        // native_pay is synchronous, not async
        let wechat_pay = self.wechat_pay.as_ref().ok_or_else(|| {
            CoreError::InternalServerError("WeChat Pay SDK not available".to_string())
        })?;

        let response = wechat_pay.native_pay(native_params).map_err(|e| {
            tracing::error!(
                out_trade_no = %out_trade_no,
                error = %e,
                "WeChat native_pay API call failed"
            );
            CoreError::InternalServerError(format!("WeChat Pay API error: {}", e))
        })?;

        let code_url = response.code_url.ok_or_else(|| {
            CoreError::InternalServerError("WeChat Pay API returned no code_url".to_string())
        })?;

        tracing::info!(
            out_trade_no = %out_trade_no,
            order_id = %order_id,
            "WeChat native order created successfully"
        );

        Ok(CreateOrderResult {
            order_id,
            out_trade_no,
            code_url,
            expires_at,
        })
    }

    /// Query order status from WeChat API
    ///
    /// Queries the order status by merchant order number (out_trade_no).
    /// Uses direct HTTP request to WeChat Pay API v3 since SDK v0.2 doesn't support it.
    pub async fn query_order(&self, out_trade_no: &str) -> Result<QueryOrderResult, CoreError> {
        use reqwest::Client;

        let client = Client::new();
        let url = format!(
            "{}/v3/pay/transactions/out-trade-no/{}?mchid={}",
            self.base_url, out_trade_no, self.mch_id
        );

        let response = client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| {
                tracing::error!(
                    out_trade_no = %out_trade_no,
                    error = %e,
                    "HTTP request to WeChat query_order API failed"
                );
                CoreError::InternalServerError(format!("HTTP client error: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            tracing::error!(
                out_trade_no = %out_trade_no,
                status = %status,
                error = %error_text,
                "WeChat query_order API returned error"
            );
            return Err(CoreError::InternalServerError(format!(
                "WeChat API error {}: {}",
                status, error_text
            )));
        }

        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct WechatQueryResponse {
            trade_state: String,
            transaction_id: Option<String>,
            amount: Option<QueryAmount>,
        }

        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct QueryAmount {
            total: i32,
        }

        let wechat_response: WechatQueryResponse = response.json().await.map_err(|e| {
            tracing::error!(
                out_trade_no = %out_trade_no,
                error = %e,
                "Failed to parse WeChat query_order response"
            );
            CoreError::InternalServerError(format!("Failed to parse response: {}", e))
        })?;

        tracing::debug!(
            out_trade_no = %out_trade_no,
            trade_state = %wechat_response.trade_state,
            "WeChat order status queried successfully"
        );

        Ok(QueryOrderResult {
            trade_state: wechat_response.trade_state,
            transaction_id: wechat_response.transaction_id,
            amount: wechat_response.amount.map(|a| a.total),
        })
    }

    /// Close an order on WeChat side
    ///
    /// After closing, the QR code will no longer be scannable.
    /// Uses direct HTTP request to WeChat Pay API v3 since SDK v0.2 doesn't support it.
    pub async fn close_order(&self, out_trade_no: &str) -> Result<(), CoreError> {
        use reqwest::Client;

        let client = Client::new();
        let url = format!(
            "{}/v3/pay/transactions/out-trade-no/{}/close",
            self.base_url, out_trade_no
        );

        #[derive(Debug, Serialize)]
        #[serde(rename_all = "camelCase")]
        struct CloseOrderRequest {
            mchid: String,
        }

        let body = CloseOrderRequest {
            mchid: self.mch_id.clone(),
        };

        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                tracing::error!(
                    out_trade_no = %out_trade_no,
                    error = %e,
                    "HTTP request to WeChat close_order API failed"
                );
                CoreError::InternalServerError(format!("HTTP client error: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            tracing::error!(
                out_trade_no = %out_trade_no,
                status = %status,
                error = %error_text,
                "WeChat close_order API returned error"
            );
            return Err(CoreError::InternalServerError(format!(
                "WeChat API error {}: {}",
                status, error_text
            )));
        }

        tracing::info!(
            out_trade_no = %out_trade_no,
            "WeChat order closed successfully"
        );

        Ok(())
    }

    /// Get the app_id
    pub fn app_id(&self) -> &str {
        &self.app_id
    }

    /// Get the mch_id
    pub fn mch_id(&self) -> &str {
        &self.mch_id
    }

    /// Get the serial_no
    pub fn serial_no(&self) -> &str {
        &self.serial_no
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_client_new_validates_pem_format() {
        let result = WechatPayClient::new(
            "wx123",
            "1234567890",
            "not-a-pem-key",
            "serial123",
            "0123456789abcdef0123456789abcdef",
            "https://example.com/webhooks",
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_client_new_accepts_valid_pem() {
        let pem_key = "-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASC\ntest_content\n-----END PRIVATE KEY-----";
        let result = WechatPayClient::new(
            "wx123",
            "1234567890",
            pem_key,
            "serial123",
            "0123456789abcdef0123456789abcdef",
            "https://example.com/webhooks",
        );
        assert!(result.is_ok());
    }
}
