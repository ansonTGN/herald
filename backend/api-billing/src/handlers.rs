use axum::{
    Json,
    extract::{Extension, Path, Query, State},
};
use chrono::Utc;
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

use herald_api_base::application::http::server::api_entities::{ApiError, ErrorResponse};
use herald_api_base::application::http::state::AppState;
// Import the trait and types from herald_core
use herald_core::domain::authentication::Identity;
use herald_core::domain::authorization::PermissionService;
use herald_core::domain::billing::{BillingRepository, Subscription};
use herald_core::domain::common::entities::app_errors::CoreError;
use herald_core::domain::realm_config::RealmConfigRepository;
use herald_core::infrastructure::creem::{
    CreateCheckoutRequest as CreemCreateCheckoutRequest, CreemClient,
};
use herald_core::infrastructure::stripe::{
    CreateCheckoutRequest as StripeCreateCheckoutRequest, StripeClient,
};

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

// ============================================================================
// Realm-specific Client Helpers
// ============================================================================

/// Get Creem client for a specific realm
async fn get_creem_client_for_realm(
    realm_id: &str,
    state: &AppState,
) -> Result<CreemClient, ApiError> {
    let api_key = state
        .realm_config_repository
        .get(
            realm_id.to_string(),
            "creem".to_string(),
            "api_key".to_string(),
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to load Creem API key from database: {}", e);
            ApiError::internal(format!("Database error: {}", e))
        })?
        .filter(|c| c.enabled)
        .map(|c| c.config_value)
        .ok_or_else(|| {
            tracing::error!("No Creem API key found for realm: {}", realm_id);
            ApiError::internal(format!("Creem not configured for realm: {}", realm_id))
        })?;

    let timeout = state
        .realm_config_repository
        .get(
            realm_id.to_string(),
            "creem".to_string(),
            "timeout".to_string(),
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to load Creem timeout from database: {}", e);
            ApiError::internal(format!("Database error: {}", e))
        })?
        .filter(|c| c.enabled)
        .and_then(|c| c.config_value.parse::<u64>().ok())
        .unwrap_or(30);

    tracing::info!("Loaded Creem config from database for realm: {}", realm_id);

    CreemClient::new(api_key, timeout).map_err(ApiError::from)
}

/// Get Stripe client for a realm
async fn get_stripe_client_for_realm(
    realm_id: &str,
    state: &AppState,
) -> Result<StripeClient, ApiError> {
    let api_key = state
        .realm_config_repository
        .get(
            realm_id.to_string(),
            "stripe".to_string(),
            "api_key".to_string(),
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to load Stripe API key from database: {}", e);
            ApiError::internal(format!("Database error: {}", e))
        })?
        .filter(|c| c.enabled)
        .map(|c| c.config_value)
        .ok_or_else(|| {
            tracing::error!("No Stripe API key found for realm: {}", realm_id);
            ApiError::internal(format!("Stripe not configured for realm: {}", realm_id))
        })?;

    let timeout = state
        .realm_config_repository
        .get(
            realm_id.to_string(),
            "stripe".to_string(),
            "timeout".to_string(),
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to load Stripe timeout from database: {}", e);
            ApiError::internal(format!("Database error: {}", e))
        })?
        .filter(|c| c.enabled)
        .and_then(|c| c.config_value.parse::<u64>().ok())
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

    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20);

    let (subs, total) = state
        .billing_repository
        .list_subscriptions(
            &realm_id,
            query.entitlement_key.as_deref(),
            query.status.as_deref(),
            query.payment_provider.as_deref(),
            page,
            page_size,
        )
        .await
        .map_err(|e| {
            tracing::error!(realm_id = %realm_id, error = %e, "Failed to list subscriptions");
            ApiError::internal("Failed to list subscriptions".to_string())
        })?;

    let items: Vec<SubscriptionListItemResponse> = subs
        .iter()
        .map(|sub| SubscriptionListItemResponse {
            id: sub.id,
            client_app_id: sub.client_app_id,
            entitlement_key: sub.entitlement_key.clone(),
            external_price_id: sub.external_price_id.clone(),
            payment_provider: sub.payment_provider.clone(),
            status: sub.status.as_str().to_string(),
            current_period_start: sub.current_period_start.map(|dt| dt.to_rfc3339()),
            current_period_end: sub.current_period_end.map(|dt| dt.to_rfc3339()),
            synced_at: sub.synced_at.map(|dt| dt.to_rfc3339()),
            created_at: sub.created_at.to_rfc3339(),
            updated_at: sub.updated_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(SubscriptionListResponse {
        items,
        total: total as i64,
    }))
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

            let mode = if mapping.billing_kind() == "one_time" {
                Some("payment".to_string())
            } else {
                None
            };

            let stripe_request = StripeCreateCheckoutRequest {
                client_app_id,
                mapping_id: mapping.id,
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
                    map.insert(
                        "herald_entitlement_key".to_string(),
                        entitlement_key.clone(),
                    );
                    map.insert("herald_billing_kind".to_string(), mapping.billing_kind());
                    map.insert("herald_user_id".to_string(), identity.user_id());
                    map.insert("herald_realm_id".to_string(), realm_id.clone());
                    map.insert(
                        "herald_client_app_id".to_string(),
                        client_app_id.to_string(),
                    );
                    map
                }),
                mode,
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
                    map.insert("herald_realm_id".to_string(), realm_id.clone());
                    map.insert("herald_user_id".to_string(), identity.user_id());
                    map.insert(
                        "herald_client_app_id".to_string(),
                        client_app_id.to_string(),
                    );
                    map.insert(
                        "herald_entitlement_key".to_string(),
                        entitlement_key.clone(),
                    );
                    map.insert("herald_billing_kind".to_string(), mapping.billing_kind());
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
