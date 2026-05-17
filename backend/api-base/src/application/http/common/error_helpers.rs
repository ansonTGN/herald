// =============================================================================
// Error Handling Helpers
// =============================================================================
//
// Provides common error handling macros and helper functions.
//
// =============================================================================

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use herald_core::domain::common::entities::app_errors::CoreError;

use crate::application::http::common::error_codes::ErrorCode;
use crate::application::http::server::api_entities::ApiError;

/// Database error handling macro - logs and converts to InternalServerError
#[macro_export]
macro_rules! db_error {
    ($expr:expr, $operation:expr) => {
        $expr.map_err(|e| {
            tracing::error!("Failed to {}: {}", $operation, e);
            $crate::application::http::server::api_entities::ApiError::internal(format!(
                "Failed to {}",
                $operation
            ))
        })
    };
}

/// Database error handling macro (with custom error message)
#[macro_export]
macro_rules! db_error_msg {
    ($expr:expr, $operation:expr, $msg:expr) => {
        $expr.map_err(|e| {
            tracing::error!("Failed to {}: {}", $operation, e);
            $crate::application::http::server::api_entities::ApiError::bad_request($msg.to_string())
        })
    };
}

/// Option to NotFound error macro
#[macro_export]
macro_rules! option_to_not_found {
    ($expr:expr, $entity:expr) => {
        $expr.ok_or_else(|| {
            $crate::application::http::server::api_entities::ApiError::not_found(format!(
                "{} not found",
                $entity
            ))
        })
    };
}

/// Converts CoreError to ApiError with appropriate mapping
pub fn core_error_to_api_error(e: CoreError, operation: &str) -> ApiError {
    match e {
        CoreError::NotFound => ApiError::not_found(format!("{operation} not found")),
        CoreError::Conflict(msg) => ApiError::conflict(msg),
        CoreError::BadRequest(msg) => ApiError::bad_request(msg),
        CoreError::Forbidden(msg) => ApiError::forbidden(msg),
        _ => ApiError::internal(format!("Failed to {operation}: {e}")),
    }
}

pub fn json_error(status: StatusCode, error_code: ErrorCode) -> Response {
    ApiError::with_code(status, error_code.as_u32(), error_code.as_str()).into_response()
}
