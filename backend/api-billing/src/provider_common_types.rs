//! Shared types for payment-provider configuration handlers.
//!
//! These types are reused across the Stripe / Creem provider
//! configuration endpoints (validation error envelopes, generic error
//! responses, and the multi-provider directory listing). They were
//! previously colocated with the (now-removed) Shopify config handlers.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// Payment provider list response
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct PaymentProvidersResponse {
    pub providers: Vec<PaymentProviderInfo>,
}

/// Payment provider information
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaymentProviderInfo {
    pub platform: String,
    pub shop_domain: Option<String>,
    pub api_version: Option<String>,
    pub webhook_endpoint: Option<String>,
    pub last_updated: Option<String>,
}

/// Error response for validation failures
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ValidationErrorResponse {
    pub error: String,
    pub details: Vec<ValidationErrorDetail>,
}

/// Individual validation error
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ValidationErrorDetail {
    pub field: String,
    pub message: String,
}

/// Generic error response
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct GenericErrorResponse {
    pub error: String,
    pub message: String,
}

pub fn validate_request<T: Validate>(req: &T) -> Result<(), ValidationErrorResponse> {
    if let Err(errors) = req.validate() {
        let details: Vec<ValidationErrorDetail> = errors
            .field_errors()
            .iter()
            .flat_map(|(field, errs)| {
                errs.iter().map(move |error| ValidationErrorDetail {
                    field: field.to_string(),
                    message: error.code.to_string(),
                })
            })
            .collect();
        Err(ValidationErrorResponse {
            error: "validation_failed".to_string(),
            details,
        })
    } else {
        Ok(())
    }
}
