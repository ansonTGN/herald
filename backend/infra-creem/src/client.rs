use crate::models::{
    CheckoutSession, CreateCheckoutRequest, CreemPagination, CreemSubscriptionList,
    CreemTransactionList, SearchSubscriptionsParams, SearchTransactionsParams,
};
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
    /// Test keys (`ck_test_*` or `creem_test_*`) automatically route to the test endpoint
    /// (`https://test-api.creem.io`). All other keys use the production endpoint.
    pub fn new(api_key: String, timeout_seconds: u64) -> Result<Self, CoreError> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()
            .map_err(|e| {
                CoreError::InternalServerError(format!("Failed to create HTTP client: {e}"))
            })?;

        let base_url = if api_key.starts_with("ck_test_") || api_key.starts_with("creem_test_") {
            "https://test-api.creem.io".to_string()
        } else {
            "https://api.creem.io".to_string()
        };

        Ok(Self {
            http,
            api_key,
            base_url,
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

    /// Create a Creem API client reusing an existing `reqwest::Client`.
    ///
    /// Avoids per-realm `reqwest::Client` reconstruction in batch jobs that
    /// iterate over many realms with different API keys but can share the
    /// underlying connection pool.
    pub fn with_http_client(http: reqwest::Client, api_key: String, base_url: String) -> Self {
        Self {
            http,
            api_key,
            base_url,
        }
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
        if self.base_url == "mock://creem" {
            let short_id = &request.product_id[request.product_id.len().saturating_sub(8)..];
            let id = format!("co_mock_{short_id}");
            return Ok(CheckoutSession {
                id: id.clone(),
                checkout_url: format!("mock://creem/checkout/{id}"),
                status: "pending".to_string(),
            });
        }

        let url = format!("{}/v1/checkouts", self.base_url);

        let response = self
            .http
            .post(&url)
            .header("x-api-key", &self.api_key)
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

    /// Search Creem transactions (paginated)
    pub async fn search_transactions(
        &self,
        params: &SearchTransactionsParams,
    ) -> Result<CreemTransactionList, CoreError> {
        if self.base_url == "mock://creem" {
            return Ok(CreemTransactionList {
                data: vec![],
                pagination: CreemPagination {
                    total_records: 0,
                    total_pages: 0,
                    current_page: params.page_number,
                    next_page: None,
                    prev_page: None,
                },
            });
        }

        let url = format!(
            "{}/v1/transactions/search?page_number={}&page_size={}",
            self.base_url, params.page_number, params.page_size
        );
        let url = match params.created_after {
            Some(ts) => format!("{url}&created_after={ts}"),
            None => url,
        };

        let response = self
            .http
            .get(&url)
            .header("x-api-key", &self.api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            tracing::error!("Creem API error: {} - {}", status, text);
            return Err(CoreError::InternalServerError(format!(
                "{} - {}",
                status.as_u16(),
                text
            )));
        }

        response.json::<CreemTransactionList>().await.map_err(|e| {
            tracing::error!("Failed to parse Creem transaction list: {}", e);
            CoreError::InternalServerError(format!("Invalid Creem response: {}", e))
        })
    }

    /// Search Creem subscriptions (paginated)
    pub async fn search_subscriptions(
        &self,
        params: &SearchSubscriptionsParams,
    ) -> Result<CreemSubscriptionList, CoreError> {
        if self.base_url == "mock://creem" {
            return Ok(CreemSubscriptionList {
                data: vec![],
                pagination: CreemPagination {
                    total_records: 0,
                    total_pages: 0,
                    current_page: params.page_number,
                    next_page: None,
                    prev_page: None,
                },
            });
        }

        let url = format!(
            "{}/v1/subscriptions/search?page_number={}&page_size={}",
            self.base_url, params.page_number, params.page_size
        );
        let url = match params.created_after {
            Some(ts) => format!("{url}&created_after={ts}"),
            None => url,
        };

        let response = self
            .http
            .get(&url)
            .header("x-api-key", &self.api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            tracing::error!("Creem API error: {} - {}", status, text);
            return Err(CoreError::InternalServerError(format!(
                "{} - {}",
                status.as_u16(),
                text
            )));
        }

        response.json::<CreemSubscriptionList>().await.map_err(|e| {
            tracing::error!("Failed to parse Creem subscription list: {}", e);
            CoreError::InternalServerError(format!("Invalid Creem response: {}", e))
        })
    }
}
