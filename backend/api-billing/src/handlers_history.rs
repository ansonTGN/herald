use axum::{
    Json,
    extract::{Extension, Path, Query, State},
};
use std::str::FromStr;
use uuid::Uuid;
use validator::Validate;

use crate::types::PlanSummary as BillingPlanSummary;
use crate::types_history::{
    PaginationMeta, SubscriptionHistoryEventResponse, SubscriptionHistoryEventWithUser,
    SubscriptionHistoryListQuery, SubscriptionHistoryListResponse, SubscriptionHistoryResponse,
    SubscriptionSummary,
};
use herald_api_base::application::http::common::pagination::calculate_total_pages;
use herald_api_base::application::http::server::api_entities::{ApiError, ErrorResponse};
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::authorization::PermissionService;
use herald_core::domain::billing::{
    BillingRepository, HistoryEventType, SUBSCRIPTION_STATUS_UNKNOWN, SortOrder,
    SubscriptionHistoryQuery,
};
use herald_core::domain::common::entities::app_errors::CoreError;

async fn require_realm_admin(
    state: &AppState,
    identity: &Identity,
    realm_id: &str,
) -> Result<(), CoreError> {
    if !identity.is_user() {
        return Err(CoreError::Forbidden(
            "Access denied: Realm Admin user required".to_string(),
        ));
    }

    let permission_checker = &state.permission_checker;

    let allowed = permission_checker
        .check_permission(realm_id, &identity.user_id(), "realm", "admin")
        .await
        .map_err(|e| CoreError::InternalServerError(format!("Permission check failed: {e}")))?;

    if !allowed {
        return Err(CoreError::Forbidden(
            "Access denied: Realm Admin permission required".to_string(),
        ));
    }

    Ok(())
}

/// Get subscription history for a specific subscription
#[utoipa::path(
    get,
    path = "/api/bill/{realmId}/subscriptions/{subscriptionId}/history",
    tag = "billing",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("subscriptionId" = Uuid, Path, description = "Subscription ID")
    ),
    responses(
        (status = 200, description = "Subscription history retrieved", body = SubscriptionHistoryResponse),
        (status = 401, description = "Unauthorized - Invalid or missing authentication", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Subscription not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("session_token" = []))
)]
pub async fn get_subscription_history(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, subscription_id)): Path<(String, Uuid)>,
) -> Result<Json<SubscriptionHistoryResponse>, ApiError> {
    if !identity.has_access_to_realm(&realm_id) {
        return Err(ApiError::forbidden(
            "Access denied: cannot access billing from a different realm".to_string(),
        ));
    }

    let subscription = match state
        .billing_repository
        .find_subscription_by_id(subscription_id)
        .await?
    {
        Some(sub) => Ok(sub),
        None => Err(CoreError::NotFound),
    }?;

    if subscription.realm_id != realm_id {
        return Err(ApiError::not_found("Subscription not found"));
    }

    if identity.is_user()
        && let Some(_client_app_id) = subscription.client_app_id
        && !identity.has_access_to_realm(&realm_id)
    {
        return Err(ApiError::forbidden(
            "You don't have permission to view this subscription's history".to_string(),
        ));
    }

    let events = state
        .billing_repository
        .get_subscription_history(&realm_id, &subscription_id)
        .await?;

    let events_response: Vec<SubscriptionHistoryEventResponse> =
        events.into_iter().map(Into::into).collect();

    let total = events_response.len();

    Ok(Json(SubscriptionHistoryResponse {
        subscription_id,
        events: events_response,
        total,
    }))
}

/// List subscription history with filtering and pagination
#[utoipa::path(
    get,
    path = "/api/bill/{realmId}/subscriptions/history",
    tag = "billing",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("userId" = Option<Uuid>, Query, description = "Filter by user ID (client_app_id)"),
        ("planId" = Option<Uuid>, Query, description = "Filter by plan ID"),
        ("eventType" = Option<String>, Query, description = "Filter by event type"),
        ("subscriptionStatus" = Option<String>, Query, description = "Filter by subscription status"),
        ("fromDate" = Option<String>, Query, description = "Filter by date range start (ISO 8601)"),
        ("toDate" = Option<String>, Query, description = "Filter by date range end (ISO 8601)"),
        ("page" = Option<u64>, Query, description = "Page number (1-based, default: 1)"),
        ("pageSize" = Option<u64>, Query, description = "Items per page (default: 20, max: 100)"),
        ("sortBy" = Option<String>, Query, description = "Sort field (default: timestamp)"),
        ("sortOrder" = Option<String>, Query, description = "Sort order - asc or desc (default: desc)")
    ),
    responses(
        (status = 200, description = "Subscription history list retrieved", body = SubscriptionHistoryListResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized - Invalid or missing authentication", body = ErrorResponse),
        (status = 403, description = "Forbidden - Realm Admin required", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("session_token" = []))
)]
pub async fn list_subscription_history(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Query(query): Query<SubscriptionHistoryListQuery>,
) -> Result<Json<SubscriptionHistoryListResponse>, ApiError> {
    if !identity.has_access_to_realm(&realm_id) {
        return Err(ApiError::forbidden(
            "Access denied: cannot access billing from a different realm".to_string(),
        ));
    }

    require_realm_admin(&state, &identity, &realm_id).await?;

    if let Err(validation_errors) = query.validate() {
        return Err(ApiError::bad_request(format!(
            "Invalid query parameters: {}",
            validation_errors
        )));
    }

    let event_type = query
        .event_type
        .and_then(|t| HistoryEventType::from_str(&t).ok());

    let sort_order = match query.sort_order.to_lowercase().as_str() {
        "asc" => Some(SortOrder::Asc),
        "desc" => Some(SortOrder::Desc),
        _ => None,
    };

    let repo_query = SubscriptionHistoryQuery {
        user_id: query.user_id,
        plan_id: query.plan_id,
        event_type,
        subscription_status: query.subscription_status,
        from_date: query.from_date,
        to_date: query.to_date,
        page: Some(query.page),
        page_size: Some(query.page_size),
        sort_by: Some(query.sort_by),
        sort_order,
    };

    let (events, total) = state
        .billing_repository
        .list_subscription_history(&realm_id, repo_query)
        .await?;

    let events_with_user = convert_events_with_details(events, &state).await;

    let total_pages = calculate_total_pages(total as i64, query.page_size as i64);

    let pagination = PaginationMeta {
        page: query.page as i64,
        page_size: query.page_size as i64,
        total_count: total as i64,
        total_pages,
    };

    Ok(Json(SubscriptionHistoryListResponse {
        events: events_with_user,
        pagination,
    }))
}

async fn convert_events_with_details(
    events: Vec<herald_core::domain::billing::SubscriptionHistoryEvent>,
    state: &AppState,
) -> Vec<SubscriptionHistoryEventWithUser> {
    let mut events_with_details = Vec::new();

    for event in events {
        let user_info = if let Some(new_state) = &event.new_state {
            if let Some(_client_app_id) = new_state.get("client_app_id") {
                if let Ok(Some(_subscription)) = state
                    .billing_repository
                    .find_subscription_by_id(event.subscription_id)
                    .await
                {
                    None
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let subscription_summary = if let Ok(Some(subscription)) = state
            .billing_repository
            .find_subscription_by_id(event.subscription_id)
            .await
        {
            let plan = if let Some(plan_id) = subscription.plan_id {
                state
                    .billing_repository
                    .find_plan_by_id(plan_id)
                    .await
                    .ok()
                    .flatten()
                    .map(|p| BillingPlanSummary {
                        id: p.id,
                        name: p.name,
                        title: p.title,
                    })
            } else {
                None
            };

            SubscriptionSummary {
                id: subscription.id,
                status: subscription.status.as_str().to_string(),
                plan: plan.clone(),
            }
        } else {
            SubscriptionSummary {
                id: event.subscription_id,
                status: SUBSCRIPTION_STATUS_UNKNOWN.to_string(),
                plan: None,
            }
        };

        events_with_details.push(SubscriptionHistoryEventWithUser {
            id: event.id,
            subscription_id: event.subscription_id,
            event_type: event.event_type.as_str().to_string(),
            timestamp: event.timestamp,
            actor: event.actor,
            changes: event.changes,
            previous_state: event.previous_state,
            new_state: event.new_state,
            user: user_info,
            subscription: subscription_summary,
        });
    }

    events_with_details
}
