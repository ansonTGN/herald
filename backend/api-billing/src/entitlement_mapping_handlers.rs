use axum::{
    Json,
    extract::{Extension, Path, Query, State},
    http::StatusCode,
};
use serde::Serialize;
use uuid::Uuid;

use crate::credit_bucket_handlers::require_points_manage_permission;
use crate::handlers::require_billing_permission;
use crate::types::{
    BatchUpdateEntitlementMappingsRequest, BatchUpdateEntitlementMappingsResponse,
    EntitlementMappingListResponse, EntitlementMappingQuery, EntitlementMappingResponse,
    OneTimeMappingItem, OneTimeMappingListResponse, PartialSyncErrorDto, SyncProviderRequest,
    SyncProviderResponse, UpdateEntitlementMappingRequest,
};
use herald_api_base::application::http::server::api_entities::ApiError;
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::billing::entities::EntitlementMapping;
use herald_core::domain::billing::{BatchMappingError, BillingRepository, SyncStatus};

/// 409 `mapping_in_use` body for a batch save blocked by the active-subscription
/// lock. The whole batch transaction is rolled back.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MappingActiveSubscriptionLockErrorBody {
    pub code: &'static str,
    pub active_subscriptions: i64,
}

/// Convert domain EntitlementMapping to API response
fn mapping_to_response(m: EntitlementMapping) -> EntitlementMappingResponse {
    EntitlementMappingResponse {
        id: m.id,
        payment_provider: m.payment_provider,
        external_product_id: m.external_product_id,
        external_price_id: m.external_price_id,
        bucket_id: m.bucket_id,
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

    require_points_manage_permission(&state, &identity, &realm_id).await?;

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
        // Preserve the bound Bucket; this handler does not expose a way to
        // reassign it via PATCH (bucket assignment is owned elsewhere).
        bucket_id: existing.bucket_id,
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

/// List enabled one-time entitlement mappings for a realm
#[utoipa::path(
    get,
    path = "/api/bill/{realmId}/one-time-mappings",
    tag = "billing",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    responses(
        (status = 200, description = "One-time mappings listed successfully", body = OneTimeMappingListResponse),
        (status = 401, description = "Unauthorized", body = herald_api_base::application::http::server::api_entities::ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = herald_api_base::application::http::server::api_entities::ErrorResponse),
        (status = 500, description = "Internal server error", body = herald_api_base::application::http::server::api_entities::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_one_time_mappings(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
) -> Result<Json<OneTimeMappingListResponse>, ApiError> {
    tracing::info!("Listing one-time mappings for realm: {}", realm_id);

    require_billing_permission(&state, &identity, &realm_id, "view").await?;

    let mappings = state
        .billing_repository
        .list_one_time_mappings(&realm_id)
        .await
        .map_err(|e| {
            tracing::error!(
                realm_id = %realm_id,
                error = %e,
                "Failed to list one-time mappings"
            );
            ApiError::internal("Failed to list one-time mappings".to_string())
        })?;

    let items: Vec<OneTimeMappingItem> = mappings
        .into_iter()
        .map(|m| OneTimeMappingItem {
            id: m.id.to_string(),
            entitlement_key: m.entitlement_key,
            bucket_id: m.bucket_id,
            provider_product_info: m.provider_product_info,
            points_per_period: m.points_per_period,
            payment_provider: m.payment_provider,
            validity_days: m.validity_days,
        })
        .collect();

    Ok(Json(OneTimeMappingListResponse { items }))
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

    require_points_manage_permission(&state, &identity, &realm_id).await?;

    let result = state
        .provider_product_sync_service
        .sync_provider_products(identity, &realm_id, &request.payment_provider)
        .await
        .map_err(ApiError::from)?;

    let sync_status = match result.sync_status {
        SyncStatus::Completed => "completed",
        SyncStatus::Partial => "partial",
        SyncStatus::Failed => "failed",
    }
    .to_string();

    Ok(Json(SyncProviderResponse {
        products_synced: result.products_synced as i64,
        prices_synced: result.prices_synced as i64,
        sync_status,
        error: result.error,
        partial_errors: result
            .partial_errors
            .into_iter()
            .map(|error| PartialSyncErrorDto {
                external_id: error.external_id,
                reason: error.reason,
            })
            .collect(),
    }))
}

/// Batch-save all price mappings for a product.
///
/// Validation/permission order:
/// 1. `billing.manage` (realm boundary + business permission).
/// 2. If any update row carries a credit-strategy field → `points.manage`.
/// 3. `entitlement_key` regex `^[a-z0-9-]{1,64}$` (DB CHECK + handler double).
///
/// Then the repository performs a single-transaction upsert of all the
/// product's price rows: shared-key rename consistency (group-wide), and
/// any row transitioning enabled true→false while protected by an active
/// subscription rolls back the WHOLE transaction (409 with
/// `{ activeSubscriptions }`). Cross-product shared-key rename leaks and
/// cross-realm/product `mapping_id` tampering surface as 400.
#[utoipa::path(
    put,
    path = "/api/bill/{realmId}/entitlement-mappings/batch",
    tag = "billing",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    request_body = BatchUpdateEntitlementMappingsRequest,
    responses(
        (status = 201, description = "Batch saved successfully", body = BatchUpdateEntitlementMappingsResponse),
        (status = 400, description = "Bad request - invalid entitlement key, cross-product shared-key rename, or mapping_id not in this product/realm", body = herald_api_base::application::http::server::api_entities::ErrorResponse),
        (status = 401, description = "Unauthorized", body = herald_api_base::application::http::server::api_entities::ErrorResponse),
        (status = 403, description = "Forbidden - missing billing.manage (or points.manage for credit fields)", body = herald_api_base::application::http::server::api_entities::ErrorResponse),
        (status = 409, description = "Conflict - active subscription protects a disabled mapping (whole batch rolled back)", body = MappingActiveSubscriptionLockErrorBody),
        (status = 500, description = "Internal server error", body = herald_api_base::application::http::server::api_entities::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn batch_update_entitlement_mappings(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Json(request): Json<BatchUpdateEntitlementMappingsRequest>,
) -> Result<(StatusCode, Json<BatchUpdateEntitlementMappingsResponse>), ApiError> {
    tracing::info!(
        provider = %request.payment_provider,
        product = %request.external_product_id,
        update_count = request.updates.len(),
        "Batch update entitlement mappings for realm {}",
        realm_id
    );

    // 1. billing.manage (realm boundary + business permission).
    require_billing_permission(&state, &identity, &realm_id, "manage").await?;

    // 2. points.manage if any row writes a credit-strategy field.
    let touches_credit_fields = request.updates.iter().any(|u| {
        u.points_per_period.is_some()
            || u.grant_period_type.is_some()
            || u.validity_days.is_some()
            || u.grant_on_subscribe.is_some()
            || u.max_periods.is_some()
    });
    if touches_credit_fields {
        require_points_manage_permission(&state, &identity, &realm_id).await?;
    }

    // 3. entitlement_key regex double-check (DB CHECK is the source of truth;
    //    handler mirrors the single-PATCH validation).
    for u in &request.updates {
        if u.entitlement_key.is_empty() || u.entitlement_key.len() > 64 {
            return Err(ApiError::bad_request(format!(
                "Invalid entitlement_key for mapping {}: must be 1-64 characters",
                u.mapping_id
            )));
        }
        if !u
            .entitlement_key
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(ApiError::bad_request(format!(
                "Invalid entitlement_key for mapping {}: must match ^[a-z0-9-]{{1,64}}$",
                u.mapping_id
            )));
        }
        if let Some(points) = u.points_per_period
            && points < 0
        {
            return Err(ApiError::bad_request(format!(
                "points_per_period must be non-negative for mapping {}",
                u.mapping_id
            )));
        }
    }

    // Delegate the transactional write to the repository.
    let input = herald_core::domain::billing::BatchUpdateMappingsInput {
        realm_id: realm_id.clone(),
        payment_provider: request.payment_provider.clone(),
        external_product_id: request.external_product_id.clone(),
        updates: request
            .updates
            .into_iter()
            .map(|u| herald_core::domain::billing::PriceMappingUpdateInput {
                mapping_id: u.mapping_id,
                entitlement_key: u.entitlement_key,
                billing_type: u.billing_type,
                billing_period: u.billing_period,
                points_per_period: u.points_per_period,
                grant_period_type: u.grant_period_type,
                validity_days: u.validity_days,
                grant_on_subscribe: u.grant_on_subscribe,
                max_periods: u.max_periods,
                enabled: u.enabled,
            })
            .collect(),
    };

    let result = state
        .billing_repository
        .batch_update_mappings(input)
        .await
        .map_err(map_batch_error)?;

    let prices = result.prices.into_iter().map(mapping_to_response).collect();
    Ok((
        StatusCode::CREATED,
        Json(BatchUpdateEntitlementMappingsResponse {
            saved: result.saved,
            prices,
        }),
    ))
}

/// Translate [`BatchMappingError`] into the HTTP error contract.
///
/// - `MappingNotInGroup` / `CrossProductSharedKeyRename` → 400 (field-level).
/// - `ActiveSubscriptionLock` → 409 with `{ code, activeSubscriptions }`.
/// - `Other(CoreError)` → preserves the wrapped status (404 / 500 / …).
fn map_batch_error(err: BatchMappingError) -> ApiError {
    match err {
        BatchMappingError::MappingNotInGroup {
            mapping_id,
            provider,
            product,
        } => ApiError::bad_request(format!(
            "mapping {mapping_id} does not belong to provider '{provider}' product '{product}' in this realm"
        )),
        BatchMappingError::CrossProductSharedKeyRename {
            provider,
            product,
            affected_count,
        } => ApiError::bad_request(format!(
            "shared-key rename would affect {affected_count} mapping(s) outside provider '{provider}' product '{product}'"
        )),
        BatchMappingError::ActiveSubscriptionLock {
            active_subscriptions,
            ..
        } => ApiError::conflict_json(MappingActiveSubscriptionLockErrorBody {
            code: "mapping_in_use",
            active_subscriptions,
        }),
        BatchMappingError::Other(core) => ApiError::from(core),
    }
}
