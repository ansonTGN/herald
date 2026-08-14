//! WeChat Pay v3 protocol models: merchant config, order scene/results,
//! platform certificate, callback notification shapes, and `out_trade_no`
//! generation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Merchant credentials for one realm, loaded from `realm_config`
/// (`config_type='wechat'`). Secrets (`private_key_pem`, `api_v3_key`) are
/// stored plaintext with `is_secret=true` (DEC-wechat-support-007).
#[derive(Debug, Clone)]
pub struct WechatPayConfig {
    pub app_id: String,
    pub mch_id: String,
    /// Merchant RSA private key (PKCS#8 PEM), used for request signing and
    /// the JSAPI `paySign`.
    pub private_key_pem: String,
    /// Merchant certificate serial number (identifies the private key to
    /// WeChat; sent in the `Authorization` header).
    pub serial_no: String,
    /// APIv3 Key (32 bytes), used to decrypt platform certificates and
    /// callback resources (AES-256-GCM).
    pub api_v3_key: String,
    pub notify_url: String,
    /// Optional manual platform public-key override (PEM). When present it is
    /// preferred over the auto-downloaded platform certificate for callback
    /// signature verification.
    pub platform_public_key_override: Option<String>,
    /// Optional sandbox / alternate base URL (e.g. `https://api.mch.weixin.qq.com`).
    pub base_url: Option<String>,
}

/// Which checkout scene to drive with `create_order`.
#[derive(Debug, Clone)]
pub enum CreateOrderScene {
    /// PC scan-to-pay; returns a `code_url` rendered as a QR code.
    Native,
    /// In-WeChat-browser payment; requires the user's `openid` (obtained via
    /// the existing WeChat OAuth login flow — DEC-wechat-support-009).
    Jsapi { openid: String },
}

/// Parameters returned to the browser for the JSAPI
/// `WeixinJSBridge.invoke('getBrandWCPayRequest', ...)` call.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct JsapiParams {
    pub app_id: String,
    pub time_stamp: String,
    pub nonce_str: String,
    /// `prepay_id=...`
    pub package: String,
    pub sign_type: String,
    pub pay_sign: String,
}

/// Output of `create_order`, branching on the requested scene.
#[derive(Debug, Clone)]
pub enum CreateOrderResult {
    /// `code_url` for QR rendering (Native scene).
    Native { code_url: String },
    /// Signed JSAPI invocation parameters (JSAPI scene).
    Jsapi(JsapiParams),
}

/// A downloaded WeChat platform certificate. `public_key_pem` is the RSA
/// public key used to verify callback request signatures; `expire_time` drives
/// the moka cache refresh threshold.
#[derive(Debug, Clone)]
pub struct PlatformCert {
    pub serial_no: String,
    pub public_key_pem: String,
    pub expire_time: DateTime<Utc>,
}

impl PlatformCert {
    /// True when the certificate is within `threshold` of expiry.
    pub fn expiring_within(&self, now: DateTime<Utc>, threshold: chrono::Duration) -> bool {
        self.expire_time - now <= threshold
    }
}

/// Minimal view of a platform certificate response entry before decryption.
/// `effective_time` is present in the response but unused.
#[derive(Debug, Deserialize)]
pub(crate) struct RawPlatformCertEntry {
    pub serial_no: String,
    pub expire_time: String,
    pub encrypt_certificate: EncryptedCert,
}

#[derive(Debug, Deserialize)]
pub(crate) struct EncryptedCert {
    #[allow(dead_code)]
    pub algorithm: String,
    pub nonce: String,
    pub associated_data: String,
    pub ciphertext: String,
}

/// Decrypted WeChat Pay v3 callback resource (`resource` field of the
/// notification, after AES-256-GCM decryption with the APIv3 Key).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DecryptedResource {
    pub out_trade_no: String,
    pub transaction_id: Option<String>,
    pub trade_state: String,
    pub amount: Option<ResourceAmount>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ResourceAmount {
    /// Total in fen (cents), matching `payment_attempts.amount` (i64).
    pub total: i64,
    pub currency: Option<String>,
}

/// Generate a merchant order number that fits WeChat's 32-character limit and
/// is realm-scoped for traceability: `CAS_{realm前4}_{v7 hex前22}` (port of the
/// historical verified implementation).
pub fn generate_out_trade_no(realm_id: &str) -> String {
    let prefix = &realm_id[..4.min(realm_id.len())];
    let uuid_str = Uuid::now_v7().to_string().replace('-', "");
    // 32 - "CAS_"(4) - prefix(4) - "_"(1) - "_"(1) = 22
    let max_uuid_chars = 22.min(uuid_str.len());
    let compact_uuid = &uuid_str[..max_uuid_chars];
    format!("CAS_{}_{}", prefix, compact_uuid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn out_trade_no_fits_32_chars_and_is_realm_scoped() {
        let no = generate_out_trade_no("realm-abcd-1234");
        assert!(no.starts_with("CAS_real"));
        assert!(
            no.len() <= 32,
            "out_trade_no must be <=32 chars for WeChat, got {} ({})",
            no.len(),
            no
        );
    }

    #[test]
    fn out_trade_no_handles_short_realm_id() {
        let no = generate_out_trade_no("ab");
        assert!(no.starts_with("CAS_ab_"));
        assert!(no.len() <= 32);
    }
}
