use axum::{
    Json,
    extract::{Extension, Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use chrono::Utc;
use sqlx::Row;
use uuid::Uuid;

use crate::payment_email::formal_payment_email;
use crate::types::{
    CancelSubscriptionRequest,
    CancelSubscriptionResponse,
    CreateCheckoutResponse,
    CreateCheckoutSessionRequest,
    // Subscription types
    SubscriptionDetailResponse,
    SubscriptionListItemResponse,
    SubscriptionListQuery,
    SubscriptionListResponse,
};
use crate::webhooks::verify_webhook_signature;

fn subscription_to_response(sub: &Subscription) -> SubscriptionDetailResponse {
    SubscriptionDetailResponse {
        id: sub.id,
        client_app_id: sub.client_app_id,
        entitlement_key: sub.entitlement_key.clone(),
        external_price_id: sub.external_price_id.clone(),
        payment_provider: sub.payment_provider.clone(),
        status: sub.status.as_str().to_string(),
        current_period_start: sub.current_period_start.map(|dt| dt.to_rfc3339()),
        current_period_end: sub.current_period_end.map(|dt| dt.to_rfc3339()),
        cancel_at: sub.cancel_at.map(|dt| dt.to_rfc3339()),
        cancel_at_period_end: Some(sub.cancel_at_period_end),
        provider_metadata: sub.provider_metadata.clone(),
        synced_at: sub.synced_at.map(|dt| dt.to_rfc3339()),
        created_at: sub.created_at.to_rfc3339(),
        updated_at: sub.updated_at.to_rfc3339(),
    }
}
use herald_api_base::application::http::server::api_entities::{ApiError, ErrorResponse};
use herald_api_base::application::http::state::AppState;
// Import the trait and types from herald_core
use herald_core::domain::authentication::Identity;
use herald_core::domain::authorization::PermissionService;
use herald_core::domain::billing::{
    ACTOR_WEBHOOK, BillingRepository, Subscription, SubscriptionHistoryService, SubscriptionStatus,
};
use herald_core::domain::common::entities::app_errors::CoreError;
use herald_core::domain::points::PointsRepository;
use herald_core::infrastructure::creem::{
    CreateCheckoutRequest as CreemCreateCheckoutRequest, CreemClient, CreemDispute, CreemRefund,
    CreemSubscription, CreemWebhookEvent,
};
use herald_core::infrastructure::stripe::{
    CreateCheckoutRequest as StripeCreateCheckoutRequest, StripeClient,
};

// ============================================================================
// Realm-specific Client Helpers
// ============================================================================

/// Get Creem client for a specific realm
async fn get_creem_client_for_realm(
    realm_id: &str,
    state: &AppState,
) -> Result<CreemClient, ApiError> {
    let api_key = sqlx::query_scalar::<_, String>(
        "SELECT config_value
         FROM realm_config
         WHERE realm_id = $1 AND config_type = 'creem' AND config_key = 'api_key' AND enabled = true
         LIMIT 1",
    )
    .bind(realm_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to load Creem API key from database: {}", e);
        ApiError::internal(format!("Database error: {}", e))
    })?
    .ok_or_else(|| {
        tracing::error!("No Creem API key found for realm: {}", realm_id);
        ApiError::internal(format!("Creem not configured for realm: {}", realm_id))
    })?;

    let timeout = sqlx::query_scalar::<_, String>(
        "SELECT config_value
         FROM realm_config
         WHERE realm_id = $1 AND config_type = 'creem' AND config_key = 'timeout' AND enabled = true
         LIMIT 1",
    )
    .bind(realm_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to load Creem timeout from database: {}", e);
        ApiError::internal(format!("Database error: {}", e))
    })?
    .and_then(|s| s.parse::<u64>().ok())
    .unwrap_or(30);

    tracing::info!("Loaded Creem config from database for realm: {}", realm_id);

    CreemClient::new(api_key, timeout).map_err(ApiError::from)
}

/// Get Stripe client for a realm
async fn get_stripe_client_for_realm(
    realm_id: &str,
    state: &AppState,
) -> Result<StripeClient, ApiError> {
    let api_key = sqlx::query_scalar::<_, String>(
        "SELECT config_value
         FROM realm_config
         WHERE realm_id = $1 AND config_type = 'stripe' AND config_key = 'api_key' AND enabled = true
         LIMIT 1",
    )
    .bind(realm_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to load Stripe API key from database: {}", e);
        ApiError::internal(format!("Database error: {}", e))
    })?
    .ok_or_else(|| {
        tracing::error!("No Stripe API key found for realm: {}", realm_id);
        ApiError::internal(format!("Stripe not configured for realm: {}", realm_id))
    })?;

    let timeout = sqlx::query_scalar::<_, String>(
        "SELECT config_value
         FROM realm_config
         WHERE realm_id = $1 AND config_type = 'stripe' AND config_key = 'timeout' AND enabled = true
         LIMIT 1",
    )
    .bind(realm_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to load Stripe timeout from database: {}", e);
        ApiError::internal(format!("Database error: {}", e))
    })?
    .and_then(|s| s.parse::<u64>().ok())
    .unwrap_or(30);

    tracing::info!("Loaded Stripe config from database for realm: {}", realm_id);

    Ok(StripeClient::new(api_key, timeout)?)
}

// ============================================================================
// Permission Check Helper
// ============================================================================

/// Check billing permissions for a realm
///
/// This helper function:
/// 1. Verifies realm boundary (identity's realm must match requested realm)
/// 2. Checks business permissions (billing.view or billing.manage)
pub async fn require_billing_permission(
    state: &AppState,
    identity: &Identity,
    realm_id: &str,
    action: &str,
) -> Result<(), ApiError> {
    let user_id = identity.user_id();
    let identity_realm_id = identity.realm_id();

    // 1. Realm boundary check
    if identity_realm_id != realm_id {
        return Err(ApiError::forbidden(format!(
            "Access denied: identity realm '{}' does not match requested realm '{}'",
            identity_realm_id, realm_id
        )));
    }

    // 2. Business permission check
    let has_permission = state
        .permission_checker
        .check_permission(realm_id, &user_id, "billing", action)
        .await
        .map_err(|e| {
            tracing::error!(
                user_id = %user_id,
                realm_id = %realm_id,
                error = %e,
                "Failed to check billing.{} permission",
                action
            );
            ApiError::internal("Failed to check permission")
        })?;

    if !has_permission {
        return Err(ApiError::forbidden(format!(
            "Insufficient permissions: billing.{} required",
            action
        )));
    }

    Ok(())
}

// ============================================================================
// Subscription Handlers
// ============================================================================

/// List subscriptions for a realm
/// TODO: Migrate raw SQL to BillingRepository trait method for schema-safety.
#[utoipa::path(
    get,
    path = "/api/bill/{realmId}/subscriptions",
    tag = "billing",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    responses(
        (status = 200, description = "Subscriptions listed successfully", body = SubscriptionListResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_subscriptions(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Query(query): Query<SubscriptionListQuery>,
) -> Result<Json<SubscriptionListResponse>, ApiError> {
    tracing::info!("Listing subscriptions for realm: {}", realm_id);

    require_billing_permission(&state, &identity, &realm_id, "view").await?;

    // Build query conditions
    let mut conditions = vec!["realm_id = $1".to_string()];
    let mut param_idx = 2u32;

    let entitlement_key_param;
    let status_param;
    let payment_provider_param;

    if let Some(ref ek) = query.entitlement_key {
        conditions.push(format!("entitlement_key = ${}", param_idx));
        entitlement_key_param = Some(ek.clone());
        param_idx += 1;
    } else {
        entitlement_key_param = None;
    }

    if let Some(ref s) = query.status {
        conditions.push(format!("status = ${}", param_idx));
        status_param = Some(s.clone());
        param_idx += 1;
    } else {
        status_param = None;
    }

    if let Some(ref pp) = query.payment_provider {
        conditions.push(format!("payment_provider = ${}", param_idx));
        payment_provider_param = Some(pp.clone());
        param_idx += 1;
    } else {
        payment_provider_param = None;
    }

    let where_clause = conditions.join(" AND ");

    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20);
    let offset = (page - 1) * page_size;

    // Count query
    let count_sql = format!(
        "SELECT COUNT(*) as count FROM subscription WHERE {}",
        where_clause
    );
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql).bind(&realm_id);
    if let Some(ref ek) = entitlement_key_param {
        count_query = count_query.bind(ek);
    }
    if let Some(ref s) = status_param {
        count_query = count_query.bind(s);
    }
    if let Some(ref pp) = payment_provider_param {
        count_query = count_query.bind(pp);
    }
    let total = count_query.fetch_one(&state.pool).await.map_err(|e| {
        tracing::error!(realm_id = %realm_id, error = %e, "Failed to count subscriptions");
        ApiError::internal("Failed to count subscriptions".to_string())
    })?;

    // Data query
    let data_sql = format!(
        "SELECT id, client_app_id, entitlement_key, external_price_id, payment_provider, status, \
         current_period_start, current_period_end, synced_at, created_at, updated_at \
         FROM subscription WHERE {} ORDER BY created_at DESC LIMIT ${} OFFSET ${}",
        where_clause,
        param_idx,
        param_idx + 1
    );
    let mut data_query = sqlx::query(&data_sql).bind(&realm_id);
    if let Some(ref ek) = entitlement_key_param {
        data_query = data_query.bind(ek);
    }
    if let Some(ref s) = status_param {
        data_query = data_query.bind(s);
    }
    if let Some(ref pp) = payment_provider_param {
        data_query = data_query.bind(pp);
    }
    // Bind LIMIT and OFFSET
    data_query = data_query.bind(page_size as i64).bind(offset as i64);

    let rows = data_query.fetch_all(&state.pool).await.map_err(|e| {
        tracing::error!(realm_id = %realm_id, error = %e, "Failed to list subscriptions");
        ApiError::internal("Failed to list subscriptions".to_string())
    })?;

    let items: Vec<SubscriptionListItemResponse> = rows
        .iter()
        .map(|row| SubscriptionListItemResponse {
            id: row.get("id"),
            client_app_id: row.get("client_app_id"),
            entitlement_key: row.get("entitlement_key"),
            external_price_id: row.get("external_price_id"),
            payment_provider: row.get("payment_provider"),
            status: row.get::<String, _>("status"),
            current_period_start: row
                .get::<Option<chrono::DateTime<chrono::Utc>>, _>("current_period_start")
                .map(|dt| dt.to_rfc3339()),
            current_period_end: row
                .get::<Option<chrono::DateTime<chrono::Utc>>, _>("current_period_end")
                .map(|dt| dt.to_rfc3339()),
            synced_at: row
                .get::<Option<chrono::DateTime<chrono::Utc>>, _>("synced_at")
                .map(|dt| dt.to_rfc3339()),
            created_at: row
                .get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                .to_rfc3339(),
            updated_at: row
                .get::<chrono::DateTime<chrono::Utc>, _>("updated_at")
                .to_rfc3339(),
        })
        .collect();

    Ok(Json(SubscriptionListResponse { items, total }))
}

/// Get a specific subscription
#[utoipa::path(
    get,
    path = "/api/bill/{realmId}/subscriptions/{subscriptionId}",
    tag = "billing",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("subscriptionId" = Uuid, Path, description = "Subscription ID")
    ),
    responses(
        (status = 200, description = "Subscription found", body = SubscriptionDetailResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Subscription not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_subscription(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, subscription_id)): Path<(String, Uuid)>,
) -> Result<Json<SubscriptionDetailResponse>, ApiError> {
    tracing::info!(
        "Getting subscription {} for realm: {}",
        subscription_id,
        realm_id
    );

    require_billing_permission(&state, &identity, &realm_id, "view").await?;

    let subscription = state
        .billing_repository
        .find_subscription_by_id(subscription_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Subscription not found"))?;

    if subscription.realm_id != realm_id {
        return Err(ApiError::not_found("Subscription not found"));
    }

    Ok(Json(subscription_to_response(&subscription)))
}

/// Get subscription for a client app
#[utoipa::path(
    get,
    path = "/api/bill/{realmId}/client/{clientAppId}/subscription",
    tag = "billing",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("clientAppId" = Uuid, Path, description = "Client App ID")
    ),
    responses(
        (status = 200, description = "Subscription found", body = SubscriptionDetailResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Subscription not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_subscription_for_client_app(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, client_app_id)): Path<(String, Uuid)>,
) -> Result<Json<SubscriptionDetailResponse>, ApiError> {
    tracing::info!(
        "Getting subscription for client app {} in realm: {}",
        client_app_id,
        realm_id
    );

    require_billing_permission(&state, &identity, &realm_id, "view").await?;

    let subscription = state
        .billing_repository
        .find_subscription_by_client_app_id(client_app_id)
        .await?
        .ok_or_else(|| CoreError::SubscriptionNotFound(client_app_id.to_string()))?;

    Ok(Json(subscription_to_response(&subscription)))
}

/// Cancel subscription for a client app
#[utoipa::path(
    post,
    path = "/api/bill/{realmId}/client/{clientAppId}/subscription/cancel",
    tag = "billing",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("clientAppId" = Uuid, Path, description = "Client App ID")
    ),
    request_body = CancelSubscriptionRequest,
    responses(
        (status = 200, description = "Subscription canceled successfully", body = CancelSubscriptionResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Subscription not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn cancel_subscription_for_client_app(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, client_app_id)): Path<(String, Uuid)>,
    Json(request): Json<CancelSubscriptionRequest>,
) -> Result<Json<CancelSubscriptionResponse>, ApiError> {
    tracing::info!(
        "Canceling subscription for client app {} in realm: {}, cancel_at_period_end: {}",
        client_app_id,
        realm_id,
        request.cancel_at_period_end
    );

    require_billing_permission(&state, &identity, &realm_id, "manage").await?;

    let subscription = state
        .billing_repository
        .find_subscription_by_client_app_id(client_app_id)
        .await?
        .ok_or_else(|| CoreError::SubscriptionNotFound(client_app_id.to_string()))?;

    let canceled_at = if request.cancel_at_period_end {
        subscription.current_period_end.unwrap_or_else(Utc::now)
    } else {
        Utc::now()
    };

    let updated = state
        .billing_repository
        .cancel_subscription(subscription.id, request.cancel_at_period_end)
        .await?;

    let message = if request.cancel_at_period_end {
        "Subscription will be canceled at the end of the billing period".to_string()
    } else {
        "Subscription canceled immediately".to_string()
    };

    Ok(Json(CancelSubscriptionResponse {
        subscription_id: updated.id.to_string(),
        canceled_at: canceled_at.to_rfc3339(),
        message,
    }))
}

/// Create checkout session for an entitlement
#[utoipa::path(
    post,
    path = "/api/bill/{realmId}/client/{clientAppId}/checkout",
    tag = "billing",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("clientAppId" = Uuid, Path, description = "Client App ID")
    ),
    request_body = CreateCheckoutSessionRequest,
    responses(
        (status = 200, description = "Checkout session created successfully", body = CreateCheckoutResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Entitlement mapping not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_checkout_session(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, client_app_id)): Path<(String, Uuid)>,
    Json(request): Json<CreateCheckoutSessionRequest>,
) -> Result<Json<CreateCheckoutResponse>, ApiError> {
    tracing::info!(
        "Creating checkout session for client app {} with entitlement {} in realm: {}",
        client_app_id,
        request.entitlement_key,
        realm_id
    );

    require_billing_permission(&state, &identity, &realm_id, "manage").await?;

    // Look up entitlement mapping by entitlement_key
    let mapping = state
        .billing_repository
        .find_entitlement_mapping_by_key(&realm_id, &request.entitlement_key)
        .await
        .map_err(|e| {
            tracing::error!(
                entitlement_key = %request.entitlement_key,
                error = %e,
                "Failed to look up entitlement mapping"
            );
            ApiError::internal("Failed to look up entitlement mapping".to_string())
        })?
        .ok_or_else(|| {
            ApiError::not_found(format!(
                "Entitlement mapping not found for key: {}",
                request.entitlement_key
            ))
        })?;

    if !mapping.enabled {
        return Err(ApiError::bad_request(format!(
            "Entitlement mapping '{}' is not enabled",
            request.entitlement_key
        )));
    }

    let payment_provider = request.payment_provider.clone();

    // Verify payment provider matches mapping
    if mapping.payment_provider != payment_provider {
        return Err(ApiError::bad_request(format!(
            "Payment provider '{}' does not match mapping provider '{}'",
            payment_provider, mapping.payment_provider
        )));
    }

    let external_product_id = mapping.external_product_id.clone();
    let user_email = formal_payment_email(&identity);
    if matches!(payment_provider.as_str(), "stripe" | "creem") && user_email.is_none() {
        return Err(ApiError::bad_request(
            "A formal user email is required for this payment provider",
        ));
    }

    // Route to appropriate payment provider
    let entitlement_key = request.entitlement_key.clone();
    let checkout_url = match payment_provider.as_str() {
        "stripe" => {
            tracing::info!(
                "Creating Stripe checkout session for entitlement: {}",
                request.entitlement_key
            );

            let stripe_client = get_stripe_client_for_realm(&realm_id, &state).await?;

            let stripe_request = StripeCreateCheckoutRequest {
                client_app_id,
                plan_id: mapping.id, // Use mapping ID as reference
                user_id: Uuid::parse_str(&identity.user_id()).ok(),
                customer_email: user_email.clone(),
                success_url: format!("{}/billing/success", state.public_base_url),
                cancel_url: format!("{}/billing/cancel", state.public_base_url),
                billing_period: mapping.billing_period.clone().unwrap_or_default(),
                trial_days: None,
                price_amount: {
                    let price = mapping
                        .provider_product_info
                        .as_ref()
                        .and_then(|info| info.get("price"))
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);
                    if price <= 0 {
                        return Err(ApiError::bad_request(
                            "Checkout requires a positive price. Configure a valid price in the entitlement mapping's provider_product_info.".to_string(),
                        ));
                    }
                    price
                },
                currency: mapping
                    .provider_product_info
                    .as_ref()
                    .and_then(|info| info.get("currency"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("usd")
                    .to_string(),
                plan_name: entitlement_key.clone(),
                realm_id: realm_id.clone(),
                webhook_url: Some(format!(
                    "{}/api/third/pay/{}/stripe/webhooks",
                    state.public_base_url, realm_id
                )),
                metadata: Some({
                    let mut map = std::collections::HashMap::new();
                    map.insert("entitlementKey".to_string(), entitlement_key.clone());
                    map
                }),
            };

            let session = stripe_client
                .create_checkout_session(&stripe_request)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to create Stripe checkout session: {}", e);
                    ApiError::internal(format!("Failed to create checkout session: {}", e))
                })?;

            session.url
        }
        "creem" => {
            tracing::info!(
                "Creating Creem checkout session for entitlement: {}",
                request.entitlement_key
            );

            let creem_client = get_creem_client_for_realm(&realm_id, &state).await?;

            let creem_request = CreemCreateCheckoutRequest {
                product_id: external_product_id,
                success_url: Some(format!("{}/billing/success", state.public_base_url)),
                customer: herald_core::infrastructure::creem::CreemCheckoutCustomer {
                    email: user_email.clone(),
                },
                metadata: {
                    let mut map = std::collections::HashMap::new();
                    map.insert("realmId".to_string(), realm_id.clone());
                    map.insert("userId".to_string(), identity.user_id());
                    map.insert("clientAppId".to_string(), client_app_id.to_string());
                    map.insert("entitlementKey".to_string(), entitlement_key.clone());
                    Some(map)
                },
            };

            let session = creem_client
                .create_checkout_session(&creem_request)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to create Creem checkout session: {}", e);
                    ApiError::internal(format!("Failed to create checkout session: {}", e))
                })?;

            session.checkout_url
        }
        provider => {
            tracing::error!("Unsupported payment provider: {}", provider);
            return Err(ApiError::bad_request(format!(
                "Unsupported payment provider: {}",
                provider
            )));
        }
    };

    tracing::info!(
        "Created checkout session for client app {} in realm {}",
        client_app_id,
        realm_id
    );

    let checkout_id = Uuid::now_v7();

    Ok(Json(CreateCheckoutResponse {
        checkout_url,
        checkout_id,
    }))
}

// ============================================================================
// Creem Webhook Handlers (BE-D04 will fully adapt these)
// ============================================================================

/// Convert a billing_period string (e.g. "monthly", "yearly") to days
fn period_to_days(period: &str) -> i64 {
    match period.to_ascii_lowercase().as_str() {
        "yearly" | "annual" | "year" => 365,
        "quarterly" | "quarter" => 90,
        "weekly" | "week" => 7,
        _ => 30, // monthly and unknown default to 30
    }
}

/// Parse Creem subscription status string to SubscriptionStatus enum
fn parse_creem_status(status_str: &str) -> Result<SubscriptionStatus, CoreError> {
    match status_str.to_lowercase().as_str() {
        "active" => Ok(SubscriptionStatus::Active),
        "trialing" => Ok(SubscriptionStatus::Trialing),
        "canceled" => Ok(SubscriptionStatus::Canceled),
        "expired" => Ok(SubscriptionStatus::Expired),
        "incomplete" => Ok(SubscriptionStatus::Incomplete),
        "paused" => Ok(SubscriptionStatus::Paused),
        "past_due" => Ok(SubscriptionStatus::PastDue),
        "scheduled_cancel" => Ok(SubscriptionStatus::ScheduledCancel),
        "dispute" => Ok(SubscriptionStatus::Dispute),
        _ => Err(CoreError::BadRequest(format!(
            "Invalid subscription status: {}",
            status_str
        ))),
    }
}

/// Get webhook secret for a realm
async fn get_webhook_secret(realm_id: &str, pool: &sqlx::PgPool) -> Result<String, CoreError> {
    let secret = sqlx::query_scalar::<_, String>(
        "SELECT config_value
         FROM realm_config
         WHERE realm_id = $1 AND config_type = 'creem' AND config_key = 'webhook_secret' AND enabled = true
         LIMIT 1"
    )
    .bind(realm_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to load webhook secret from database: {}", e);
        CoreError::InternalServerError(format!("Database error: {}", e))
    })?
    .ok_or_else(|| {
        tracing::error!(
            realm_id = %realm_id,
            "Webhook secret not found in database"
        );
        CoreError::InternalServerError(format!(
            "Webhook secret not configured for realm: {}",
            realm_id
        ))
    })?;

    Ok(secret)
}

/// Handle webhook events from Creem payment provider
#[utoipa::path(
    post,
    path = "/billing/webhooks",
    tag = "billing",
    request_body = serde_json::Value,
    responses(
        (status = 200, description = "Webhook processed successfully (idempotent)"),
        (status = 400, description = "Invalid webhook payload or signature format"),
        (status = 500, description = "Internal server error - event not processed")
    )
)]
pub async fn webhook_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: bytes::Bytes,
) -> Result<StatusCode, CoreError> {
    let event: CreemWebhookEvent = serde_json::from_slice(&body)?;
    let realm_id = extract_realm_id(&event)?;

    let signature_header = headers
        .get("creem-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            tracing::error!("Missing or invalid creem-signature header");
            CoreError::InvalidWebhookSignature
        })?;

    let webhook_secret = get_webhook_secret(&realm_id, &state.pool).await?;

    verify_webhook_signature(&body, signature_header, &webhook_secret).map_err(|e| {
        tracing::error!(
            error = %e,
            event_id = %event.id,
            event_type = %event.event_type,
            "Webhook signature verification failed"
        );
        e
    })?;

    tracing::info!(
        "Received webhook event: {} ({}) for realm: {}",
        event.event_type,
        event.id,
        realm_id
    );

    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

    handle_webhook_with_idempotency(&state, &mut tx, event).await?;

    tx.commit()
        .await
        .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

    Ok(StatusCode::OK)
}

/// Handle webhook event with idempotency
async fn handle_webhook_with_idempotency(
    state: &AppState,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    event: CreemWebhookEvent,
) -> Result<(), CoreError> {
    let realm_id = extract_realm_id(&event)?;

    sqlx::query(
        r#"
        INSERT INTO payment_event (id, realm_id, creem_event_id, event_type, subscription_id, payload, processed, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (creem_event_id) DO NOTHING
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(&realm_id)
    .bind(&event.id)
    .bind(&event.event_type)
    .bind(Option::<Uuid>::None)
    .bind(serde_json::to_value(&event)?)
    .bind(false)
    .bind(Utc::now())
    .execute(&mut **tx)
    .await
    .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

    let event_row =
        sqlx::query("SELECT id, processed FROM payment_event WHERE creem_event_id = $1 FOR UPDATE")
            .bind(&event.id)
            .fetch_one(&mut **tx)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

    let payment_event_id: Uuid = event_row
        .try_get("id")
        .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
    let already_processed: bool = event_row
        .try_get("processed")
        .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

    if already_processed {
        tracing::info!(
            event_id = %event.id,
            "Duplicate processed webhook event, skipping"
        );
        return Ok(());
    }

    match event.event_type.as_str() {
        "checkout.completed" => handle_checkout_completed(state, &event).await?,
        "subscription.active" => handle_subscription_active(state, &event, realm_id).await?,
        "subscription.trialing" => handle_subscription_trialing(state, &event, realm_id).await?,
        "subscription.paid" => handle_subscription_paid(state, &event, realm_id).await?,
        "subscription.paused" => handle_subscription_paused(state, &event, realm_id).await?,
        "subscription.canceled" => handle_subscription_canceled(state, &event, realm_id).await?,
        "subscription.expired" => handle_subscription_expired(state, &event, realm_id).await?,
        "subscription.updated" => handle_subscription_updated(state, &event, realm_id).await?,
        "subscription.past_due" => handle_subscription_past_due(state, &event, realm_id).await?,
        "subscription.scheduled_cancel" => {
            handle_subscription_scheduled_cancel(state, &event, realm_id).await?
        }
        "dispute.created" => handle_dispute_created(state, &event, realm_id).await?,
        "refund.created" => handle_refund_created(state, &event, realm_id).await?,
        _ => {
            tracing::info!("Unhandled event type: {}", event.event_type);
        }
    }

    sqlx::query("UPDATE payment_event SET processed = true WHERE id = $1")
        .bind(payment_event_id)
        .execute(&mut **tx)
        .await
        .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

    Ok(())
}

/// Extract realm_id from webhook event metadata
pub(crate) fn extract_realm_id(event: &CreemWebhookEvent) -> Result<String, CoreError> {
    event
        .object
        .get("metadata")
        .and_then(|m: &serde_json::Value| m.get("realmId"))
        .and_then(|id: &serde_json::Value| id.as_str())
        .map(|s: &str| s.to_string())
        .ok_or_else(|| CoreError::BadRequest("Missing realm_id in event metadata".to_string()))
}

/// Type alias for subscription metadata extracted from webhook events
type SubscriptionMetadata = (Option<Uuid>, Option<Uuid>, Option<String>);

/// Extract user_id, client_app_id and entitlement_key from webhook event metadata
pub(crate) fn extract_subscription_metadata(
    event: &CreemWebhookEvent,
) -> Result<SubscriptionMetadata, CoreError> {
    let user_id = event
        .object
        .get("metadata")
        .and_then(|m: &serde_json::Value| m.get("userId"))
        .and_then(|id: &serde_json::Value| id.as_str())
        .and_then(|s: &str| Uuid::parse_str(s).ok());

    let client_app_id = event
        .object
        .get("metadata")
        .and_then(|m: &serde_json::Value| m.get("clientAppId"))
        .and_then(|id: &serde_json::Value| id.as_str())
        .and_then(|s: &str| Uuid::parse_str(s).ok());

    // Extract entitlement_key (replaces planId)
    let entitlement_key = event
        .object
        .get("metadata")
        .and_then(|m: &serde_json::Value| m.get("entitlementKey"))
        .and_then(|v: &serde_json::Value| v.as_str())
        .map(|s: &str| s.to_string());

    Ok((user_id, client_app_id, entitlement_key))
}

/// Handle checkout.completed event
/// DEPRECATED: Legacy Creem webhook handler, not wired to routes.
/// Active handler is in webhook_handlers.rs::handle_checkout_completed.
async fn handle_checkout_completed(
    state: &AppState,
    event: &CreemWebhookEvent,
) -> Result<(), CoreError> {
    tracing::info!("Handling checkout.completed: {}", event.id);

    let realm_id = extract_realm_id(event)?;
    let (user_id, client_app_id, entitlement_key) = extract_subscription_metadata(event)?;

    let creem_sub_id = format!("sub_{}", event.id);
    if let Some(_existing_sub) = state
        .billing_repository
        .find_by_external_subscription_id(&creem_sub_id, "creem")
        .await?
    {
        tracing::info!(
            "Subscription already exists for checkout.completed: {}",
            creem_sub_id
        );
        return Ok(());
    }

    let now = Utc::now();

    // Determine entitlement_key: from metadata, or fallback to provider mapping
    let entitlement_key = match entitlement_key {
        Some(key) if !key.is_empty() => key,
        _ => {
            // Fallback: look up by provider + external_product_id
            let external_product_id = event
                .object
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("default_product");
            match state
                .billing_repository
                .find_entitlement_mapping_by_provider_product(
                    &realm_id,
                    "creem",
                    external_product_id,
                )
                .await?
            {
                Some(mapping) => mapping.entitlement_key,
                None => {
                    return Err(CoreError::BadRequest(format!(
                        "No entitlement mapping found for provider 'creem', product '{}' in realm '{}'. Configure an entitlement mapping before processing webhooks.",
                        external_product_id, realm_id
                    )));
                }
            }
        }
    };

    // Resolve billing period from entitlement mapping
    let period_days = state
        .points_repository
        .find_points_policy_by_entitlement_key(&realm_id, &entitlement_key)
        .await?
        .and_then(|m| m.billing_period.as_deref().map(period_to_days))
        .unwrap_or(30);

    let sub = Subscription {
        id: Uuid::now_v7(),
        realm_id: realm_id.clone(),
        user_id,
        external_subscription_id: creem_sub_id,
        external_product_id: event
            .object
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("default_product")
            .to_string(),
        payment_provider: "creem".to_string(),
        status: SubscriptionStatus::Active,
        entitlement_key,
        external_price_id: None,
        provider_metadata: None,
        synced_at: Some(now),
        client_app_id,
        current_period_start: Some(now),
        current_period_end: Some(now + chrono::Duration::days(period_days)),
        cancel_at_period_end: false,
        cancel_at: None,
        created_at: now,
        updated_at: now,
    };

    let sub = state.billing_repository.create_subscription(sub).await?;

    let history_event = SubscriptionHistoryService::create_subscription_created_event(
        &sub,
        Some(ACTOR_WEBHOOK.to_string()),
    );
    state
        .billing_repository
        .save_history_event(history_event)
        .await?;

    tracing::info!(
        "Subscription created from checkout.completed for realm: {}",
        realm_id
    );

    Ok(())
}

/// Handle subscription.paid event
/// DEPRECATED: Legacy Creem webhook handler, not wired to routes.
/// Active handler is in webhook_handlers.rs::handle_subscription_paid.
async fn handle_subscription_paid(
    state: &AppState,
    event: &CreemWebhookEvent,
    realm_id: String,
) -> Result<(), CoreError> {
    tracing::info!("Handling subscription.paid for realm: {}", realm_id);

    let creem_sub: CreemSubscription =
        serde_json::from_value(event.object.clone()).map_err(|e| {
            CoreError::InternalServerError(format!("Failed to parse subscription object: {}", e))
        })?;

    let existing_sub = state
        .billing_repository
        .find_by_external_subscription_id(&creem_sub.id, "creem")
        .await?;
    let is_new_subscription = existing_sub.is_none();

    let now = Utc::now();
    let (user_id, client_app_id, entitlement_key) = extract_subscription_metadata(event)?;
    let status = parse_creem_status(&creem_sub.status)?;

    if let Some(mut sub) = existing_sub {
        if !sub.status.can_transition_to(&status) {
            tracing::warn!(
                subscription_id = %sub.id,
                from_status = %sub.status.as_str(),
                to_status = %status.as_str(),
                event_id = %event.id,
                "Ignoring invalid subscription status transition"
            );
            return Ok(());
        }

        sub.status = status;
        sub.current_period_start = creem_sub
            .current_period_start
            .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0));
        sub.current_period_end = creem_sub
            .current_period_end
            .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0));
        sub.updated_at = now;
        if let Some(ek) = entitlement_key {
            sub.entitlement_key = ek;
        }

        state.billing_repository.update_subscription(sub).await?;
    } else {
        let entitlement_key = match entitlement_key {
            Some(key) => key,
            None => {
                return Err(CoreError::BadRequest(
                    "Cannot create subscription without entitlement_key. No mapping found."
                        .to_string(),
                ));
            }
        };

        let sub = Subscription {
            id: Uuid::now_v7(),
            realm_id: realm_id.clone(),
            user_id,
            external_subscription_id: creem_sub.id.clone(),
            external_product_id: creem_sub.product.id.clone(),
            payment_provider: "creem".to_string(),
            status,
            entitlement_key,
            external_price_id: None,
            provider_metadata: None,
            synced_at: Some(now),
            client_app_id,
            current_period_start: creem_sub
                .current_period_start
                .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0)),
            current_period_end: creem_sub
                .current_period_end
                .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0)),
            cancel_at_period_end: creem_sub.cancel_at_period_end.unwrap_or(false),
            cancel_at: None,
            created_at: now,
            updated_at: now,
        };

        let sub = state.billing_repository.create_subscription(sub).await?;
        let history_event = SubscriptionHistoryService::create_subscription_created_event(
            &sub,
            Some(ACTOR_WEBHOOK.to_string()),
        );
        state
            .billing_repository
            .save_history_event(history_event)
            .await?;
    }

    tracing::info!("Subscription activated for realm: {}", realm_id);
    let _ = (is_new_subscription, user_id);
    Ok(())
}

/// Handle subscription.canceled event
async fn handle_subscription_canceled(
    state: &AppState,
    event: &CreemWebhookEvent,
    realm_id: String,
) -> Result<(), CoreError> {
    tracing::info!("Handling subscription.canceled for realm: {}", realm_id);

    let creem_sub: CreemSubscription =
        serde_json::from_value(event.object.clone()).map_err(|e| {
            CoreError::InternalServerError(format!("Failed to parse subscription object: {}", e))
        })?;

    if let Some(mut sub) = state
        .billing_repository
        .find_by_external_subscription_id(&creem_sub.id, "creem")
        .await?
    {
        if !sub.status.can_transition_to(&SubscriptionStatus::Canceled) {
            tracing::warn!(
                subscription_id = %sub.id,
                from_status = %sub.status.as_str(),
                to_status = %SubscriptionStatus::Canceled.as_str(),
                event_id = %event.id,
                "Ignoring invalid subscription status transition"
            );
            return Ok(());
        }

        let cancel_at_period_end = creem_sub.cancel_at_period_end.unwrap_or(false);
        let history_event = SubscriptionHistoryService::create_subscription_canceled_event(
            &sub,
            cancel_at_period_end,
            Some(ACTOR_WEBHOOK.to_string()),
        );

        sub.status = SubscriptionStatus::Canceled;
        sub.updated_at = Utc::now();
        state
            .billing_repository
            .update_subscription(sub.clone())
            .await?;
        state
            .billing_repository
            .save_history_event(history_event)
            .await?;
    }

    Ok(())
}

/// Handle subscription.expired event
async fn handle_subscription_expired(
    state: &AppState,
    event: &CreemWebhookEvent,
    realm_id: String,
) -> Result<(), CoreError> {
    tracing::info!("Handling subscription.expired for realm: {}", realm_id);

    let creem_sub: CreemSubscription =
        serde_json::from_value(event.object.clone()).map_err(|e| {
            CoreError::InternalServerError(format!("Failed to parse subscription object: {}", e))
        })?;

    if let Some(mut sub) = state
        .billing_repository
        .find_by_external_subscription_id(&creem_sub.id, "creem")
        .await?
    {
        if !sub.status.can_transition_to(&SubscriptionStatus::Expired) {
            tracing::warn!(
                subscription_id = %sub.id,
                from_status = %sub.status.as_str(),
                to_status = %SubscriptionStatus::Expired.as_str(),
                event_id = %event.id,
                "Ignoring invalid subscription status transition"
            );
            return Ok(());
        }

        sub.status = SubscriptionStatus::Expired;
        sub.updated_at = Utc::now();
        state.billing_repository.update_subscription(sub).await?;
    }

    Ok(())
}

/// Handle subscription.active event
async fn handle_subscription_active(
    state: &AppState,
    event: &CreemWebhookEvent,
    realm_id: String,
) -> Result<(), CoreError> {
    tracing::info!("Handling subscription.active for realm: {}", realm_id);
    handle_subscription_update_internal(state, event, realm_id, SubscriptionStatus::Active).await
}

/// Handle subscription.trialing event
async fn handle_subscription_trialing(
    state: &AppState,
    event: &CreemWebhookEvent,
    realm_id: String,
) -> Result<(), CoreError> {
    tracing::info!("Handling subscription.trialing for realm: {}", realm_id);
    handle_subscription_update_internal(state, event, realm_id, SubscriptionStatus::Trialing).await
}

/// Handle subscription.paused event
async fn handle_subscription_paused(
    state: &AppState,
    event: &CreemWebhookEvent,
    realm_id: String,
) -> Result<(), CoreError> {
    tracing::info!("Handling subscription.paused for realm: {}", realm_id);
    handle_subscription_update_internal(state, event, realm_id, SubscriptionStatus::Paused).await
}

/// Handle subscription.updated event
async fn handle_subscription_updated(
    state: &AppState,
    event: &CreemWebhookEvent,
    realm_id: String,
) -> Result<(), CoreError> {
    tracing::info!("Handling subscription.updated for realm: {}", realm_id);

    let creem_sub: CreemSubscription =
        serde_json::from_value(event.object.clone()).map_err(|e| {
            CoreError::InternalServerError(format!("Failed to parse subscription object: {}", e))
        })?;

    let status = parse_creem_status(&creem_sub.status)?;
    handle_subscription_update_internal(state, event, realm_id, status).await
}

/// Handle subscription.past_due event
async fn handle_subscription_past_due(
    state: &AppState,
    event: &CreemWebhookEvent,
    realm_id: String,
) -> Result<(), CoreError> {
    tracing::warn!("Handling subscription.past_due for realm: {}", realm_id);

    let creem_sub: CreemSubscription = serde_json::from_value(event.object.clone())?;

    if let Some(mut sub) = state
        .billing_repository
        .find_by_external_subscription_id(&creem_sub.id, "creem")
        .await?
    {
        if !sub.status.can_transition_to(&SubscriptionStatus::PastDue) {
            tracing::warn!(
                subscription_id = %sub.id,
                from_status = %sub.status.as_str(),
                to_status = %SubscriptionStatus::PastDue.as_str(),
                event_id = %event.id,
                "Ignoring invalid subscription status transition"
            );
            return Ok(());
        }

        sub.status = SubscriptionStatus::PastDue;
        sub.updated_at = Utc::now();
        state
            .billing_repository
            .update_subscription(sub.clone())
            .await?;

        let history_event = SubscriptionHistoryService::create_subscription_past_due_event(
            &sub,
            Some(ACTOR_WEBHOOK.to_string()),
        );
        state
            .billing_repository
            .save_history_event(history_event)
            .await?;
    }

    Ok(())
}

/// Handle dispute.created event
async fn handle_dispute_created(
    state: &AppState,
    event: &CreemWebhookEvent,
    realm_id: String,
) -> Result<(), CoreError> {
    tracing::warn!("Handling dispute.created for realm: {}", realm_id);

    let dispute: CreemDispute = serde_json::from_value(event.object.clone())?;

    if let Some(mut sub) = state
        .billing_repository
        .find_by_external_subscription_id(&dispute.subscription_id, "creem")
        .await?
    {
        if !sub.status.can_transition_to(&SubscriptionStatus::Dispute) {
            tracing::warn!(
                subscription_id = %sub.id,
                from_status = %sub.status.as_str(),
                to_status = %SubscriptionStatus::Dispute.as_str(),
                event_id = %event.id,
                "Ignoring invalid subscription status transition"
            );
            return Ok(());
        }

        sub.status = SubscriptionStatus::Dispute;
        sub.updated_at = Utc::now();
        state
            .billing_repository
            .update_subscription(sub.clone())
            .await?;

        let changes = serde_json::json!({
            "dispute_id": dispute.id,
            "amount": dispute.amount,
            "reason": dispute.reason,
        });
        let history_event = SubscriptionHistoryService::create_subscription_disputed_event(
            &sub,
            changes,
            Some(ACTOR_WEBHOOK.to_string()),
        );
        state
            .billing_repository
            .save_history_event(history_event)
            .await?;
    }

    Ok(())
}

/// Handle subscription.scheduled_cancel event
async fn handle_subscription_scheduled_cancel(
    state: &AppState,
    event: &CreemWebhookEvent,
    realm_id: String,
) -> Result<(), CoreError> {
    tracing::info!(
        "Handling subscription.scheduled_cancel for realm: {}",
        realm_id
    );
    handle_subscription_update_internal(state, event, realm_id, SubscriptionStatus::ScheduledCancel)
        .await
}

/// Handle refund.created event (audit only)
async fn handle_refund_created(
    state: &AppState,
    event: &CreemWebhookEvent,
    _realm_id: String,
) -> Result<(), CoreError> {
    let refund: CreemRefund = serde_json::from_value(event.object.clone())?;

    tracing::info!(
        "Refund created - subscription: {}, amount: {} {}, reason: {:?}",
        refund.subscription_id,
        refund.amount,
        refund.currency,
        refund.reason
    );

    if let Some(sub) = state
        .billing_repository
        .find_by_external_subscription_id(&refund.subscription_id, "creem")
        .await?
    {
        let changes = serde_json::json!({
            "refund_id": refund.id,
            "amount": refund.amount,
            "currency": refund.currency,
            "reason": refund.reason,
            "created_at": refund.created_at,
        });
        let history_event = SubscriptionHistoryService::create_subscription_refunded_event(
            &sub,
            changes,
            Some(ACTOR_WEBHOOK.to_string()),
        );
        state
            .billing_repository
            .save_history_event(history_event)
            .await?;
    }

    Ok(())
}

/// Internal helper to handle subscription update events
async fn handle_subscription_update_internal(
    state: &AppState,
    event: &CreemWebhookEvent,
    realm_id: String,
    expected_status: SubscriptionStatus,
) -> Result<(), CoreError> {
    let creem_sub: CreemSubscription =
        serde_json::from_value(event.object.clone()).map_err(|e| {
            CoreError::InternalServerError(format!("Failed to parse subscription object: {}", e))
        })?;

    let existing_sub = state
        .billing_repository
        .find_by_external_subscription_id(&creem_sub.id, "creem")
        .await?;

    let now = Utc::now();
    let _metadata = extract_subscription_metadata(event)?;

    let status_str = expected_status.as_str();

    if let Some(sub) = existing_sub {
        if !sub.status.can_transition_to(&expected_status) {
            tracing::warn!(
                subscription_id = %sub.id,
                from_status = %sub.status.as_str(),
                to_status = %status_str,
                event_id = %event.id,
                "Ignoring invalid subscription status transition"
            );
            return Ok(());
        }

        let mut updated_sub = sub.clone();
        updated_sub.status = expected_status;
        updated_sub.external_product_id = creem_sub.product.id.clone();
        updated_sub.current_period_start = creem_sub
            .current_period_start
            .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0));
        updated_sub.current_period_end = creem_sub
            .current_period_end
            .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0));
        updated_sub.cancel_at_period_end = creem_sub.cancel_at_period_end.unwrap_or(false);
        updated_sub.updated_at = now;

        let history_event = SubscriptionHistoryService::create_subscription_updated_event(
            &sub,
            &updated_sub,
            Some(ACTOR_WEBHOOK.to_string()),
        );

        state
            .billing_repository
            .update_subscription(updated_sub)
            .await?;
        state
            .billing_repository
            .save_history_event(history_event)
            .await?;
    }

    tracing::info!(
        "Subscription updated to status {} for realm: {}",
        status_str,
        realm_id
    );
    Ok(())
}
