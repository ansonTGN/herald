use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub code: u32,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug)]
pub enum AuthError {
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    NotFound(String),
    TooManyRequests(String),
    Conflict(String),
    InternalServerError(String),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::BadRequest(msg) => write!(f, "Bad Request: {}", msg),
            AuthError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            AuthError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            AuthError::NotFound(msg) => write!(f, "Not Found: {}", msg),
            AuthError::TooManyRequests(msg) => write!(f, "Too Many Requests: {}", msg),
            AuthError::Conflict(msg) => write!(f, "Conflict: {}", msg),
            AuthError::InternalServerError(msg) => write!(f, "Internal Server Error: {}", msg),
        }
    }
}

// Implement From<CoreError> for AuthError
impl From<herald_core::domain::common::entities::app_errors::CoreError> for AuthError {
    fn from(err: herald_core::domain::common::entities::app_errors::CoreError) -> Self {
        match &err {
            herald_core::domain::common::entities::app_errors::CoreError::NotFound => {
                tracing::debug!("CoreError::NotFound converted to AuthError::NotFound");
                AuthError::NotFound("Resource not found".to_string())
            }
            herald_core::domain::common::entities::app_errors::CoreError::Forbidden(msg) => {
                tracing::debug!(
                    "CoreError::Forbidden converted to AuthError::Forbidden: {}",
                    msg
                );
                AuthError::Forbidden(msg.clone())
            }
            herald_core::domain::common::entities::app_errors::CoreError::Unauthorized => {
                tracing::debug!("CoreError::Unauthorized converted to AuthError::Unauthorized");
                AuthError::Unauthorized("Unauthorized".to_string())
            }
            herald_core::domain::common::entities::app_errors::CoreError::BadRequest(msg) => {
                tracing::debug!("CoreError::BadRequest converted: {}", msg);
                AuthError::BadRequest(msg.clone())
            }
            herald_core::domain::common::entities::app_errors::CoreError::Conflict(msg) => {
                tracing::debug!("CoreError::Conflict: {}", msg);
                AuthError::Conflict(msg.clone())
            }
            herald_core::domain::common::entities::app_errors::CoreError::RateLimitExceeded => {
                tracing::debug!("CoreError::RateLimitExceeded");
                AuthError::TooManyRequests("Rate limit exceeded".to_string())
            }
            _ => {
                tracing::error!(
                    "Unexpected CoreError converted to InternalServerError: {:?}",
                    err
                );
                AuthError::InternalServerError("Internal Server Error".to_string())
            }
        }
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AuthError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AuthError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            AuthError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
            AuthError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AuthError::TooManyRequests(msg) => (StatusCode::TOO_MANY_REQUESTS, msg),
            AuthError::Conflict(msg) => (StatusCode::CONFLICT, msg),
            AuthError::InternalServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(ErrorResponse {
            code: status.as_u16() as u32,
            message: error_message,
            details: None,
        });

        (status, body).into_response()
    }
}
