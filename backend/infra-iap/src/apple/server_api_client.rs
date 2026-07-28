//! App Store Server API client (design §5.1).
//!
//! Thin wrapper around
//! [`app_store_server_library::api_client::api::app_store_server_api::AppStoreServerApiClient`]
//! that:
//!
//! 1. Plugs in Herald's own `reqwest::Client` (rustls, shared connection pool)
//!    via a custom [`Transport`] impl — the upstream ships a
//!    `ReqwestHttpTransport::new()` that constructs a bare `Client::new()` and
//!    cannot be told to reuse a connection pool.
//! 2. Surfaces the three compensation/reconciliation endpoints the IAP
//!    reconciliation job needs (`get_all_subscription_status`,
//!    `get_transaction_history`, `get_notification_history`) and maps the
//!    upstream's verbose error type into [`IapError`].
//!
//! The signing key passed in is the raw bytes of the App Store Connect `.p8`
//! private key (PEM or DER — the upstream `EncodingKey::from_ec_pem` requires
//! PEM, so callers should provide PEM; see BE-D03 for the credential loader).

use crate::apple::models::{
    Environment, HistoryResponse, NotificationHistoryRequest, NotificationHistoryResponse, Status,
    StatusResponse, TransactionHistoryRequest,
};
use crate::error::IapError;
use app_store_server_library::api_client::api::app_store_server_api::{
    ApiError, AppStoreServerApiClient, GetTransactionHistoryVersion,
};
use app_store_server_library::api_client::error::ConfigurationError;
use app_store_server_library::api_client::transport::{Transport, TransportError};
use std::sync::Arc;

/// Herald-owned [`Transport`] over a shared `reqwest::Client`.
///
/// Mirrors the upstream `ReqwestHttpTransport` but lets the caller supply the
/// `reqwest::Client` (rustls, tuned timeouts, shared pool). `Clone` is cheap
/// because `reqwest::Client` is `Arc`-backed internally.
///
/// `base_url_override` is an optional scheme+host+port substitution applied at
/// the transport boundary. The upstream `ApiClient` builds each request URL as
/// `format!("{}{}", environment.base_url(), path)` and gives no way to override
/// that base, so to drive the App Store Server API against a wiremock we
/// intercept the URI in `send()` and swap the scheme+authority for the override
/// while preserving the path + query. Mirrors the Stripe/Creem `base_url`
/// realm-config injection pattern; production leaves it `None`.
#[derive(Clone)]
pub struct HeraldReqwestTransport {
    client: reqwest::Client,
    base_url_override: Option<String>,
}

impl HeraldReqwestTransport {
    pub fn new(client: reqwest::Client) -> Self {
        Self {
            client,
            base_url_override: None,
        }
    }

    /// Build a transport that rewrites every outgoing request's
    /// scheme+authority to `base_url` (path + query preserved). Used to point
    /// the App Store Server API at a wiremock in tests / staged environments.
    pub fn with_base_url_override(client: reqwest::Client, base_url: String) -> Self {
        Self {
            client,
            base_url_override: Some(base_url),
        }
    }
}

impl Transport for HeraldReqwestTransport {
    async fn send(
        &self,
        req: http::Request<Vec<u8>>,
    ) -> Result<http::Response<Vec<u8>>, TransportError> {
        let (parts, body_bytes) = req.into_parts();

        // Apply the optional base-URL override: parse the upstream-built URI,
        // strip it down to path+query, and re-prefix the override scheme+host.
        // On any parse failure we fall back to the original URI (the override
        // is a test affordance, never a production-critical rewrite).
        let final_uri = match (&self.base_url_override, parts.uri.query()) {
            (Some(base), _) => rewrite_uri_with_base(base, &parts.uri),
            (None, _) => parts.uri.to_string(),
        };

        let mut reqwest_request = self.client.request(parts.method, final_uri);
        for (name, value) in parts.headers.iter() {
            reqwest_request = reqwest_request.header(name.as_str(), value.as_bytes());
        }
        reqwest_request = reqwest_request.body(body_bytes);

        let response = reqwest_request.send().await.map_err(map_reqwest_error)?;

        let status = http::StatusCode::from_u16(response.status().as_u16())
            .map_err(TransportError::InvalidStatusCode)?;

        let mut http_response_builder = http::Response::builder().status(status);
        for (name, value) in response.headers().iter() {
            http_response_builder = http_response_builder.header(name.as_str(), value.as_bytes());
        }

        let body_bytes = response.bytes().await.map_err(map_reqwest_error)?.to_vec();

        http_response_builder
            .body(body_bytes)
            .map_err(|e| TransportError::InvalidResponse(e.to_string()))
    }
}

/// Build a new URL string by taking the path + query from `original` and
/// prefixing the scheme+authority from `base`. Used by the transport's
/// base-URL override.
fn rewrite_uri_with_base(base: &str, original: &http::Uri) -> String {
    let trimmed_base = base.trim_end_matches('/');
    let path_and_query = original
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("");
    format!("{trimmed_base}{path_and_query}")
}

fn map_reqwest_error(err: reqwest::Error) -> TransportError {
    if err.is_timeout() {
        TransportError::Timeout
    } else if err.is_connect() {
        TransportError::NetworkError(format!("Connection failed: {err}"))
    } else if err.is_request() {
        TransportError::RequestFailed(format!("Request error: {err}"))
    } else {
        TransportError::Other(err.to_string())
    }
}

/// App Store Server API client.
///
/// Construct once per realm. The underlying `ApiClient` is not `Clone`, so the
/// wrapper holds it behind an `Arc`; each call mints a fresh ES256 JWT and the
/// HTTP transport reuses a shared `reqwest::Client`.
#[derive(Clone)]
pub struct AppleServerApiClient {
    inner: Arc<AppStoreServerApiClient<HeraldReqwestTransport>>,
}

impl AppleServerApiClient {
    /// Build a client.
    ///
    /// # Arguments
    /// * `signing_key` - App Store Connect `.p8` private key as **PEM** bytes.
    /// * `key_id` - App Store Connect Key ID.
    /// * `issuer_id` - App Store Connect Issuer ID.
    /// * `bundle_id` - App Bundle ID.
    /// * `environment` - `Sandbox` or `Production` (Xcode is rejected by the
    ///   upstream; `LocalTesting` is test-only).
    /// * `http` - Shared `reqwest::Client` (rustls).
    pub fn new(
        signing_key: Vec<u8>,
        key_id: String,
        issuer_id: String,
        bundle_id: String,
        environment: Environment,
        http: reqwest::Client,
    ) -> Result<Self, IapError> {
        Self::with_transport(
            signing_key,
            &key_id,
            &issuer_id,
            &bundle_id,
            environment,
            HeraldReqwestTransport::new(http),
        )
    }

    /// Build a client whose outgoing requests are rewritten to the supplied
    /// `base_url` (scheme+host+port; path + query preserved) at the transport
    /// boundary. Used to point the App Store Server API at a wiremock in tests
    /// / staged environments (mirrors the Stripe/Creem `base_url` realm-config
    /// injection pattern). The `environment` is still required because the
    /// upstream constructor derives its own internal `base_url` from it before
    /// our transport overrides it.
    pub fn with_base_url(
        signing_key: Vec<u8>,
        key_id: String,
        issuer_id: String,
        bundle_id: String,
        environment: Environment,
        http: reqwest::Client,
        base_url: String,
    ) -> Result<Self, IapError> {
        Self::with_transport(
            signing_key,
            &key_id,
            &issuer_id,
            &bundle_id,
            environment,
            HeraldReqwestTransport::with_base_url_override(http, base_url),
        )
    }

    /// Build a client from a fully-formed transport — the shared body of
    /// [`new`](Self::new) and [`with_base_url`](Self::with_base_url), which
    /// differ only in the transport they construct.
    fn with_transport(
        signing_key: Vec<u8>,
        key_id: &str,
        issuer_id: &str,
        bundle_id: &str,
        environment: Environment,
        transport: HeraldReqwestTransport,
    ) -> Result<Self, IapError> {
        let inner = AppStoreServerApiClient::new(
            signing_key,
            key_id,
            issuer_id,
            bundle_id,
            environment,
            transport,
        )
        .map_err(map_config_error)?;
        Ok(Self {
            inner: Arc::new(inner),
        })
    }

    /// Get the status of all auto-renewable subscriptions for a customer.
    ///
    /// Maps to Apple's `GET /inApps/v1/subscriptions/{transactionId}`. The
    /// `status` filter is optional; pass `None` for all statuses.
    pub async fn get_all_subscription_status(
        &self,
        transaction_id: &str,
        status: Option<&Vec<Status>>,
    ) -> Result<StatusResponse, IapError> {
        self.inner
            .get_all_subscription_statuses(transaction_id, status)
            .await
            .map_err(map_api_error)
    }

    /// Get a paginated slice of a customer's transaction history.
    ///
    /// Maps to Apple's `GET /inApps/v2/history/{transactionId}`. Pagination is
    /// external: the caller reads `revision` / `hasMore` from the returned
    /// [`HistoryResponse`] and re-invokes with the next `revision` token.
    pub async fn get_transaction_history(
        &self,
        transaction_id: &str,
        revision: Option<&str>,
        request: TransactionHistoryRequest,
    ) -> Result<HistoryResponse, IapError> {
        self.inner
            .get_transaction_history_with_version(
                transaction_id,
                revision,
                &request,
                GetTransactionHistoryVersion::V2,
            )
            .await
            .map_err(map_api_error)
    }

    /// Get App Store Server Notification history (compensation polling).
    ///
    /// Maps to Apple's `POST /inApps/v1/notifications/history`. Pagination is
    /// external via the `paginationToken` parameter (empty string on first
    /// call); the caller reads `hasMore` / `paginationToken` from the
    /// returned [`NotificationHistoryResponse`].
    pub async fn get_notification_history(
        &self,
        pagination_token: &str,
        request: &NotificationHistoryRequest,
    ) -> Result<NotificationHistoryResponse, IapError> {
        self.inner
            .get_notification_history(pagination_token, request)
            .await
            .map_err(map_api_error)
    }
}

fn map_config_error(err: ConfigurationError) -> IapError {
    IapError::ServiceAccountAuth(format!("apple server api misconfigured: {err}"))
}

/// Flatten the upstream `ApiError` (HTTP status + Apple error code + message)
/// into [`IapError::GoogleApi`]-style surfaces. Apple API failures share the
/// 422 mapping semantics with Google (`verification_failed`), so they reuse
/// the generic API-error variant shape — the variant is named `AppleVerification`
/// upstream-of-here but the api layer maps both `AppleVerification` and
/// `GoogleApi` to 422, so a single string-bearing mapping is sufficient.
fn map_api_error(err: ApiError) -> IapError {
    IapError::AppleVerification(format!(
        "status={} code={:?} msg={}",
        err.http_status_code,
        err.error_code,
        err.error_message.unwrap_or_default()
    ))
}
