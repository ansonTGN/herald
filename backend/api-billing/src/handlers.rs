use axum::{
    Json,
    extract::{Extension, Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use chrono::Utc;
use herald_core::domain::points::{PointsErrorExt, grant_schedule::GrantPeriodType};
use serde::Deserialize;
use sqlx::Row;
use uuid::Uuid;
use validator::Validate;

use crate::types::{
    CancelSubscriptionRequest,
    CancelSubscriptionResponse,
    CreateCheckoutResponse,
    CreateCheckoutSessionRequest,
    CreateProductRequest,
    // Subscription plan types
    CreateSubscriptionPlanRequest,
    ListProductsResponse,
    ListSubscriptionPlanAssignmentsResponse,
    ListSubscriptionPlansResponse,
    ProductDetailResponse,
    ProductResponse,
    // Subscription types
    SubscriptionDetailResponse,
    // Subscription plan assignment types
    SubscriptionPlanAssignmentRequest,
    SubscriptionPlanAssignmentResponse,
    SubscriptionPlanPaymentProviderResponse,
    SubscriptionPlanResponse,
    SubscriptionPlanSummaryForProduct,
    ToggleSubscriptionPlanAssignmentRequest,
    ToggleSubscriptionPlanPaymentProviderRequest,
    UpdateProductRequest,
    UpdateSubscriptionPlanPaymentProviderRequest,
    UpdateSubscriptionPlanRequest,
};
use crate::webhooks::verify_webhook_signature;
use herald_api_base::application::http::server::api_entities::{ApiError, ErrorResponse};
use herald_api_base::application::http::state::AppState;
// Import the trait and types from herald_core
use herald_core::domain::authentication::Identity;
use herald_core::domain::authorization::PermissionService;
use herald_core::domain::billing::{
    ACTOR_WEBHOOK, BillingRepository, ClientAppSubscriptionPlan, CreateProductInput, Product,
    Subscription, SubscriptionHistoryService, SubscriptionPlan, SubscriptionPlanType,
    SubscriptionStatus, SubscriptionTier, UpdateProductInput,
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
// Conversion Helpers
// ============================================================================

/// Convert domain SubscriptionPlan to API SubscriptionPlanResponse (without providers)
pub fn subscription_plan_to_response(plan: SubscriptionPlan) -> SubscriptionPlanResponse {
    subscription_plan_to_response_with_providers_inner(plan, Vec::new())
}

/// Convert domain SubscriptionPlan with payment providers to API SubscriptionPlanResponse
pub async fn subscription_plan_to_response_with_providers(
    plan: SubscriptionPlan,
    state: &AppState,
) -> Result<SubscriptionPlanResponse, ApiError> {
    let providers = state
        .billing_repository
        .list_subscription_plan_payment_providers(plan.id)
        .await
        .map_err(|e| {
            tracing::error!(
                "Failed to load payment providers for plan {}: {}",
                plan.id,
                e
            );
            ApiError::internal("Failed to load payment providers".to_string())
        })?;

    Ok(subscription_plan_to_response_with_providers_inner(
        plan, providers,
    ))
}

/// Inner helper to convert SubscriptionPlan with providers to response
fn subscription_plan_to_response_with_providers_inner(
    plan: SubscriptionPlan,
    providers: Vec<herald_core::domain::billing::SubscriptionPlanPaymentProvider>,
) -> SubscriptionPlanResponse {
    let payment_providers: Vec<crate::types::PaymentProviderSummary> = providers
        .into_iter()
        .map(|p| crate::types::PaymentProviderSummary {
            id: p.id,
            payment_provider: p.payment_provider,
            external_product_id: p.external_product_id,
            external_price_id: p.external_price_id,
            enabled: p.enabled,
        })
        .collect();

    SubscriptionPlanResponse {
        id: plan.id,
        realm_id: plan.realm_id,
        name: plan.name,
        title: plan.title,
        description: plan.description,
        r#type: plan.r#type.as_str().to_string(),
        price: plan.price,
        currency: plan.currency,
        checkout_url: plan.checkout_url,
        active: plan.active,
        trial_days: plan.trial_days,
        sort_order: plan.sort_order,
        product_id: plan.product_id,
        payment_providers,
        created_at: plan.created_at.to_rfc3339(),
        updated_at: plan.updated_at.to_rfc3339(),
    }
}

/// Convert domain SubscriptionPlanPaymentProvider to API response
pub fn subscription_plan_payment_provider_to_response(
    mapping: herald_core::domain::billing::SubscriptionPlanPaymentProvider,
) -> SubscriptionPlanPaymentProviderResponse {
    SubscriptionPlanPaymentProviderResponse {
        id: mapping.id,
        plan_id: mapping.plan_id,
        payment_provider: mapping.payment_provider,
        external_product_id: mapping.external_product_id,
        external_price_id: mapping.external_price_id,
        enabled: mapping.enabled,
        created_at: mapping.created_at.to_rfc3339(),
        updated_at: mapping.updated_at.to_rfc3339(),
    }
}

/// Convert domain ClientAppSubscriptionPlan to API SubscriptionPlanAssignmentResponse
fn client_app_subscription_plan_to_response(
    assignment: ClientAppSubscriptionPlan,
) -> SubscriptionPlanAssignmentResponse {
    SubscriptionPlanAssignmentResponse {
        id: assignment.id,
        client_app_id: assignment.client_app_id,
        plan_id: assignment.plan_id,
        enabled: assignment.enabled,
        created_at: assignment.created_at.to_rfc3339(),
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

// ============================================================================
// Realm-specific Creem Client Helper
// ============================================================================

/// Get Creem client for a specific realm
///
/// This function loads the Creem configuration from the database for the given realm,
/// falling back to TOML config if not found. It creates a CreemClient instance
/// with the realm-specific API key and timeout settings.
///
/// # Arguments
///
/// * `realm_id` - The realm ID to load config for
/// * `state` - Application state containing database pool and TOML config
///
/// # Returns
///
/// * `Ok(CreemClient)` - Creem client configured for the realm
/// * `Err(ApiError)` - Error loading config or creating client
async fn get_creem_client_for_realm(
    realm_id: &str,
    state: &AppState,
) -> Result<CreemClient, ApiError> {
    // Load Creem API key from database
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

    // Load timeout from database (default to 30 if not found)
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

    // Create Creem client with realm-specific config
    CreemClient::new(api_key, timeout).map_err(ApiError::from)
}

/// Get Stripe client for a realm
///
/// This function loads the Stripe API key from the database for the given realm.
/// The API key is stored in the realm_config table with config_type='stripe'
/// and config_key='api_key'.
///
/// # Arguments
/// * `realm_id` - The realm ID
/// * `state` - Application state containing database pool
///
/// # Returns
/// * `Ok(StripeClient)` - Stripe client configured for the realm
/// * `Err(ApiError)` - Error loading config or creating client
async fn get_stripe_client_for_realm(
    realm_id: &str,
    state: &AppState,
) -> Result<StripeClient, ApiError> {
    // Load Stripe API key from database
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

    // Load timeout from database (default to 30 if not found)
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

    // Create Stripe client with realm-specific config
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
///
/// # Arguments
/// * `state` - Application state containing permission checker
/// * `identity` - User identity containing user_id and realm_id
/// * `realm_id` - Requested realm ID
/// * `action` - Permission action: "view" or "manage"
///
/// # Returns
/// * `Ok(())` if permission is granted
/// * `Err(ApiError)` with 403 status if permission is denied
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

async fn ensure_mapping_belongs_to_plan(
    state: &AppState,
    realm_id: &str,
    plan_id: Uuid,
    mapping_id: Uuid,
) -> Result<(), ApiError> {
    let mapping = state
        .billing_repository
        .find_subscription_plan_payment_provider_by_id(mapping_id)
        .await?
        .ok_or_else(|| {
            ApiError::from(CoreError::BadRequest(format!(
                "Payment provider mapping not found: {}",
                mapping_id
            )))
        })?;

    let plan = state
        .billing_repository
        .find_subscription_plan_by_id(plan_id)
        .await?
        .ok_or_else(|| CoreError::SubscriptionPlanNotFound {
            realm_id: realm_id.to_string(),
            plan_id: plan_id.to_string(),
        })?;

    if plan.realm_id != realm_id {
        return Err(ApiError::from(CoreError::SubscriptionPlanNotFound {
            realm_id: realm_id.to_string(),
            plan_id: plan_id.to_string(),
        }));
    }

    if mapping.plan_id != plan_id {
        return Err(ApiError::bad_request(format!(
            "Payment provider mapping {} does not belong to plan {}",
            mapping_id, plan_id
        )));
    }

    Ok(())
}

// ============================================================================
// Plan Management Handlers
// ============================================================================

/// List all plans for a realm
#[utoipa::path(
    get,
    path = "/api/bill/{realmId}/plans",
    tag = "billing",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    responses(
        (status = 200, description = "Plans listed successfully", body = ListSubscriptionPlansResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_plans(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
) -> Result<Json<ListSubscriptionPlansResponse>, ApiError> {
    tracing::info!("Listing plans for realm: {}", realm_id);

    // Check billing.view permission
    require_billing_permission(&state, &identity, &realm_id, "view").await?;

    let plans = state
        .billing_service
        .list_plans(identity, &realm_id)
        .await?;

    // Batch load all payment providers for all plans to avoid N+1 queries
    let plan_ids: Vec<Uuid> = plans.iter().map(|p| p.id).collect();
    let all_providers = state
        .billing_repository
        .list_subscription_plan_payment_providers_batch(&plan_ids)
        .await
        .map_err(|e| {
            tracing::error!("Failed to load payment providers for plans: {}", e);
            ApiError::internal("Failed to load payment providers".to_string())
        })?;

    // Group providers by plan_id
    let mut providers_by_plan: std::collections::HashMap<
        Uuid,
        Vec<herald_core::domain::billing::SubscriptionPlanPaymentProvider>,
    > = std::collections::HashMap::new();
    for provider in all_providers {
        providers_by_plan
            .entry(provider.plan_id)
            .or_default()
            .push(provider);
    }

    // Convert plans to responses with their providers
    let plan_responses: Vec<SubscriptionPlanResponse> = plans
        .into_iter()
        .map(|plan| {
            let providers = providers_by_plan.get(&plan.id).cloned().unwrap_or_default();
            subscription_plan_to_response_with_providers_inner(plan, providers)
        })
        .collect();

    Ok(Json(ListSubscriptionPlansResponse {
        plans: plan_responses,
    }))
}

/// Create a new plan for a realm
#[utoipa::path(
    post,
    path = "/api/bill/{realmId}/plans",
    tag = "billing",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    request_body = CreateSubscriptionPlanRequest,
    responses(
        (status = 201, description = "Plan created successfully", body = SubscriptionPlanResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_plan(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Json(request): Json<CreateSubscriptionPlanRequest>,
) -> Result<Json<SubscriptionPlanResponse>, ApiError> {
    tracing::info!("Creating plan '{}' for realm: {}", request.name, realm_id);

    // Check billing.manage permission
    require_billing_permission(&state, &identity, &realm_id, "manage").await?;

    // Validate request using validator crate
    request
        .validate()
        .map_err(|e: validator::ValidationErrors| {
            CoreError::BadRequest(format!("Validation failed: {}", e))
        })?;

    let plan_type: SubscriptionPlanType = request.r#type.parse()?;

    let input = herald_core::domain::billing::CreateSubscriptionPlanInput {
        realm_id: realm_id.clone(),
        name: request.name.clone(),
        title: request.title.clone(),
        description: request.description,
        r#type: plan_type,
        price: request.price,
        currency: request.currency.clone(),
        checkout_url: request.checkout_url,
        trial_days: request.trial_days,
        sort_order: request.sort_order,
        product_id: request.product_id,
    };

    let plan = state
        .billing_service
        .create_plan(identity, &realm_id, input)
        .await?;

    Ok(Json(subscription_plan_to_response(plan)))
}

/// Get a specific plan
#[utoipa::path(
    get,
    path = "/api/bill/{realmId}/plans/{planId}",
    tag = "billing",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("planId" = Uuid, Path, description = "Plan ID")
    ),
    responses(
        (status = 200, description = "Plan found", body = SubscriptionPlanResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Plan not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_plan(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, plan_id)): Path<(String, Uuid)>,
) -> Result<Json<SubscriptionPlanResponse>, ApiError> {
    tracing::info!("Getting plan {} for realm: {}", plan_id, realm_id);

    // Check billing.view permission
    require_billing_permission(&state, &identity, &realm_id, "view").await?;

    let plan = state
        .billing_service
        .get_plan(identity, &realm_id, plan_id)
        .await?;

    // Load payment providers for this plan
    let response = subscription_plan_to_response_with_providers(plan, &state).await?;

    Ok(Json(response))
}

/// Update a plan
#[utoipa::path(
    patch,
    path = "/api/bill/{realmId}/plans/{planId}",
    tag = "billing",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("planId" = Uuid, Path, description = "Plan ID")
    ),
    request_body = UpdateSubscriptionPlanRequest,
    responses(
        (status = 200, description = "Plan updated successfully", body = SubscriptionPlanResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Plan not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_plan(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, plan_id)): Path<(String, Uuid)>,
    Json(request): Json<UpdateSubscriptionPlanRequest>,
) -> Result<Json<SubscriptionPlanResponse>, ApiError> {
    tracing::info!("Updating plan {} for realm: {}", plan_id, realm_id);

    // Check billing.manage permission
    require_billing_permission(&state, &identity, &realm_id, "manage").await?;

    let input = herald_core::domain::billing::UpdateSubscriptionPlanInput {
        name: request.name,
        title: request.title,
        description: request.description,
        r#type: request.r#type.map(|s| s.parse()).transpose()?,
        price: request.price,
        currency: request.currency,
        checkout_url: request.checkout_url,
        active: request.active,
        trial_days: request.trial_days,
        sort_order: request.sort_order,
        product_id: request.product_id,
    };

    let plan = state
        .billing_service
        .update_plan(identity, &realm_id, plan_id, input)
        .await?;

    Ok(Json(subscription_plan_to_response(plan)))
}

/// Delete a plan
#[utoipa::path(
    delete,
    path = "/api/bill/{realmId}/plans/{planId}",
    tag = "billing",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("planId" = Uuid, Path, description = "Plan ID")
    ),
    responses(
        (status = 204, description = "Plan deleted successfully"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions or plan has active subscriptions", body = ErrorResponse),
        (status = 404, description = "Plan not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_plan(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, plan_id)): Path<(String, Uuid)>,
) -> Result<StatusCode, ApiError> {
    tracing::info!("Deleting plan {} for realm: {}", plan_id, realm_id);

    // Check billing.manage permission
    require_billing_permission(&state, &identity, &realm_id, "manage").await?;

    state
        .billing_service
        .delete_plan(identity, &realm_id, plan_id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Plan Payment Provider Mapping Handlers
// ============================================================================

/// List payment providers configured for a plan
#[utoipa::path(
    get,
    path = "/api/bill/{realmId}/plans/{planId}/providers",
    tag = "Billing - Plan Payment Providers",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("planId" = Uuid, Path, description = "Plan ID")
    ),
    responses(
        (status = 200, description = "Payment providers listed successfully", body = Vec<SubscriptionPlanPaymentProviderResponse>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Plan not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_plan_payment_providers(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, plan_id)): Path<(String, Uuid)>,
) -> Result<Json<Vec<SubscriptionPlanPaymentProviderResponse>>, ApiError> {
    tracing::info!(
        "Listing payment providers for plan {} in realm: {}",
        plan_id,
        realm_id
    );

    // Check billing.view permission
    require_billing_permission(&state, &identity, &realm_id, "view").await?;

    let providers = state
        .billing_service
        .list_plan_payment_providers(identity, &realm_id, plan_id)
        .await?;

    Ok(Json(
        providers
            .into_iter()
            .map(subscription_plan_payment_provider_to_response)
            .collect(),
    ))
}

/// Add a payment provider mapping to a plan
#[utoipa::path(
    post,
    path = "/api/bill/{realmId}/plans/{planId}/providers",
    tag = "Billing - Plan Payment Providers",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("planId" = Uuid, Path, description = "Plan ID")
    ),
    request_body = crate::types::CreateSubscriptionPlanPaymentProviderRequest,
    responses(
        (status = 201, description = "Payment provider mapping created successfully", body = SubscriptionPlanPaymentProviderResponse),
        (status = 400, description = "Bad request - Payment provider already configured", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Plan not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn add_payment_provider_to_plan(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, plan_id)): Path<(String, Uuid)>,
    Json(request): Json<crate::types::CreateSubscriptionPlanPaymentProviderRequest>,
) -> Result<Json<SubscriptionPlanPaymentProviderResponse>, ApiError> {
    tracing::info!(
        "Adding payment provider '{}' to plan {} in realm: {}",
        request.payment_provider,
        plan_id,
        realm_id
    );

    // Check billing.manage permission
    require_billing_permission(&state, &identity, &realm_id, "manage").await?;

    // Validate request
    request
        .validate()
        .map_err(|e: validator::ValidationErrors| {
            CoreError::BadRequest(format!("Validation failed: {}", e))
        })?;

    let mapping = state
        .billing_service
        .add_payment_provider_to_plan(
            identity,
            &realm_id,
            plan_id,
            request.payment_provider,
            request.external_product_id,
            request.external_price_id,
            request.enabled.unwrap_or(true),
        )
        .await?;

    Ok(Json(subscription_plan_payment_provider_to_response(
        mapping,
    )))
}

/// Update a payment provider mapping
#[utoipa::path(
    patch,
    path = "/api/bill/{realmId}/plans/{planId}/providers/{mappingId}",
    tag = "Billing - Plan Payment Providers",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("planId" = Uuid, Path, description = "Plan ID"),
        ("mappingId" = Uuid, Path, description = "Payment Provider Mapping ID")
    ),
    request_body = UpdateSubscriptionPlanPaymentProviderRequest,
    responses(
        (status = 200, description = "Payment provider mapping updated successfully", body = SubscriptionPlanPaymentProviderResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Mapping not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_plan_payment_provider(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, plan_id, mapping_id)): Path<(String, Uuid, Uuid)>,
    Json(request): Json<UpdateSubscriptionPlanPaymentProviderRequest>,
) -> Result<Json<SubscriptionPlanPaymentProviderResponse>, ApiError> {
    tracing::info!(
        "Updating payment provider mapping {} in realm: {}",
        mapping_id,
        realm_id
    );

    // Check billing.manage permission
    require_billing_permission(&state, &identity, &realm_id, "manage").await?;

    // Validate request
    request
        .validate()
        .map_err(|e: validator::ValidationErrors| {
            CoreError::BadRequest(format!("Validation failed: {}", e))
        })?;

    ensure_mapping_belongs_to_plan(&state, &realm_id, plan_id, mapping_id).await?;

    let mapping = state
        .billing_service
        .update_plan_payment_provider(
            identity,
            &realm_id,
            mapping_id,
            request.external_product_id,
            request.external_price_id,
            request.enabled,
        )
        .await?;

    Ok(Json(subscription_plan_payment_provider_to_response(
        mapping,
    )))
}

/// Toggle payment provider enabled status
#[utoipa::path(
    patch,
    path = "/api/bill/{realmId}/plans/{planId}/providers/{mappingId}/toggle",
    tag = "Billing - Plan Payment Providers",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("planId" = Uuid, Path, description = "Plan ID"),
        ("mappingId" = Uuid, Path, description = "Payment Provider Mapping ID")
    ),
    request_body = ToggleSubscriptionPlanPaymentProviderRequest,
    responses(
        (status = 200, description = "Payment provider mapping toggled successfully", body = SubscriptionPlanPaymentProviderResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Mapping not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn toggle_plan_payment_provider(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, plan_id, mapping_id)): Path<(String, Uuid, Uuid)>,
    Json(request): Json<ToggleSubscriptionPlanPaymentProviderRequest>,
) -> Result<Json<SubscriptionPlanPaymentProviderResponse>, ApiError> {
    tracing::info!(
        "Toggling payment provider mapping {} to enabled: {} in realm: {}",
        mapping_id,
        request.enabled,
        realm_id
    );

    // Check billing.manage permission
    require_billing_permission(&state, &identity, &realm_id, "manage").await?;

    ensure_mapping_belongs_to_plan(&state, &realm_id, plan_id, mapping_id).await?;

    let mapping = state
        .billing_service
        .toggle_plan_payment_provider(identity, &realm_id, mapping_id, request.enabled)
        .await?;

    Ok(Json(subscription_plan_payment_provider_to_response(
        mapping,
    )))
}

/// Remove a payment provider mapping from a plan
#[utoipa::path(
    delete,
    path = "/api/bill/{realmId}/plans/{planId}/providers/{mappingId}",
    tag = "Billing - Plan Payment Providers",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("planId" = Uuid, Path, description = "Plan ID"),
        ("mappingId" = Uuid, Path, description = "Payment Provider Mapping ID")
    ),
    responses(
        (status = 204, description = "Payment provider mapping removed successfully"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Mapping not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn remove_payment_provider_from_plan(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, plan_id, mapping_id)): Path<(String, Uuid, Uuid)>,
) -> Result<StatusCode, ApiError> {
    tracing::info!(
        "Removing payment provider mapping {} from realm: {}",
        mapping_id,
        realm_id
    );

    // Check billing.manage permission
    require_billing_permission(&state, &identity, &realm_id, "manage").await?;

    ensure_mapping_belongs_to_plan(&state, &realm_id, plan_id, mapping_id).await?;

    state
        .billing_service
        .remove_payment_provider_from_plan(identity, &realm_id, mapping_id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Plan Assignment Handlers
// ============================================================================

/// Assign a plan to a client app
#[utoipa::path(
    post,
    path = "/api/bill/{realmId}/client/{clientAppId}/plans",
    tag = "billing",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("clientAppId" = Uuid, Path, description = "Client App ID")
    ),
    request_body = SubscriptionPlanAssignmentRequest,
    responses(
        (status = 201, description = "Plan assigned successfully", body = SubscriptionPlanAssignmentResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Plan not found", body = ErrorResponse),
        (status = 409, description = "Plan already assigned", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn assign_plan_to_client_app(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, client_app_id)): Path<(String, Uuid)>,
    Json(request): Json<SubscriptionPlanAssignmentRequest>,
) -> Result<Json<SubscriptionPlanAssignmentResponse>, ApiError> {
    tracing::info!(
        "Assigning plan {} to client app {} for realm: {}",
        request.plan_id,
        client_app_id,
        realm_id
    );

    // Check billing.manage permission
    require_billing_permission(&state, &identity, &realm_id, "manage").await?;

    let assignment = state
        .billing_service
        .assign_plan_to_client_app(identity, &realm_id, client_app_id, request.plan_id)
        .await?;

    Ok(Json(client_app_subscription_plan_to_response(assignment)))
}

/// List plans assigned to a client app
#[utoipa::path(
    get,
    path = "/api/bill/{realmId}/client/{clientAppId}/plans",
    tag = "billing",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("clientAppId" = Uuid, Path, description = "Client App ID")
    ),
    responses(
        (status = 200, description = "Plans listed successfully", body = ListSubscriptionPlanAssignmentsResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_plan_assignments(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, client_app_id)): Path<(String, Uuid)>,
) -> Result<Json<ListSubscriptionPlanAssignmentsResponse>, ApiError> {
    tracing::info!(
        "Listing plan assignments for client app {} in realm: {}",
        client_app_id,
        realm_id
    );

    // Check billing.view permission
    require_billing_permission(&state, &identity, &realm_id, "view").await?;

    let assignments = state
        .billing_service
        .list_plans_for_client_app(identity, &realm_id, client_app_id)
        .await?;

    Ok(Json(ListSubscriptionPlanAssignmentsResponse {
        assignments: assignments
            .into_iter()
            .map(client_app_subscription_plan_to_response)
            .collect(),
    }))
}

/// Query parameters for batch plan assignments endpoint
#[derive(Debug, Deserialize)]
pub struct BatchPlanAssignmentsQuery {
    pub client_app_ids: Option<String>,
}

/// Batch list plan assignments for multiple client apps
#[utoipa::path(
    get,
    path = "/api/bill/{realmId}/client/plans/batch",
    tag = "billing",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("clientAppIds" = Option<String>, Query, description = "Comma-separated client app IDs (UUIDs)")
    ),
    responses(
        (status = 200, description = "Plan assignments listed successfully", body = ListSubscriptionPlanAssignmentsResponse),
        (status = 400, description = "Bad request - Invalid client app IDs", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_plan_assignments_batch(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Query(query): Query<BatchPlanAssignmentsQuery>,
) -> Result<Json<ListSubscriptionPlanAssignmentsResponse>, ApiError> {
    // Parse client_app_ids from query parameter
    let client_app_ids_str = query.client_app_ids.ok_or_else(|| {
        ApiError::bad_request("clientAppIds query parameter is required".to_string())
    })?;

    let client_app_ids: Vec<Uuid> = client_app_ids_str
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| {
            Uuid::parse_str(s)
                .map_err(|_| ApiError::bad_request(format!("Invalid UUID in clientAppIds: {}", s)))
        })
        .collect::<Result<Vec<_>, ApiError>>()?;

    if client_app_ids.is_empty() {
        return Ok(Json(ListSubscriptionPlanAssignmentsResponse {
            assignments: Vec::new(),
        }));
    }

    tracing::info!(
        "Batch listing plan assignments for {} client apps in realm: {}",
        client_app_ids.len(),
        realm_id
    );

    // Check billing.view permission
    require_billing_permission(&state, &identity, &realm_id, "view").await?;

    let assignments = state
        .billing_repository
        .list_subscription_plan_assignments_batch(&client_app_ids)
        .await
        .map_err(|e| {
            tracing::error!("Failed to batch list plan assignments: {}", e);
            ApiError::internal("Failed to list plan assignments".to_string())
        })?;

    Ok(Json(ListSubscriptionPlanAssignmentsResponse {
        assignments: assignments
            .into_iter()
            .map(client_app_subscription_plan_to_response)
            .collect(),
    }))
}

/// Toggle plan assignment enabled status
#[utoipa::path(
    patch,
    path = "/api/bill/{realmId}/client/{clientAppId}/plans/{assignmentId}",
    tag = "billing",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("clientAppId" = Uuid, Path, description = "Client App ID"),
        ("assignmentId" = Uuid, Path, description = "Assignment ID")
    ),
    request_body = ToggleSubscriptionPlanAssignmentRequest,
    responses(
        (status = 200, description = "Assignment toggled successfully", body = SubscriptionPlanAssignmentResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Assignment not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn toggle_plan_assignment(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, _client_app_id, assignment_id)): Path<(String, Uuid, Uuid)>,
    Json(request): Json<ToggleSubscriptionPlanAssignmentRequest>,
) -> Result<Json<SubscriptionPlanAssignmentResponse>, ApiError> {
    tracing::info!(
        "Toggling assignment {} to enabled: {} for realm: {}",
        assignment_id,
        request.enabled,
        realm_id
    );

    // Check billing.manage permission
    require_billing_permission(&state, &identity, &realm_id, "manage").await?;

    let assignment = state
        .billing_service
        .toggle_plan_assignment(identity, &realm_id, assignment_id, request.enabled)
        .await?;

    Ok(Json(client_app_subscription_plan_to_response(assignment)))
}

/// Remove plan assignment from client app
#[utoipa::path(
    delete,
    path = "/api/bill/{realmId}/client/{clientAppId}/plans/{assignmentId}",
    tag = "billing",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("clientAppId" = Uuid, Path, description = "Client App ID"),
        ("assignmentId" = Uuid, Path, description = "Assignment ID")
    ),
    responses(
        (status = 204, description = "Assignment removed successfully"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Assignment not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn remove_plan_assignment(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, _client_app_id, assignment_id)): Path<(String, Uuid, Uuid)>,
) -> Result<StatusCode, ApiError> {
    tracing::info!(
        "Removing plan assignment {} for realm: {}",
        assignment_id,
        realm_id
    );

    // Check billing.manage permission
    require_billing_permission(&state, &identity, &realm_id, "manage").await?;

    state
        .billing_service
        .remove_plan_from_client_app(identity, &realm_id, assignment_id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Subscription Handlers (Simplified)
// ============================================================================

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

    // Check billing.view permission (includes realm boundary check)
    require_billing_permission(&state, &identity, &realm_id, "view").await?;

    let subscription = state
        .billing_repository
        .find_subscription_by_client_app_id(client_app_id)
        .await?
        .ok_or_else(|| CoreError::SubscriptionNotFound(client_app_id.to_string()))?;

    // Fetch plan if available
    let plan_response = if let Some(plan_id) = subscription.plan_id {
        state
            .billing_repository
            .find_subscription_plan_by_id(plan_id)
            .await
            .ok()
            .flatten()
            .map(subscription_plan_to_response)
    } else {
        None
    };

    Ok(Json(SubscriptionDetailResponse {
        id: subscription.id,
        client_app_id: subscription.client_app_id,
        plan_id: subscription.plan_id,
        plan: plan_response,
        status: subscription.status.as_str().to_string(),
        billing_period: subscription.billing_period.to_string(),
        current_period_start: subscription.current_period_start.map(|dt| dt.to_rfc3339()),
        current_period_end: subscription.current_period_end.map(|dt| dt.to_rfc3339()),
        cancel_at: subscription.cancel_at.map(|dt| dt.to_rfc3339()),
        cancel_at_period_end: Some(subscription.cancel_at_period_end),
        created_at: subscription.created_at.to_rfc3339(),
        updated_at: subscription.updated_at.to_rfc3339(),
    }))
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

    // Check billing.manage permission (includes realm boundary check)
    require_billing_permission(&state, &identity, &realm_id, "manage").await?;

    // client_app_id is already a UUID, use it directly
    // Get subscription first
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

/// Create checkout session for a plan
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
        (status = 404, description = "Plan not found", body = ErrorResponse),
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
        "Creating checkout session for client app {} with plan {} in realm: {}",
        client_app_id,
        request.plan_id,
        realm_id
    );

    // Check billing.manage permission
    require_billing_permission(&state, &identity, &realm_id, "manage").await?;

    // Get plan
    let plan = state
        .billing_repository
        .find_subscription_plan_by_id(request.plan_id)
        .await?
        .ok_or_else(|| CoreError::SubscriptionPlanNotFound {
            realm_id: realm_id.clone(),
            plan_id: request.plan_id.to_string(),
        })?;

    // Verify plan belongs to realm
    if plan.realm_id != realm_id {
        return Err(ApiError::from(CoreError::SubscriptionPlanNotFound {
            realm_id,
            plan_id: request.plan_id.to_string(),
        }));
    }

    // Verify plan is active
    if !plan.active {
        return Err(ApiError::bad_request(format!(
            "Plan {} is not active",
            plan.id
        )));
    }

    // Verify plan type matches requested billing period
    let plan_type_str = plan.r#type.as_str();
    if plan_type_str != request.billing_period.as_str() {
        return Err(ApiError::bad_request(format!(
            "Plan type ({}) does not match requested billing period ({})",
            plan_type_str, request.billing_period
        )));
    }

    let provider_mappings = state
        .billing_repository
        .list_subscription_plan_payment_providers(plan.id)
        .await
        .map_err(|e| {
            tracing::error!(
                "Failed to load payment providers for plan {}: {}",
                plan.id,
                e
            );
            ApiError::internal("Failed to load payment providers".to_string())
        })?;

    let payment_provider = request.payment_provider.clone();

    // Look up the SubscriptionPlanPaymentProvider mapping to get the external product ID
    let provider_mapping = provider_mappings
        .into_iter()
        .find(|mapping| mapping.payment_provider == payment_provider)
        .ok_or_else(|| {
            ApiError::bad_request(format!(
                "Payment provider '{}' is not configured for this plan",
                payment_provider
            ))
        })?;

    if !provider_mapping.enabled {
        return Err(ApiError::bad_request(format!(
            "Payment provider '{}' is not enabled for this plan",
            payment_provider
        )));
    }

    let product_id = provider_mapping.external_product_id;

    // Route to appropriate payment provider based on request parameter
    let checkout_url = match payment_provider.as_str() {
        "stripe" => {
            tracing::info!("Creating Stripe checkout session for plan: {}", plan.id);

            // Get realm-specific Stripe client
            let stripe_client = get_stripe_client_for_realm(&realm_id, &state).await?;

            // Create Stripe checkout request
            // client_app_id is already a UUID, use it directly
            let stripe_request = StripeCreateCheckoutRequest {
                client_app_id,
                plan_id: plan.id,
                user_id: Uuid::parse_str(&identity.user_id()).ok(),
                success_url: format!("{}/billing/success", state.public_base_url),
                cancel_url: format!("{}/billing/cancel", state.public_base_url),
                billing_period: request.billing_period.clone(),
                trial_days: if plan.trial_days > 0 {
                    Some(plan.trial_days as u32)
                } else {
                    None
                },
                price_amount: plan.price as i64,
                currency: plan.currency.clone(),
                plan_name: plan.name.clone(),
                realm_id: realm_id.clone(),
                webhook_url: Some(format!(
                    "{}/api/third/pay/{}/stripe/webhooks",
                    state.public_base_url, realm_id
                )),
                metadata: None,
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
            tracing::info!("Creating Creem checkout session for plan: {}", plan.id);

            // Get realm-specific Creem client
            let creem_client = get_creem_client_for_realm(&realm_id, &state).await?;

            // Create Creem checkout request
            let creem_request = CreemCreateCheckoutRequest {
                product_id,
                success_url: format!("{}/billing/success", state.public_base_url),
                cancel_url: format!("{}/billing/cancel", state.public_base_url),
                customer_email: format!("client-app-{}@{}", client_app_id, realm_id),
                metadata: {
                    let mut map = std::collections::HashMap::new();
                    map.insert("realmId".to_string(), realm_id.clone());
                    map.insert("userId".to_string(), identity.user_id());
                    map.insert("clientAppId".to_string(), client_app_id.to_string());
                    map.insert("planId".to_string(), request.plan_id.to_string());
                    map.insert("billing_period".to_string(), request.billing_period.clone());
                    Some(map)
                },
                webhook_url: Some(format!(
                    "{}/api/third/pay/{}/creem/webhooks",
                    state.public_base_url, realm_id
                )),
            };

            let session = creem_client
                .create_checkout_session(&creem_request)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to create Creem checkout session: {}", e);
                    ApiError::internal(format!("Failed to create checkout session: {}", e))
                })?;

            session.url
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

    // Generate a UUID for the checkout session (for API compatibility)
    let checkout_id = Uuid::now_v7();

    Ok(Json(CreateCheckoutResponse {
        checkout_url,
        checkout_id,
    }))
}

/// Get webhook secret for a realm
///
/// This function loads the webhook secret from the database for the given realm.
/// The webhook secret is stored in the realm_config table with config_type='creem'
/// and config_key='webhook_secret'.
///
/// # Arguments
/// * `realm_id` - The realm ID
/// * `pool` - Database connection pool
///
/// # Returns
/// * `Result<String, CoreError>` - The webhook secret
async fn get_webhook_secret(realm_id: &str, pool: &sqlx::PgPool) -> Result<String, CoreError> {
    // Load webhook secret from database
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
///
/// Processes asynchronous payment events from Creem including:
/// - `checkout.completed`: One-time payment successful
/// - `subscription.paid`: Recurring subscription payment
/// - `subscription.canceled`: Subscription canceled
/// - `subscription.expired`: Subscription period ended
///
/// # Security
///
/// Webhook signature is verified using HMAC-SHA256 with the webhook secret.
///
/// # Idempotency
///
/// Each webhook event is tracked in database to prevent duplicate processing.
/// The `creem_event_id` is used as an idempotency key.
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
    // 1. Parse the webhook payload first (for input validation)
    let event: CreemWebhookEvent = serde_json::from_slice(&body)?;

    // 2. Extract realm_id from event metadata (input validation before auth)
    let realm_id = extract_realm_id(&event)?;

    // 3. Extract and validate signature header
    let signature_header = headers
        .get("creem-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            tracing::error!("Missing or invalid creem-signature header");
            CoreError::InvalidWebhookSignature
        })?;

    // 4. Get webhook secret for this realm
    let webhook_secret = get_webhook_secret(&realm_id, &state.pool).await?;

    // 5. Verify the signature
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
    // Extract realm_id from event metadata
    let realm_id = extract_realm_id(&event)?;

    // Idempotency registration: insert once, ignore duplicate.
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

    // Lock event row to serialize same-event concurrent processing.
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

    // Process the event.
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

    // Mark as processed in the same transaction that serialized event handling.
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

/// Recharge points for a subscription event
///
/// This function is called by billing webhook handlers to automatically recharge points
/// when a subscription is created or renewed.
async fn recharge_points_for_subscription(
    state: &AppState,
    realm_id: &str,
    user_id: Uuid,
    plan_id: Uuid,
    recharge_type: herald_core::domain::points::RechargeType,
    event_id: String,
) -> Result<(), CoreError> {
    // Get plan config for this plan - use repository directly for internal operations
    let plan_config: Vec<herald_core::domain::points::PointsPlanConfig> =
        state.points_repository.list_plan_configs(realm_id).await?;

    // Find config for this plan
    let config = plan_config
        .into_iter()
        .find(|c| c.plan_id == plan_id)
        .ok_or_else(|| CoreError::plan_config_not_found(&plan_id.to_string()))?;

    let amount = match recharge_type {
        herald_core::domain::points::RechargeType::Subscribe => {
            // Only grant points on subscribe if grant_on_subscribe is true
            if !config.active || !config.grant_on_subscribe || config.points_per_period <= 0 {
                tracing::info!(
                    plan_id = %plan_id,
                    "Points recharge skipped: config inactive, grant_on_subscribe false, or no points"
                );
                return Ok(());
            }
            config.points_per_period
        }
        herald_core::domain::points::RechargeType::Renewal => {
            // For renewals, grant points if this is a periodic grant (not "once")
            if !config.active
                || config.grant_period_type == GrantPeriodType::Once.as_str()
                || config.points_per_period <= 0
            {
                tracing::info!(
                    plan_id = %plan_id,
                    "Points recharge skipped: config inactive, one-time grant, or no points"
                );
                return Ok(());
            }
            config.points_per_period
        }
    };

    // Call internal recharge
    let _transaction = state
        .points_service
        .recharge_points_internal(
            realm_id,
            user_id,
            Some(plan_id), // Use plan_id to find plan config for max_periods check
            amount,
            recharge_type.clone(),
            Some(event_id), // Use webhook event_id as external_ref_id
        )
        .await?;

    tracing::info!(
        realm_id = %realm_id,
        plan_id = %plan_id,
        amount,
        recharge_type = %recharge_type.as_str(),
        "Points recharged successfully for subscription event"
    );

    Ok(())
}

/// Type alias for subscription metadata extracted from webhook events
type SubscriptionMetadata = (Option<Uuid>, Option<Uuid>, Option<Uuid>);

/// Extract user_id, client_app_id and plan_id from webhook event metadata
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

    let plan_id = event
        .object
        .get("metadata")
        .and_then(|m: &serde_json::Value| m.get("planId"))
        .and_then(|id: &serde_json::Value| id.as_str())
        .and_then(|s: &str| Uuid::parse_str(s).ok());

    Ok((user_id, client_app_id, plan_id))
}

/// Handle checkout.completed event
async fn handle_checkout_completed(
    state: &AppState,
    event: &CreemWebhookEvent,
) -> Result<(), CoreError> {
    tracing::info!("Handling checkout.completed: {}", event.id);

    // Extract realm_id from event metadata
    let realm_id = extract_realm_id(event)?;

    // Extract subscription metadata
    let (user_id, client_app_id, plan_id) = extract_subscription_metadata(event)?;

    // Check if subscription already exists (idempotency)
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
        return Ok(()); // Idempotent: already processed
    }

    // For checkout.completed, we create a new subscription
    let now = Utc::now();

    // Check if plan exists and validate
    let plan = if let Some(pid) = plan_id {
        match state
            .billing_repository
            .find_subscription_plan_by_id(pid)
            .await
        {
            Ok(Some(plan)) => {
                if plan.realm_id != realm_id {
                    return Err(CoreError::BadRequest(
                        "Plan realm mismatch in webhook metadata".to_string(),
                    ));
                }
                Some(plan)
            }
            Ok(None) => {
                // Plan not found, log warning but continue with defaults
                tracing::warn!(
                    plan_id = %pid,
                    "Plan not found in webhook metadata, using default values"
                );
                None
            }
            Err(e) => return Err(e),
        }
    } else {
        None
    };

    // Determine subscription status based on plan trial_days
    let (status, current_period_end) = if let Some(ref p) = plan {
        if p.trial_days > 0 {
            // Trial period: subscription starts in trialing status
            let trial_end = now + chrono::Duration::days(p.trial_days as i64);
            (SubscriptionStatus::Trialing, trial_end)
        } else {
            // No trial: subscription starts in active status
            (SubscriptionStatus::Active, now + chrono::Duration::days(30))
        }
    } else {
        // No plan associated: default to active
        (SubscriptionStatus::Active, now + chrono::Duration::days(30))
    };

    // Determine billing period from plan
    let billing_period = if let Some(ref p) = plan {
        match p.r#type {
            SubscriptionPlanType::Monthly => {
                herald_core::domain::billing::entities::BillingPeriod::Monthly
            }
            SubscriptionPlanType::Yearly => {
                herald_core::domain::billing::entities::BillingPeriod::Yearly
            }
        }
    } else {
        herald_core::domain::billing::entities::BillingPeriod::Monthly
    };

    // Create new subscription for checkout completion
    let tier = SubscriptionTier::Starter; // Default tier for new subscriptions

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
        status,
        tier,
        client_app_id,
        plan_id,
        billing_period,
        cancel_at: None,
        current_period_start: Some(now),
        current_period_end: Some(current_period_end),
        cancel_at_period_end: false,
        created_at: now,
        updated_at: now,
    };

    let sub = state.billing_repository.create_subscription(sub).await?;

    // Create history event
    let history_event = SubscriptionHistoryService::create_subscription_created_event(
        &sub,
        Some(ACTOR_WEBHOOK.to_string()),
    );
    state
        .billing_repository
        .save_history_event(history_event)
        .await?;

    // Recharge points if user_id and plan_id are present
    if let Some(pid) = plan_id
        && let Some(uid) = user_id
        && let Err(e) = recharge_points_for_subscription(
            state,
            &realm_id,
            uid,
            pid,
            herald_core::domain::points::RechargeType::Subscribe,
            event.id.clone(),
        )
        .await
    {
        tracing::error!(
            error = %e,
            subscription_id = %sub.id,
            plan_id = %pid,
            user_id = %uid,
            "Failed to recharge points for checkout completion"
        );
        // Don't fail the webhook processing, just log the error
    }

    tracing::info!(
        "Subscription created from checkout.completed for realm: {}",
        realm_id
    );

    Ok(())
}

/// Handle subscription.paid event
async fn handle_subscription_paid(
    state: &AppState,
    event: &CreemWebhookEvent,
    realm_id: String,
) -> Result<(), CoreError> {
    tracing::info!("Handling subscription.paid for realm: {}", realm_id);

    // Parse subscription object
    let creem_sub: CreemSubscription =
        serde_json::from_value(event.object.clone()).map_err(|e| {
            CoreError::InternalServerError(format!("Failed to parse subscription object: {}", e))
        })?;

    // Check if subscription already exists
    let existing_sub = state
        .billing_repository
        .find_by_external_subscription_id(&creem_sub.id, "creem")
        .await?;
    let is_new_subscription = existing_sub.is_none();

    let now = Utc::now();

    // Extract user_id, client_app_id and plan_id from webhook metadata
    let (user_id, client_app_id, plan_id) = extract_subscription_metadata(event)?;

    // Parse the actual status from Creem instead of hardcoding
    let status = parse_creem_status(&creem_sub.status)?;

    // If plan_id is present in metadata, validate product/currency consistency.
    if let Some(pid) = plan_id
        && let Some(plan) = state
            .billing_repository
            .find_subscription_plan_by_id(pid)
            .await?
    {
        if plan.realm_id != realm_id {
            return Err(CoreError::BadRequest(
                "Plan realm mismatch in webhook metadata".to_string(),
            ));
        }

        // Validate external_product_id using SubscriptionPlanPaymentProvider mapping
        let providers = state
            .billing_repository
            .list_subscription_plan_payment_providers(plan.id)
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to load payment providers for plan {}: {}",
                    plan.id,
                    e
                );
                CoreError::DatabaseError(format!("Failed to validate payment provider: {}", e))
            })?;

        let creem_provider = providers
            .iter()
            .find(|p| p.payment_provider == "creem" && p.enabled)
            .ok_or_else(|| {
                CoreError::BadRequest(
                    "Plan does not have an active Creem payment provider configured".to_string(),
                )
            })?;

        if creem_provider.external_product_id != creem_sub.product.id {
            return Err(CoreError::BadRequest(
                "Plan external_product_id does not match webhook product".to_string(),
            ));
        }

        if !plan
            .currency
            .eq_ignore_ascii_case(&creem_sub.product.currency)
        {
            return Err(CoreError::BadRequest(
                "Plan currency does not match webhook product currency".to_string(),
            ));
        }
    }

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

        // Update existing subscription
        sub.status = status;
        sub.current_period_start = creem_sub
            .current_period_start
            .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0));
        sub.current_period_end = creem_sub
            .current_period_end
            .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0));
        sub.updated_at = now;

        state.billing_repository.update_subscription(sub).await?;
    } else {
        // Create new subscription
        let tier = determine_tier_from_product(&creem_sub.product.id);

        let sub = Subscription {
            id: Uuid::now_v7(),
            realm_id: realm_id.clone(),
            user_id,
            external_subscription_id: creem_sub.id.clone(),
            external_product_id: creem_sub.product.id.clone(),
            payment_provider: "creem".to_string(),
            status, // Use parsed status instead of hardcoding
            tier,
            client_app_id,
            plan_id,
            billing_period: herald_core::domain::billing::entities::BillingPeriod::Monthly,
            cancel_at: None,
            current_period_start: creem_sub
                .current_period_start
                .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0)),
            current_period_end: creem_sub
                .current_period_end
                .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0)),
            cancel_at_period_end: creem_sub.cancel_at_period_end.unwrap_or(false),
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

    if let Some(pid) = plan_id {
        if let Some(uid) = user_id {
            let recharge_type = if is_new_subscription {
                herald_core::domain::points::RechargeType::Subscribe
            } else {
                herald_core::domain::points::RechargeType::Renewal
            };

            if let Err(e) = recharge_points_for_subscription(
                state,
                &realm_id,
                uid,
                pid,
                recharge_type.clone(),
                event.id.clone(),
            )
            .await
            {
                tracing::error!(
                    error = %e,
                    subscription_id = %creem_sub.id,
                    plan_id = %pid,
                    user_id = %uid,
                    recharge_type = %recharge_type.as_str(),
                    "Failed to recharge points for subscription event"
                );
                // Don't fail the webhook processing, just log the error
            }
        } else {
            tracing::warn!(
                subscription_id = %creem_sub.id,
                plan_id = %pid,
                event_id = %event.id,
                "Skipping points recharge for subscription event because userId metadata is missing"
            );
        }
    }

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

        // Create history event before updating
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

        // Save history event
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
        sub.tier = SubscriptionTier::Free; // Downgrade to free
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

/// Handle subscription.updated event (plan upgrade/downgrade)
async fn handle_subscription_updated(
    state: &AppState,
    event: &CreemWebhookEvent,
    realm_id: String,
) -> Result<(), CoreError> {
    tracing::info!("Handling subscription.updated for realm: {}", realm_id);

    // Parse subscription object
    let creem_sub: CreemSubscription =
        serde_json::from_value(event.object.clone()).map_err(|e| {
            CoreError::InternalServerError(format!("Failed to parse subscription object: {}", e))
        })?;

    // Use the actual status from Creem
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

        // Create history event
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

        // Create history event with dispute details
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

/// Handle refund.created event (audit only - no access revocation)
async fn handle_refund_created(
    _state: &AppState,
    event: &CreemWebhookEvent,
    _realm_id: String,
) -> Result<(), CoreError> {
    let refund: CreemRefund = serde_json::from_value(event.object.clone())?;

    // Only log for audit - do not revoke access
    tracing::info!(
        "Refund created - subscription: {}, amount: {} {}, reason: {:?}",
        refund.subscription_id,
        refund.amount,
        refund.currency,
        refund.reason
    );

    // Future: Could create a refunds table for audit tracking
    // Currently, just log the event

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
    // Note: client_app_id and plan_id are not used in update handlers, only in paid event
    let _metadata = extract_subscription_metadata(event)?;

    // Save status string before moving the value
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

        // Create updated subscription for history event
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

        // Create history event
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

/// Determine subscription tier from product ID
pub(crate) fn determine_tier_from_product(product_id: &str) -> SubscriptionTier {
    // Check in order of specificity (most specific first)
    // Product IDs are case-sensitive

    // Check for enterprise
    if product_id.contains("enterprise") {
        SubscriptionTier::Enterprise
    }
    // Check for professional (contains "pro", so check before "pro")
    else if product_id.contains("professional") {
        SubscriptionTier::Professional
    }
    // Check for starter
    else if product_id.contains("starter") {
        SubscriptionTier::Starter
    }
    // Check for free before pro (since "free" doesn't contain "pro")
    else if product_id.contains("free") {
        SubscriptionTier::Free
    }
    // Check for pro as a separate word (not part of another word)
    else if product_id.split('_').any(|word| word == "pro") {
        SubscriptionTier::Professional
    } else {
        SubscriptionTier::Free
    }
}

// ============================================================================
// Product Management Handlers
// ============================================================================

/// Convert domain Product to API ProductResponse
pub fn product_to_response(product: Product) -> ProductResponse {
    ProductResponse {
        id: product.id,
        realm_id: product.realm_id,
        code: product.code,
        title: product.title,
        description: product.description,
        enabled: product.enabled,
        plans_count: product.plans_count,
        created_at: product.created_at.to_rfc3339(),
        updated_at: product.updated_at.to_rfc3339(),
    }
}

/// Convert domain SubscriptionPlan to SubscriptionPlanSummaryForProduct
fn subscription_plan_to_product_summary(
    plan: SubscriptionPlan,
) -> SubscriptionPlanSummaryForProduct {
    SubscriptionPlanSummaryForProduct {
        id: plan.id,
        name: plan.name,
        title: plan.title,
        r#type: plan.r#type.as_str().to_string(),
        price: plan.price,
        currency: plan.currency,
        active: plan.active,
        sort_order: plan.sort_order,
    }
}

/// List all products for a realm
#[utoipa::path(
    get,
    path = "/api/bill/{realmId}/products",
    tag = "billing",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    responses(
        (status = 200, description = "Products listed successfully", body = ListProductsResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_products(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
) -> Result<Json<ListProductsResponse>, ApiError> {
    tracing::info!("Listing products for realm: {}", realm_id);

    // Check billing.view permission
    require_billing_permission(&state, &identity, &realm_id, "view").await?;

    let products = state
        .product_service
        .list_products(identity, &realm_id, None)
        .await?;

    Ok(Json(ListProductsResponse {
        products: products.into_iter().map(product_to_response).collect(),
    }))
}

/// Create a new product for a realm
#[utoipa::path(
    post,
    path = "/api/bill/{realmId}/products",
    tag = "billing",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    request_body = CreateProductRequest,
    responses(
        (status = 201, description = "Product created successfully", body = ProductResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_product(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Json(request): Json<CreateProductRequest>,
) -> Result<(StatusCode, Json<ProductResponse>), ApiError> {
    tracing::info!(
        "Creating product '{}' for realm: {}",
        request.code,
        realm_id
    );

    // Check billing.manage permission
    require_billing_permission(&state, &identity, &realm_id, "manage").await?;

    request
        .validate()
        .map_err(|e: validator::ValidationErrors| {
            CoreError::BadRequest(format!("Validation failed: {}", e))
        })?;

    let input = CreateProductInput {
        realm_id: realm_id.clone(),
        code: request.code,
        title: request.title,
        description: request.description,
    };

    let product = state
        .product_service
        .create_product(identity, &realm_id, input)
        .await?;

    Ok((StatusCode::CREATED, Json(product_to_response(product))))
}

/// Get a specific product with its plans
#[utoipa::path(
    get,
    path = "/api/bill/{realmId}/products/{productId}",
    tag = "billing",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("productId" = Uuid, Path, description = "Product ID")
    ),
    responses(
        (status = 200, description = "Product found", body = ProductDetailResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Product not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_product(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, product_id)): Path<(String, Uuid)>,
) -> Result<Json<ProductDetailResponse>, ApiError> {
    tracing::info!("Getting product {} for realm: {}", product_id, realm_id);

    // Check billing.view permission
    require_billing_permission(&state, &identity, &realm_id, "view").await?;

    let product = state
        .product_service
        .get_product(identity, &realm_id, product_id)
        .await?;

    let plans = state
        .billing_repository
        .find_subscription_plans_by_product(&realm_id, product_id)
        .await?;

    Ok(Json(ProductDetailResponse {
        id: product.id,
        realm_id: product.realm_id,
        code: product.code,
        title: product.title,
        description: product.description,
        enabled: product.enabled,
        plans_count: product.plans_count,
        plans: plans
            .into_iter()
            .map(subscription_plan_to_product_summary)
            .collect(),
        created_at: product.created_at.to_rfc3339(),
        updated_at: product.updated_at.to_rfc3339(),
    }))
}

/// Update a product
#[utoipa::path(
    patch,
    path = "/api/bill/{realmId}/products/{productId}",
    tag = "billing",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("productId" = Uuid, Path, description = "Product ID")
    ),
    request_body = UpdateProductRequest,
    responses(
        (status = 200, description = "Product updated successfully", body = ProductResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Product not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_product(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, product_id)): Path<(String, Uuid)>,
    Json(request): Json<UpdateProductRequest>,
) -> Result<(StatusCode, Json<ProductResponse>), ApiError> {
    tracing::info!("Updating product {} for realm: {}", product_id, realm_id);

    // Check billing.manage permission
    require_billing_permission(&state, &identity, &realm_id, "manage").await?;

    request
        .validate()
        .map_err(|e: validator::ValidationErrors| {
            CoreError::BadRequest(format!("Validation failed: {}", e))
        })?;

    let input = UpdateProductInput {
        title: request.title,
        description: request.description,
        enabled: request.enabled,
    };

    let product = state
        .product_service
        .update_product(identity, &realm_id, product_id, input)
        .await?;

    Ok((StatusCode::OK, Json(product_to_response(product))))
}

/// Delete a product
#[utoipa::path(
    delete,
    path = "/api/bill/{realmId}/products/{productId}",
    tag = "billing",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("productId" = Uuid, Path, description = "Product ID")
    ),
    responses(
        (status = 204, description = "Product deleted successfully"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions or product has associated plans", body = ErrorResponse),
        (status = 404, description = "Product not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_product(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, product_id)): Path<(String, Uuid)>,
) -> Result<StatusCode, ApiError> {
    tracing::info!("Deleting product {} for realm: {}", product_id, realm_id);

    // Check billing.manage permission
    require_billing_permission(&state, &identity, &realm_id, "manage").await?;

    state
        .product_service
        .delete_product(identity, &realm_id, product_id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Get plans for a specific product
#[utoipa::path(
    get,
    path = "/api/bill/{realmId}/products/{productId}/plans",
    tag = "billing",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("productId" = Uuid, Path, description = "Product ID")
    ),
    responses(
        (status = 200, description = "Product plans listed successfully", body = ListSubscriptionPlansResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Product not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_product_plans(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, product_id)): Path<(String, Uuid)>,
) -> Result<Json<ListSubscriptionPlansResponse>, ApiError> {
    tracing::info!(
        "Getting plans for product {} in realm: {}",
        product_id,
        realm_id
    );

    // Check billing.view permission
    require_billing_permission(&state, &identity, &realm_id, "view").await?;

    let plans = state
        .product_service
        .list_plans_for_product(identity, &realm_id, product_id)
        .await?;

    Ok(Json(ListSubscriptionPlansResponse {
        plans: plans
            .into_iter()
            .map(subscription_plan_to_response)
            .collect(),
    }))
}
