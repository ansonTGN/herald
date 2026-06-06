// Points API for Third-Party Integration
//
// Allows third-party apps to query and consume points using API Key authentication.

use axum::{
    Json,
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::authz::require_principal_permission;
use crate::client_app_scope::ensure_client_app_scope;
use herald_api_base::application::http::common::error_codes::ErrorCode;
use herald_api_base::application::http::common::error_helpers::json_error;
use herald_api_base::application::http::rate_limit::{RateLimitConfig, rate_limit};
use herald_api_base::application::http::server::api_entities::ErrorResponse;
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::points::dtos::{ConsumePointsInput, GrantPointsInput};
use herald_core::domain::points::entities::CreditSourceType;
use herald_core::domain::points::ports::TransactionFilters;

const REALM_RATE_LIMIT_PREFIX: &str = "points:realm:";
const USER_RATE_LIMIT_PREFIX: &str = "points:user:";
const REALM_RATE_LIMIT: RateLimitConfig = RateLimitConfig {
    max_requests: 100,
    window_secs: 60,
    enforce_in_dev: true,
};
const USER_RATE_LIMIT: RateLimitConfig = RateLimitConfig {
    max_requests: 20,
    window_secs: 60,
    enforce_in_dev: true,
};

/// Balance response (SDK-compatible)
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExtPointsBalanceResponse {
    pub user_id: String,
    pub balance: i64,
    pub total_recharged: i64,
    pub total_consumed: i64,
    pub unit: String,
    pub currency: String,
    pub updated_at: String,
}

/// Consume points request (SDK-compatible)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExtConsumePointsRequest {
    pub user_id: String,
    pub client_app_id: String,
    pub amount: i64,
    pub description: Option<String>,
    pub idempotency_key: Option<String>,
}

/// Consume points response (SDK-compatible)
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExtConsumePointsResponse {
    pub transaction_id: String,
    pub wallet_id: String,
    pub user_id: String,
    pub amount: i64,
    pub balance_after: i64,
}

/// Grant points request (SDK-compatible)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExtGrantPointsRequest {
    pub user_id: String,
    pub amount: i64,
    pub reason: String,
    pub validity_days: Option<i64>,
}

/// Grant points response (SDK-compatible)
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExtGrantPointsResponse {
    pub transaction_id: String,
    pub user_id: String,
    pub amount: i64,
    pub granted_balance: i64,
    pub balance: i64,
    pub expires_at: Option<String>,
}

/// Get user points balance
///
/// Returns the current points balance for a user in the specified realm.
///
/// # Authentication
/// Requires valid API Key via X-API-Key header
///
/// # Realm Isolation
/// The API key must belong to the same realm as the requested realm.
/// Cross-realm requests will return 403 Forbidden.
///
/// # Example
/// ```bash
/// curl -X GET \
///   https://api.example.com/api/ext/points/realm123/balance?userId=user-123 \
///   -H "X-API-Key: your-api-key"
/// ```
#[utoipa::path(
    get,
    path = "/api/ext/points/{realmId}/balance",
    tag = "ext",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("userId" = Option<String>, Query, description = "User ID (optional)")
    ),
    responses(
        (status = 200, description = "Balance retrieved successfully", body = ExtPointsBalanceResponse),
        (status = 400, description = "Bad request - Invalid user ID format", body = ErrorResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API Key", body = ErrorResponse),
        (status = 403, description = "Forbidden - Cross-realm access attempt", body = ErrorResponse),
        (status = 404, description = "Not found - User or account not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("api_key" = []))
)]
pub async fn get_balance_ext(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> Response {
    let api_key_realm_id = identity.realm_id();

    tracing::info!(
        api_key_realm_id = %api_key_realm_id,
        request_realm_id = %realm_id,
        "Balance query requested"
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

    if let Err(resp) =
        require_principal_permission(&state, &identity, &realm_id, "points", "view").await
    {
        return resp.into_response();
    }

    // 2. Extract user_id from query parameter
    let user_id = match query.get("userId") {
        Some(user_id_str) => match user_id_str.parse::<Uuid>() {
            Ok(uuid) => uuid,
            Err(_) => {
                tracing::warn!("Invalid user ID format: {}", user_id_str);
                return json_error(StatusCode::BAD_REQUEST, ErrorCode::InvalidUserIdFormat);
            }
        },
        None => {
            tracing::warn!("Missing userId parameter");
            return json_error(StatusCode::BAD_REQUEST, ErrorCode::MissingUserId);
        }
    };

    // 3. Query balance
    let balance = match state
        .points_service
        .get_balance(identity, &realm_id, user_id)
        .await
    {
        Ok(balance) => balance,
        Err(e) => {
            tracing::error!("Failed to query balance: {}", e);
            return match e {
                herald_core::domain::common::entities::app_errors::CoreError::Unauthorized => {
                    json_error(StatusCode::UNAUTHORIZED, ErrorCode::Unauthorized)
                }
                herald_core::domain::common::entities::app_errors::CoreError::Forbidden(_) => {
                    json_error(StatusCode::FORBIDDEN, ErrorCode::Forbidden)
                }
                herald_core::domain::common::entities::app_errors::CoreError::NotFound => {
                    json_error(StatusCode::NOT_FOUND, ErrorCode::WalletNotFound)
                }
                _ => json_error(StatusCode::INTERNAL_SERVER_ERROR, ErrorCode::InternalError),
            };
        }
    };

    // 4. Build response
    let response = ExtPointsBalanceResponse {
        user_id: balance.user_id.to_string(),
        balance: balance.balance,
        total_recharged: balance.total_recharged,
        total_consumed: balance.total_consumed,
        unit: balance.unit.clone(),
        currency: balance.unit,
        updated_at: balance.updated_at.to_rfc3339(),
    };

    tracing::info!(
        user_id = %balance.user_id,
        balance = %balance.balance,
        "Balance retrieved successfully"
    );

    Json(response).into_response()
}

/// Consume points from user account
///
/// Consumes points from a user's account for paid operations.
/// Returns transaction details including the new balance.
///
/// # Authentication
/// Requires valid API Key via X-API-Key header
///
/// # Realm Isolation
/// The API key must belong to the same realm as the requested realm.
/// Cross-realm requests will return 403 Forbidden.
///
/// # Example
/// ```bash
/// curl -X POST \
///   https://api.example.com/api/ext/points/realm123/consume \
///   -H "X-API-Key: your-api-key" \
///   -H "Content-Type: application/json" \
///   -d '{
///     "userId": "user-123",
///     "clientAppId": "app-abc",
///     "amount": 100,
///     "description": "AI API call"
///   }'
/// ```
#[utoipa::path(
    post,
    path = "/api/ext/points/{realmId}/consume",
    tag = "ext",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    request_body = ExtConsumePointsRequest,
    responses(
        (status = 200, description = "Points consumed successfully", body = ExtConsumePointsResponse),
        (status = 400, description = "Bad request (insufficient points, invalid amount, frozen/closed account)", body = ErrorResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API Key", body = ErrorResponse),
        (status = 403, description = "Forbidden - Cross-realm access attempt", body = ErrorResponse),
        (status = 404, description = "Not found - User or account not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("api_key" = []))
)]
pub async fn consume_points_ext(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Json(request): Json<ExtConsumePointsRequest>,
) -> Response {
    let api_key_realm_id = identity.realm_id();

    tracing::info!(
        api_key_realm_id = %api_key_realm_id,
        request_realm_id = %realm_id,
        user_id = %request.user_id,
        amount = request.amount,
        "Points consumption requested"
    );

    // 0. Apply rate limiting (parallel checks for better performance)
    let (realm_result, user_result) = tokio::join!(
        rate_limit(
            &state,
            format!("{}{}", REALM_RATE_LIMIT_PREFIX, realm_id),
            REALM_RATE_LIMIT
        ),
        rate_limit(
            &state,
            format!("{}{}:{}", USER_RATE_LIMIT_PREFIX, realm_id, request.user_id),
            USER_RATE_LIMIT
        )
    );

    if let Err(e) = realm_result {
        tracing::warn!(
            realm_id = %realm_id,
            error = %e,
            "Realm-level rate limit exceeded"
        );
        return json_error(StatusCode::TOO_MANY_REQUESTS, ErrorCode::RateLimitExceeded);
    }

    if let Err(e) = user_result {
        tracing::warn!(
            realm_id = %realm_id,
            user_id = %request.user_id,
            error = %e,
            "User-level rate limit exceeded"
        );
        return json_error(StatusCode::TOO_MANY_REQUESTS, ErrorCode::RateLimitExceeded);
    }

    // 1. Check realm isolation - API key must be for the requested realm
    if !identity.has_access_to_realm(&realm_id) {
        tracing::warn!(
            api_key_realm_id = %api_key_realm_id,
            request_realm_id = %realm_id,
            "Cross-realm access attempt blocked"
        );
        return json_error(StatusCode::FORBIDDEN, ErrorCode::CrossRealmAccessForbidden);
    }

    if let Err(resp) =
        require_principal_permission(&state, &identity, &realm_id, "points", "manage").await
    {
        return resp.into_response();
    }

    // 2. Check idempotency if key is provided
    if let Some(ref idempotency_key) = request.idempotency_key {
        let idempotency_service = &state.idempotency_service;
        let request_data = serde_json::to_string(&request).unwrap_or_else(|_| {
            tracing::warn!(
                idempotency_key = %idempotency_key,
                "Failed to serialize idempotency request data, using empty string"
            );
            String::new()
        });

        match idempotency_service
            .check_or_create(&realm_id, idempotency_key, &request_data)
            .await
        {
            Ok(herald_core::domain::points::IdempotencyResult::Cached { transaction }) => {
                // Return cached response
                let response = ExtConsumePointsResponse {
                    transaction_id: transaction.id.to_string(),
                    wallet_id: transaction.wallet_id.to_string(),
                    user_id: transaction.user_id.to_string(),
                    amount: transaction.amount,
                    balance_after: transaction.balance_after,
                };

                tracing::info!(
                    idempotency_key = %idempotency_key,
                    transaction_id = %transaction.id,
                    "Returning cached idempotent response"
                );

                return Json(response).into_response();
            }
            Ok(herald_core::domain::points::IdempotencyResult::New) => {
                // Proceed with normal processing
            }
            Err(e) => {
                tracing::error!(
                    idempotency_key = %idempotency_key,
                    error = %e,
                    "Idempotency check failed"
                );
                return match e {
                    herald_core::domain::common::entities::app_errors::CoreError::Conflict(_) => {
                        json_error(StatusCode::CONFLICT, ErrorCode::IdempotencyConflict)
                    }
                    _ => json_error(StatusCode::INTERNAL_SERVER_ERROR, ErrorCode::InternalError),
                };
            }
        }
    }

    // 3. Validate amount range (1 point to 1,000,000 points)
    if request.amount <= 0 || request.amount > 1_000_000 {
        return json_error(StatusCode::BAD_REQUEST, ErrorCode::InvalidAmount);
    }

    // 3. Parse user_id
    let user_id = match request.user_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            tracing::warn!("Invalid user ID format: {}", request.user_id);
            return json_error(StatusCode::BAD_REQUEST, ErrorCode::InvalidUserIdFormat);
        }
    };

    // 4. Parse client_app_id
    let client_app_id = match request.client_app_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            tracing::warn!("Invalid client_app_id format: {}", request.client_app_id);
            return json_error(StatusCode::BAD_REQUEST, ErrorCode::InvalidClientAppIdFormat);
        }
    };

    if let Err(resp) = ensure_client_app_scope(&state, &identity, client_app_id).await {
        return resp;
    }

    // 5. Create input DTO
    let input = ConsumePointsInput {
        user_id: user_id.to_string(),
        client_app_id: client_app_id.to_string(),
        amount: request.amount,
        description: request.description.clone(),
    };

    // 6. Consume points
    let transaction = match state
        .points_service
        .consume_points(identity, &realm_id, input)
        .await
    {
        Ok(transaction) => transaction,
        Err(e) => {
            tracing::error!("Failed to consume points: {}", e);
            return match e {
                herald_core::domain::common::entities::app_errors::CoreError::Unauthorized => {
                    json_error(StatusCode::UNAUTHORIZED, ErrorCode::Unauthorized)
                }
                herald_core::domain::common::entities::app_errors::CoreError::Forbidden(_) => {
                    json_error(StatusCode::FORBIDDEN, ErrorCode::Forbidden)
                }
                herald_core::domain::common::entities::app_errors::CoreError::NotFound => {
                    json_error(StatusCode::NOT_FOUND, ErrorCode::WalletNotFound)
                }
                herald_core::domain::common::entities::app_errors::CoreError::BadRequest(msg) => {
                    if msg.contains("Insufficient points") {
                        json_error(StatusCode::BAD_REQUEST, ErrorCode::InsufficientPoints)
                    } else if msg.contains("Cannot consume points from") {
                        json_error(StatusCode::BAD_REQUEST, ErrorCode::WalletFrozenOrClosed)
                    } else {
                        json_error(StatusCode::BAD_REQUEST, ErrorCode::InvalidAmount)
                    }
                }
                herald_core::domain::common::entities::app_errors::CoreError::Conflict(_) => {
                    json_error(StatusCode::CONFLICT, ErrorCode::ConcurrentModification)
                }
                _ => json_error(StatusCode::INTERNAL_SERVER_ERROR, ErrorCode::InternalError),
            };
        }
    };

    // 7. Build response
    let response = ExtConsumePointsResponse {
        transaction_id: transaction.id.to_string(),
        wallet_id: transaction.wallet_id.to_string(),
        user_id: transaction.user_id.to_string(),
        amount: transaction.amount,
        balance_after: transaction.balance_after,
    };

    tracing::info!(
        transaction_id = %transaction.id,
        user_id = %transaction.user_id,
        amount = transaction.amount,
        balance_after = %transaction.balance_after,
        "Points consumed successfully"
    );

    // 8. Save idempotency result if key was provided
    if let Some(ref idempotency_key) = request.idempotency_key {
        let idempotency_service = &state.idempotency_service;
        if let Err(e) = idempotency_service
            .save_result(&realm_id, idempotency_key, &transaction)
            .await
        {
            tracing::error!(
                idempotency_key = %idempotency_key,
                transaction_id = %transaction.id,
                error = %e,
                "Failed to save idempotency result"
            );
            // Non-critical error, don't fail the request
        }
    }

    Json(response).into_response()
}

/// Grant points to a user account (SDK)
///
/// Grants points to a user's account from a third-party application.
/// Returns transaction details including the new balance.
///
/// # Authentication
/// Requires valid API Key via X-API-Key header
///
/// # Realm Isolation
/// The API key must belong to the same realm as the requested realm.
/// Cross-realm requests will return 403 Forbidden.
///
/// # Example
/// ```bash
/// curl -X POST \
///   https://api.example.com/api/ext/points/realm123/grant \
///   -H "X-API-Key: your-api-key" \
///   -H "Content-Type: application/json" \
///   -d '{
///     "userId": "user-123",
///     "amount": 100,
///     "reason": "Promotional grant"
///   }'
/// ```
#[utoipa::path(
    post,
    path = "/api/ext/points/{realmId}/grant",
    tag = "ext",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    request_body = ExtGrantPointsRequest,
    responses(
        (status = 200, description = "Points granted successfully", body = ExtGrantPointsResponse),
        (status = 400, description = "Bad request (invalid amount, invalid user ID, empty reason)", body = ErrorResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API Key", body = ErrorResponse),
        (status = 403, description = "Forbidden - Cross-realm access attempt", body = ErrorResponse),
        (status = 404, description = "Not found - User or account not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("api_key" = []))
)]
pub async fn grant_points_ext(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Json(request): Json<ExtGrantPointsRequest>,
) -> Response {
    let api_key_realm_id = identity.realm_id();

    tracing::info!(
        api_key_realm_id = %api_key_realm_id,
        request_realm_id = %realm_id,
        user_id = %request.user_id,
        amount = request.amount,
        "Points grant requested"
    );

    // 0. Apply rate limiting (realm 100/min, user 20/min)
    let (realm_result, user_result) = tokio::join!(
        rate_limit(
            &state,
            format!("{}{}", REALM_RATE_LIMIT_PREFIX, realm_id),
            REALM_RATE_LIMIT
        ),
        rate_limit(
            &state,
            format!("{}{}:{}", USER_RATE_LIMIT_PREFIX, realm_id, request.user_id),
            USER_RATE_LIMIT
        )
    );

    if let Err(e) = realm_result {
        tracing::warn!(
            realm_id = %realm_id,
            error = %e,
            "Realm-level rate limit exceeded"
        );
        return json_error(StatusCode::TOO_MANY_REQUESTS, ErrorCode::RateLimitExceeded);
    }

    if let Err(e) = user_result {
        tracing::warn!(
            realm_id = %realm_id,
            user_id = %request.user_id,
            error = %e,
            "User-level rate limit exceeded"
        );
        return json_error(StatusCode::TOO_MANY_REQUESTS, ErrorCode::RateLimitExceeded);
    }

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
        require_principal_permission(&state, &identity, &realm_id, "points", "manage").await
    {
        return resp.into_response();
    }

    // 2. Validate amount (1 to 1,000,000)
    if request.amount <= 0 || request.amount > 1_000_000 {
        return json_error(StatusCode::BAD_REQUEST, ErrorCode::InvalidAmount);
    }

    // 3. Parse user_id as UUID
    let user_id = match request.user_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            tracing::warn!("Invalid user ID format: {}", request.user_id);
            return json_error(StatusCode::BAD_REQUEST, ErrorCode::InvalidUserIdFormat);
        }
    };

    // 4. Validate reason non-empty
    if request.reason.trim().is_empty() {
        return json_error(StatusCode::BAD_REQUEST, ErrorCode::ValidationError);
    }

    // 5. Validate validity_days is None or > 0
    if let Some(days) = request.validity_days
        && days <= 0
    {
        return json_error(StatusCode::BAD_REQUEST, ErrorCode::ValidationError);
    }

    // 6. Determine source_id from API key identity
    let source_id = identity
        .as_third_party()
        .map(|api_key| {
            api_key
                .client_app_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| api_key.id.clone())
        })
        .unwrap_or_default();

    // 7. Build input and call service
    let input = GrantPointsInput {
        user_id,
        amount: request.amount,
        reason: request.reason,
        validity_days: request.validity_days,
        source_type: CreditSourceType::SdkGrant,
        source_id,
    };

    let output = match state
        .points_service
        .grant_points_for_sdk(&realm_id, input)
        .await
    {
        Ok(output) => output,
        Err(e) => {
            tracing::error!("Failed to grant points: {}", e);
            return match e {
                herald_core::domain::common::entities::app_errors::CoreError::Unauthorized => {
                    json_error(StatusCode::UNAUTHORIZED, ErrorCode::Unauthorized)
                }
                herald_core::domain::common::entities::app_errors::CoreError::Forbidden(_) => {
                    json_error(StatusCode::FORBIDDEN, ErrorCode::Forbidden)
                }
                herald_core::domain::common::entities::app_errors::CoreError::NotFound => {
                    json_error(StatusCode::NOT_FOUND, ErrorCode::UserNotFound)
                }
                herald_core::domain::common::entities::app_errors::CoreError::BadRequest(_) => {
                    json_error(StatusCode::BAD_REQUEST, ErrorCode::ValidationError)
                }
                _ => json_error(StatusCode::INTERNAL_SERVER_ERROR, ErrorCode::InternalError),
            };
        }
    };

    // 8. Build response (ext convention: balance instead of totalBalance)
    let response = ExtGrantPointsResponse {
        transaction_id: output.transaction_id.to_string(),
        user_id: output.user_id.to_string(),
        amount: output.amount,
        granted_balance: output.granted_balance,
        balance: output.total_balance,
        expires_at: output.expires_at.map(|dt| dt.to_rfc3339()),
    };

    tracing::info!(
        transaction_id = %output.transaction_id,
        user_id = %output.user_id,
        amount = output.amount,
        balance = output.total_balance,
        "Points granted successfully"
    );

    Json(response).into_response()
}

/// Transaction response (SDK-compatible)
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExtTransactionResponse {
    pub transaction_id: String,
    pub wallet_id: String,
    pub user_id: String,
    pub transaction_type: String,
    pub amount: i64,
    pub balance_after: i64,
    pub description: Option<String>,
    pub client_app_id: Option<String>,
    pub subscription_id: Option<String>,
    pub external_ref_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct ExtTransactionByRefQuery {
    pub user_id: Option<String>,
}

fn transaction_to_response(
    transaction: &herald_core::domain::points::entities::PointsTransaction,
) -> ExtTransactionResponse {
    ExtTransactionResponse {
        transaction_id: transaction.id.to_string(),
        wallet_id: transaction.wallet_id.to_string(),
        user_id: transaction.user_id.to_string(),
        transaction_type: transaction.transaction_type.to_string(),
        amount: transaction.amount,
        balance_after: transaction.balance_after,
        description: transaction.description.clone(),
        client_app_id: transaction.client_app_id.map(|id| id.to_string()),
        subscription_id: transaction.subscription_id.map(|id| id.to_string()),
        external_ref_id: transaction.external_ref_id.clone(),
        created_at: transaction.created_at.to_rfc3339(),
    }
}

/// Get transaction by ID
///
/// Returns details of a specific points transaction.
///
/// # Authentication
/// Requires valid API Key via X-API-Key header
///
/// # Realm Isolation
/// The API key must belong to the same realm as the requested realm.
/// Cross-realm requests will return 403 Forbidden.
///
/// # Example
/// ```bash
/// curl -X GET \
///   https://api.example.com/api/ext/points/realm123/transactions/uuid \
///   -H "X-API-Key: your-api-key"
/// ```
#[utoipa::path(
    get,
    path = "/api/ext/points/{realmId}/transactions/{transactionId}",
    tag = "ext",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("transactionId" = String, Path, description = "Transaction ID")
    ),
    responses(
        (status = 200, description = "Transaction retrieved successfully", body = ExtTransactionResponse),
        (status = 400, description = "Bad request - Invalid ID format", body = ErrorResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API Key", body = ErrorResponse),
        (status = 403, description = "Forbidden - Cross-realm access attempt", body = ErrorResponse),
        (status = 404, description = "Not found - Transaction not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("api_key" = []))
)]
pub async fn get_transaction_ext(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, transaction_id_str)): Path<(String, String)>,
) -> Response {
    let api_key_realm_id = identity.realm_id();

    tracing::info!(
        api_key_realm_id = %api_key_realm_id,
        request_realm_id = %realm_id,
        transaction_id = %transaction_id_str,
        "Transaction query requested"
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

    if let Err(resp) =
        require_principal_permission(&state, &identity, &realm_id, "points", "view").await
    {
        return resp.into_response();
    }

    // 2. Parse transaction_id
    let transaction_id = match transaction_id_str.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            tracing::warn!("Invalid transaction ID format: {}", transaction_id_str);
            return json_error(
                StatusCode::BAD_REQUEST,
                ErrorCode::InvalidTransactionIdFormat,
            );
        }
    };

    // 3. Get transaction
    let transaction = match state
        .points_service
        .get_transaction(identity, &realm_id, transaction_id)
        .await
    {
        Ok(transaction) => transaction,
        Err(e) => {
            tracing::error!("Failed to get transaction: {}", e);
            return match e {
                herald_core::domain::common::entities::app_errors::CoreError::Unauthorized => {
                    json_error(StatusCode::UNAUTHORIZED, ErrorCode::Unauthorized)
                }
                herald_core::domain::common::entities::app_errors::CoreError::Forbidden(_) => {
                    json_error(StatusCode::FORBIDDEN, ErrorCode::Forbidden)
                }
                herald_core::domain::common::entities::app_errors::CoreError::NotFound => {
                    json_error(StatusCode::NOT_FOUND, ErrorCode::TransactionNotFound)
                }
                _ => json_error(StatusCode::INTERNAL_SERVER_ERROR, ErrorCode::InternalError),
            };
        }
    };

    let response = transaction_to_response(&transaction);

    tracing::info!(
        transaction_id = %transaction.id,
        "Transaction retrieved successfully"
    );

    Json(response).into_response()
}

/// Get transaction by external reference ID
///
/// Returns details of a specific points transaction by `externalRefId`.
#[utoipa::path(
    get,
    path = "/api/ext/points/{realmId}/transactions/by-external-ref/{externalRefId}",
    tag = "ext",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("externalRefId" = String, Path, description = "External reference ID"),
        ExtTransactionByRefQuery
    ),
    responses(
        (status = 200, description = "Transaction retrieved successfully", body = ExtTransactionResponse),
        (status = 400, description = "Bad request - invalid user ID or non-unique external reference", body = ErrorResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API Key", body = ErrorResponse),
        (status = 403, description = "Forbidden - Cross-realm access attempt", body = ErrorResponse),
        (status = 404, description = "Not found - Transaction not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("api_key" = []))
)]
pub async fn get_transaction_by_external_ref_ext(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, external_ref_id)): Path<(String, String)>,
    Query(query): Query<ExtTransactionByRefQuery>,
) -> Response {
    let api_key_realm_id = identity.realm_id();

    tracing::info!(
        api_key_realm_id = %api_key_realm_id,
        request_realm_id = %realm_id,
        external_ref_id = %external_ref_id,
        "Transaction query by external ref requested"
    );

    if external_ref_id.trim().is_empty() {
        return json_error(StatusCode::BAD_REQUEST, ErrorCode::ValidationError);
    }

    if !identity.has_access_to_realm(&realm_id) {
        return json_error(StatusCode::FORBIDDEN, ErrorCode::CrossRealmAccessForbidden);
    }

    if let Err(resp) =
        require_principal_permission(&state, &identity, &realm_id, "points", "view").await
    {
        return resp.into_response();
    }

    let user_id = match query.user_id {
        Some(raw_user_id) => match raw_user_id.parse::<Uuid>() {
            Ok(uuid) => Some(uuid),
            Err(_) => return json_error(StatusCode::BAD_REQUEST, ErrorCode::InvalidUserIdFormat),
        },
        None => None,
    };

    let filters = TransactionFilters {
        user_id,
        external_ref_id,
        page: Some(1),
        page_size: Some(2),
        ..Default::default()
    };

    let result = match state
        .points_service
        .list_transactions(identity.clone(), &realm_id, filters)
        .await
    {
        Ok(result) => result,
        Err(e) => {
            tracing::error!("Failed to get transaction by external ref: {}", e);
            return match e {
                herald_core::domain::common::entities::app_errors::CoreError::Unauthorized => {
                    json_error(StatusCode::UNAUTHORIZED, ErrorCode::Unauthorized)
                }
                herald_core::domain::common::entities::app_errors::CoreError::Forbidden(_) => {
                    json_error(StatusCode::FORBIDDEN, ErrorCode::Forbidden)
                }
                _ => json_error(StatusCode::INTERNAL_SERVER_ERROR, ErrorCode::InternalError),
            };
        }
    };

    if result.data.is_empty() {
        return json_error(StatusCode::NOT_FOUND, ErrorCode::TransactionNotFound);
    }
    if result.data.len() > 1 {
        return json_error(StatusCode::BAD_REQUEST, ErrorCode::ValidationError);
    }

    let transaction = result.data.into_iter().next().expect("checked non-empty");
    if let Some(client_app_id) = transaction.client_app_id
        && let Err(resp) = ensure_client_app_scope(&state, &identity, client_app_id).await
    {
        return resp;
    }

    Json(transaction_to_response(&transaction)).into_response()
}
