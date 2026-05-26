//! Shopify Configuration Management Handlers

use axum::{
    Json,
    extract::{Extension, Path, State},
    http::StatusCode,
};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::shopify_config_types::{
    GenericErrorResponse, PaymentProviderInfo, PaymentProvidersResponse, ShopifyConfigRequest,
    ShopifyConfigResponse, ShopifyConfigUpdateRequest, TestConnectionRequest,
    TestConnectionResponse, TestConnectionResults, ValidationErrorDetail, ValidationErrorResponse,
    validate_request,
};
use crate::wechat_config_handlers::get_wechat_config_for_providers;
use herald_api_base::application::http::common::auth_utils::require_authenticated_user_in_realm;
use herald_api_base::application::http::server::api_entities::ApiError;
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::authorization::PermissionService;
use herald_core::infrastructure::shopify::{ShopifyAdminClient, ShopifyStorefrontClient};

// ============================================================================
// Configuration Management Handlers
// ============================================================================

#[utoipa::path(
    get,
    path = "/api/third/pay/{realmId}/providers",
    params(
        ("realmId" = String, Path, description = "Realm UUID")
    ),
    responses(
        (status = 200, description = "Payment providers retrieved successfully.", body = PaymentProvidersResponse),
        (status = 401, description = "Unauthorized - No valid authentication token"),
        (status = 403, description = "Forbidden - User does not have access to this realm"),
        (status = 404, description = "Realm not found")
    ),
    tag = "billing.payment-providers",
    operation_id = "list_payment_providers"
)]
pub async fn list_payment_providers(
    State(state): State<AppState>,
    Path(realm_id): Path<String>,
    Extension(identity): Extension<Identity>,
) -> Result<Json<PaymentProvidersResponse>, ApiError> {
    let _user_id =
        require_authenticated_user_in_realm(&identity, &realm_id, "list payment providers")?;

    let can_manage = if identity.is_user() {
        state
            .permission_checker
            .check_permission(&realm_id, &identity.user_id(), "billing", "manage")
            .await
            .unwrap_or(false)
    } else {
        false
    };

    let mut providers = Vec::new();

    if let Some(config) =
        get_shopify_config_internal(&state.pool, &realm_id, &state.public_base_url).await?
        && (can_manage || config.enabled)
    {
        providers.push(PaymentProviderInfo {
            platform: "shopify".to_string(),
            shop_domain: Some(config.shop_domain),
            api_version: Some(config.api_version),
            webhook_endpoint: Some(config.webhook_endpoint),
            last_updated: Some(config.last_updated),
            enabled: config.enabled,
        });
    }

    if let Some(wechat_config) = get_wechat_config_for_providers(&state.pool, &realm_id).await?
        && (can_manage || wechat_config.enabled)
    {
        providers.push(wechat_config);
    }

    if let Some(stripe_config) = get_stripe_config_for_providers(&state.pool, &realm_id).await?
        && (can_manage || stripe_config.enabled)
    {
        providers.push(stripe_config);
    }

    if let Some(creem_config) = get_creem_config_for_providers(&state.pool, &realm_id).await?
        && (can_manage || creem_config.enabled)
    {
        providers.push(creem_config);
    }

    Ok(Json(PaymentProvidersResponse { providers }))
}

#[utoipa::path(
    post,
    path = "/api/third/pay/{realmId}/providers/shopify",
    params(
        ("realmId" = String, Path, description = "Realm UUID")
    ),
    request_body = ShopifyConfigRequest,
    responses(
        (status = 201, description = "Configuration created successfully", body = ShopifyConfigResponse),
        (status = 400, description = "Validation failed", body = ValidationErrorResponse),
        (status = 401, description = "Unauthorized - No valid authentication token"),
        (status = 403, description = "Forbidden - Realm Admin permission required"),
        (status = 409, description = "Configuration already exists", body = GenericErrorResponse),
        (status = 422, description = "Shopify connection test failed", body = GenericErrorResponse)
    ),
    tag = "billing.payment-providers",
    operation_id = "create_shopify_config"
)]
pub async fn create_shopify_config(
    State(state): State<AppState>,
    Path(realm_id): Path<String>,
    Extension(identity): Extension<Identity>,
    Json(request): Json<ShopifyConfigRequest>,
) -> Result<Json<ShopifyConfigResponse>, ApiError> {
    crate::handlers::require_billing_permission(&state, &identity, &realm_id, "manage").await?;

    validate_request(&request).map_err(ApiError::bad_request_json)?;

    if !request.shop_domain.ends_with(".myshopify.com") {
        return Err(ApiError::bad_request_json(ValidationErrorResponse {
            error: "validation_failed".to_string(),
            details: vec![ValidationErrorDetail {
                field: "shopDomain".to_string(),
                message: "Shop Domain must end with .myshopify.com".to_string(),
            }],
        }));
    }

    if !request.admin_access_token.starts_with("shpat_") {
        return Err(ApiError::bad_request_json(ValidationErrorResponse {
            error: "validation_failed".to_string(),
            details: vec![ValidationErrorDetail {
                field: "adminAccessToken".to_string(),
                message: "Invalid Admin Access Token format (must start with shpat_)".to_string(),
            }],
        }));
    }

    if !request.storefront_access_token.starts_with("shp_") {
        return Err(ApiError::bad_request_json(ValidationErrorResponse {
            error: "validation_failed".to_string(),
            details: vec![ValidationErrorDetail {
                field: "storefrontAccessToken".to_string(),
                message: "Invalid Storefront Access Token format (must start with shp_)"
                    .to_string(),
            }],
        }));
    }

    if !request.skip_connection_test {
        let test_result = test_shopify_connection(&request, &state).await;
        if !test_result.success {
            return Err(ApiError::unprocessable_entity_json(GenericErrorResponse {
                error: "shopify_connection_failed".to_string(),
                message: "Failed to connect to Shopify. Please verify your credentials."
                    .to_string(),
            }));
        }
    }

    save_shopify_config(&state.pool, &realm_id, &request).await?;

    let webhook_endpoint = generate_webhook_endpoint(&state, &realm_id);

    let response = ShopifyConfigResponse {
        platform: "shopify".to_string(),
        shop_domain: request.shop_domain,
        admin_access_token: mask_token(&request.admin_access_token),
        storefront_access_token: mask_token(&request.storefront_access_token),
        app_client_secret: mask_token(&request.app_client_secret),
        api_version: request.api_version,
        webhook_subscription_mode: request.webhook_subscription_mode,
        timeout: request.timeout,
        webhook_endpoint,
        created_at: chrono::Utc::now().to_rfc3339(),
        last_updated: chrono::Utc::now().to_rfc3339(),
        enabled: true,
    };

    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/api/third/pay/{realmId}/providers/shopify",
    params(
        ("realmId" = String, Path, description = "Realm UUID")
    ),
    responses(
        (status = 200, description = "Configuration retrieved successfully", body = ShopifyConfigResponse),
        (status = 401, description = "Unauthorized - No valid authentication token"),
        (status = 403, description = "Forbidden - Realm Admin permission required"),
        (status = 404, description = "Configuration not found")
    ),
    tag = "billing.payment-providers",
    operation_id = "get_shopify_config"
)]
pub async fn get_shopify_config(
    State(state): State<AppState>,
    Path(realm_id): Path<String>,
    Extension(identity): Extension<Identity>,
) -> Result<Json<ShopifyConfigResponse>, ApiError> {
    crate::handlers::require_billing_permission(&state, &identity, &realm_id, "view").await?;

    let config = get_shopify_config_internal(&state.pool, &realm_id, &state.public_base_url)
        .await?
        .ok_or_else(|| ApiError::not_found("Configuration not found"))?;

    Ok(Json(config))
}

#[utoipa::path(
    put,
    path = "/api/third/pay/{realmId}/providers/shopify",
    params(
        ("realmId" = String, Path, description = "Realm UUID")
    ),
    request_body = ShopifyConfigUpdateRequest,
    responses(
        (status = 200, description = "Configuration updated successfully", body = ShopifyConfigResponse),
        (status = 400, description = "Validation failed", body = ValidationErrorResponse),
        (status = 401, description = "Unauthorized - No valid authentication token"),
        (status = 403, description = "Forbidden - Realm Admin permission required"),
        (status = 404, description = "Configuration not found"),
        (status = 422, description = "Shopify connection test failed", body = GenericErrorResponse)
    ),
    tag = "billing.payment-providers",
    operation_id = "update_shopify_config"
)]
pub async fn update_shopify_config(
    State(state): State<AppState>,
    Path(realm_id): Path<String>,
    Extension(identity): Extension<Identity>,
    Json(request): Json<ShopifyConfigUpdateRequest>,
) -> Result<Json<ShopifyConfigResponse>, ApiError> {
    crate::handlers::require_billing_permission(&state, &identity, &realm_id, "manage").await?;

    validate_request(&request).map_err(ApiError::bad_request_json)?;

    let _existing = get_shopify_config_internal(&state.pool, &realm_id, &state.public_base_url)
        .await?
        .ok_or_else(|| ApiError::not_found("Configuration not found"))?;

    update_shopify_config_internal(&state.pool, &realm_id, &request).await?;

    let config = get_shopify_config_internal(&state.pool, &realm_id, &state.public_base_url)
        .await?
        .ok_or_else(|| ApiError::internal("Shopify configuration not found after update"))?;

    Ok(Json(config))
}

#[utoipa::path(
    delete,
    path = "/api/third/pay/{realmId}/providers/shopify",
    params(
        ("realmId" = String, Path, description = "Realm UUID")
    ),
    responses(
        (status = 204, description = "Configuration deleted successfully"),
        (status = 401, description = "Unauthorized - No valid authentication token"),
        (status = 403, description = "Forbidden - Realm Admin permission required"),
        (status = 404, description = "Configuration not found"),
        (status = 409, description = "Active subscriptions exist", body = GenericErrorResponse)
    ),
    tag = "billing.payment-providers",
    operation_id = "delete_shopify_config"
)]
pub async fn delete_shopify_config(
    State(state): State<AppState>,
    Path(realm_id): Path<String>,
    Extension(identity): Extension<Identity>,
) -> Result<StatusCode, ApiError> {
    crate::handlers::require_billing_permission(&state, &identity, &realm_id, "manage").await?;

    let _existing = get_shopify_config_internal(&state.pool, &realm_id, &state.public_base_url)
        .await?
        .ok_or_else(|| ApiError::not_found("Configuration not found"))?;

    let active_count = count_active_shopify_subscriptions(&state.pool, &realm_id).await?;
    if active_count > 0 {
        return Err(ApiError::conflict_json(GenericErrorResponse {
            error: "active_subscriptions_exist".to_string(),
            message: format!(
                "Cannot delete payment provider with {} active subscription(s)",
                active_count
            ),
        }));
    }

    delete_shopify_config_internal(&state.pool, &realm_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/third/pay/{realmId}/providers/shopify/test",
    params(
        ("realmId" = String, Path, description = "Realm UUID")
    ),
    request_body = TestConnectionRequest,
    responses(
        (status = 200, description = "Test completed", body = TestConnectionResponse),
        (status = 400, description = "Validation failed"),
        (status = 401, description = "Unauthorized - No valid authentication token"),
        (status = 403, description = "Forbidden - Realm Admin permission required"),
        (status = 422, description = "Test failed", body = TestConnectionResponse)
    ),
    tag = "billing.payment-providers",
    operation_id = "test_shopify_connection"
)]
pub async fn test_shopify_connection_endpoint(
    State(state): State<AppState>,
    Path(realm_id): Path<String>,
    Extension(identity): Extension<Identity>,
    Json(request): Json<TestConnectionRequest>,
) -> Result<Json<TestConnectionResponse>, ApiError> {
    crate::handlers::require_billing_permission(&state, &identity, &realm_id, "manage").await?;

    validate_request(&request).map_err(ApiError::bad_request_json)?;

    let config_request = ShopifyConfigRequest {
        shop_domain: request.shop_domain,
        admin_access_token: request.admin_access_token,
        storefront_access_token: request.storefront_access_token,
        app_client_secret: "test_secret".to_string(),
        api_version: "2024-01".to_string(),
        webhook_subscription_mode: "admin_api".to_string(),
        timeout: 30,
        skip_connection_test: false,
    };

    let result = test_shopify_connection(&config_request, &state).await;

    Ok(Json(result))
}

// ============================================================================
// Internal Helper Functions
// ============================================================================

async fn get_shopify_config_internal(
    db: &PgPool,
    realm_id: &str,
    public_base_url: &str,
) -> Result<Option<ShopifyConfigResponse>, ApiError> {
    let rows = sqlx::query(
        r#"
        SELECT
            rc.config_value,
            rc.config_key,
            rc.created_at,
            rc.updated_at,
            rc.enabled
        FROM realm_config rc
        WHERE rc.realm_id = $1 AND rc.config_type = 'shopify'
        "#,
    )
    .bind(realm_id)
    .fetch_all(db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if rows.is_empty() {
        return Ok(None);
    }

    let mut config = ShopifyConfigResponse {
        platform: "shopify".to_string(),
        shop_domain: String::new(),
        admin_access_token: String::new(),
        storefront_access_token: String::new(),
        app_client_secret: String::new(),
        api_version: "2024-01".to_string(),
        webhook_subscription_mode: "admin_api".to_string(),
        timeout: 30,
        webhook_endpoint: String::new(),
        created_at: String::new(),
        last_updated: String::new(),
        enabled: true,
    };

    let mut created_at: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut updated_at: Option<chrono::DateTime<chrono::Utc>> = None;

    for row in rows {
        let key: String = row.get("config_key");
        let value: String = row.get("config_value");
        let row_created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
        let row_updated_at: chrono::DateTime<chrono::Utc> = row.get("updated_at");
        let row_enabled: bool = row.get("enabled");

        created_at = Some(created_at.map_or(row_created_at, |current| current.min(row_created_at)));
        updated_at = Some(updated_at.map_or(row_updated_at, |current| current.max(row_updated_at)));
        config.enabled &= row_enabled;

        match key.as_str() {
            "shop_domain" => config.shop_domain = value,
            "admin_access_token" => {
                config.admin_access_token = mask_token(&value);
            }
            "storefront_access_token" => {
                config.storefront_access_token = mask_token(&value);
            }
            "app_client_secret" => {
                config.app_client_secret = mask_token(&value);
            }
            "api_version" => config.api_version = value,
            "webhook_subscription_mode" => config.webhook_subscription_mode = value,
            "timeout" => {
                config.timeout = value.parse().unwrap_or(30);
            }
            _ => {}
        }
    }

    config.webhook_endpoint = format!(
        "{}/api/third/pay/{}/shopify/webhooks",
        public_base_url.trim_end_matches('/'),
        realm_id
    );
    config.created_at = created_at.unwrap_or_else(chrono::Utc::now).to_rfc3339();
    config.last_updated = updated_at.unwrap_or_else(chrono::Utc::now).to_rfc3339();

    Ok(Some(config))
}

async fn save_shopify_config(
    db: &PgPool,
    realm_id: &str,
    request: &ShopifyConfigRequest,
) -> Result<(), ApiError> {
    let timeout_str = request.timeout.to_string();

    let config_items = vec![
        ("shop_domain", &request.shop_domain, false),
        ("admin_access_token", &request.admin_access_token, true),
        (
            "storefront_access_token",
            &request.storefront_access_token,
            true,
        ),
        ("app_client_secret", &request.app_client_secret, true),
        ("api_version", &request.api_version, false),
        (
            "webhook_subscription_mode",
            &request.webhook_subscription_mode,
            false,
        ),
        ("timeout", &timeout_str, false),
    ];

    let mut tx = db
        .begin()
        .await
        .map_err(|e| ApiError::internal(format!("Failed to begin transaction: {}", e)))?;

    for (key, value, is_secret) in config_items {
        let result = sqlx::query(
            r#"
            INSERT INTO realm_config (id, realm_id, config_type, config_key, config_value, is_secret, created_at, updated_at)
            VALUES ($1, $2, 'shopify', $3, $4, $5, NOW(), NOW())
            ON CONFLICT (realm_id, config_type, config_key) DO NOTHING
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(realm_id)
        .bind(key)
        .bind(value)
        .bind(is_secret)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to save config: {}", e)))?;

        if result.rows_affected() == 0 {
            tracing::warn!(realm_id = %realm_id, key = %key, "Config key already exists, skipping");
        }
    }

    tx.commit()
        .await
        .map_err(|e| ApiError::internal(format!("Failed to commit transaction: {}", e)))?;
    Ok(())
}

async fn update_shopify_config_internal(
    db: &PgPool,
    realm_id: &str,
    request: &ShopifyConfigUpdateRequest,
) -> Result<(), ApiError> {
    let mut updates: Vec<(&str, String)> = vec![];

    if let Some(shop_domain) = &request.shop_domain {
        updates.push(("shop_domain", shop_domain.clone()));
    }
    if let Some(admin_access_token) = &request.admin_access_token {
        updates.push(("admin_access_token", admin_access_token.clone()));
    }
    if let Some(storefront_access_token) = &request.storefront_access_token {
        updates.push(("storefront_access_token", storefront_access_token.clone()));
    }
    if let Some(app_client_secret) = &request.app_client_secret {
        updates.push(("app_client_secret", app_client_secret.clone()));
    }
    if let Some(api_version) = &request.api_version {
        updates.push(("api_version", api_version.clone()));
    }
    if let Some(webhook_subscription_mode) = &request.webhook_subscription_mode {
        updates.push((
            "webhook_subscription_mode",
            webhook_subscription_mode.clone(),
        ));
    }
    if let Some(timeout) = request.timeout {
        updates.push(("timeout", timeout.to_string()));
    }

    let mut tx = db
        .begin()
        .await
        .map_err(|e| ApiError::internal(format!("Failed to begin transaction: {}", e)))?;

    for (key, value) in updates {
        sqlx::query(
            r#"
            UPDATE realm_config
            SET config_value = $1, updated_at = NOW()
            WHERE realm_id = $2
              AND config_type = 'shopify'
              AND config_key = $3
            "#,
        )
        .bind(&value)
        .bind(realm_id)
        .bind(key)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to update config: {}", e)))?;
    }

    tx.commit()
        .await
        .map_err(|e| ApiError::internal(format!("Failed to commit transaction: {}", e)))?;
    Ok(())
}

async fn delete_shopify_config_internal(db: &PgPool, realm_id: &str) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        DELETE FROM realm_config
        WHERE realm_id = $1
          AND config_type = 'shopify'
        "#,
    )
    .bind(realm_id)
    .execute(db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to delete config: {}", e)))?;

    Ok(())
}

async fn count_active_shopify_subscriptions(db: &PgPool, realm_id: &str) -> Result<i64, ApiError> {
    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM subscription s
        JOIN shopify_subscription_binding ssb ON s.external_subscription_id = ssb.contract_id
        WHERE s.realm_id = $1
          AND s.payment_provider = 'shopify'
          AND s.status = 'active'
        "#,
    )
    .bind(realm_id)
    .fetch_one(db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to count subscriptions: {}", e)))?;

    Ok(count)
}

async fn test_shopify_connection(
    request: &ShopifyConfigRequest,
    _state: &AppState,
) -> TestConnectionResponse {
    let admin_client = ShopifyAdminClient::new(
        request.shop_domain.clone(),
        request.admin_access_token.clone(),
        request.api_version.clone(),
    );
    let storefront_client = ShopifyStorefrontClient::new(
        request.shop_domain.clone(),
        request.storefront_access_token.clone(),
    );

    let (admin_result, storefront_result) = tokio::join!(
        admin_client.test_connection(),
        storefront_client.test_connection()
    );

    let (admin_api_connection, admin_error, admin_ok) =
        map_shopify_connection_result("Admin API", admin_result);
    let (storefront_api_connection, storefront_error, storefront_ok) =
        map_shopify_connection_result("Storefront API", storefront_result);

    let mut errors = Vec::new();
    if let Some(error) = admin_error {
        errors.push(error);
    }
    if let Some(error) = storefront_error {
        errors.push(error);
    }

    TestConnectionResponse {
        success: admin_ok && storefront_ok,
        results: TestConnectionResults {
            admin_api_connection,
            storefront_api_connection,
            shop_access: if admin_ok { "success" } else { "failed" }.to_string(),
        },
        errors,
    }
}

fn map_shopify_connection_result(
    target: &str,
    result: Result<bool, herald_core::infrastructure::shopify::ShopifyClientError>,
) -> (String, Option<String>, bool) {
    match result {
        Ok(true) => ("success".to_string(), None, true),
        Ok(false) => (
            "failed".to_string(),
            Some(format!("{} authentication failed", target)),
            false,
        ),
        Err(error) => (
            "failed".to_string(),
            Some(format!("{}: {}", target, error)),
            false,
        ),
    }
}

fn generate_webhook_endpoint(state: &AppState, realm_id: &str) -> String {
    format!(
        "{}/api/third/pay/{}/shopify/webhooks",
        state.public_base_url.trim_end_matches('/'),
        realm_id
    )
}

fn mask_token(token: &str) -> String {
    if token.len() <= 10 {
        return "****".to_string();
    }

    let prefix = &token[..5];
    let suffix = &token[token.len() - 3..];
    format!("{}***{}", prefix, suffix)
}

pub async fn get_stripe_config_for_providers(
    db: &PgPool,
    realm_id: &str,
) -> Result<Option<PaymentProviderInfo>, ApiError> {
    let rows = sqlx::query(
        r#"
        SELECT config_key, config_value, created_at, updated_at, enabled
        FROM realm_config
        WHERE realm_id = $1 AND config_type = 'stripe'
        "#,
    )
    .bind(realm_id)
    .fetch_all(db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to load Stripe configuration: {}", e);
        ApiError::internal(format!("Database error: {}", e))
    })?;

    if rows.is_empty() {
        return Ok(None);
    }

    let mut last_updated: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut enabled = true;
    let mut has_config = false;

    for row in &rows {
        let config_key: String = row.get("config_key");
        let row_updated: chrono::DateTime<chrono::Utc> = row.get("updated_at");
        let row_enabled: bool = row.get("enabled");
        last_updated = Some(last_updated.map_or(row_updated, |current| current.max(row_updated)));
        enabled &= row_enabled;

        if config_key == "publishable_key" {
            has_config = true;
        }
    }

    if !has_config {
        return Ok(None);
    }

    Ok(Some(PaymentProviderInfo {
        platform: "stripe".to_string(),
        shop_domain: None,
        api_version: None,
        webhook_endpoint: Some("Stripe webhooks configured".to_string()),
        last_updated: last_updated.map(|dt| dt.to_rfc3339()),
        enabled,
    }))
}

pub async fn get_creem_config_for_providers(
    db: &PgPool,
    realm_id: &str,
) -> Result<Option<PaymentProviderInfo>, ApiError> {
    let rows = sqlx::query(
        r#"
        SELECT config_key, config_value, created_at, updated_at, enabled
        FROM realm_config
        WHERE realm_id = $1 AND config_type = 'creem'
        "#,
    )
    .bind(realm_id)
    .fetch_all(db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to load Creem configuration: {}", e);
        ApiError::internal(format!("Database error: {}", e))
    })?;

    if rows.is_empty() {
        return Ok(None);
    }

    let mut last_updated: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut enabled = true;
    let mut has_config = false;

    for row in &rows {
        let config_key: String = row.get("config_key");
        let row_updated: chrono::DateTime<chrono::Utc> = row.get("updated_at");
        let row_enabled: bool = row.get("enabled");
        last_updated = Some(last_updated.map_or(row_updated, |current| current.max(row_updated)));
        enabled &= row_enabled;

        if config_key == "api_key" {
            has_config = true;
        }
    }

    if !has_config {
        return Ok(None);
    }

    Ok(Some(PaymentProviderInfo {
        platform: "creem".to_string(),
        shop_domain: None,
        api_version: None,
        webhook_endpoint: Some("Creem webhooks configured".to_string()),
        last_updated: last_updated.map(|dt| dt.to_rfc3339()),
        enabled,
    }))
}
