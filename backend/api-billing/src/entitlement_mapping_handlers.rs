use axum::{
    Json,
    extract::{Extension, Path, Query, State},
};
use uuid::Uuid;

use crate::handlers::require_billing_permission;
use crate::types::{
    EntitlementMappingListResponse, EntitlementMappingQuery, EntitlementMappingResponse,
    SyncProviderRequest, SyncProviderResponse, UpdateEntitlementMappingRequest,
};
use herald_api_base::application::http::server::api_entities::ApiError;
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::billing::BillingRepository;
use herald_core::domain::billing::entities::EntitlementMapping;

/// Convert domain EntitlementMapping to API response
fn mapping_to_response(m: EntitlementMapping) -> EntitlementMappingResponse {
    EntitlementMappingResponse {
        id: m.id,
        payment_provider: m.payment_provider,
        external_product_id: m.external_product_id,
        external_price_id: m.external_price_id,
        entitlement_key: m.entitlement_key,
        billing_type: m.billing_type.map(|bt| bt.as_str().to_string()),
        billing_period: m.billing_period,
        points_per_period: m.points_per_period,
        grant_period_type: m.grant_period_type,
        validity_days: m.validity_days,
        grant_on_subscribe: m.grant_on_subscribe,
        max_periods: m.max_periods,
        enabled: m.enabled,
        provider_product_info: m.provider_product_info,
        synced_at: m.synced_at.map(|dt| dt.to_rfc3339()),
        created_at: m.created_at.to_rfc3339(),
        updated_at: m.updated_at.to_rfc3339(),
    }
}

/// List entitlement mappings for a realm
#[utoipa::path(
    get,
    path = "/api/bill/{realmId}/entitlement-mappings",
    tag = "billing",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    responses(
        (status = 200, description = "Entitlement mappings listed successfully", body = EntitlementMappingListResponse),
        (status = 401, description = "Unauthorized", body = herald_api_base::application::http::server::api_entities::ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = herald_api_base::application::http::server::api_entities::ErrorResponse),
        (status = 500, description = "Internal server error", body = herald_api_base::application::http::server::api_entities::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_entitlement_mappings(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Query(query): Query<EntitlementMappingQuery>,
) -> Result<Json<EntitlementMappingListResponse>, ApiError> {
    tracing::info!("Listing entitlement mappings for realm: {}", realm_id);

    require_billing_permission(&state, &identity, &realm_id, "view").await?;

    let (mappings, total) = state
        .billing_repository
        .list_entitlement_mappings(
            &realm_id,
            query.payment_provider.as_deref(),
            query.enabled,
            query.page,
            query.page_size,
        )
        .await
        .map_err(|e| {
            tracing::error!(
                realm_id = %realm_id,
                error = %e,
                "Failed to list entitlement mappings"
            );
            ApiError::internal("Failed to list entitlement mappings".to_string())
        })?;

    let items: Vec<EntitlementMappingResponse> =
        mappings.into_iter().map(mapping_to_response).collect();

    Ok(Json(EntitlementMappingListResponse {
        items,
        total: total as i64,
    }))
}

/// Get a single entitlement mapping
#[utoipa::path(
    get,
    path = "/api/bill/{realmId}/entitlement-mappings/{mappingId}",
    tag = "billing",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("mappingId" = Uuid, Path, description = "Mapping ID")
    ),
    responses(
        (status = 200, description = "Entitlement mapping found", body = EntitlementMappingResponse),
        (status = 401, description = "Unauthorized", body = herald_api_base::application::http::server::api_entities::ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = herald_api_base::application::http::server::api_entities::ErrorResponse),
        (status = 404, description = "Mapping not found", body = herald_api_base::application::http::server::api_entities::ErrorResponse),
        (status = 500, description = "Internal server error", body = herald_api_base::application::http::server::api_entities::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_entitlement_mapping(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, mapping_id)): Path<(String, Uuid)>,
) -> Result<Json<EntitlementMappingResponse>, ApiError> {
    tracing::info!(
        "Getting entitlement mapping {} for realm: {}",
        mapping_id,
        realm_id
    );

    require_billing_permission(&state, &identity, &realm_id, "view").await?;

    let mapping = state
        .billing_repository
        .find_entitlement_mapping_by_id(mapping_id)
        .await
        .map_err(|e| {
            tracing::error!(
                mapping_id = %mapping_id,
                error = %e,
                "Failed to get entitlement mapping"
            );
            ApiError::internal("Failed to get entitlement mapping".to_string())
        })?
        .ok_or_else(|| ApiError::not_found("Mapping not found"))?;

    if mapping.realm_id != realm_id {
        return Err(ApiError::not_found("Mapping not found"));
    }

    Ok(Json(mapping_to_response(mapping)))
}

/// Update an entitlement mapping
#[utoipa::path(
    patch,
    path = "/api/bill/{realmId}/entitlement-mappings/{mappingId}",
    tag = "billing",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("mappingId" = Uuid, Path, description = "Mapping ID")
    ),
    request_body = UpdateEntitlementMappingRequest,
    responses(
        (status = 200, description = "Entitlement mapping updated successfully", body = EntitlementMappingResponse),
        (status = 400, description = "Bad request", body = herald_api_base::application::http::server::api_entities::ErrorResponse),
        (status = 401, description = "Unauthorized", body = herald_api_base::application::http::server::api_entities::ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = herald_api_base::application::http::server::api_entities::ErrorResponse),
        (status = 404, description = "Mapping not found", body = herald_api_base::application::http::server::api_entities::ErrorResponse),
        (status = 500, description = "Internal server error", body = herald_api_base::application::http::server::api_entities::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_entitlement_mapping(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, mapping_id)): Path<(String, Uuid)>,
    Json(request): Json<UpdateEntitlementMappingRequest>,
) -> Result<Json<EntitlementMappingResponse>, ApiError> {
    tracing::info!(
        "Updating entitlement mapping {} for realm: {}",
        mapping_id,
        realm_id
    );

    require_billing_permission(&state, &identity, &realm_id, "manage").await?;

    // Validate entitlement_key format if provided
    if let Some(ref key) = request.entitlement_key {
        if key.is_empty() || key.len() > 64 {
            return Err(ApiError::bad_request(
                "Invalid entitlement key (must be 1-64 characters)".to_string(),
            ));
        }
        if !key
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(ApiError::bad_request(
                "Invalid entitlement key (must match [a-z0-9-])".to_string(),
            ));
        }
    }

    let existing = state
        .billing_repository
        .find_entitlement_mapping_by_id(mapping_id)
        .await
        .map_err(|e| {
            tracing::error!(
                mapping_id = %mapping_id,
                error = %e,
                "Failed to find entitlement mapping for update"
            );
            ApiError::internal("Failed to find entitlement mapping".to_string())
        })?
        .ok_or_else(|| ApiError::not_found("Mapping not found"))?;

    if existing.realm_id != realm_id {
        return Err(ApiError::not_found("Mapping not found"));
    }

    if let Some(points) = request.points_per_period
        && points < 0
    {
        return Err(ApiError::bad_request(
            "points_per_period must be non-negative".to_string(),
        ));
    }

    let updated = EntitlementMapping {
        id: existing.id,
        realm_id: existing.realm_id,
        payment_provider: existing.payment_provider,
        external_product_id: existing.external_product_id,
        external_price_id: existing.external_price_id,
        entitlement_key: request.entitlement_key.unwrap_or(existing.entitlement_key),
        billing_type: existing.billing_type,
        billing_period: existing.billing_period,
        points_per_period: request.points_per_period.or(existing.points_per_period),
        grant_period_type: request.grant_period_type.or(existing.grant_period_type),
        validity_days: request.validity_days.or(existing.validity_days),
        grant_on_subscribe: request
            .grant_on_subscribe
            .unwrap_or(existing.grant_on_subscribe),
        max_periods: request.max_periods.or(existing.max_periods),
        enabled: request.enabled.unwrap_or(existing.enabled),
        provider_product_info: existing.provider_product_info,
        synced_at: existing.synced_at,
        created_at: existing.created_at,
        updated_at: chrono::Utc::now(),
    };

    let updated = state
        .billing_repository
        .update_entitlement_mapping(updated)
        .await
        .map_err(|e| {
            tracing::error!(
                mapping_id = %mapping_id,
                error = %e,
                "Failed to update entitlement mapping"
            );
            ApiError::internal("Failed to update entitlement mapping".to_string())
        })?;

    Ok(Json(mapping_to_response(updated)))
}

/// Sync provider products into entitlement mappings
#[utoipa::path(
    post,
    path = "/api/bill/{realmId}/entitlement-mappings/sync",
    tag = "billing",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    request_body = SyncProviderRequest,
    responses(
        (status = 200, description = "Provider products synced successfully", body = SyncProviderResponse),
        (status = 400, description = "Bad request - Provider not configured", body = herald_api_base::application::http::server::api_entities::ErrorResponse),
        (status = 401, description = "Unauthorized", body = herald_api_base::application::http::server::api_entities::ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = herald_api_base::application::http::server::api_entities::ErrorResponse),
        (status = 500, description = "Internal server error", body = herald_api_base::application::http::server::api_entities::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn sync_provider_products(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Json(request): Json<SyncProviderRequest>,
) -> Result<Json<SyncProviderResponse>, ApiError> {
    tracing::info!(
        "Syncing provider products for provider '{}' in realm: {}",
        request.payment_provider,
        realm_id
    );

    require_billing_permission(&state, &identity, &realm_id, "manage").await?;

    // Provider product sync requires the ProviderApiPort infrastructure
    // which is wired in BE-D05. For now, return an error indicating
    // the sync service is not yet wired.
    // NOTE: This handler will be fully functional once BE-D05 adds
    // provider_product_sync_service to AppState.
    Err(ApiError::internal(
        "Provider product sync service not yet configured. This feature will be available after infrastructure wiring (BE-D05).".to_string(),
    ))
}
