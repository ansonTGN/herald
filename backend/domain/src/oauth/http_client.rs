// HTTP client abstraction for OAuth providers
// This allows domain layer to be independent of HTTP implementation

use std::collections::HashMap;

/// HTTP client error type for domain layer
#[derive(Debug, thiserror::Error)]
pub enum HttpClientError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("HTTP request failed with status {0}: {1}")]
    HttpError(u16, String),

    #[error("Failed to parse response: {0}")]
    ParseError(String),

    #[error("Request timeout")]
    Timeout,

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
}

impl From<HttpClientError> for crate::common::entities::app_errors::CoreError {
    fn from(err: HttpClientError) -> Self {
        match err {
            HttpClientError::Network(msg) => {
                crate::common::entities::app_errors::CoreError::InternalServerError(format!(
                    "Network error: {}",
                    msg
                ))
            }
            HttpClientError::HttpError(status, msg) => {
                if (400..500).contains(&status) {
                    crate::common::entities::app_errors::CoreError::BadRequest(format!(
                        "HTTP error {}: {}",
                        status, msg
                    ))
                } else {
                    crate::common::entities::app_errors::CoreError::InternalServerError(format!(
                        "HTTP error {}: {}",
                        status, msg
                    ))
                }
            }
            HttpClientError::ParseError(msg) => {
                crate::common::entities::app_errors::CoreError::InternalServerError(format!(
                    "Parse error: {}",
                    msg
                ))
            }
            HttpClientError::Timeout => {
                crate::common::entities::app_errors::CoreError::InternalServerError(
                    "Request timeout".to_string(),
                )
            }
            HttpClientError::InvalidUrl(msg) => {
                crate::common::entities::app_errors::CoreError::BadRequest(format!(
                    "Invalid URL: {}",
                    msg
                ))
            }
        }
    }
}

/// HTTP response abstraction
#[derive(Debug, Clone)]
pub struct HttpClientResponse {
    pub status_code: u16,
    pub body: Vec<u8>,
    pub headers: HashMap<String, String>,
}

impl HttpClientResponse {
    /// Create a new HTTP response
    pub fn new(status_code: u16, body: Vec<u8>) -> Self {
        Self {
            status_code,
            body,
            headers: HashMap::new(),
        }
    }

    /// Check if response is successful (2xx status code)
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status_code)
    }

    /// Get response body as string
    pub fn body_as_string(&self) -> Result<String, HttpClientError> {
        std::str::from_utf8(&self.body)
            .map(|s| s.to_string())
            .map_err(|e| HttpClientError::ParseError(format!("Failed to decode body: {}", e)))
    }

    /// Add header to response
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Replace all headers with the provided map
    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = headers;
        self
    }
}

/// HTTP request builder
#[derive(Debug, Clone)]
pub struct HttpClientRequest {
    pub url: String,
    pub method: HttpMethod,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub query_params: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
}

/// HTTP client trait for OAuth providers
pub trait HttpClient: Send + Sync {
    /// Perform HTTP GET request
    fn get(
        &self,
        url: &str,
    ) -> impl Future<Output = Result<HttpClientResponse, HttpClientError>> + Send;

    /// Perform HTTP POST request with body
    fn post(
        &self,
        url: &str,
        body: Vec<u8>,
    ) -> impl Future<Output = Result<HttpClientResponse, HttpClientError>> + Send;

    /// Perform HTTP request with custom method
    fn request(
        &self,
        request: HttpClientRequest,
    ) -> impl Future<Output = Result<HttpClientResponse, HttpClientError>> + Send;
}

/// Helper trait for building HTTP requests
pub trait HttpRequestBuilder {
    /// Create a GET request builder
    fn get(url: impl Into<String>) -> HttpClientRequestBuilder;

    /// Create a POST request builder
    fn post(url: impl Into<String>) -> HttpClientRequestBuilder;
}

pub struct HttpClientRequestBuilder {
    request: HttpClientRequest,
}

impl HttpClientRequestBuilder {
    pub fn new(url: impl Into<String>, method: HttpMethod) -> Self {
        Self {
            request: HttpClientRequest {
                url: url.into(),
                method,
                headers: HashMap::new(),
                body: None,
                query_params: HashMap::new(),
            },
        }
    }

    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.request.headers.insert(key.into(), value.into());
        self
    }

    pub fn query_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.request.query_params.insert(key.into(), value.into());
        self
    }

    pub fn body(mut self, body: Vec<u8>) -> Self {
        self.request.body = Some(body);
        self
    }

    pub fn bearer_auth(mut self, token: impl Into<String>) -> Self {
        self.request.headers.insert(
            "Authorization".to_string(),
            format!("Bearer {}", token.into()),
        );
        self
    }

    pub fn build(self) -> HttpClientRequest {
        self.request
    }
}

impl HttpRequestBuilder for HttpClientRequestBuilder {
    fn get(url: impl Into<String>) -> HttpClientRequestBuilder {
        Self::new(url, HttpMethod::Get)
    }

    fn post(url: impl Into<String>) -> HttpClientRequestBuilder {
        Self::new(url, HttpMethod::Post)
    }
}
