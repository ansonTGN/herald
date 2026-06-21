// Herald WeChat SDK.
//
// Only the low-level client/models SDK is exported here. The WeChat *Pay*
// provider integration (order repository, subscription service, webhook/order
// handlers) was removed alongside the Shopify provider drop. WeChat OAuth login
// is unaffected and lives under `api-oauth` / `infra/src/oauth/providers`.
pub mod client;
pub mod models;

pub use client::WechatPayClient;
pub use models::*;
