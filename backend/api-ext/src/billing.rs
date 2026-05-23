// Billing API for Third-Party Integration
//
// Allows third-party apps to query billing information using API Key authentication.

use axum::{
    Json,
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use herald_core::domain::authentication::Identity;
use herald_core::domain::billing::BillingRepository;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::client_helper::ClientAppLookup;
use herald_api_base::application::http::common::error_codes::ErrorCode;
use herald_api_base::application::http::common::error_helpers::json_error;
use herald_api_base::application::http::server::api_entities::{ApiError, ErrorResponse};
use herald_api_base::application::http::state::AppState;
use herald_api_billing::handlers::subscription_plan_to_response;
use herald_api_billing::types::SubscriptionPlanResponse;

/// Subscription detail response (SDK-compatible)
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionDetail {
    pub id: String,
    pub client_app_id: Option<String>,
    pub plan_id: Option<String>,
    pub plan: Option<SubscriptionPlan>,
    pub status: String,
    pub billing_period: String,
    pub current_period_start: Option<String>,
    pub current_period_end: Option<String>,
    pub cancel_at: Option<String>,
    pub cancel_at_period_end: Option<bool>,
    pub created_at: String,
    pub updated_at: String,
}

/// Subscription plan structure (SDK-compatible)
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionPlan {
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

/// Subscription plan assignment (SDK-compatible)
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionPlanAssignment {
    pub id: String,
    pub client_app_id: String,
    pub plan_id: String,
    pub enabled: bool,
    pub created_at: String,
}

/// Subscription plans list response
#[derive(Debug, Serialize, ToSchema)]
pub struct SubscriptionPlansListResponse {
    pub plans: Vec<SubscriptionPlan>,
}

/// Subscription plan assignments list response
#[derive(Debug, Serialize, ToSchema)]
pub struct SubscriptionPlanAssignmentsListResponse {
    pub assignments: Vec<SubscriptionPlanAssignment>,
}

/// Convert domain SubscriptionPlanResponse to SDK-compatible SubscriptionPlan
pub(crate) fn subscription_plan_response_to_sdk_plan(
    plan: SubscriptionPlanResponse,
) -> SubscriptionPlan {
    SubscriptionPlan {
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
            Ok(Some(domain_plan)) => Some(subscription_plan_response_to_sdk_plan(
                subscription_plan_to_response(domain_plan),
            )),
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
        (status = 200, description = "Plans listed successfully", body = SubscriptionPlansListResponse),
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
    let sdk_plans: Vec<SubscriptionPlan> = plans
        .into_iter()
        .map(|plan| subscription_plan_response_to_sdk_plan(subscription_plan_to_response(plan)))
        .collect();

    tracing::info!(
        realm_id = %realm_id,
        plans_count = sdk_plans.len(),
        "Plans retrieved successfully"
    );

    Json(SubscriptionPlansListResponse { plans: sdk_plans }).into_response()
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
        (status = 200, description = "Plan assignments listed successfully", body = SubscriptionPlanAssignmentsListResponse),
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
        .list_subscription_plans_for_client_app(client_app_uuid)
        .await
    {
        Ok(assignments) => assignments,
        Err(e) => {
            tracing::error!("Failed to list plan assignments: {}", e);
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, ErrorCode::InternalError);
        }
    };

    // 4. Convert to SDK-compatible format
    let sdk_assignments: Vec<SubscriptionPlanAssignment> = assignments
        .into_iter()
        .map(|assignment| SubscriptionPlanAssignment {
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

    Json(SubscriptionPlanAssignmentsListResponse {
        assignments: sdk_assignments,
    })
    .into_response()
}

/// Batch list plan assignments for multiple client apps
///
/// Returns all plan assignments for the specified client apps in the realm.
///
/// # Authentication
/// Requires valid API Key via X-API-Key header
///
/// # Realm Isolation
/// The API key must belong to the same realm as the client apps.
/// Cross-realm requests will return 403 Forbidden.
///
/// # Example
/// ```bash
/// curl -X GET \
///   "https://api.example.com/api/ext/bill/realm123/client/plans/batch?clientAppIds=id1,id2,id3" \
///   -H "X-API-Key: your-api-key"
/// ```
#[utoipa::path(
    get,
    path = "/api/ext/bill/{realmId}/client/plans/batch",
    tag = "ext",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("clientAppIds" = Option<String>, Query, description = "Comma-separated client app IDs (client_id or UUID)")
    ),
    responses(
        (status = 200, description = "Plan assignments listed successfully", body = SubscriptionPlanAssignmentsListResponse),
        (status = 400, description = "Bad request - Invalid client app IDs", body = ErrorResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API Key", body = ErrorResponse),
        (status = 403, description = "Forbidden - Cross-realm access attempt", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("api_key" = []))
)]
pub async fn list_plan_assignments_batch(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Query(params): Query<BatchPlanAssignmentsQuery>,
) -> Response {
    let api_key_realm_id = identity.realm_id();

    tracing::info!(
        api_key_realm_id = %api_key_realm_id,
        request_realm_id = %realm_id,
        "Batch plan assignments list requested"
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

    // 2. Parse client_app_ids from query parameter
    let client_app_ids_str = match params.client_app_ids {
        Some(s) if !s.trim().is_empty() => s,
        _ => {
            return ApiError::bad_request("clientAppIds query parameter is required")
                .into_response();
        }
    };

    let identifiers: Vec<&str> = client_app_ids_str
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if identifiers.is_empty() {
        return Json(SubscriptionPlanAssignmentsListResponse {
            assignments: Vec::new(),
        })
        .into_response();
    }

    // 3. Resolve client app identifiers to UUIDs
    let client_app_lookup = ClientAppLookup::new(state.pool.clone());
    let mut client_app_uuids = Vec::with_capacity(identifiers.len());
    for identifier in &identifiers {
        match client_app_lookup
            .find_uuid_by_identifier_required(identifier, &realm_id)
            .await
        {
            Ok(uuid) => client_app_uuids.push(uuid),
            Err(e) => return e,
        }
    }

    // 4. Query plan assignments batch
    let assignments = match state
        .billing_repository
        .list_subscription_plan_assignments_batch(&client_app_uuids)
        .await
    {
        Ok(assignments) => assignments,
        Err(e) => {
            tracing::error!("Failed to batch list plan assignments: {}", e);
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, ErrorCode::InternalError);
        }
    };

    // 5. Convert to SDK-compatible format
    let sdk_assignments: Vec<SubscriptionPlanAssignment> = assignments
        .into_iter()
        .map(|assignment| SubscriptionPlanAssignment {
            id: assignment.id.to_string(),
            client_app_id: assignment.client_app_id.to_string(),
            plan_id: assignment.plan_id.to_string(),
            enabled: assignment.enabled,
            created_at: assignment.created_at.to_rfc3339(),
        })
        .collect();

    tracing::info!(
        realm_id = %realm_id,
        client_app_count = client_app_uuids.len(),
        assignments_count = sdk_assignments.len(),
        "Batch plan assignments retrieved successfully"
    );

    Json(SubscriptionPlanAssignmentsListResponse {
        assignments: sdk_assignments,
    })
    .into_response()
}

/// Query parameters for batch plan assignments endpoint
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchPlanAssignmentsQuery {
    pub client_app_ids: Option<String>,
}
