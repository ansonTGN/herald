use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

#[derive(Clone, Debug, Error, PartialEq)]
pub enum CoreError {
    #[error("Resource not found")]
    NotFound,
    #[error("Invalid realm: {0}")]
    InvalidRealm(String),
    #[error("Internal server error: {0}")]
    InternalServerError(String),
    #[error("Forbidden: {0}")]
    Forbidden(String),
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Invalid input: {0}")]
    BadRequest(String),
    #[error("Conflict: {0}")]
    Conflict(String),
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    #[error("Email verification failed")]
    EmailVerificationFailed,
    #[error("Password reset failed")]
    PasswordResetFailed,
    #[error("Billing error: {0}")]
    BillingError(String),
    #[error("Subscription not found for realm: {0}")]
    SubscriptionNotFound(String),
    #[error("Invalid subscription status: {0}")]
    InvalidSubscriptionStatus(String),
    #[error("Creem API error: {0}")]
    CreemApiError(String),
    #[error("Invalid webhook signature")]
    InvalidWebhookSignature,
    #[error("Webhook timestamp expired")]
    WebhookTimestampExpired,
    #[error("Invalid webhook payload")]
    InvalidWebhookPayload,
    #[error("Invalid webhook secret")]
    InvalidWebhookSecret,
    #[error("Duplicate webhook event: {0}")]
    DuplicateWebhookEvent(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Entitlement mapping not found")]
    EntitlementMappingNotFound,
}

// From impls for external error types
impl From<sea_orm::DbErr> for CoreError {
    fn from(err: sea_orm::DbErr) -> Self {
        tracing::debug!("Database error: {:?}", err);
        CoreError::DatabaseError(err.to_string())
    }
}

impl From<sqlx::Error> for CoreError {
    fn from(err: sqlx::Error) -> Self {
        tracing::debug!("SQLx error: {:?}", err);
        CoreError::DatabaseError(err.to_string())
    }
}

impl From<redis::RedisError> for CoreError {
    fn from(err: redis::RedisError) -> Self {
        tracing::debug!("Redis error: {:?}", err);
        CoreError::InternalServerError(format!("Redis error: {err}"))
    }
}

impl From<serde_json::Error> for CoreError {
    fn from(err: serde_json::Error) -> Self {
        tracing::debug!("JSON serialization error: {:?}", err);
        CoreError::InternalServerError(format!("JSON error: {err}"))
    }
}

impl From<bcrypt::BcryptError> for CoreError {
    fn from(err: bcrypt::BcryptError) -> Self {
        tracing::debug!("Bcrypt error: {:?}", err);
        CoreError::InternalServerError(format!("Bcrypt error: {err}"))
    }
}

impl From<url::ParseError> for CoreError {
    fn from(err: url::ParseError) -> Self {
        tracing::debug!("URL parse error: {:?}", err);
        CoreError::InternalServerError(format!("URL parse error: {err}"))
    }
}

impl From<reqwest::Error> for CoreError {
    fn from(err: reqwest::Error) -> Self {
        tracing::debug!("HTTP client error: {:?}", err);
        CoreError::CreemApiError(err.to_string())
    }
}

impl CoreError {
    pub fn not_found(context: &str) -> Self {
        tracing::debug!("Resource not found: {}", context);
        CoreError::NotFound
    }

    pub fn forbidden(context: &str, reason: &str) -> Self {
        tracing::debug!(
            "Access forbidden - context: {}, reason: {}",
            context,
            reason
        );
        CoreError::Forbidden(reason.to_string())
    }

    pub fn bad_request(context: &str, reason: &str) -> Self {
        tracing::warn!("Bad request - context: {}, reason: {}", context, reason);
        CoreError::BadRequest(reason.to_string())
    }
}

// IntoResponse for CoreError (used by webhook handlers)
impl IntoResponse for CoreError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            CoreError::NotFound => (StatusCode::NOT_FOUND, "Resource not found".to_string()),
            CoreError::InvalidRealm(msg) => (StatusCode::BAD_REQUEST, msg),
            CoreError::InternalServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            CoreError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
            CoreError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()),
            CoreError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            CoreError::Conflict(msg) => (StatusCode::CONFLICT, msg),
            CoreError::DatabaseError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            CoreError::RateLimitExceeded => (
                StatusCode::TOO_MANY_REQUESTS,
                "Rate limit exceeded".to_string(),
            ),
            CoreError::EmailVerificationFailed => (
                StatusCode::BAD_REQUEST,
                "Email verification failed".to_string(),
            ),
            CoreError::PasswordResetFailed => {
                (StatusCode::BAD_REQUEST, "Password reset failed".to_string())
            }
            CoreError::BillingError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            CoreError::SubscriptionNotFound(msg) => (
                StatusCode::NOT_FOUND,
                format!("Subscription not found: {}", msg),
            ),
            CoreError::InvalidSubscriptionStatus(msg) => (
                StatusCode::BAD_REQUEST,
                format!("Invalid subscription status: {}", msg),
            ),
            CoreError::CreemApiError(msg) => {
                (StatusCode::BAD_GATEWAY, format!("Creem API error: {}", msg))
            }
            CoreError::InvalidWebhookSignature => (
                StatusCode::UNAUTHORIZED,
                "Invalid webhook signature".to_string(),
            ),
            CoreError::WebhookTimestampExpired => (
                StatusCode::UNAUTHORIZED,
                "Webhook timestamp expired".to_string(),
            ),
            CoreError::InvalidWebhookPayload => (
                StatusCode::BAD_REQUEST,
                "Invalid webhook payload".to_string(),
            ),
            CoreError::InvalidWebhookSecret => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Invalid webhook secret".to_string(),
            ),
            CoreError::DuplicateWebhookEvent(msg) => (
                StatusCode::CONFLICT,
                format!("Duplicate webhook event: {}", msg),
            ),
            CoreError::SerializationError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Serialization error: {}", msg),
            ),
            CoreError::EntitlementMappingNotFound => (
                StatusCode::NOT_FOUND,
                "Entitlement mapping not found".to_string(),
            ),
        };

        let body = Json(serde_json::json!({
            "error": error_message
        }));

        (status, body).into_response()
    }
}
