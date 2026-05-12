//! Shopify API clients
//!
//! Provides clients for Shopify Admin API and Storefront API using reqwest.

use reqwest::Client;
use thiserror::Error;

use herald_domain::common::entities::app_errors::CoreError;

/// Shopify client error types
#[derive(Error, Debug)]
pub enum ShopifyClientError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Shopify API error: {0}")]
    ApiError(String),

    #[error("Unauthorized: invalid access token")]
    Unauthorized,

    #[error("Shop domain not found")]
    ShopNotFound,
}

impl From<ShopifyClientError> for CoreError {
    fn from(err: ShopifyClientError) -> Self {
        match err {
            ShopifyClientError::Unauthorized => CoreError::Unauthorized,
            ShopifyClientError::ShopNotFound => CoreError::NotFound,
            _ => CoreError::InternalServerError(err.to_string()),
        }
    }
}

/// Shopify Admin API client
///
/// Used for querying subscription contracts, customers, and shop information.
pub struct ShopifyAdminClient {
    pub shop_domain: String,
    pub access_token: String,
    pub api_version: String,
    http_client: Client,
}

impl ShopifyAdminClient {
    /// Create a new Shopify Admin API client
    ///
    /// # Arguments
    /// * `shop_domain` - Shop domain (e.g., "demo-store.myshopify.com")
    /// * `access_token` - Admin API access token (must start with "shpat_")
    /// * `api_version` - API version (e.g., "2024-01")
    pub fn new(shop_domain: String, access_token: String, api_version: String) -> Self {
        Self {
            shop_domain,
            access_token,
            api_version,
            http_client: Client::new(),
        }
    }

    /// Get the base URL for Admin API requests
    pub fn admin_api_url(&self) -> String {
        // Normalize shop_domain: remove .myshopify.com suffix if present
        let domain = self
            .shop_domain
            .strip_suffix(".myshopify.com")
            .unwrap_or(&self.shop_domain);
        format!(
            "https://{}.myshopify.com/admin/api/{}",
            domain, self.api_version
        )
    }

    /// Test connection to Shopify Admin API
    ///
    /// Queries shop information to verify credentials.
    pub async fn test_connection(&self) -> Result<bool, ShopifyClientError> {
        let url = format!("{}/shop.json", self.admin_api_url());

        let response = self
            .http_client
            .get(&url)
            .header("X-Shopify-Access-Token", &self.access_token)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(true)
        } else if response.status().as_u16() == 401 {
            Ok(false)
        } else {
            Err(ShopifyClientError::ApiError(format!(
                "Unexpected status: {}",
                response.status()
            )))
        }
    }

    /// Query subscription contract details
    ///
    /// Used for compensation queries when webhook payload is incomplete.
    pub async fn get_subscription_contract(
        &self,
        contract_id: &str,
    ) -> Result<serde_json::Value, ShopifyClientError> {
        let query = format!(
            r#"
            {{
                subscriptionContract(id: "{}") {{
                    id
                    adminGraphqlApiId
                    customerId
                    originOrder {{
                        id
                    }}
                    sellingPlanId
                    currentPeriodEnd
                    status
                    createdAt
                    updatedAt
                }}
            }}
            "#,
            contract_id
        );

        let url = format!("{}/graphql.json", self.admin_api_url());

        let response = self
            .http_client
            .post(&url)
            .header("X-Shopify-Access-Token", &self.access_token)
            .json(&serde_json::json!({ "query": query }))
            .send()
            .await?;

        if response.status().is_success() {
            let json: serde_json::Value = response.json().await?;
            Ok(json)
        } else {
            Err(ShopifyClientError::ApiError(format!(
                "Failed to query contract: {}",
                response.status()
            )))
        }
    }

    /// Query customer details
    ///
    /// Used for compensation queries to retrieve customer attributes.
    pub async fn get_customer(
        &self,
        customer_id: &str,
    ) -> Result<serde_json::Value, ShopifyClientError> {
        let query = format!(
            r#"
            {{
                customer(id: "{}") {{
                    id
                    email
                    firstName
                    lastName
                    phone
                    defaultAddress {{
                        address1
                        city
                        province
                        country
                        zip
                    }}
                    orders(first: 1) {{
                        edges {{
                            node {{
                                id
                                attributes {{
                                    key
                                    value
                                }}
                            }}
                        }}
                    }}
                }}
            }}
            "#,
            customer_id
        );

        let url = format!("{}/graphql.json", self.admin_api_url());

        let response = self
            .http_client
            .post(&url)
            .header("X-Shopify-Access-Token", &self.access_token)
            .json(&serde_json::json!({ "query": query }))
            .send()
            .await?;

        if response.status().is_success() {
            let json: serde_json::Value = response.json().await?;
            Ok(json)
        } else {
            Err(ShopifyClientError::ApiError(format!(
                "Failed to query customer: {}",
                response.status()
            )))
        }
    }
}

/// Shopify Storefront API client
///
/// Used for customer-facing operations and checkout queries.
pub struct ShopifyStorefrontClient {
    pub shop_domain: String,
    pub access_token: String,
    http_client: Client,
}

impl ShopifyStorefrontClient {
    /// Create a new Shopify Storefront API client
    ///
    /// # Arguments
    /// * `shop_domain` - Shop domain (e.g., "demo-store.myshopify.com")
    /// * `access_token` - Storefront API access token (must start with "shp_")
    pub fn new(shop_domain: String, access_token: String) -> Self {
        Self {
            shop_domain,
            access_token,
            http_client: Client::new(),
        }
    }

    /// Get the base URL for Storefront API requests
    pub fn storefront_api_url(&self) -> String {
        // Normalize shop_domain: remove .myshopify.com suffix if present
        let domain = self
            .shop_domain
            .strip_suffix(".myshopify.com")
            .unwrap_or(&self.shop_domain);
        format!("https://{}.myshopify.com/api/2024-01", domain)
    }

    /// Test connection to Shopify Storefront API
    ///
    /// Performs a simple query to verify credentials.
    pub async fn test_connection(&self) -> Result<bool, ShopifyClientError> {
        let query = r#"
            {
                shop {
                    name
                }
            }
        "#;

        let url = format!("{}/graphql.json", self.storefront_api_url());

        let response = self
            .http_client
            .post(&url)
            .header("X-Shopify-Storefront-Access-Token", &self.access_token)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({ "query": query }))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(true)
        } else if response.status().as_u16() == 401 {
            Ok(false)
        } else {
            Err(ShopifyClientError::ApiError(format!(
                "Unexpected status: {}",
                response.status()
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admin_client_creation() {
        let client = ShopifyAdminClient::new(
            "test-shop.myshopify.com".to_string(),
            "shpat_test_token".to_string(),
            "2024-01".to_string(),
        );

        assert_eq!(client.shop_domain, "test-shop.myshopify.com");
        assert_eq!(client.access_token, "shpat_test_token");
        assert_eq!(client.api_version, "2024-01");
    }

    #[test]
    fn test_storefront_client_creation() {
        let client = ShopifyStorefrontClient::new(
            "test-shop.myshopify.com".to_string(),
            "shp_test_token".to_string(),
        );

        assert_eq!(client.shop_domain, "test-shop.myshopify.com");
        assert_eq!(client.access_token, "shp_test_token");
    }

    #[test]
    fn test_admin_api_url() {
        let client = ShopifyAdminClient::new(
            "test-shop.myshopify.com".to_string(),
            "shpat_test_token".to_string(),
            "2024-01".to_string(),
        );

        assert_eq!(
            client.admin_api_url(),
            "https://test-shop.myshopify.com/admin/api/2024-01"
        );
    }

    #[test]
    fn test_storefront_api_url() {
        let client = ShopifyStorefrontClient::new(
            "test-shop.myshopify.com".to_string(),
            "shp_test_token".to_string(),
        );

        assert_eq!(
            client.storefront_api_url(),
            "https://test-shop.myshopify.com/api/2024-01"
        );
    }
}
