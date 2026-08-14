//! Error type for the WeChat Pay v3 infrastructure crate.

use thiserror::Error;

/// Errors emitted by the WeChat Pay client (v3 protocol, signing, decryption,
/// platform-certificate handling). HTTP status codes for these are mapped by
/// the caller in `api-billing`; the webhook handler turns verification /
/// decryption failures into WeChat's `{"code":"FAIL"}` response so WeChat
/// retries.
#[derive(Debug, Error)]
pub enum WechatPayError {
    #[error("wechat pay signature verification failed")]
    SignatureInvalid,
    #[error("wechat pay decrypt failed")]
    DecryptFailed,
    #[error("wechat pay invalid nonce length: expected 12 bytes, got {0}")]
    InvalidNonceLength(usize),
    #[error("wechat pay api error: status={status}, body={body}")]
    Api { status: u16, body: String },
    #[error("wechat pay api returned no code_url")]
    NoCodeUrl,
    #[error("wechat pay api returned no prepay_id")]
    NoPrepayId,
    #[error("wechat pay platform certificate not found for serial {0}")]
    PlatformCertNotFound(String),
    #[error("wechat pay config missing: {0}")]
    ConfigMissing(&'static str),
    #[error("wechat pay config invalid: {0}")]
    ConfigInvalid(String),
    #[error("wechat pay response parse failed: {0}")]
    Parse(String),
    #[error(transparent)]
    Transport(#[from] reqwest::Error),
    #[error(transparent)]
    Crypto(#[from] rsa::Error),
    #[error(transparent)]
    Base64(#[from] base64::DecodeError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Utf8(#[from] std::string::FromUtf8Error),
}

impl WechatPayError {
    /// True when the failure is a security-relevant rejection (verification,
    /// decryption, amount/config mismatch) that must NOT mutate attempt state.
    pub fn is_security_rejection(&self) -> bool {
        matches!(
            self,
            Self::SignatureInvalid
                | Self::DecryptFailed
                | Self::InvalidNonceLength(_)
                | Self::PlatformCertNotFound(_)
                | Self::ConfigMissing(_)
                | Self::ConfigInvalid(_)
        )
    }
}
