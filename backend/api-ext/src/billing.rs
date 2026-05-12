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

use crate::client_helper::ClientAppLookup;
use herald_api_base::application::http::common::error_codes::ErrorCode;
use herald_api_base::application::http::server::api_entities::{ApiError, ErrorResponse};
use herald_api_base::application::http::state::AppState;
use herald_api_billing::handlers::plan_to_response;
use herald_api_billing::types::PlanResponse;

/// Create a JSON error response
fn json_error(status: StatusCode, error_code: ErrorCode) -> Response {
    ApiError::with_code(status, error_code.as_u32(), error_code.as_str()).into_response()
}

/// Subscription detail response (SDK-compatible)
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionDetail {
    pub id: String,
    pub client_app_id: Option<String>,
    pub plan_id: Option<String>,
    pub plan: Option<Plan>,
    pub status: String,
    pub billing_period: String,
    pub current_period_start: Option<String>,
    pub current_period_end: Option<String>,
    pub cancel_at: Option<String>,
    pub cancel_at_period_end: Option<bool>,
    pub created_at: String,
    pub updated_at: String,
}

/// Plan structure (SDK-compatible)
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub id: String,
    pub realm_id: String,
    pub name: String,
    pub title: String,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub plan_type: String,
    pub price: i32,
    pub currency: String,
    pub checkout_url: Option<String>,
    pub active: bool,
    pub trial_days: i32,
    pub sort_order: i32,
}

/// Plan assignment (SDK-compatible)
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlanAssignment {
    pub id: String,
    pub client_app_id: String,
    pub plan_id: String,
    pub enabled: bool,
    pub created_at: String,
}

/// Plans list response
#[derive(Debug, Serialize, ToSchema)]
pub struct PlansListResponse {
    pub plans: Vec<Plan>,
}

/// Plan assignments list response
#[derive(Debug, Serialize, ToSchema)]
pub struct AssignmentsListResponse {
    pub assignments: Vec<PlanAssignment>,
}

/// Convert domain PlanResponse to SDK-compatible Plan
pub(crate) fn plan_response_to_sdk_plan(plan: PlanResponse) -> Plan {
    Plan {
        id: plan.id.to_string(),
        realm_id: plan.realm_id,
        name: plan.name,
        title: plan.title,
        description: plan.description,
        plan_type: plan.r#type,
        price: plan.price,
        currency: plan.currency,
        checkout_url: plan.checkout_url,
        active: plan.active,
        trial_days: plan.trial_days,
        sort_order: plan.sort_order,
    }
}

/// Get subscription for a client app
///
/// Returns subscription details including plan information for the specified client app.
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
    let (realm_id, client_app_id) = params; // Destructure here to enable cloning
    let client_app_id = client_app_id.clone(); // Clone to use later

    tracing::info!(
        api_key_realm_id = %api_key_realm_id,
        request_realm_id = %realm_id,
        client_app_id = %client_app_id,
        "Subscription detail query requested"
    );

    // 1. Check realm isolation - API key must be for the requested realm
    if api_key_realm_id != realm_id {
        tracing::warn!(
            api_key_realm_id = %api_key_realm_id,
            request_realm_id = %realm_id,
            "Cross-realm access attempt blocked"
        );
        return json_error(StatusCode::FORBIDDEN, ErrorCode::CrossRealmAccessForbidden);
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

    // 4. Fetch plan if available
    let plan = match subscription.plan_id {
        Some(plan_id) => match state
            .billing_repository
            .find_public_plan_by_id(&realm_id, plan_id)
            .await
        {
            Ok(Some(domain_plan)) => Some(plan_response_to_sdk_plan(plan_to_response(domain_plan))),
            Ok(None) => None,
            Err(e) => {
                tracing::error!("Failed to fetch plan: {}", e);
                None
            }
        },
        None => None,
    };

    // 5. Build response
    let response = SubscriptionDetail {
        id: subscription.id.to_string(),
        client_app_id: Some(client_app_id.clone()),
        plan_id: subscription.plan_id.map(|id| id.to_string()),
        plan,
        status: subscription.status.as_str().to_string(),
        billing_period: subscription.billing_period.as_str().to_string(),
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

/// List all available plans for a realm
///
/// Returns all plans available for the specified realm.
///
/// # Authentication
/// Requires valid API Key via X-API-Key header
///
/// # Realm Isolation
/// The API key must belong to the same realm as the requested plans.
/// Cross-realm requests will return 403 Forbidden.
///
/// # Example
/// ```bash
/// curl -X GET \
///   https://api.example.com/api/ext/bill/realm123/plans \
///   -H "X-API-Key: your-api-key"
/// ```
#[utoipa::path(
    get,
    path = "/api/ext/bill/{realmId}/plans",
    tag = "ext",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    responses(
        (status = 200, description = "Plans listed successfully", body = PlansListResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API Key", body = ErrorResponse),
        (status = 403, description = "Forbidden - Cross-realm access attempt", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("api_key" = []))
)]
pub async fn list_plans(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
) -> Response {
    let api_key_realm_id = identity.realm_id();

    tracing::info!(
        api_key_realm_id = %api_key_realm_id,
        request_realm_id = %realm_id,
        "Plans list requested"
    );

    // 1. Check realm isolation - API key must be for the requested realm
    if api_key_realm_id != realm_id {
        tracing::warn!(
            api_key_realm_id = %api_key_realm_id,
            request_realm_id = %realm_id,
            "Cross-realm access attempt blocked"
        );
        return json_error(StatusCode::FORBIDDEN, ErrorCode::CrossRealmAccessForbidden);
    }

    // 2. Query all plans for the realm
    let plans = match state
        .billing_repository
        .list_public_plans_by_realm(&realm_id)
        .await
    {
        Ok(plans) => plans,
        Err(e) => {
            tracing::error!("Failed to list plans: {}", e);
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, ErrorCode::InternalError);
        }
    };

    // 3. Convert to SDK-compatible format
    let sdk_plans: Vec<Plan> = plans
        .into_iter()
        .map(|plan| plan_response_to_sdk_plan(plan_to_response(plan)))
        .collect();

    tracing::info!(
        realm_id = %realm_id,
        plans_count = sdk_plans.len(),
        "Plans retrieved successfully"
    );

    Json(PlansListResponse { plans: sdk_plans }).into_response()
}

/// List plan assignments for a client app
///
/// Returns all plan assignments for the specified client app in the realm.
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
///   https://api.example.com/api/ext/bill/realm123/client/client-app-123/plans \
///   -H "X-API-Key: your-api-key"
/// ```
#[utoipa::path(
    get,
    path = "/api/ext/bill/{realmId}/client/{clientAppId}/plans",
    tag = "ext",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("clientAppId" = String, Path, description = "Client app ID (client_id or UUID)")
    ),
    responses(
        (status = 200, description = "Plan assignments listed successfully", body = AssignmentsListResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API Key", body = ErrorResponse),
        (status = 403, description = "Forbidden - Cross-realm access attempt", body = ErrorResponse),
        (status = 404, description = "Client app not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("api_key" = []))
)]
pub async fn list_plan_assignments(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(params): Path<(String, String)>,
) -> Response {
    let api_key_realm_id = identity.realm_id();
    let (realm_id, client_app_id) = params; // Destructure here to enable cloning
    let client_app_id = client_app_id.clone(); // Clone to use later

    tracing::info!(
        api_key_realm_id = %api_key_realm_id,
        request_realm_id = %realm_id,
        client_app_id = %client_app_id,
        "Plan assignments list requested"
    );

    // 1. Check realm isolation - API key must be for the requested realm
    if api_key_realm_id != realm_id {
        tracing::warn!(
            api_key_realm_id = %api_key_realm_id,
            request_realm_id = %realm_id,
            "Cross-realm access attempt blocked"
        );
        return json_error(StatusCode::FORBIDDEN, ErrorCode::CrossRealmAccessForbidden);
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

    // 3. Query plan assignments
    let assignments = match state
        .billing_repository
        .list_plans_for_client_app(client_app_uuid)
        .await
    {
        Ok(assignments) => assignments,
        Err(e) => {
            tracing::error!("Failed to list plan assignments: {}", e);
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, ErrorCode::InternalError);
        }
    };

    // 4. Convert to SDK-compatible format
    let sdk_assignments: Vec<PlanAssignment> = assignments
        .into_iter()
        .map(|assignment| PlanAssignment {
            id: assignment.id.to_string(),
            client_app_id: assignment.client_app_id.to_string(),
            plan_id: assignment.plan_id.to_string(),
            enabled: assignment.enabled,
            created_at: assignment.created_at.to_rfc3339(),
        })
        .collect();

    tracing::info!(
        realm_id = %realm_id,
        client_app_id = %client_app_id,
        assignments_count = sdk_assignments.len(),
        "Plan assignments retrieved successfully"
    );

    Json(AssignmentsListResponse {
        assignments: sdk_assignments,
    })
    .into_response()
}
