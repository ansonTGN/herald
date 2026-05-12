use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub code: u32,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct ApiError {
    status: StatusCode,
    body: ErrorBody,
}

#[derive(Debug, Clone)]
enum ErrorBody {
    Standard(ErrorResponse),
    Custom(serde_json::Value),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.body {
            ErrorBody::Standard(body) => write!(f, "{}: {}", self.status, body.message),
            ErrorBody::Custom(_) => write!(f, "{}: Error", self.status),
        }
    }
}

impl std::error::Error for ApiError {}

impl ApiError {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            body: ErrorBody::Standard(ErrorResponse {
                code: status.as_u16() as u32,
                message: message.into(),
                details: None,
            }),
        }
    }

    pub fn with_code(status: StatusCode, code: u32, message: impl Into<String>) -> Self {
        Self {
            status,
            body: ErrorBody::Standard(ErrorResponse {
                code,
                message: message.into(),
                details: None,
            }),
        }
    }

    pub fn with_json<T: Serialize>(status: StatusCode, body: T) -> Self {
        let json_body = serde_json::to_value(&body).unwrap_or_else(|_| {
            tracing::warn!(
                status = %status,
                "Failed to serialize error body to JSON, using empty object"
            );
            serde_json::Value::Object(Default::default())
        });
        Self {
            status,
            body: ErrorBody::Custom(json_body),
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, message)
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, message)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, message)
    }

    pub fn too_many_requests(message: impl Into<String>) -> Self {
        Self::new(StatusCode::TOO_MANY_REQUESTS, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, message)
    }

    pub fn unprocessable_entity(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, message)
    }

    pub fn unprocessable_entity_json<T: Serialize>(body: T) -> Self {
        Self::with_json(StatusCode::UNPROCESSABLE_ENTITY, body)
    }

    pub fn bad_request_json<T: Serialize>(body: T) -> Self {
        Self::with_json(StatusCode::BAD_REQUEST, body)
    }

    pub fn conflict_json<T: Serialize>(body: T) -> Self {
        Self::with_json(StatusCode::CONFLICT, body)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self.body {
            ErrorBody::Standard(body) => (self.status, Json(body)).into_response(),
            ErrorBody::Custom(json_body) => (self.status, Json(json_body)).into_response(),
        }
    }
}

impl From<herald_core::domain::common::entities::app_errors::CoreError> for ApiError {
    fn from(err: herald_core::domain::common::entities::app_errors::CoreError) -> Self {
        use herald_core::domain::common::entities::app_errors::CoreError;

        match err {
            CoreError::NotFound => Self::not_found("Resource not found"),
            CoreError::InvalidRealm(msg) => Self::bad_request(msg),
            CoreError::InternalServerError(msg) => Self::internal(msg),
            CoreError::Forbidden(msg) => Self::forbidden(msg),
            CoreError::Unauthorized => Self::unauthorized("Unauthorized"),
            CoreError::BadRequest(msg) => Self::bad_request(msg),
            CoreError::Conflict(msg) => Self::conflict(msg),
            CoreError::DatabaseError(msg) => Self::internal(msg),
            CoreError::RateLimitExceeded => Self::too_many_requests("Rate limit exceeded"),
            CoreError::EmailVerificationFailed => Self::bad_request("Email verification failed"),
            CoreError::PasswordResetFailed => Self::bad_request("Password reset failed"),
            CoreError::BillingError(msg) => Self::internal(msg),
            CoreError::SubscriptionNotFound(msg) => {
                Self::not_found(format!("Subscription not found: {msg}"))
            }
            CoreError::InvalidSubscriptionStatus(msg) => {
                Self::bad_request(format!("Invalid subscription status: {msg}"))
            }
            CoreError::CreemApiError(msg) => {
                Self::new(StatusCode::BAD_GATEWAY, format!("Creem API error: {msg}"))
            }
            CoreError::InvalidWebhookSignature => Self::unauthorized("Invalid webhook signature"),
            CoreError::WebhookTimestampExpired => Self::unauthorized("Webhook timestamp expired"),
            CoreError::InvalidWebhookPayload => Self::bad_request("Invalid webhook payload"),
            CoreError::InvalidWebhookSecret => Self::internal("Invalid webhook secret"),
            CoreError::DuplicateWebhookEvent(msg) => {
                Self::conflict(format!("Duplicate webhook event: {msg}"))
            }
            CoreError::PlanNotFound { realm_id, plan_id } => Self::not_found(format!(
                "Plan not found for realm: {realm_id}, plan_id: {plan_id}"
            )),
            CoreError::DuplicatePlan { realm_id, name } => Self::conflict(format!(
                "Duplicate plan name for realm: {realm_id}, name: {name}"
            )),
            CoreError::PlanHasActiveSubscriptions { plan_id } => Self::bad_request(format!(
                "Plan has active subscriptions and cannot be deleted: {plan_id}"
            )),
            CoreError::PlanNotAssignedToClientApp {
                client_app_id,
                plan_id,
            } => Self::not_found(format!(
                "Plan not assigned to client app: client_app_id={client_app_id}, plan_id={plan_id}"
            )),
            CoreError::SerializationError(msg) => {
                Self::internal(format!("Serialization error: {msg}"))
            }
            CoreError::ProductNotFound {
                realm_id,
                product_id,
            } => Self::not_found(format!(
                "Product not found for realm: {realm_id}, product_id: {product_id}"
            )),
            CoreError::ProductNameExists { realm_id, name } => Self::conflict(format!(
                "Product name '{name}' already exists in realm: {realm_id}"
            )),
            CoreError::ProductHasPlans { product_id } => Self::conflict(format!(
                "Cannot delete product with existing plans: {product_id}"
            )),
            CoreError::PointsPackageNotFound(msg) => {
                Self::not_found(format!("Points package not found: {msg}"))
            }
            CoreError::PaymentProviderAlreadyConfigured(provider) => Self::conflict(format!(
                "Payment provider '{provider}' already configured for this package"
            )),
            CoreError::PackageHasPurchaseRecords => {
                Self::conflict("Cannot delete package with existing purchase records")
            }
            CoreError::InvalidPointsAmount(amount) => Self::bad_request(format!(
                "Invalid points amount: {amount}. Must be greater than 0"
            )),
            CoreError::InvalidPrice(price) => {
                Self::bad_request(format!("Invalid price: {price}. Must be greater than 0"))
            }
        }
    }
}

impl From<crate::application::http::auth::error::AuthError> for ApiError {
    fn from(err: crate::application::http::auth::error::AuthError) -> Self {
        match err {
            crate::application::http::auth::error::AuthError::BadRequest(msg) => {
                Self::bad_request(msg)
            }
            crate::application::http::auth::error::AuthError::Unauthorized(msg) => {
                Self::unauthorized(msg)
            }
            crate::application::http::auth::error::AuthError::Forbidden(msg) => {
                Self::forbidden(msg)
            }
            crate::application::http::auth::error::AuthError::NotFound(msg) => Self::not_found(msg),
            crate::application::http::auth::error::AuthError::TooManyRequests(msg) => {
                Self::new(StatusCode::TOO_MANY_REQUESTS, msg)
            }
            crate::application::http::auth::error::AuthError::Conflict(msg) => Self::conflict(msg),
            crate::application::http::auth::error::AuthError::InternalServerError(msg) => {
                tracing::error!("AuthError::InternalServerError: {}", msg);
                Self::internal("Internal server error")
            }
        }
    }
}
