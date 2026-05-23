// Points Package API Handlers

use axum::{
    Json,
    extract::{Extension, Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use herald_api_base::application::http::common::error_helpers::core_error_to_api_error;
use herald_api_base::application::http::server::api_entities::ApiError;
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::points_package::{
    CreatePaymentProviderMappingInput, CreatePointsPackageInput, UpdatePaymentProviderMappingInput,
    UpdatePointsPackageInput,
};

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreatePointsPackageRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    #[validate(length(min = 1))]
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(max = 500))]
    pub description: Option<String>,
    #[validate(range(min = 1))]
    pub points: i64,
    #[validate(range(min = 1))]
    pub price: i64,
    #[validate(length(min = 3, max = 3))]
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePointsPackageRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 1))]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(max = 500))]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 1))]
    pub price: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 3, max = 3))]
    pub currency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PointsPackageResponse {
    pub id: Uuid,
    pub realm_id: String,
    pub name: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub points: i64,
    pub price: i64,
    pub currency: String,
    pub sort_order: i32,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListPointsPackagesResponse {
    pub packages: Vec<PointsPackageResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreatePaymentProviderMappingRequest {
    #[validate(length(min = 1))]
    pub payment_provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_product_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePaymentProviderMappingRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_product_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaymentProviderMappingResponse {
    pub id: Uuid,
    pub points_package_id: Uuid,
    pub payment_provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_product_id: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListPaymentProviderMappingsResponse {
    pub mappings: Vec<PaymentProviderMappingResponse>,
}

// ============================================================================
// Conversion Helpers
// ============================================================================

fn points_package_to_response(
    package: herald_core::domain::points_package::PointsPackage,
) -> PointsPackageResponse {
    PointsPackageResponse {
        id: package.id,
        realm_id: package.realm_id,
        name: package.name,
        title: package.title,
        description: package.description,
        points: package.points,
        price: package.price,
        currency: package.currency,
        sort_order: package.sort_order,
        enabled: package.enabled,
        created_at: package.created_at.to_rfc3339(),
        updated_at: package.updated_at.to_rfc3339(),
    }
}

fn payment_provider_mapping_to_response(
    mapping: herald_core::domain::points_package::PointsPackagePaymentProvider,
) -> PaymentProviderMappingResponse {
    PaymentProviderMappingResponse {
        id: mapping.id,
        points_package_id: mapping.points_package_id,
        payment_provider: mapping.payment_provider,
        external_product_id: mapping.external_product_id,
        enabled: mapping.enabled,
        created_at: mapping.created_at.to_rfc3339(),
        updated_at: mapping.updated_at.to_rfc3339(),
    }
}

#[utoipa::path(
    post,
    path = "/api/bill/{realmId}/points-packages",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    request_body = CreatePointsPackageRequest,
    responses(
        (status = 201, description = "Points package created successfully", body = PointsPackageResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - insufficient permissions"),
        (status = 409, description = "Points package name already exists")
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_points_package(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Json(input): Json<CreatePointsPackageRequest>,
) -> Result<(StatusCode, Json<PointsPackageResponse>), ApiError> {
    input
        .validate()
        .map_err(|e| ApiError::bad_request(format!("Invalid request: {}", e)))?;

    let service = &state.points_package_service;

    let create_input = CreatePointsPackageInput {
        realm_id: realm_id.clone(),
        name: input.name,
        title: input.title,
        description: input.description,
        points: input.points,
        price: input.price,
        currency: input.currency,
        sort_order: input.sort_order,
        enabled: input.enabled,
    };

    let package = service
        .create_points_package_authorized(
            &identity,
            state.permission_checker.as_ref(),
            &realm_id,
            create_input,
        )
        .await
        .map_err(|e| core_error_to_api_error(e, "Create points package"))?;

    Ok((
        StatusCode::CREATED,
        Json(points_package_to_response(package)),
    ))
}

#[utoipa::path(
    get,
    path = "/api/bill/{realmId}/points-packages",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("enabledOnly" = Option<bool>, Query, description = "Filter to only enabled packages")
    ),
    responses(
        (status = 200, description = "Points packages listed successfully", body = ListPointsPackagesResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - insufficient permissions")
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_points_packages(
    State(state): State<AppState>,
    Path(realm_id): Path<String>,
    Extension(identity): Extension<Identity>,
) -> Result<Json<ListPointsPackagesResponse>, ApiError> {
    let service = &state.points_package_service;

    let packages = service
        .list_visible_points_packages(&identity, state.permission_checker.as_ref(), &realm_id)
        .await
        .map_err(|e| core_error_to_api_error(e, "List points packages"))?;

    let response = ListPointsPackagesResponse {
        packages: packages
            .into_iter()
            .map(points_package_to_response)
            .collect(),
    };

    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/api/bill/{realmId}/points-packages/{packageId}",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("packageId" = Uuid, Path, description = "Points package ID")
    ),
    responses(
        (status = 200, description = "Points package retrieved successfully", body = PointsPackageResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - insufficient permissions"),
        (status = 404, description = "Points package not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_points_package(
    State(state): State<AppState>,
    Path((realm_id, package_id)): Path<(String, Uuid)>,
    Extension(identity): Extension<Identity>,
) -> Result<Json<PointsPackageResponse>, ApiError> {
    let service = &state.points_package_service;

    let package = service
        .get_visible_points_package(
            &identity,
            state.permission_checker.as_ref(),
            &realm_id,
            package_id,
        )
        .await
        .map_err(|e| core_error_to_api_error(e, "Get points package"))?;

    Ok(Json(points_package_to_response(package)))
}

#[utoipa::path(
    patch,
    path = "/api/bill/{realmId}/points-packages/{packageId}",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("packageId" = Uuid, Path, description = "Points package ID")
    ),
    request_body = UpdatePointsPackageRequest,
    responses(
        (status = 200, description = "Points package updated successfully", body = PointsPackageResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - insufficient permissions"),
        (status = 404, description = "Points package not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_points_package(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, package_id)): Path<(String, Uuid)>,
    Json(input): Json<UpdatePointsPackageRequest>,
) -> Result<Json<PointsPackageResponse>, ApiError> {
    input
        .validate()
        .map_err(|e| ApiError::bad_request(format!("Invalid request: {}", e)))?;

    let service = &state.points_package_service;

    let update_input = UpdatePointsPackageInput {
        id: package_id,
        realm_id: realm_id.clone(),
        title: input.title,
        description: input.description,
        price: input.price,
        currency: input.currency,
        sort_order: input.sort_order,
        enabled: input.enabled,
    };

    let package = service
        .update_points_package_authorized(
            &identity,
            state.permission_checker.as_ref(),
            &realm_id,
            package_id,
            update_input,
        )
        .await
        .map_err(|e| core_error_to_api_error(e, "Update points package"))?;

    Ok(Json(points_package_to_response(package)))
}

#[utoipa::path(
    delete,
    path = "/api/bill/{realmId}/points-packages/{packageId}",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("packageId" = Uuid, Path, description = "Points package ID")
    ),
    responses(
        (status = 204, description = "Points package deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - insufficient permissions"),
        (status = 404, description = "Points package not found"),
        (status = 409, description = "Cannot delete points package with purchase records")
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_points_package(
    State(state): State<AppState>,
    Path((realm_id, package_id)): Path<(String, Uuid)>,
    Extension(identity): Extension<Identity>,
) -> Result<(), ApiError> {
    let service = &state.points_package_service;

    service
        .delete_points_package_authorized(
            &identity,
            state.permission_checker.as_ref(),
            &realm_id,
            package_id,
        )
        .await
        .map_err(|e| core_error_to_api_error(e, "Delete points package"))?;

    Ok(())
}

#[utoipa::path(
    get,
    path = "/api/bill/{realmId}/points-packages/{packageId}/providers",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("packageId" = Uuid, Path, description = "Points package ID")
    ),
    responses(
        (status = 200, description = "Payment provider mappings listed successfully", body = ListPaymentProviderMappingsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - insufficient permissions"),
        (status = 404, description = "Points package not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_payment_provider_mappings(
    State(state): State<AppState>,
    Path((realm_id, package_id)): Path<(String, Uuid)>,
    Extension(identity): Extension<Identity>,
) -> Result<Json<ListPaymentProviderMappingsResponse>, ApiError> {
    let service = &state.points_package_service;

    let mappings = service
        .list_payment_provider_mappings_authorized(
            &identity,
            state.permission_checker.as_ref(),
            &realm_id,
            package_id,
        )
        .await
        .map_err(|e| core_error_to_api_error(e, "List payment provider mappings"))?;

    let response = ListPaymentProviderMappingsResponse {
        mappings: mappings
            .into_iter()
            .map(payment_provider_mapping_to_response)
            .collect(),
    };

    Ok(Json(response))
}

#[utoipa::path(
    post,
    path = "/api/bill/{realmId}/points-packages/{packageId}/providers",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("packageId" = Uuid, Path, description = "Points package ID")
    ),
    request_body = CreatePaymentProviderMappingRequest,
    responses(
        (status = 201, description = "Payment provider mapping created successfully", body = PaymentProviderMappingResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - insufficient permissions"),
        (status = 404, description = "Points package not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_payment_provider_mapping(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, package_id)): Path<(String, Uuid)>,
    Json(input): Json<CreatePaymentProviderMappingRequest>,
) -> Result<(StatusCode, Json<PaymentProviderMappingResponse>), ApiError> {
    input
        .validate()
        .map_err(|e| ApiError::bad_request(format!("Invalid request: {}", e)))?;

    let service = &state.points_package_service;

    let create_input = CreatePaymentProviderMappingInput {
        points_package_id: package_id,
        payment_provider: input.payment_provider,
        external_product_id: input.external_product_id,
        enabled: input.enabled.unwrap_or(true),
    };

    let mapping = service
        .add_payment_provider_mapping_authorized(
            &identity,
            state.permission_checker.as_ref(),
            &realm_id,
            create_input,
        )
        .await
        .map_err(|e| core_error_to_api_error(e, "Create payment provider mapping"))?;

    Ok((
        StatusCode::CREATED,
        Json(payment_provider_mapping_to_response(mapping)),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/bill/{realmId}/points-packages/{packageId}/providers/{mappingId}",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("packageId" = Uuid, Path, description = "Points package ID"),
        ("mappingId" = Uuid, Path, description = "Payment provider mapping ID")
    ),
    request_body = UpdatePaymentProviderMappingRequest,
    responses(
        (status = 200, description = "Payment provider mapping updated successfully", body = PaymentProviderMappingResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - insufficient permissions"),
        (status = 404, description = "Payment provider mapping not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_payment_provider_mapping(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, package_id, mapping_id)): Path<(String, Uuid, Uuid)>,
    Json(input): Json<UpdatePaymentProviderMappingRequest>,
) -> Result<Json<PaymentProviderMappingResponse>, ApiError> {
    input
        .validate()
        .map_err(|e| ApiError::bad_request(format!("Invalid request: {}", e)))?;

    let service = &state.points_package_service;

    let update_input = UpdatePaymentProviderMappingInput {
        id: mapping_id,
        external_product_id: input.external_product_id,
        enabled: input.enabled.unwrap_or(true),
    };

    let mapping = service
        .update_payment_provider_mapping_authorized(
            &identity,
            state.permission_checker.as_ref(),
            &realm_id,
            package_id,
            mapping_id,
            update_input,
        )
        .await
        .map_err(|e| core_error_to_api_error(e, "Update payment provider mapping"))?;

    Ok(Json(payment_provider_mapping_to_response(mapping)))
}

#[utoipa::path(
    delete,
    path = "/api/bill/{realmId}/points-packages/{packageId}/providers/{mappingId}",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("packageId" = Uuid, Path, description = "Points package ID"),
        ("mappingId" = Uuid, Path, description = "Payment provider mapping ID")
    ),
    responses(
        (status = 204, description = "Payment provider mapping deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - insufficient permissions"),
        (status = 404, description = "Payment provider mapping not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_payment_provider_mapping(
    State(state): State<AppState>,
    Path((realm_id, package_id, mapping_id)): Path<(String, Uuid, Uuid)>,
    Extension(identity): Extension<Identity>,
) -> Result<(), ApiError> {
    let service = &state.points_package_service;

    service
        .remove_payment_provider_mapping_authorized(
            &identity,
            state.permission_checker.as_ref(),
            &realm_id,
            package_id,
            mapping_id,
        )
        .await
        .map_err(|e| core_error_to_api_error(e, "Delete payment provider mapping"))?;

    Ok(())
}
