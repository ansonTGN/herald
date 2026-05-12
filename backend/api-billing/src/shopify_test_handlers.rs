//! Shopify Test Data Creation Handlers
//!
//! **TEST ONLY**: These endpoints are only available in test builds.

use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};

use chrono::{Duration, Utc};

use uuid::Uuid;

use crate::shopify_test_types::{
    CreateUnclaimedSubscriptionRequest, CreateUnclaimedSubscriptionResponse,
};

use herald_api_base::application::http::server::api_entities::ApiError;

mod constants {
    pub const PAYMENT_PROVIDER: &str = "shopify";
    pub const SUBSCRIPTION_TIER: &str = "premium";
    pub const BILLING_PERIOD: &str = "monthly";
    pub const BINDING_STATUS: &str = "active";
    pub const TEST_API_TOKEN_HEADER: &str = "x-test-api-token";
}

/// Test realm prefixes - inline since we can't depend on the api crate's test module
const TEST_REALM_PREFIXES: &[&str] = &["test-", "realm-"];

const ADMIN_REALM_ID: &str = "admin";

fn is_test_realm(realm_id: &str) -> bool {
    TEST_REALM_PREFIXES
        .iter()
        .any(|prefix| realm_id.starts_with(prefix))
        || realm_id == ADMIN_REALM_ID
}

#[utoipa::path(
    post,
    path = "/api/test/{realmId}/shopify/unclaimed-subscriptions",
    tag = "billing.test",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    request_body = CreateUnclaimedSubscriptionRequest,
    responses(
        (status = 201, description = "Unclaimed subscription created successfully", body = CreateUnclaimedSubscriptionResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Missing or invalid test API token"),
        (status = 403, description = "Realm not allowed for test data creation"),
        (status = 500, description = "Internal server error")
    ),
    security(("test_api_token" = []))
)]

pub async fn create_unclaimed_subscription(
    State(state): State<herald_api_base::application::http::state::AppState>,
    Path(realm_id): Path<String>,
    headers: HeaderMap,
    Json(request): Json<CreateUnclaimedSubscriptionRequest>,
) -> Result<Json<CreateUnclaimedSubscriptionResponse>, ApiError> {
    let pool = &state.pool;
    validate_test_api_token(&headers)?;
    validate_test_realm(&realm_id)?;

    let plan_id = Uuid::parse_str(&request.plan_id)
        .map_err(|e| ApiError::bad_request(format!("Invalid plan ID format: {e}")))?;

    let status = request.status.clone();

    let subscription_id = Uuid::now_v7();

    let now = Utc::now();
    let period_start = now;
    let period_end = now + Duration::days(30);

    sqlx::query(
        "INSERT INTO subscription (
            id, realm_id, user_id, external_subscription_id, external_product_id,
            payment_provider, status, tier, current_period_start, current_period_end,
            plan_id, billing_period, created_at, updated_at
        ) VALUES ($1, $2, NULL, $3, $4, $5, $6, $7, $8, $9, $10, NOW(), NOW())",
    )
    .bind(subscription_id)
    .bind(&realm_id)
    .bind(format!("shopify_test_{}", subscription_id))
    .bind(format!("shopify_product_{}", subscription_id))
    .bind(constants::PAYMENT_PROVIDER)
    .bind(&status)
    .bind(constants::SUBSCRIPTION_TIER)
    .bind(period_start)
    .bind(period_end)
    .bind(plan_id)
    .bind(constants::BILLING_PERIOD)
    .execute(pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create subscription: {e}")))?;

    sqlx::query(
        "INSERT INTO shopify_subscription_binding (
            subscription_id, realm_id, shop_domain, customer_id, customer_gid,
            contract_id, contract_gid, last_order_id, status, created_at, updated_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW(), NOW())",
    )
    .bind(subscription_id)
    .bind(&realm_id)
    .bind(&request.shop_domain)
    .bind(&request.shopify_customer_id)
    .bind(request.shopify_customer_gid.unwrap_or_default())
    .bind(&request.contract_id)
    .bind(&request.contract_id)
    .bind(request.last_order_id.unwrap_or_default())
    .bind(constants::BINDING_STATUS)
    .execute(pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create Shopify binding: {e}")))?;

    tracing::info!(
        subscription_id = %subscription_id,
        realm_id = %realm_id,
        shopify_customer_id = %request.shopify_customer_id,
        "Created unclaimed Shopify subscription for testing"
    );

    Ok(Json(CreateUnclaimedSubscriptionResponse {
        subscription_id: subscription_id.to_string(),
        realm_id,
        shopify_customer_id: request.shopify_customer_id,
        contract_id: request.contract_id,
        status: request.status,
        current_period_end: period_end.to_rfc3339(),
    }))
}

fn validate_test_api_token(headers: &HeaderMap) -> Result<(), ApiError> {
    let provided_token = headers
        .get(constants::TEST_API_TOKEN_HEADER)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError::unauthorized("Missing test API token".to_string()))?;

    let expected_token = std::env::var("TEST_API_TOKEN")
        .map_err(|_| ApiError::internal("TEST_API_TOKEN not configured".to_string()))?;

    if provided_token != expected_token {
        return Err(ApiError::unauthorized("Invalid test API token".to_string()));
    }

    Ok(())
}

fn validate_test_realm(realm_id: &str) -> Result<(), ApiError> {
    if !is_test_realm(realm_id) {
        return Err(ApiError::forbidden(
            "Test data creation is only allowed for test realms".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_test_realm;

    #[test]
    fn validate_test_realm_rejects_production_realms() {
        assert!(validate_test_realm("production-realm").is_err());
        assert!(validate_test_realm("prod-001").is_err());
        assert!(validate_test_realm("customer-realm").is_err());
    }

    #[test]
    fn validate_test_realm_allows_test_realms() {
        assert!(validate_test_realm("test-001").is_ok());
        assert!(validate_test_realm("test-realm").is_ok());
        assert!(validate_test_realm("realm-001").is_ok());
        assert!(validate_test_realm("admin").is_ok());
    }
}
