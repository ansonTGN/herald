// Reqwest HTTP client implementation for OAuth providers

pub use herald_domain::oauth::http_client::{
    HttpClient, HttpClientError, HttpClientRequest, HttpClientResponse, HttpMethod,
};
use herald_domain::security_constants::{
    DEFAULT_HTTP_CLIENT_CONNECT_TIMEOUT_SECS, DEFAULT_HTTP_CLIENT_TIMEOUT_SECS,
};
use herald_domain::telemetry::external_http::timed_external_http_span;
use std::time::Duration;

/// Reqwest HTTP client implementation
pub struct ReqwestHttpClient {
    client: reqwest::Client,
}

impl ReqwestHttpClient {
    /// Create a new Reqwest HTTP client with default settings
    pub fn new() -> Result<Self, HttpClientError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(DEFAULT_HTTP_CLIENT_TIMEOUT_SECS))
            .connect_timeout(Duration::from_secs(
                DEFAULT_HTTP_CLIENT_CONNECT_TIMEOUT_SECS,
            ))
            .build()
            .map_err(|e| {
                HttpClientError::Network(format!("Failed to create HTTP client: {}", e))
            })?;

        Ok(Self { client })
    }

    /// Create a new Reqwest HTTP client with custom timeout settings
    pub fn with_timeout(timeout_secs: u64) -> Result<Self, HttpClientError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .connect_timeout(Duration::from_secs(timeout_secs / 3))
            .build()
            .map_err(|e| {
                HttpClientError::Network(format!("Failed to create HTTP client: {}", e))
            })?;

        Ok(Self { client })
    }
}

impl HttpClient for ReqwestHttpClient {
    // 使用 impl Future 而非 async fn 以支持动态分发（trait 对象）
    // 参见：https://rust-lang.github.io/rust-clippy/master/index.html#manual_async_fn
    #[allow(clippy::manual_async_fn)]
    fn get(
        &self,
        url: &str,
    ) -> impl Future<Output = Result<HttpClientResponse, HttpClientError>> + Send {
        let url = url.to_string();
        async move {
            // BE-D10: external.http span + duration histogram. The helper's
            // sanitize_host strips path/query (OAuth callback URLs may carry
            // `code=...`/`state=...`) so only the bare host reaches telemetry.
            let timing = timed_external_http_span(&url, "GET");
            let _span_enter = timing.span().enter();

            let response = self.client.get(&url).send().await.map_err(|e| {
                let error_msg = e.to_string();
                if error_msg.contains("timeout") || error_msg.contains("timed out") {
                    HttpClientError::Timeout
                } else if error_msg.contains("URL") || error_msg.contains("url") {
                    HttpClientError::InvalidUrl(error_msg)
                } else {
                    HttpClientError::Network(error_msg)
                }
            })?;

            let status_code = response.status().as_u16();

            // Collect headers before moving response
            let mut headers = std::collections::HashMap::new();
            for (key, value) in response.headers().iter() {
                let key_str = key.to_string();
                if let Ok(value_str) = value.to_str() {
                    headers.insert(key_str, value_str.to_string());
                }
            }

            let body = response.bytes().await.map_err(|e| {
                HttpClientError::Network(format!("Failed to read response body: {}", e))
            })?;

            Ok(HttpClientResponse::new(status_code, body.to_vec()).with_headers(headers))
        }
    }

    // 使用 impl Future 而非 async fn 以支持动态分发（trait 对象）
    // 参见：https://rust-lang.github.io/rust-clippy/master/index.html#manual_async_fn
    #[allow(clippy::manual_async_fn)]
    fn post(
        &self,
        url: &str,
        body: Vec<u8>,
    ) -> impl Future<Output = Result<HttpClientResponse, HttpClientError>> + Send {
        let url = url.to_string();
        async move {
            // BE-D10: external.http span + duration histogram. Host-only
            // (token-exchange bodies are not recorded).
            let timing = timed_external_http_span(&url, "POST");
            let _span_enter = timing.span().enter();

            let response = self
                .client
                .post(&url)
                .body(body)
                .send()
                .await
                .map_err(|e| {
                    let error_msg = e.to_string();
                    if error_msg.contains("timeout") || error_msg.contains("timed out") {
                        HttpClientError::Timeout
                    } else if error_msg.contains("URL") || error_msg.contains("url") {
                        HttpClientError::InvalidUrl(error_msg)
                    } else {
                        HttpClientError::Network(error_msg)
                    }
                })?;

            let status_code = response.status().as_u16();

            // Collect headers before moving response
            let mut headers = std::collections::HashMap::new();
            for (key, value) in response.headers().iter() {
                let key_str = key.to_string();
                if let Ok(value_str) = value.to_str() {
                    headers.insert(key_str, value_str.to_string());
                }
            }

            let body = response.bytes().await.map_err(|e| {
                HttpClientError::Network(format!("Failed to read response body: {}", e))
            })?;

            Ok(HttpClientResponse::new(status_code, body.to_vec()).with_headers(headers))
        }
    }

    // 使用 impl Future 而非 async fn 以支持动态分发（trait 对象）
    // 参见：https://rust-lang.github.io/rust-clippy/master/index.html#manual_async_fn
    #[allow(clippy::manual_async_fn)]
    fn request(
        &self,
        request: HttpClientRequest,
    ) -> impl Future<Output = Result<HttpClientResponse, HttpClientError>> + Send {
        async move {
            // Build URL with query parameters
            let mut url = request.url.clone();
            if !request.query_params.is_empty() {
                let query_string: Vec<String> = request
                    .query_params
                    .iter()
                    .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
                    .collect();

                let separator = if url.contains('?') { "&" } else { "?" };
                url = format!("{}{}{}", url, separator, query_string.join("&"));
            }

            // Build request
            let mut reqwest_req = match request.method {
                HttpMethod::Get => self.client.get(&url),
                HttpMethod::Post => self.client.post(&url),
                HttpMethod::Put => self.client.put(&url),
                HttpMethod::Delete => self.client.delete(&url),
                HttpMethod::Patch => self.client.patch(&url),
            };

            // Add headers
            for (key, value) in request.headers.iter() {
                reqwest_req = reqwest_req.header(key, value);
            }

            // Add body if present
            if let Some(body) = request.body {
                reqwest_req = reqwest_req.body(body);
            }

            // Execute request
            // BE-D10: external.http span + duration histogram. Host-only
            // (query params / headers / body not recorded).
            let method_str = match request.method {
                HttpMethod::Get => "GET",
                HttpMethod::Post => "POST",
                HttpMethod::Put => "PUT",
                HttpMethod::Delete => "DELETE",
                HttpMethod::Patch => "PATCH",
            };
            let timing = timed_external_http_span(&url, method_str);
            let _span_enter = timing.span().enter();

            let response = reqwest_req.send().await.map_err(|e| {
                let error_msg = e.to_string();
                if error_msg.contains("timeout") || error_msg.contains("timed out") {
                    HttpClientError::Timeout
                } else if error_msg.contains("URL") || error_msg.contains("url") {
                    HttpClientError::InvalidUrl(error_msg)
                } else {
                    HttpClientError::Network(error_msg)
                }
            })?;

            let status_code = response.status().as_u16();

            // Collect headers before moving response
            let mut headers = std::collections::HashMap::new();
            for (key, value) in response.headers().iter() {
                let key_str = key.to_string();
                if let Ok(value_str) = value.to_str() {
                    headers.insert(key_str, value_str.to_string());
                }
            }

            let body = response.bytes().await.map_err(|e| {
                HttpClientError::Network(format!("Failed to read response body: {}", e))
            })?;

            Ok(HttpClientResponse::new(status_code, body.to_vec()).with_headers(headers))
        }
    }
}
