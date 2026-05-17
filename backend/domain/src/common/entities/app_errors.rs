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
    #[error("Subscription plan not found for realm: {realm_id}, plan_id: {plan_id}")]
    SubscriptionPlanNotFound { realm_id: String, plan_id: String },
    #[error("Duplicate subscription plan name for realm: {realm_id}, name: {name}")]
    DuplicateSubscriptionPlan { realm_id: String, name: String },
    #[error("Subscription plan has active subscriptions and cannot be deleted: {plan_id}")]
    SubscriptionPlanHasActiveSubscriptions { plan_id: String },
    #[error(
        "Subscription plan not assigned to client app: client_app_id={client_app_id}, plan_id={plan_id}"
    )]
    SubscriptionPlanNotAssignedToClientApp {
        client_app_id: String,
        plan_id: String,
    },
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Product not found for realm: {realm_id}, product_id: {product_id}")]
    ProductNotFound {
        realm_id: String,
        product_id: String,
    },
    #[error("Product code '{code}' already exists in realm: {realm_id}")]
    ProductCodeExists { realm_id: String, code: String },
    #[error("Cannot delete product with existing plans: {product_id}")]
    ProductHasSubscriptionPlans { product_id: String },
    #[error("Points package not found: {0}")]
    PointsPackageNotFound(String),
    #[error("Payment provider '{0}' already configured for this package")]
    PaymentProviderAlreadyConfigured(String),
    #[error("Cannot delete package with existing purchase records")]
    PackageHasPurchaseRecords,
    #[error("Invalid points amount: {0}. Must be greater than 0")]
    InvalidPointsAmount(i64),
    #[error("Invalid price: {0}. Must be greater than 0")]
    InvalidPrice(i64),
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
            CoreError::SubscriptionPlanNotFound { realm_id, plan_id } => (
                StatusCode::NOT_FOUND,
                format!(
                    "Subscription plan not found for realm: {}, plan_id: {}",
                    realm_id, plan_id
                ),
            ),
            CoreError::DuplicateSubscriptionPlan { realm_id, name } => (
                StatusCode::CONFLICT,
                format!(
                    "Duplicate subscription plan name for realm: {}, name: {}",
                    realm_id, name
                ),
            ),
            CoreError::SubscriptionPlanHasActiveSubscriptions { plan_id } => (
                StatusCode::BAD_REQUEST,
                format!(
                    "Subscription plan has active subscriptions and cannot be deleted: {}",
                    plan_id
                ),
            ),
            CoreError::SubscriptionPlanNotAssignedToClientApp {
                client_app_id,
                plan_id,
            } => (
                StatusCode::NOT_FOUND,
                format!(
                    "Subscription plan not assigned to client app: client_app_id={}, plan_id={}",
                    client_app_id, plan_id
                ),
            ),
            CoreError::SerializationError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Serialization error: {}", msg),
            ),
            CoreError::ProductNotFound {
                realm_id,
                product_id,
            } => (
                StatusCode::NOT_FOUND,
                format!(
                    "Product not found for realm: {}, product_id: {}",
                    realm_id, product_id
                ),
            ),
            CoreError::ProductCodeExists { realm_id, code } => (
                StatusCode::CONFLICT,
                format!(
                    "Product code '{}' already exists in realm: {}",
                    code, realm_id
                ),
            ),
            CoreError::ProductHasSubscriptionPlans { product_id } => (
                StatusCode::CONFLICT,
                format!("Cannot delete product with existing plans: {}", product_id),
            ),
            CoreError::PointsPackageNotFound(msg) => (
                StatusCode::NOT_FOUND,
                format!("Points package not found: {}", msg),
            ),
            CoreError::PaymentProviderAlreadyConfigured(provider) => (
                StatusCode::CONFLICT,
                format!(
                    "Payment provider '{}' already configured for this package",
                    provider
                ),
            ),
            CoreError::PackageHasPurchaseRecords => (
                StatusCode::CONFLICT,
                "Cannot delete package with existing purchase records".to_string(),
            ),
            CoreError::InvalidPointsAmount(amount) => (
                StatusCode::BAD_REQUEST,
                format!("Invalid points amount: {}. Must be greater than 0.", amount),
            ),
            CoreError::InvalidPrice(price) => (
                StatusCode::BAD_REQUEST,
                format!("Invalid price: {}. Must be greater than 0.", price),
            ),
        };

        let body = Json(serde_json::json!({
            "error": error_message
        }));

        (status, body).into_response()
    }
}
