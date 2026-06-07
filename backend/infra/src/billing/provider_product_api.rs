use std::future::Future;
use std::pin::Pin;

use herald_domain::billing::{ProviderApiPort, ProviderProduct};
use herald_domain::common::entities::app_errors::CoreError;
use serde_json::Value;
use sqlx::PgPool;

#[derive(Clone)]
pub struct ConfiguredProviderProductApi {
    pool: PgPool,
    http: reqwest::Client,
}

impl ConfiguredProviderProductApi {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            http: reqwest::Client::new(),
        }
    }

    async fn load_provider_config(
        &self,
        realm_id: &str,
        payment_provider: &str,
    ) -> Result<(String, String), CoreError> {
        let api_key = sqlx::query_scalar::<_, String>(
            "SELECT config_value
             FROM realm_config
             WHERE realm_id = $1 AND config_type = $2 AND config_key = 'api_key' AND enabled = true
             LIMIT 1",
        )
        .bind(realm_id)
        .bind(payment_provider)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| CoreError::DatabaseError(e.to_string()))?
        .ok_or_else(|| {
            CoreError::BadRequest(format!(
                "{} is not configured for realm {}",
                payment_provider, realm_id
            ))
        })?;

        let base_url = sqlx::query_scalar::<_, String>(
            "SELECT config_value
             FROM realm_config
             WHERE realm_id = $1 AND config_type = $2 AND config_key = 'mock_base_url' AND enabled = true
             LIMIT 1",
        )
        .bind(realm_id)
        .bind(payment_provider)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| CoreError::DatabaseError(e.to_string()))?
        .unwrap_or_else(|| match payment_provider {
            "stripe" => "https://api.stripe.com".to_string(),
            "creem" => {
                if api_key.starts_with("ck_test_") || api_key.starts_with("creem_test_") {
                    "https://test-api.creem.io".to_string()
                } else {
                    "https://api.creem.io".to_string()
                }
            }
            _ => String::new(),
        });

        Ok((api_key, base_url))
    }

    async fn fetch_stripe_products(
        &self,
        realm_id: &str,
    ) -> Result<Vec<ProviderProduct>, CoreError> {
        let (api_key, base_url) = self.load_provider_config(realm_id, "stripe").await?;
        if base_url == "mock://stripe" {
            return Ok(Vec::new());
        }

        let products_url = format!("{}/v1/products?active=true&limit=100", base_url);
        let response = self
            .http
            .get(products_url)
            .bearer_auth(&api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(CoreError::InternalServerError(format!(
                "Stripe product sync failed: {} - {}",
                status.as_u16(),
                text
            )));
        }

        let body: Value = response.json().await?;
        let mut synced = Vec::new();

        for product in body["data"].as_array().into_iter().flatten() {
            let Some(product_id) = product["id"].as_str() else {
                continue;
            };

            let price_url = format!(
                "{}/v1/prices?active=true&limit=100&product={}",
                base_url, product_id
            );
            let price = self
                .http
                .get(price_url)
                .bearer_auth(&api_key)
                .send()
                .await
                .ok()
                .filter(|res| res.status().is_success());

            let price_json = match price {
                Some(res) => res.json::<Value>().await.unwrap_or(Value::Null),
                None => Value::Null,
            };
            let first_price = price_json["data"]
                .as_array()
                .and_then(|prices| prices.first())
                .cloned()
                .unwrap_or(Value::Null);

            synced.push(ProviderProduct {
                external_product_id: product_id.to_string(),
                external_price_id: first_price["id"].as_str().map(str::to_string),
                name: product["name"].as_str().unwrap_or(product_id).to_string(),
                description: product["description"].as_str().map(str::to_string),
                price: first_price["unit_amount"].as_i64(),
                currency: first_price["currency"].as_str().map(str::to_string),
                billing_type: Some(if first_price["recurring"].is_object() {
                    "recurring".to_string()
                } else {
                    "one_time".to_string()
                }),
                billing_period: first_price["recurring"]["interval"]
                    .as_str()
                    .map(str::to_string),
            });
        }

        Ok(synced)
    }

    async fn fetch_creem_products(
        &self,
        realm_id: &str,
    ) -> Result<Vec<ProviderProduct>, CoreError> {
        let (api_key, base_url) = self.load_provider_config(realm_id, "creem").await?;
        if base_url == "mock://creem" {
            return Ok(Vec::new());
        }

        let response = self
            .http
            .get(format!("{}/v1/products/search?page_number=1&page_size=100", base_url))
            .header("x-api-key", api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(CoreError::InternalServerError(format!(
                "Creem product sync failed: {} - {}",
                status.as_u16(),
                text
            )));
        }

        let body: Value = response.json().await?;
        let products = body
            .get("items")
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
            .filter_map(|product| {
                let id = product["id"].as_str()?;
                Some(ProviderProduct {
                    external_product_id: id.to_string(),
                    external_price_id: None,
                    name: product["name"].as_str().unwrap_or(id).to_string(),
                    description: product["description"].as_str().map(str::to_string),
                    price: product["price"].as_i64(),
                    currency: product["currency"].as_str().map(str::to_string),
                    billing_type: product["billing_type"].as_str().map(str::to_string),
                    billing_period: product["billing_period"].as_str().map(str::to_string),
                })
            })
            .collect();

        Ok(products)
    }
}

impl ProviderApiPort for ConfiguredProviderProductApi {
    fn fetch_products(
        &self,
        realm_id: &str,
        payment_provider: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ProviderProduct>, CoreError>> + Send + '_>> {
        let realm_id = realm_id.to_string();
        let payment_provider = payment_provider.to_string();
        Box::pin(async move {
            match payment_provider.as_str() {
                "stripe" => self.fetch_stripe_products(&realm_id).await,
                "creem" => self.fetch_creem_products(&realm_id).await,
                other => Err(CoreError::BadRequest(format!(
                    "Provider product sync is not supported for {}",
                    other
                ))),
            }
        })
    }
}
