// Purchase API Handlers

use axum::{
    Json,
    extract::{Extension, Path, Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use herald_api_base::application::http::common::auth_utils::require_authenticated_user_in_realm;
use herald_api_base::application::http::common::error_helpers::core_error_to_api_error;
use herald_api_base::application::http::server::api_entities::ApiError;
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::purchase::{
    CompletePaymentAttemptInput, FulfillmentResult, PaymentCompletionSource,
    PreparePaymentAttemptInput,
};

use crate::payment_email::formal_payment_email;

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreatePaymentAttemptRequest {
    #[validate(custom(function = "validate_purchasable_target"))]
    pub target_type: String,
    pub target_id: Uuid,
    #[validate(custom(function = "validate_payment_provider"))]
    pub payment_provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatePaymentAttemptResponse {
    pub id: Uuid,
    pub realm_id: String,
    pub user_id: Uuid,
    pub payment_provider: String,
    pub target_type: String,
    pub target_id: Uuid,
    pub amount: i64,
    pub currency: String,
    pub status: String,
    pub expires_at: String,
    pub payment_context: PaymentContextDto,
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaymentContextDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wechat_code_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stripe_checkout_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creem_checkout_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaymentAttemptStatusResponse {
    pub id: Uuid,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_status: Option<String>,
    pub target_type: String,
    pub target_id: Uuid,
    pub amount: i64,
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fulfillment: Option<FulfillmentResultDto>,
    pub created_at: String,
    pub expires_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FulfillmentResultDto {
    #[serde(rename = "type")]
    pub fulfillment_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscription_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub points_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub points: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_id: Option<Uuid>,
    pub granted_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FulfillPaymentResponse {
    #[serde(rename = "type")]
    pub fulfillment_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscription_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub points_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub points: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_id: Option<Uuid>,
    pub granted_at: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct PurchaseHistoryFilters {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseHistoryResponse {
    pub purchases: Vec<PurchaseHistoryItemDto>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseHistoryItemDto {
    pub id: Uuid,
    pub realm_id: String,
    pub user_id: Uuid,
    pub points_package_id: Uuid,
    pub payment_attempt_id: Uuid,
    pub points: i64,
    pub amount: i64,
    pub currency: String,
    pub payment_provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub points_transaction_id: Option<Uuid>,
    pub created_at: String,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct FulfillPaymentRequest {
    pub provider_status: String,
    pub provider_transaction_id: String,
    pub completed_at: String,
}

// ============================================================================
// Validation Functions
// ============================================================================

fn validate_purchasable_target(target_type: &str) -> Result<(), validator::ValidationError> {
    if matches!(target_type, "subscription_entitlement" | "points_package") {
        Ok(())
    } else {
        Err(validator::ValidationError::new("invalid target_type"))
    }
}

fn validate_payment_provider(payment_provider: &str) -> Result<(), validator::ValidationError> {
    if matches!(payment_provider, "wechat" | "stripe" | "creem") {
        Ok(())
    } else {
        Err(validator::ValidationError::new("invalid payment_provider"))
    }
}

// ============================================================================
// Conversion Helpers
// ============================================================================

fn payment_context_to_dto(
    context: herald_core::domain::payment_attempt::PaymentContext,
) -> PaymentContextDto {
    PaymentContextDto {
        wechat_code_url: context.wechat_code_url,
        stripe_checkout_url: context.stripe_checkout_url,
        creem_checkout_url: context.creem_checkout_url,
        client_secret: context.client_secret,
    }
}

fn fulfillment_result_to_response(result: FulfillmentResult) -> FulfillPaymentResponse {
    FulfillPaymentResponse {
        fulfillment_type: match result.fulfillment_type {
            herald_core::domain::purchase::FulfillmentType::SubscriptionCreated => {
                "subscription_created".to_string()
            }
            herald_core::domain::purchase::FulfillmentType::SubscriptionUpdated => {
                "subscription_updated".to_string()
            }
            herald_core::domain::purchase::FulfillmentType::PointsGranted => {
                "points_granted".to_string()
            }
        },
        subscription_id: result.subscription_id,
        points_type: result
            .points_granted
            .as_ref()
            .map(|pg| pg.points_type.clone()),
        points: result.points_granted.as_ref().map(|pg| pg.points),
        transaction_id: result.points_granted.as_ref().map(|pg| pg.transaction_id),
        granted_at: result.granted_at.to_rfc3339(),
    }
}

// ============================================================================
// Handlers
// ============================================================================

#[utoipa::path(
    post,
    path = "/api/bill/{realmId}/purchase/payment-attempts",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    request_body = CreatePaymentAttemptRequest,
    responses(
        (status = 201, description = "Payment attempt created successfully", body = CreatePaymentAttemptResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Target not found or not enabled"),
        (status = 409, description = "Payment provider not configured for target")
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_payment_attempt(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Json(input): Json<CreatePaymentAttemptRequest>,
) -> Result<(StatusCode, Json<CreatePaymentAttemptResponse>), ApiError> {
    input
        .validate()
        .map_err(|e| ApiError::bad_request(format!("Invalid request: {}", e)))?;

    let user_id = require_authenticated_user_in_realm(&identity, &realm_id, "purchase APIs")?;

    let created = state
        .purchase_service
        .create_payment_attempt(PreparePaymentAttemptInput {
            realm_id: realm_id.clone(),
            user_id,
            user_email: formal_payment_email(&identity),
            payment_provider: input.payment_provider.clone(),
            target_type: input.target_type.clone(),
            target_id: input.target_id,
            metadata: input.metadata,
        })
        .await
        .map_err(|e| core_error_to_api_error(e, "Create payment attempt"))?;

    let response = CreatePaymentAttemptResponse {
        id: created.attempt.id,
        realm_id: created.attempt.realm_id,
        user_id: created.attempt.user_id,
        payment_provider: created.attempt.payment_provider,
        target_type: created.attempt.target_type.to_string(),
        target_id: created.attempt.target_id,
        amount: created.attempt.amount,
        currency: created.attempt.currency,
        status: created.attempt.status.to_string(),
        expires_at: created.attempt.expires_at.to_rfc3339(),
        payment_context: payment_context_to_dto(created.context),
        created_at: created.attempt.created_at.to_rfc3339(),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

#[utoipa::path(
    get,
    path = "/api/bill/{realmId}/purchase/payment-attempts/{attemptId}",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("attemptId" = Uuid, Path, description = "Payment Attempt ID")
    ),
    responses(
        (status = 200, description = "Payment attempt status retrieved", body = PaymentAttemptStatusResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not your attempt"),
        (status = 404, description = "Payment attempt not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_payment_attempt_status(
    State(state): State<AppState>,
    Path((realm_id, attempt_id)): Path<(String, Uuid)>,
    Extension(identity): Extension<Identity>,
) -> Result<Json<PaymentAttemptStatusResponse>, ApiError> {
    let user_id = require_authenticated_user_in_realm(&identity, &realm_id, "purchase APIs")?;

    let service = &state.payment_attempt_service;

    let attempt = service
        .get_payment_attempt_status(&realm_id, attempt_id, user_id)
        .await
        .map_err(|e| core_error_to_api_error(e, "Get payment attempt status"))?;

    let response = PaymentAttemptStatusResponse {
        id: attempt.id,
        status: attempt.status.to_string(),
        provider_status: attempt.provider_status,
        target_type: attempt.target_type.to_string(),
        target_id: attempt.target_id,
        amount: attempt.amount,
        currency: attempt.currency,
        completed_at: attempt.completed_at.map(|dt| dt.to_rfc3339()),
        fulfillment: None,
        created_at: attempt.created_at.to_rfc3339(),
        expires_at: attempt.expires_at.to_rfc3339(),
    };

    Ok(Json(response))
}

#[utoipa::path(
    post,
    path = "/api/bill/{realmId}/purchase/payment-attempts/{attemptId}/cancel",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("attemptId" = Uuid, Path, description = "Payment Attempt ID")
    ),
    responses(
        (status = 200, description = "Payment attempt cancelled", body = PaymentAttemptStatusResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not your attempt"),
        (status = 404, description = "Payment attempt not found"),
        (status = 409, description = "Cannot cancel attempt in current state")
    ),
    security(("bearer_auth" = []))
)]
pub async fn cancel_payment_attempt(
    State(state): State<AppState>,
    Path((realm_id, attempt_id)): Path<(String, Uuid)>,
    Extension(identity): Extension<Identity>,
) -> Result<Json<PaymentAttemptStatusResponse>, ApiError> {
    let user_id = require_authenticated_user_in_realm(&identity, &realm_id, "purchase APIs")?;

    let service = &state.payment_attempt_service;

    let attempt = service
        .cancel_payment_attempt(&realm_id, attempt_id, user_id)
        .await
        .map_err(|e| core_error_to_api_error(e, "Cancel payment attempt"))?;

    let response = PaymentAttemptStatusResponse {
        id: attempt.id,
        status: attempt.status.to_string(),
        provider_status: attempt.provider_status,
        target_type: attempt.target_type.to_string(),
        target_id: attempt.target_id,
        amount: attempt.amount,
        currency: attempt.currency,
        completed_at: attempt.completed_at.map(|dt| dt.to_rfc3339()),
        fulfillment: None,
        created_at: attempt.created_at.to_rfc3339(),
        expires_at: attempt.expires_at.to_rfc3339(),
    };

    Ok(Json(response))
}

#[utoipa::path(
    post,
    path = "/api/internal/bill/purchase/payment-attempts/{attemptId}/fulfill",
    params(
        ("attemptId" = Uuid, Path, description = "Payment Attempt ID")
    ),
    request_body = FulfillPaymentRequest,
    responses(
        (status = 200, description = "Fulfillment completed", body = FulfillPaymentResponse),
        (status = 400, description = "Fulfillment failed"),
        (status = 404, description = "Payment attempt not found"),
        (status = 409, description = "Already fulfilled")
    )
)]
pub async fn fulfill_payment(
    State(state): State<AppState>,
    Path(attempt_id): Path<Uuid>,
    Json(input): Json<FulfillPaymentRequest>,
) -> Result<Json<FulfillPaymentResponse>, ApiError> {
    let completed_at = chrono::DateTime::parse_from_rfc3339(&input.completed_at)
        .map_err(|_| ApiError::bad_request("Invalid completed_at format"))?
        .with_timezone(&chrono::Utc);

    let result = state
        .purchase_service
        .complete_succeeded_payment_attempt(CompletePaymentAttemptInput {
            attempt_id,
            provider_status: input.provider_status,
            provider_transaction_id: input.provider_transaction_id,
            completed_at,
            source: PaymentCompletionSource::InternalApi,
        })
        .await
        .map_err(|e| core_error_to_api_error(e, "Fulfill payment"))?;

    Ok(Json(fulfillment_result_to_response(result)))
}

#[utoipa::path(
    get,
    path = "/api/realms/{realmId}/billing/purchase/points-packages/history",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("paymentProvider" = Option<String>, Query, description = "Filter by payment provider"),
        ("limit" = Option<u64>, Query, description = "Limit number of results (default 50, max 100)"),
        ("offset" = Option<u64>, Query, description = "Offset for pagination")
    ),
    responses(
        (status = 200, description = "Purchase history retrieved", body = PurchaseHistoryResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_points_package_purchase_history(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Query(filters): Query<PurchaseHistoryFilters>,
) -> Result<Json<PurchaseHistoryResponse>, ApiError> {
    let user_id = require_authenticated_user_in_realm(&identity, &realm_id, "purchase APIs")?;

    if let Some(ref provider) = filters.payment_provider
        && !matches!(provider.as_str(), "wechat" | "stripe" | "creem")
    {
        return Err(ApiError::bad_request("Invalid payment_provider"));
    }

    let limit = filters.limit.unwrap_or(20).min(100);

    let purchases = state
        .purchase_service
        .list_points_package_purchases(
            &realm_id,
            user_id,
            filters.payment_provider,
            Some(limit),
            filters.offset,
        )
        .await
        .map_err(|e| core_error_to_api_error(e, "Fetch purchase history"))?;

    let purchase_dtos: Vec<PurchaseHistoryItemDto> = purchases
        .into_iter()
        .map(|p| PurchaseHistoryItemDto {
            id: p.id,
            realm_id: p.realm_id,
            user_id: p.user_id,
            points_package_id: p.points_package_id,
            payment_attempt_id: p.payment_attempt_id,
            points: p.points,
            amount: p.amount,
            currency: p.currency,
            payment_provider: p.payment_provider,
            points_transaction_id: p.points_transaction_id,
            created_at: p.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(PurchaseHistoryResponse {
        purchases: purchase_dtos,
    }))
}

#[utoipa::path(
    get,
    path = "/api/realms/{realmId}/billing/purchase/points-packages/history/{purchaseId}",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("purchaseId" = Uuid, Path, description = "Purchase ID")
    ),
    responses(
        (status = 200, description = "Purchase details retrieved", body = PurchaseHistoryItemDto),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not your purchase"),
        (status = 404, description = "Purchase not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_points_package_purchase_details(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, purchase_id)): Path<(String, Uuid)>,
) -> Result<Json<PurchaseHistoryItemDto>, ApiError> {
    let user_id = require_authenticated_user_in_realm(&identity, &realm_id, "purchase APIs")?;

    let purchase = state
        .purchase_service
        .get_points_package_purchase(&realm_id, user_id, purchase_id)
        .await
        .map_err(|e| core_error_to_api_error(e, "Fetch purchase"))?;

    let purchase_dto = PurchaseHistoryItemDto {
        id: purchase.id,
        realm_id: purchase.realm_id,
        user_id: purchase.user_id,
        points_package_id: purchase.points_package_id,
        payment_attempt_id: purchase.payment_attempt_id,
        points: purchase.points,
        amount: purchase.amount,
        currency: purchase.currency,
        payment_provider: purchase.payment_provider,
        points_transaction_id: purchase.points_transaction_id,
        created_at: purchase.created_at.to_rfc3339(),
    };

    Ok(Json(purchase_dto))
}
