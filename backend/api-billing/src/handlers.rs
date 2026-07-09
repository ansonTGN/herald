use axum::{
    Json,
    extract::{Extension, Path, Query, State},
};
use chrono::Utc;
use uuid::Uuid;

use crate::types::{
    CancelSubscriptionRequest,
    CancelSubscriptionResponse,
    PurchaseOptionListResponse,
    PurchaseOptionView,
    // Subscription types
    SubscriptionDetailResponse,
    SubscriptionListItemResponse,
    SubscriptionListQuery,
    SubscriptionListResponse,
};

use herald_api_base::application::http::common::auth_utils::{
    AdminIdentity, require_authenticated_user_in_realm,
};
use herald_api_base::application::http::common::error_helpers::core_error_to_api_error;
use herald_api_base::application::http::server::api_entities::{ApiError, ErrorResponse};
use herald_api_base::application::http::state::AppState;
// Import the trait and types from herald_core
use herald_core::domain::authentication::Identity;
use herald_core::domain::billing::entities::BillingType;
use herald_core::domain::billing::{BillingRepository, EntitlementMapping, Subscription};
use herald_core::domain::common::entities::app_errors::CoreError;
use herald_core::domain::payment_attempt::PaymentAttemptRepository;
use herald_core::domain::user::UserRoleRepository;

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
    let admin = AdminIdentity::require(identity.clone(), realm_id, "billing")?;
    admin.require_permission(state, "billing", action).await
}

async fn require_client_app_in_realm(
    state: &AppState,
    realm_id: &str,
    client_app_id: Uuid,
) -> Result<(), ApiError> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM client_app WHERE id = $1 AND realm_id = $2)",
    )
    .bind(client_app_id)
    .bind(realm_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!(
            realm_id = %realm_id,
            client_app_id = %client_app_id,
            error = %e,
            "Failed to validate client app realm ownership"
        );
        ApiError::internal("Failed to validate client app")
    })?;

    if !exists {
        return Err(ApiError::not_found("Client app not found"));
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

    if subscription.realm_id != realm_id {
        return Err(ApiError::not_found("Subscription not found"));
    }

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

    if subscription.realm_id != realm_id {
        return Err(ApiError::not_found("Subscription not found"));
    }

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

/// Map a price-level entitlement mapping to the purchase-page view.
///
/// `display_name` / `amount` / `currency` are read from the
/// `provider_product_info` JSONB cache populated by sync (same source the
/// one-time-mappings read model and checkout price_amount use).
fn mapping_to_purchase_option(m: EntitlementMapping) -> PurchaseOptionView {
    let info = m.provider_product_info.as_ref();
    // Only the one_time+role combo is the gated one-per-user entitlement
    // (design §4.3.2 / §5.4). Points packages and subscriptions are never
    // gated, so `grants_role` is `false` for them even if they carry role
    // grants. `already_owned` is computed per-user in the handler; seeded
    // `false` here and overwritten for gated options.
    let grants_role =
        m.billing_type == Some(BillingType::OneTime) && !m.granted_role_ids.is_empty();
    PurchaseOptionView {
        mapping_id: m.id,
        external_product_id: m.external_product_id,
        external_price_id: m.external_price_id,
        payment_provider: m.payment_provider,
        entitlement_key: m.entitlement_key,
        billing_type: m.billing_type.map(|t| t.as_str().to_string()),
        billing_period: m.billing_period,
        display_name: info
            .and_then(|i| i.get("name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        amount: info.and_then(|i| i.get("price")).and_then(|v| v.as_i64()),
        currency: info
            .and_then(|i| i.get("currency"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        points_per_period: m.points_per_period,
        enabled: m.enabled,
        grants_role,
        already_owned: false,
    }
}

/// List purchasable price-level options for a client app.
///
/// Returns a FLAT list of enabled price-granularity mappings (recurring +
/// one_time) for the purchase page; the frontend groups by
/// `external_product_id` / billing period. Replaces the purchase page's
/// dependency on `list_one_time_mappings` (which only covered one_time).
#[utoipa::path(
    get,
    path = "/api/bill/{realmId}/client/{clientAppId}/purchase-options",
    tag = "billing",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("clientAppId" = Uuid, Path, description = "Client App ID")
    ),
    responses(
        (status = 200, description = "Purchase options listed successfully", body = PurchaseOptionListResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_purchase_options(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, client_app_id)): Path<(String, Uuid)>,
) -> Result<Json<PurchaseOptionListResponse>, ApiError> {
    tracing::info!(
        "Listing purchase options for client app {} in realm {}",
        client_app_id,
        realm_id
    );

    // Purchase-page read is an authenticated-user action. The user id drives
    // the per-option `alreadyOwned` computation (design §4.2.2).
    let user_id = require_authenticated_user_in_realm(&identity, &realm_id, "purchase-options")?;
    require_client_app_in_realm(&state, &realm_id, client_app_id).await?;

    // List ALL enabled price-granularity mappings for the realm (recurring +
    // one_time). Page size is set high to return the full purchasable set in a
    // single page; the purchase page expects a flat list, not pagination.
    let (mappings, _total) = state
        .billing_repository
        .list_entitlement_mappings(&realm_id, None, Some(true), Some(1), Some(200))
        .await
        .map_err(|e| {
            tracing::error!(realm_id = %realm_id, error = %e, "Failed to list purchase options");
            ApiError::internal("Failed to list purchase options".to_string())
        })?;

    // Build the base views (sets `grants_role` from mapping fields), then for
    // the gated combo (one_time + non-empty granted_role_ids) compute
    // `already_owned` for the authenticated user. The options list is per-realm
    // and typically small, so a per-option ownership query is acceptable (the
    // role check is a single indexed lookup; the attempt check is indexed too).
    let mut items: Vec<PurchaseOptionView> = Vec::with_capacity(mappings.len());
    for m in mappings {
        // Capture the gated-combo inputs before `m` is moved into the mapper.
        let grants_role =
            m.billing_type == Some(BillingType::OneTime) && !m.granted_role_ids.is_empty();
        let granted_role_ids: Vec<Uuid> = if grants_role {
            m.granted_role_ids.clone()
        } else {
            Vec::new()
        };
        let mapping_id = m.id;
        let mut view = mapping_to_purchase_option(m);
        if grants_role {
            let has_role = state
                .user_role_repository
                .user_has_any_payment_role(&realm_id, user_id, &granted_role_ids)
                .await
                .map_err(CoreError::from)
                .map_err(|e| core_error_to_api_error(e, "Purchase options ownership check"))?;
            let has_attempt = state
                .payment_attempt_repository
                .has_succeeded_attempt(user_id, mapping_id)
                .await
                .map_err(|e| core_error_to_api_error(e, "Purchase options ownership check"))?;
            view.already_owned = has_role || has_attempt;
        }
        items.push(view);
    }

    Ok(Json(PurchaseOptionListResponse { items }))
}
