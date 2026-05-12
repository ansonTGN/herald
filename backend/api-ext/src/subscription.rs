// Subscription Status API for Third-Party Integration
//
// Allows third-party apps to query subscription status for client apps.

use axum::{
    Json,
    extract::{Extension, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use herald_core::domain::authentication::Identity;
use herald_core::domain::billing::{BillingRepository, Subscription};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::client_helper::ClientAppLookup;
use herald_api_base::application::http::common::error_codes::ErrorCode;
use herald_api_base::application::http::server::api_entities::{ApiError, ErrorResponse};
use herald_api_base::application::http::state::AppState;

/// Create a JSON error response
fn json_error(status: StatusCode, error_code: ErrorCode) -> Response {
    ApiError::with_code(status, error_code.as_u32(), error_code.as_str()).into_response()
}

/// Subscription status response
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionResponse {
    /// Client app ID
    pub client_app_id: String,

    /// Whether the app has an active subscription
    pub has_subscription: bool,

    /// Subscription status
    /// - "active": Subscription is active and paid
    /// - "canceled": Subscription was canceled but still active until period end
    /// - "expired": Subscription has expired
    /// - "trialing": Free trial period
    /// - "none": No subscription
    pub status: String,

    /// Subscription tier
    /// - "free": Free tier
    /// - "starter": Starter plan
    /// - "professional": Professional plan
    /// - "enterprise": Enterprise plan
    pub tier: String,

    /// Plan name (only present if has_subscription=true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_name: Option<String>,
}

/// Get subscription status for a client app
///
/// Returns subscription information for the specified client app.
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
///   https://api.example.com/api/ext/subscription/client-app-123 \
///   -H "X-API-Key: your-api-key"
/// ```
#[utoipa::path(
    get,
    path = "/api/ext/subscription/{clientAppId}",
    tag = "ext",
    params(
        ("clientAppId" = String, Path, description = "Client app ID")
    ),
    responses(
        (status = 200, description = "Subscription status retrieved", body = SubscriptionResponse),
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
    Path(client_app_id): Path<Uuid>,
) -> Response {
    let api_key_realm_id = identity.realm_id();

    tracing::info!(
        api_key_realm_id = %api_key_realm_id,
        client_app_id = %client_app_id,
        "Subscription query requested"
    );

    // 1. Verify client app exists and check realm isolation
    let client_app_lookup = ClientAppLookup::new(state.pool.clone());
    let client_app_realm_id: String = match client_app_lookup
        .verify_client_app_exists(client_app_id)
        .await
    {
        Ok(realm_id) => realm_id,
        Err(e) => return e,
    };

    // 2. Check realm isolation
    if !identity.has_access_to_realm(&client_app_realm_id) {
        tracing::warn!(
            api_key_realm_id = %api_key_realm_id,
            client_app_realm_id = %client_app_realm_id,
            "Cross-realm access attempt blocked"
        );
        return json_error(StatusCode::FORBIDDEN, ErrorCode::CrossRealmAccessForbidden);
    }

    // 3. Query subscription status
    let subscription: Option<Subscription> = match state
        .billing_repository
        .find_subscription_by_client_app_id(client_app_id)
        .await
    {
        Ok(sub) => sub,
        Err(e) => {
            tracing::error!("Failed to query subscription: {}", e);
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, ErrorCode::InternalError);
        }
    };

    // 4. Build response
    let response = match subscription {
        Some(sub) => {
            // Fetch plan name if plan_id exists (fixed: plan.title -> plan.name)
            let plan_name = match sub.plan_id {
                Some(plan_id) => state
                    .billing_repository
                    .find_public_plan_by_id(&client_app_realm_id, plan_id)
                    .await
                    .ok()
                    .flatten()
                    .map(|plan| plan.name),
                None => None,
            };

            tracing::info!(
                client_app_id = %client_app_id,
                status = %sub.status.as_str(),
                tier = %sub.tier.as_str(),
                plan_name = ?plan_name,
                "Subscription found"
            );

            SubscriptionResponse {
                client_app_id: client_app_id.to_string(),
                has_subscription: true,
                status: sub.status.as_str().to_string(),
                tier: sub.tier.as_str().to_string(),
                plan_name,
            }
        }
        None => {
            tracing::info!(
                client_app_id = %client_app_id,
                "No subscription found (free tier)"
            );

            SubscriptionResponse {
                client_app_id: client_app_id.to_string(),
                has_subscription: false,
                status: "none".to_string(),
                tier: "free".to_string(),
                plan_name: None,
            }
        }
    };

    Json(response).into_response()
}
