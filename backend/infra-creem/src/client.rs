use crate::models::{CheckoutSession, CreateCheckoutRequest};
use herald_domain::common::entities::app_errors::CoreError;
use std::time::Duration;

#[derive(Clone)]
pub struct CreemClient {
    pub(crate) http: reqwest::Client,
    pub(crate) api_key: String,
    pub(crate) base_url: String,
}

impl CreemClient {
    /// Create a new Creem API client
    ///
    /// # Arguments
    ///
    /// * `api_key` - Creem API key
    /// * `timeout_seconds` - HTTP request timeout in seconds
    ///
    /// # Note
    ///
    /// Always uses the production API endpoint (https://api.creem.io).
    /// Test mode is no longer supported - all environments use the same endpoint.
    pub fn new(api_key: String, timeout_seconds: u64) -> Result<Self, CoreError> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()
            .map_err(|e| {
                CoreError::InternalServerError(format!("Failed to create HTTP client: {e}"))
            })?;

        Ok(Self {
            http,
            api_key,
            base_url: "https://api.creem.io".to_string(),
        })
    }

    /// Create a new Creem API client with a custom base URL
    ///
    /// This is primarily useful for testing with mock servers.
    ///
    /// # Arguments
    ///
    /// * `api_key` - Creem API key
    /// * `base_url` - Custom base URL for the API
    /// * `timeout_seconds` - HTTP request timeout in seconds
    pub fn with_base_url(
        api_key: String,
        base_url: String,
        timeout_seconds: u64,
    ) -> Result<Self, CoreError> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()
            .map_err(|e| {
                CoreError::InternalServerError(format!("Failed to create HTTP client: {e}"))
            })?;

        Ok(Self {
            http,
            api_key,
            base_url,
        })
    }

    /// Create a checkout session for a product
    ///
    /// # Arguments
    ///
    /// * `request` - Checkout session creation request
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The API request fails
    /// - Invalid API key
    /// - Network connectivity issues
    pub async fn create_checkout_session(
        &self,
        request: &CreateCheckoutRequest,
    ) -> Result<CheckoutSession, CoreError> {
        let url = format!("{}/v1/checkout", self.base_url);

        let response = self
            .http
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            tracing::error!("Creem API error: {} - {}", status, text);
            // Format error message to include status code as expected by tests
            return Err(CoreError::InternalServerError(format!(
                "{} - {}",
                status.as_u16(),
                text
            )));
        }

        response.json::<CheckoutSession>().await.map_err(|e| {
            tracing::error!("Failed to parse Creem response: {}", e);
            CoreError::InternalServerError(format!("Invalid Creem response: {}", e))
        })
    }
}
