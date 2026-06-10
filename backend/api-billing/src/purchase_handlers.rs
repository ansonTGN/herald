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
use herald_core::domain::payment_attempt::PaymentAttemptRepository;
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

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct PurchaseHistoryQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub payment_provider: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseHistoryResponse {
    pub items: Vec<PurchaseHistoryItemDto>,
    pub total: i64,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseHistoryItemDto {
    pub attempt_id: Uuid,
    pub target_mapping_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub points: Option<i64>,
    pub amount: i64,
    pub currency: String,
    pub payment_provider: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
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
    if matches!(target_type, "entitlement_mapping") {
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
            billing_type_override: None,
        })
        .await
        .map_err(|e| core_error_to_api_error(e, "Fulfill payment"))?;

    Ok(Json(fulfillment_result_to_response(result)))
}

#[utoipa::path(
    get,
    path = "/api/bill/{realmId}/purchase/history",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        PurchaseHistoryQuery
    ),
    responses(
        (status = 200, description = "Purchase history retrieved", body = PurchaseHistoryResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_purchase_history(
    State(state): State<AppState>,
    Path(realm_id): Path<String>,
    Extension(identity): Extension<Identity>,
    Query(filters): Query<PurchaseHistoryQuery>,
) -> Result<Json<PurchaseHistoryResponse>, ApiError> {
    let user_id = require_authenticated_user_in_realm(&identity, &realm_id, "purchase history")?;

    let page = filters.page.unwrap_or(1).max(1);
    let page_size = filters.page_size.unwrap_or(20).min(100);

    let (rows, total) = state
        .payment_attempt_repository
        .list_purchase_history(
            &realm_id,
            user_id,
            filters.payment_provider.as_deref(),
            filters.start_date.as_deref(),
            filters.end_date.as_deref(),
            page,
            page_size,
        )
        .await
        .map_err(|e| {
            tracing::error!(realm_id = %realm_id, error = %e, "Failed to fetch purchase history");
            ApiError::internal("Failed to fetch purchase history")
        })?;

    let items: Vec<PurchaseHistoryItemDto> = rows
        .into_iter()
        .map(|row| PurchaseHistoryItemDto {
            attempt_id: row.attempt_id,
            target_mapping_id: row.target_mapping_id,
            product_name: row.product_name,
            points: row.points,
            amount: row.amount,
            currency: row.currency,
            payment_provider: row.payment_provider,
            status: row.status,
            completed_at: row.completed_at.map(|dt| dt.to_rfc3339()),
            created_at: row.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(PurchaseHistoryResponse { items, total }))
}
