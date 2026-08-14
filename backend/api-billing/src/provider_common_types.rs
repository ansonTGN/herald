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
#[derive(Debug, Clone, Default, Deserialize, Serialize, ToSchema)]
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

/// Whitelist of recognized payment-provider identifiers across the billing
/// surface (design support-iap §3.4). Shared by the entitlement-mapping create
/// and payment-attempt create validators so IAP attempts stay recognizable for
/// status/lookup queries even though IAP receipt submission itself goes through
/// a dedicated endpoint (§4.2.1).
pub fn validate_payment_provider_value(provider: &str) -> Result<(), validator::ValidationError> {
    if matches!(provider, "stripe" | "creem" | "apple" | "google" | "wechat") {
        Ok(())
    } else {
        Err(validator::ValidationError::new("invalid_payment_provider"))
    }
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
