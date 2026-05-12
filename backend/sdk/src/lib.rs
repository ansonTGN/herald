use dashmap::DashMap;
use futures::future::join_all;
use moka::future::Cache;
use reqwest::{Client as ReqwestClient, Method};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::debug;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("network error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("json error: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("unauthorized (401): {0}")]
    Unauthorized(String),
    #[error("forbidden (403): {0}")]
    Forbidden(String),
    #[error("not found (404): {0}")]
    NotFound(String),
    #[error("internal server error (500): {0}")]
    InternalServerError(String),
    #[error("api error ({status}): {message}")]
    ApiError { status: u16, message: String },
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub struct Rule {
    pub resource: String,
    pub action: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub struct PermissionCheckRequest {
    pub token: String,
    #[serde(default)]
    pub rules: Option<Vec<Rule>>,
    pub client_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PermissionCheckResponse {
    pub allowed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionDetail {
    pub id: String,
    pub client_app_id: Option<String>,
    pub plan_id: Option<String>,
    pub plan: Option<Plan>,
    pub status: String,
    pub billing_period: String,
    pub current_period_start: Option<String>,
    pub current_period_end: Option<String>,
    pub cancel_at: Option<String>,
    pub cancel_at_period_end: Option<bool>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub id: String,
    pub realm_id: String,
    pub name: String,
    pub title: String,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub plan_type: String,
    pub price: i32,
    pub currency: String,
    pub checkout_url: Option<String>,
    pub active: bool,
    pub trial_days: i32,
    pub sort_order: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlanAssignment {
    pub id: String,
    pub client_app_id: String,
    pub plan_id: String,
    pub enabled: bool,
    pub created_at: String,
}

/// Points balance response
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PointsBalanceResponse {
    pub user_id: String,
    pub balance: i64,
    pub total_recharged: i64,
    pub total_consumed: i64,
    pub currency: String,
    pub updated_at: String,
}

/// Points consume request
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConsumePointsRequest {
    pub user_id: String,
    pub client_app_id: String,
    pub amount: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
}

/// Points consume response
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConsumePointsResponse {
    pub transaction_id: String,
    pub account_id: String,
    pub user_id: String,
    pub amount: i64,
    pub balance_after: i64,
}

type TokenIndex = Arc<DashMap<String, Vec<PermissionCheckRequest>>>;

async fn handle_response<T>(response: reqwest::Response) -> Result<T, Error>
where
    T: for<'de> serde::Deserialize<'de>,
{
    let status = response.status();
    let text = response.text().await.map_err(Error::Reqwest)?;

    match status.as_u16() {
        401 => Err(Error::Unauthorized(text)),
        403 => Err(Error::Forbidden(text)),
        404 => Err(Error::NotFound(text)),
        500 => Err(Error::InternalServerError(text)),
        200..299 => serde_json::from_str(&text).map_err(Error::SerdeJson),
        code => Err(Error::ApiError {
            status: code,
            message: text,
        }),
    }
}

#[derive(Clone)]
pub struct Client {
    http_client: ReqwestClient,
    base_url: String,
    cache: Cache<PermissionCheckRequest, PermissionCheckResponse>,
    token_index: TokenIndex,
    api_key: String,
    token_cache: Arc<DashMap<String, (PermissionCheckResponse, Instant)>>,
}

impl Client {
    pub fn new(base_url: String, api_key: String, cache_duration: Option<Duration>) -> Self {
        let duration = cache_duration.unwrap_or_else(|| Duration::from_secs(300));
        let token_index: TokenIndex = Arc::new(DashMap::new());

        let index_for_eviction = Arc::clone(&token_index);
        let cache = Cache::builder()
            .time_to_live(duration)
            .eviction_listener(move |key: Arc<PermissionCheckRequest>, _value, _cause| {
                let index = Arc::clone(&index_for_eviction);
                if let Some(mut keys) = index.get_mut(&key.token) {
                    keys.retain(|k| k != key.as_ref());
                }
            })
            .build();

        Self {
            http_client: ReqwestClient::builder()
                .no_proxy()
                .build()
                .expect("Failed to create SDK HTTP client"),
            base_url,
            cache,
            token_index,
            api_key,
            token_cache: Arc::new(DashMap::new()),
        }
    }

    fn build_request(&self, method: Method, url: &str) -> reqwest::RequestBuilder {
        self.http_client
            .request(method, url)
            .header("X-API-Key", &self.api_key)
    }

    /// Checks if a user has a specific permission
    ///
    /// # Arguments
    ///
    /// * `req` - Permission check request containing user, resource, and action
    ///
    /// # Returns
    ///
    /// Returns `Ok(PermissionCheckResponse)` if the check was performed successfully
    ///
    /// # Errors
    ///
    /// Returns `Err(Error::Network)` if the HTTP request fails
    /// Returns `Err(Error::Timeout)` if the request times out
    /// Returns `Err(Error::Unauthorized)` if the client credentials are invalid
    /// Returns `Err(Error::Api)` if the API returns an error response
    pub async fn check_permission(
        &self,
        req: PermissionCheckRequest,
    ) -> Result<PermissionCheckResponse, Error> {
        // Check if token is expired
        if self.is_token_expired(&req.token) {
            self.invalidate_cache(&req.token).await;
        }

        if let Some(resp) = self.cache.get(&req).await {
            return Ok(resp);
        }

        let url = format!("{}/api/ext/permission/check", self.base_url);
        let response = self
            .build_request(Method::POST, &url)
            .json(&req)
            .send()
            .await?;

        let resp: PermissionCheckResponse = handle_response(response).await?;

        // Update token cache timestamp
        self.token_cache
            .insert(req.token.clone(), (resp.clone(), Instant::now()));

        self.token_index
            .entry(req.token.clone())
            .or_default()
            .push(req.clone());
        self.cache.insert(req, resp.clone()).await;

        Ok(resp)
    }

    fn is_token_expired(&self, token: &str) -> bool {
        if let Some(entry) = self.token_cache.get(token) {
            let (_, timestamp) = &*entry;
            return timestamp.elapsed() > Duration::from_secs(300); // 5分钟阈值
        }
        false
    }

    pub async fn invalidate_cache(&self, token: &str) {
        // ATOMIC: Remove and get keys in one operation
        if let Some((_, keys)) = self.token_index.remove(token) {
            // Batch invalidate all cache entries for this token
            let invalidation_futures: Vec<_> = keys
                .iter()
                .map(|key| async {
                    self.cache.invalidate(key).await;
                })
                .collect();

            // Wait for all invalidations to complete
            join_all(invalidation_futures).await;
        }
    }

    /// Get subscription details for a client app
    ///
    /// # Arguments
    /// * `realm_id` - The realm ID
    /// * `client_app_id` - The client app ID
    ///
    /// # Returns
    /// * `Ok(SubscriptionDetail)` if the request succeeds
    /// * `Err(Error)` if network or parsing fails
    pub async fn get_subscription(
        &self,
        realm_id: &str,
        client_app_id: &str,
    ) -> Result<SubscriptionDetail, Error> {
        let url = format!(
            "{}/api/ext/bill/{}/client/{}/subscription",
            self.base_url, realm_id, client_app_id
        );

        let response = self.build_request(Method::GET, &url).send().await?;

        let status = response.status();
        let resp = handle_response(response).await;
        debug!(status = %status, "API Response for get_subscription: {:?}", resp);
        resp
    }

    /// List all available plans for a realm
    ///
    /// # Arguments
    /// * `realm_id` - The realm ID
    ///
    /// # Returns
    /// * `Ok(Vec<Plan>)` if the request succeeds
    /// * `Err(Error)` if network or parsing fails
    pub async fn list_plans(&self, realm_id: &str) -> Result<Vec<Plan>, Error> {
        let url = format!("{}/api/ext/bill/{}/plans", self.base_url, realm_id);

        let response = self.build_request(Method::GET, &url).send().await?;

        let status = response.status();
        let resp: Result<serde_json::Value, Error> = handle_response(response).await;
        debug!(status = %status, "API Response for list_plans: {:?}", resp);
        let json_value = resp?;
        serde_json::from_value(json_value["plans"].clone()).map_err(Error::SerdeJson)
    }

    /// List plan assignments for a client app
    ///
    /// # Arguments
    /// * `realm_id` - The realm ID
    /// * `client_app_id` - The client app ID
    ///
    /// # Returns
    /// * `Ok(Vec<PlanAssignment>)` if the request succeeds
    /// * `Err(Error)` if network or parsing fails
    pub async fn list_plan_assignments(
        &self,
        realm_id: &str,
        client_app_id: &str,
    ) -> Result<Vec<PlanAssignment>, Error> {
        let url = format!(
            "{}/api/ext/bill/{}/client/{}/plans",
            self.base_url, realm_id, client_app_id
        );

        let response = self.build_request(Method::GET, &url).send().await?;

        let status = response.status();
        let resp: Result<serde_json::Value, Error> = handle_response(response).await;
        debug!(status = %status, "API Response for list_plan_assignments: {:?}", resp);
        let json_value = resp?;
        serde_json::from_value(json_value["assignments"].clone()).map_err(Error::SerdeJson)
    }

    /// Get user points balance
    ///
    /// # Arguments
    ///
    /// * `realm_id` - The realm ID
    /// * `user_id` - The user ID
    ///
    /// # Returns
    ///
    /// Returns `Ok(PointsBalanceResponse)` if the request succeeds
    ///
    /// # Errors
    ///
    /// Returns `Err(Error::Unauthorized)` if the API key is invalid
    /// Returns `Err(Error::Forbidden)` if cross-realm access is attempted
    /// Returns `Err(Error::NotFound)` if the account is not found
    pub async fn get_balance(
        &self,
        realm_id: &str,
        user_id: &str,
    ) -> Result<PointsBalanceResponse, Error> {
        let url = format!("{}/api/ext/points/{}/balance", self.base_url, realm_id);

        let response = self
            .http_client
            .request(Method::GET, &url)
            .header("X-API-Key", &self.api_key)
            .query(&[("userId", user_id)])
            .send()
            .await?;

        let status = response.status();
        let resp = handle_response(response).await;
        debug!(status = %status, "API Response for get_balance: {:?}", resp);
        resp
    }

    /// Consume points from user account
    ///
    /// # Arguments
    ///
    /// * `realm_id` - The realm ID
    /// * `user_id` - The user ID
    /// * `client_app_id` - The client app ID consuming points
    /// * `amount` - The amount of points to consume
    /// * `description` - Optional description of the consumption
    /// * `idempotency_key` - Optional idempotency key to prevent duplicate charges
    ///
    /// # Returns
    ///
    /// Returns `Ok(ConsumePointsResponse)` if the request succeeds
    ///
    /// # Errors
    ///
    /// Returns `Err(Error::Unauthorized)` if the API key is invalid
    /// Returns `Err(Error::Forbidden)` if cross-realm access is attempted
    /// Returns `Err(Error::NotFound)` if the account is not found
    /// Returns `Err(Error::ApiError)` for other API errors (e.g., insufficient points)
    pub async fn consume_points(
        &self,
        realm_id: &str,
        user_id: &str,
        client_app_id: &str,
        amount: i64,
        description: Option<String>,
        idempotency_key: Option<String>,
    ) -> Result<ConsumePointsResponse, Error> {
        let url = format!("{}/api/ext/points/{}/consume", self.base_url, realm_id);

        let request = ConsumePointsRequest {
            user_id: user_id.to_string(),
            client_app_id: client_app_id.to_string(),
            amount,
            description,
            idempotency_key,
        };

        let response = self
            .build_request(Method::POST, &url)
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        let resp = handle_response(response).await;
        debug!(status = %status, "API Response for consume_points: {:?}", resp);
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_check_permission() {
        let server = MockServer::start().await;
        let client = Client::new(server.uri(), "test-api-key".to_string(), None);

        let user_id = uuid::Uuid::now_v7().to_string();
        let resp = PermissionCheckResponse {
            allowed: true,
            user_id: Some(user_id.clone()),
        };

        Mock::given(method("POST"))
            .and(path("/api/ext/permission/check"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&resp))
            .expect(1)
            .mount(&server)
            .await;

        let req = PermissionCheckRequest {
            token: "test_token".to_string(),
            rules: None,
            client_id: uuid::Uuid::now_v7().to_string(),
        };

        let result = client.check_permission(req).await.unwrap();
        assert!(result.allowed);
        assert_eq!(result.user_id, Some(user_id));

        server.verify().await;
    }

    #[tokio::test]
    async fn test_caching() {
        let server = MockServer::start().await;
        let client = Client::new(
            server.uri(),
            "test-api-key".to_string(),
            Some(Duration::from_secs(1)),
        );

        let user_id = uuid::Uuid::now_v7().to_string();
        let resp = PermissionCheckResponse {
            allowed: true,
            user_id: Some(user_id.clone()),
        };

        Mock::given(method("POST"))
            .and(path("/api/ext/permission/check"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&resp))
            .expect(1)
            .mount(&server)
            .await;

        let req = PermissionCheckRequest {
            token: "test_token".to_string(),
            rules: None,
            client_id: uuid::Uuid::now_v7().to_string(),
        };

        // First call, should hit the server
        let _ = client.check_permission(req.clone()).await.unwrap();

        // Second call, should be cached
        let _ = client.check_permission(req.clone()).await.unwrap();

        server.verify().await;

        tokio::time::sleep(Duration::from_secs(2)).await;

        // Third call, after cache expiration, should hit the server again
        server.reset().await;
        Mock::given(method("POST"))
            .and(path("/api/ext/permission/check"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&resp))
            .expect(1)
            .mount(&server)
            .await;

        let _ = client.check_permission(req.clone()).await.unwrap();
        server.verify().await;
    }

    #[tokio::test]
    async fn test_invalidate_cache() {
        let server = MockServer::start().await;
        let client = Client::new(server.uri(), "test-api-key".to_string(), None);

        let user_id = uuid::Uuid::now_v7().to_string();
        let resp = PermissionCheckResponse {
            allowed: true,
            user_id: Some(user_id.clone()),
        };

        Mock::given(method("POST"))
            .and(path("/api/ext/permission/check"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&resp))
            .expect(3)
            .mount(&server)
            .await;

        let req1 = PermissionCheckRequest {
            token: "token1".to_string(),
            rules: None,
            client_id: uuid::Uuid::now_v7().to_string(),
        };

        let req2 = PermissionCheckRequest {
            token: "token2".to_string(),
            rules: None,
            client_id: uuid::Uuid::now_v7().to_string(),
        };

        // First calls, should hit the server
        let _ = client.check_permission(req1.clone()).await.unwrap();
        let _ = client.check_permission(req2.clone()).await.unwrap();

        // Invalidate cache for token1
        client.invalidate_cache("token1").await;

        // Call again, req1 should hit the server, req2 should be cached
        let _ = client.check_permission(req1.clone()).await.unwrap();
        let _ = client.check_permission(req2.clone()).await.unwrap();

        server.verify().await;
    }

    // ========================================================================
    // Billing API Tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_subscription_success() {
        let server = MockServer::start().await;
        let client = Client::new(server.uri(), "test-api-key".to_string(), None);

        let subscription_response = json!({
            "id": "sub-123",
            "clientAppId": "client1",
            "planId": "plan-basic",
            "status": "active",
            "billingPeriod": "monthly",
            "plan": {
                "id": "plan-basic",
                "realmId": "realm1",
                "name": "basic",
                "title": "Basic Plan",
                "description": "Basic subscription plan",
                "type": "standard",
                "price": 1000,
                "currency": "USD",
                "checkoutUrl": null,
                "active": true,
                "trialDays": 0,
                "sortOrder": 1
            },
            "currentPeriodStart": null,
            "currentPeriodEnd": null,
            "cancelAt": null,
            "cancelAtPeriodEnd": null,
            "createdAt": "2025-01-01T00:00:00Z",
            "updatedAt": "2025-01-01T00:00:00Z"
        });

        Mock::given(method("GET"))
            .and(path("/api/ext/bill/realm1/client/client1/subscription"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&subscription_response))
            .expect(1)
            .mount(&server)
            .await;

        let result = client.get_subscription("realm1", "client1").await;

        assert!(
            result.is_ok(),
            "get_subscription should succeed, got error: {:?}",
            result
        );
        let subscription = result.unwrap();
        assert_eq!(subscription.status, "active");
        assert!(subscription.plan.is_some());
        assert_eq!(subscription.plan.unwrap().name, "basic");

        server.verify().await;
    }

    #[tokio::test]
    async fn test_list_plans_success() {
        let server = MockServer::start().await;
        let client = Client::new(server.uri(), "test-api-key".to_string(), None);

        let plans_response = json!({
            "plans": [
                {
                    "id": "plan-basic",
                    "realmId": "realm1",
                    "name": "basic",
                    "title": "Basic Plan",
                    "description": "Basic subscription plan",
                    "type": "standard",
                    "price": 1000,
                    "currency": "USD",
                    "checkoutUrl": null,
                    "active": true,
                    "trialDays": 0,
                    "sortOrder": 1
                },
                {
                    "id": "plan-premium",
                    "realmId": "realm1",
                    "name": "premium",
                    "title": "Premium Plan",
                    "description": "Premium subscription plan",
                    "type": "standard",
                    "price": 2500,
                    "currency": "USD",
                    "checkoutUrl": null,
                    "active": true,
                    "trialDays": 0,
                    "sortOrder": 2
                }
            ]
        });

        Mock::given(method("GET"))
            .and(path("/api/ext/bill/realm1/plans"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&plans_response))
            .expect(1)
            .mount(&server)
            .await;

        let result = client.list_plans("realm1").await;

        assert!(
            result.is_ok(),
            "list_plans should succeed, got error: {:?}",
            result
        );
        let plans = result.unwrap();
        assert_eq!(plans.len(), 2);
        assert!(plans.iter().any(|p| p.name == "basic"));
        assert!(plans.iter().any(|p| p.name == "premium"));

        server.verify().await;
    }

    #[tokio::test]
    async fn test_list_plan_assignments_success() {
        let server = MockServer::start().await;
        let client = Client::new(server.uri(), "test-api-key".to_string(), None);

        let assignments_response = json!({
            "assignments": [
                {
                    "id": "assign-1",
                    "clientAppId": "client1",
                    "planId": "plan-basic",
                    "enabled": true,
                    "createdAt": "2025-01-01T00:00:00Z"
                }
            ]
        });

        Mock::given(method("GET"))
            .and(path("/api/ext/bill/realm1/client/client1/plans"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&assignments_response))
            .expect(1)
            .mount(&server)
            .await;

        let result = client.list_plan_assignments("realm1", "client1").await;

        assert!(
            result.is_ok(),
            "list_plan_assignments should succeed, got error: {:?}",
            result
        );
        let assignments = result.unwrap();
        assert_eq!(assignments.len(), 1);
        assert!(assignments[0].enabled);

        server.verify().await;
    }

    #[tokio::test]
    async fn test_sdk_error_handling_unauthorized() {
        let server = MockServer::start().await;
        let client = Client::new(server.uri(), "test-api-key".to_string(), None);

        Mock::given(method("GET"))
            .and(path("/api/ext/bill/realm1/client/client1/subscription"))
            .respond_with(ResponseTemplate::new(401))
            .expect(1)
            .mount(&server)
            .await;

        let result = client.get_subscription("realm1", "client1").await;

        assert!(result.is_err(), "Invalid token should return error");

        server.verify().await;
    }

    #[tokio::test]
    async fn test_sdk_timeout_handling() {
        let server = MockServer::start().await;
        let client = Client::new(
            server.uri(),
            "test-api-key".to_string(),
            Some(std::time::Duration::from_millis(100)), // Short timeout
        );

        Mock::given(method("GET"))
            .and(path(
                "/api/realms/realm1/billing/client-apps/client1/subscription",
            ))
            .respond_with(
                ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(2)), // 2 second delay
            )
            .mount(&server)
            .await;

        let result = client.get_subscription("realm1", "client1").await;

        assert!(result.is_err(), "Timeout should return error");
    }
}
