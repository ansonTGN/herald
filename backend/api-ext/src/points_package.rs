// Points Package API for Third-Party Integration
//
// Allows third-party apps to list visible points packages using API Key authentication.

use axum::{
    Json,
    extract::{Extension, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use utoipa::ToSchema;

use herald_api_base::application::http::common::error_codes::ErrorCode;
use herald_api_base::application::http::common::error_helpers::json_error;
use herald_api_base::application::http::server::api_entities::ErrorResponse;
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;

/// Single points package item in the user-facing list response
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExtPointsPackageItem {
    pub id: String,
    pub name: String,
    pub title: String,
    pub description: Option<String>,
    pub points: i64,
    pub price: i64,
    pub currency: String,
    pub package_type: String,
    pub original_price: Option<i64>,
    pub promo_start_time: Option<String>,
    pub promo_end_time: Option<String>,
    pub discount_percent: Option<i32>,
    pub sort_order: i32,
}

/// User-facing points packages list response
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExtPointsPackageListResponse {
    pub packages: Vec<ExtPointsPackageItem>,
}

/// List user-visible points packages for a realm
///
/// Returns all enabled, currently-visible points packages (standard packages and
/// active promotional packages) sorted by active promos first, then sort_order
/// descending, then creation time ascending.
///
/// # Authentication
/// Requires valid API Key via X-API-Key header
///
/// # Realm Isolation
/// The API key must belong to the same realm as the requested packages.
/// Cross-realm requests will return 403 Forbidden.
///
/// # Example
/// ```bash
/// curl -X GET \
///   https://api.example.com/api/ext/realm123/points-packages \
///   -H "X-API-Key: your-api-key"
/// ```
#[utoipa::path(
    get,
    path = "/api/ext/{realmId}/points-packages",
    tag = "ext",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    responses(
        (status = 200, description = "Points packages listed successfully", body = ExtPointsPackageListResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API Key", body = ErrorResponse),
        (status = 403, description = "Forbidden - Cross-realm access attempt", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("api_key" = []))
)]
pub async fn list_points_packages_ext(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
) -> Response {
    let api_key_realm_id = identity.realm_id();

    tracing::info!(
        api_key_realm_id = %api_key_realm_id,
        request_realm_id = %realm_id,
        "Points packages list requested"
    );

    // 1. Check realm isolation - API key must be for the requested realm
    if !identity.has_access_to_realm(&realm_id) {
        tracing::warn!(
            api_key_realm_id = %api_key_realm_id,
            request_realm_id = %realm_id,
            "Cross-realm access attempt blocked"
        );
        return json_error(StatusCode::FORBIDDEN, ErrorCode::CrossRealmAccessForbidden);
    }

    // 2. Query user-visible packages
    let packages = match state
        .points_package_service
        .list_user_visible_packages(&realm_id)
        .await
    {
        Ok(packages) => packages,
        Err(e) => {
            tracing::error!("Failed to list points packages: {}", e);
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, ErrorCode::InternalError);
        }
    };

    // 3. Convert to ext response format
    let items: Vec<ExtPointsPackageItem> = packages
        .into_iter()
        .map(|pkg| {
            let discount_percent = pkg.discount_percent();
            ExtPointsPackageItem {
                id: pkg.id.to_string(),
                name: pkg.name,
                title: pkg.title,
                description: pkg.description,
                points: pkg.points,
                price: pkg.price,
                currency: pkg.currency,
                package_type: pkg.package_type.to_string(),
                original_price: pkg.original_price,
                promo_start_time: pkg.promo_start_time.map(|dt| dt.to_rfc3339()),
                promo_end_time: pkg.promo_end_time.map(|dt| dt.to_rfc3339()),
                discount_percent,
                sort_order: pkg.sort_order,
            }
        })
        .collect();

    tracing::info!(
        realm_id = %realm_id,
        packages_count = items.len(),
        "Points packages retrieved successfully"
    );

    Json(ExtPointsPackageListResponse { packages: items }).into_response()
}
