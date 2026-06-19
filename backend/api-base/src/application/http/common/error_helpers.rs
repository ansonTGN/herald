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
///
/// Status codes mirror `CoreError::into_response` in
/// `domain/.../app_errors.rs` so the API surface and the domain-layer
/// (webhook handlers) agree on client-vs-server semantics.
pub fn core_error_to_api_error(e: CoreError, operation: &str) -> ApiError {
    match e {
        CoreError::NotFound => ApiError::not_found(format!("{operation} not found")),
        CoreError::Conflict(msg) => ApiError::conflict(msg),
        CoreError::BadRequest(msg) => ApiError::bad_request(msg),
        CoreError::Forbidden(msg) => ApiError::forbidden(msg),
        // Credit-bucket routing errors (design credit-bucket §4.2.3 / §5.5).
        // Must surface as client errors (4xx) instead of being swallowed by
        // the 500 fallback; status codes align with `app_errors.rs`.
        CoreError::EntitlementMappingNotFound => {
            ApiError::not_found("Entitlement mapping not found".to_string())
        }
        CoreError::EntitlementMappingNotAttachedToBucket { mapping_id } => {
            ApiError::unprocessable_entity(format!(
                "Entitlement mapping {mapping_id} is not attached to a credit bucket"
            ))
        }
        CoreError::SubscriptionBucketNotResolved { subscription_id } => {
            ApiError::unprocessable_entity(format!(
                "Subscription {subscription_id} is not bound to a credit bucket"
            ))
        }
        CoreError::NoCoveredPointsPool { client_app_id } => ApiError::bad_request(format!(
            "Client app {client_app_id} does not cover any available credit bucket"
        )),
        CoreError::GrantBucketRequired => {
            ApiError::bad_request("Points grant requires an explicit target bucket".to_string())
        }
        _ => ApiError::internal(format!("Failed to {operation}: {e}")),
    }
}

pub fn json_error(status: StatusCode, error_code: ErrorCode) -> Response {
    ApiError::with_code(status, error_code.as_u32(), error_code.as_str()).into_response()
}
