//! herald-infra-wechatpay: WeChat Pay v3 infrastructure.
//!
//! Self-contained client for the WeChat Pay v3 API: unified order (Native
//! scan-to-pay + JSAPI in-WeChat), platform-certificate download with an
//! in-memory moka cache, and callback signature verification + AES-256-GCM
//! decryption.
//!
//! Built entirely on the workspace pure-Rust crypto stack (`rsa`, `sha2`,
//! `aes-gcm`, `reqwest` with rustls) — no openssl, no native-tls, no
//! third-party WeChat SDK (DEC-wechat-support-004).
//!
//! Fulfilment, callback idempotency and credential storage are owned by the
//! existing unified billing pipeline (`payment_attempt`, `payment_event`,
//! `realm_config`); this crate only speaks the v3 protocol.

pub mod client;
pub mod error;
pub mod models;
pub mod platform_certs;
pub mod signing;

pub use client::{EncryptedResource, WechatPayClient};
pub use error::WechatPayError;
pub use models::{
    CreateOrderResult, CreateOrderScene, DecryptedResource, JsapiParams, PlatformCert,
    ResourceAmount, WechatPayConfig, generate_out_trade_no,
};
pub use platform_certs::PlatformCertCache;
