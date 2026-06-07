// Billing API for Third-Party Integration
//
// Allows third-party apps to query billing information using API Key authentication.

use axum::{
    Json,
    extract::{Extension, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use herald_core::domain::authentication::Identity;
use herald_core::domain::billing::BillingRepository;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::authz::require_principal_permission;
use crate::client_app_scope::ensure_client_app_scope;
use crate::client_helper::ClientAppLookup;
use herald_api_base::application::http::common::error_codes::ErrorCode;
use herald_api_base::application::http::common::error_helpers::json_error;
use herald_api_base::application::http::server::api_entities::ErrorResponse;
use herald_api_base::application::http::state::AppState;

/// Subscription detail response (SDK-compatible)
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionDetail {
    pub id: String,
    pub client_app_id: Option<String>,
    pub status: String,
    pub entitlement_key: String,
    pub payment_provider: String,
    pub current_period_start: Option<String>,
    pub current_period_end: Option<String>,
    pub cancel_at: Option<String>,
    pub cancel_at_period_end: Option<bool>,
    pub created_at: String,
    pub updated_at: String,
}

/// Get subscription for a client app
///
/// Returns subscription details for the specified client app.
///
/// # Authentication
/// Requires valid API Key via X-API-Key header
///
/// # Realm Isolation
/// The API key must belong to the same realm as the client app.
/// Cross-realm requests will return 403 Forbidden.
///
/// # Example
/// ```bash
/// curl -X GET \
///   https://api.example.com/api/ext/bill/realm123/client/client-app-123/subscription \
///   -H "X-API-Key: your-api-key"
/// ```
#[utoipa::path(
    get,
    path = "/api/ext/bill/{realmId}/client/{clientAppId}/subscription",
    tag = "ext",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("clientAppId" = String, Path, description = "Client app ID (client_id or UUID)")
    ),
    responses(
        (status = 200, description = "Subscription details retrieved", body = SubscriptionDetail),
        (status = 401, description = "Unauthorized - Invalid or missing API Key", body = ErrorResponse),
        (status = 403, description = "Forbidden - Cross-realm access attempt", body = ErrorResponse),
        (status = 404, description = "Client app not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("api_key" = []))
)]
pub async fn get_subscription(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(params): Path<(String, String)>,
) -> Response {
    let api_key_realm_id = identity.realm_id();
    let (realm_id, client_app_id) = params;
    let client_app_id = client_app_id.clone();

    tracing::info!(
        api_key_realm_id = %api_key_realm_id,
        request_realm_id = %realm_id,
        client_app_id = %client_app_id,
        "Subscription detail query requested"
    );

    // 1. Check realm isolation
    if api_key_realm_id != realm_id {
        tracing::warn!(
            api_key_realm_id = %api_key_realm_id,
            request_realm_id = %realm_id,
            "Cross-realm access attempt blocked"
        );
        return json_error(StatusCode::FORBIDDEN, ErrorCode::CrossRealmAccessForbidden);
    }

    if let Err(resp) =
        require_principal_permission(&state, &identity, &realm_id, "billing", "view").await
    {
        return resp.into_response();
    }

    // 2. Find client app by identifier to get the UUID
    let client_app_lookup = ClientAppLookup::new(state.pool.clone());
    let client_app_uuid: Uuid = match client_app_lookup
        .find_uuid_by_identifier_required(&client_app_id, &realm_id)
        .await
    {
        Ok(uuid) => uuid,
        Err(e) => return e,
    };

    if let Err(resp) = ensure_client_app_scope(&state, &identity, client_app_uuid).await {
        return resp;
    }

    // 3. Query subscription
    let subscription = match state
        .billing_repository
        .find_subscription_by_client_app_id(client_app_uuid)
        .await
    {
        Ok(Some(sub)) => sub,
        Ok(None) => {
            tracing::info!(
                client_app_id = %client_app_id,
                "No subscription found for client app"
            );
            return json_error(StatusCode::NOT_FOUND, ErrorCode::SubscriptionNotFound);
        }
        Err(e) => {
            tracing::error!("Failed to query subscription: {}", e);
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, ErrorCode::InternalError);
        }
    };

    // 4. Build response
    let response = SubscriptionDetail {
        id: subscription.id.to_string(),
        client_app_id: Some(client_app_id.clone()),
        status: subscription.status.as_str().to_string(),
        entitlement_key: subscription.entitlement_key,
        payment_provider: subscription.payment_provider,
        current_period_start: subscription.current_period_start.map(|dt| dt.to_rfc3339()),
        current_period_end: subscription.current_period_end.map(|dt| dt.to_rfc3339()),
        cancel_at: subscription.cancel_at.map(|dt| dt.to_rfc3339()),
        cancel_at_period_end: Some(subscription.cancel_at_period_end),
        created_at: subscription.created_at.to_rfc3339(),
        updated_at: subscription.updated_at.to_rfc3339(),
    };

    tracing::info!(
        client_app_id = %client_app_id,
        subscription_id = %subscription.id,
        status = %response.status,
        "Subscription retrieved successfully"
    );

    Json(response).into_response()
}

// Entitlement mappings are managed via the admin billing API.

/// Single one-time mapping item for external API
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OneTimeMappingItem {
    pub id: String,
    pub entitlement_key: String,
    pub provider_product_info: Option<serde_json::Value>,
    pub points_per_period: Option<i64>,
    pub payment_provider: String,
    pub validity_days: Option<i64>,
}

/// Response for one-time mappings external endpoint
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OneTimeMappingExtResponse {
    pub items: Vec<OneTimeMappingItem>,
}

/// Get purchasable one-time entitlement mappings
///
/// Returns enabled one-time entitlement mappings that have provider product info configured.
/// Used by frontend/SDK to display purchasable products.
///
/// # Authentication
/// Requires valid API Key via X-API-Key header
///
/// # Realm Isolation
/// The API key must belong to the same realm as the requested realm.
/// Cross-realm requests will return 403 Forbidden.
#[utoipa::path(
    get,
    path = "/api/ext/{realmId}/one-time-mappings",
    tag = "ext",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    responses(
        (status = 200, description = "One-time mappings retrieved", body = OneTimeMappingExtResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API Key", body = ErrorResponse),
        (status = 403, description = "Forbidden - Cross-realm access attempt", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("api_key" = []))
)]
pub async fn get_one_time_mappings(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
) -> Response {
    let api_key_realm_id = identity.realm_id();

    tracing::info!(
        api_key_realm_id = %api_key_realm_id,
        request_realm_id = %realm_id,
        "One-time mappings query requested"
    );

    // 1. Check realm isolation
    if !identity.has_access_to_realm(&realm_id) {
        tracing::warn!(
            api_key_realm_id = %api_key_realm_id,
            request_realm_id = %realm_id,
            "Cross-realm access attempt blocked"
        );
        return json_error(StatusCode::FORBIDDEN, ErrorCode::CrossRealmAccessForbidden);
    }

    if let Err(resp) =
        require_principal_permission(&state, &identity, &realm_id, "billing", "view").await
    {
        return resp.into_response();
    }

    // 2. Query one-time mappings
    let mappings = match state
        .billing_repository
        .list_one_time_mappings(&realm_id)
        .await
    {
        Ok(m) => m,
        Err(e) => {
            tracing::error!("Failed to query one-time mappings: {}", e);
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, ErrorCode::InternalError);
        }
    };

    // 3. Build response
    let items: Vec<OneTimeMappingItem> = mappings
        .into_iter()
        .map(|m| OneTimeMappingItem {
            id: m.id.to_string(),
            entitlement_key: m.entitlement_key,
            provider_product_info: m.provider_product_info,
            points_per_period: m.points_per_period,
            payment_provider: m.payment_provider,
            validity_days: m.validity_days,
        })
        .collect();

    tracing::info!(
        realm_id = %realm_id,
        count = items.len(),
        "One-time mappings retrieved"
    );

    Json(OneTimeMappingExtResponse { items }).into_response()
}
