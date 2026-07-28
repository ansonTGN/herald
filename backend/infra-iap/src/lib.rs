//! herald-infra-iap: Apple App Store IAP + Google Play Billing infrastructure.
//!
//! Module layout and trust posture follow `.ai/design/support-iap.md` §5.1 /
//! §5.6 / §5.9. The crate is self-contained: it does **not** depend on
//! `herald-domain` (the design's "能不加则不加" rule) and exposes only the
//! types and constructors BE-D03 (api-billing wiring) and BE-D04 (worker
//! reconciliation) consume.
//!
//! # Quick map
//!
//! - [`apple::AppleVerifier`] — JWS/x5c verifier rooted at the bundled Apple
//!   Root CA - G3 (ES256, OCSP off).
//! - [`apple::AppleServerApiClient`] — App Store Server API client
//!   (subscription status, transaction history, notification history).
//! - [`google::GoogleServiceAccountAuth`] — RS256 JWT grant + access-token
//!   cache.
//! - [`google::GoogleDeveloperClient`] — Play Developer API thin client
//!   (6 endpoints).
//! - [`error::IapError`] — single error enum with documented HTTP mapping.

pub mod apple;
pub mod error;
pub mod google;

pub use apple::models::Environment as AppleEnvironment;
pub use apple::server_api_client::AppleServerApiClient;
pub use apple::verifier::{APPLE_ROOT_CA_G3, AppleVerifier};
pub use error::IapError;
pub use google::developer_api_client::GoogleDeveloperClient;
pub use google::models::{ProductPurchase, SubscriptionPurchaseV2, VoidedPurchasesList};
pub use google::service_account::{GOOGLE_TOKEN_URI, GoogleServiceAccountAuth, PLAY_DEV_SCOPE};
