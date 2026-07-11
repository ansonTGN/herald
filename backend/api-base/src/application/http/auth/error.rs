use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub status: u16,
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
    #[serde(rename = "requestId")]
    pub request_id: Option<String>,
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
        let (status, mut error_message) = match self {
            AuthError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AuthError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            AuthError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
            AuthError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AuthError::TooManyRequests(msg) => (StatusCode::TOO_MANY_REQUESTS, msg),
            AuthError::Conflict(msg) => (StatusCode::CONFLICT, msg),
            AuthError::InternalServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };
        if status.is_server_error() {
            tracing::error!(%status, error = %error_message, "Authentication API request failed");
            error_message = "Internal server error".to_string();
        }

        let body = Json(ErrorResponse {
            status: status.as_u16(),
            code: match status {
                StatusCode::BAD_REQUEST => "bad_request",
                StatusCode::UNAUTHORIZED => "unauthorized",
                StatusCode::FORBIDDEN => "forbidden",
                StatusCode::NOT_FOUND => "not_found",
                StatusCode::TOO_MANY_REQUESTS => "rate_limit_exceeded",
                StatusCode::CONFLICT => "conflict",
                _ => "internal_error",
            }
            .to_string(),
            message: error_message,
            details: None,
            request_id: crate::application::http::request_context::current_request_id(),
        });

        (status, body).into_response()
    }
}
