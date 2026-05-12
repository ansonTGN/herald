// Shared API Key Authentication Utilities
//
// Common utilities for API key validation and error handling
// used across multiple authentication middleware implementations.

use super::error_codes::ErrorCode;
use chrono::Utc;
use herald_core::domain::client_api_keys::entities::ClientApiKey;
use herald_core::infrastructure::client_api_keys::cache::ApiKeyCacheValue;

/// Cache TTL for API keys in seconds (5 minutes)
pub const API_KEY_CACHE_TTL_SECONDS: u64 = 300;

/// Status result from validating an API key
#[derive(Debug, PartialEq)]
pub enum ApiKeyValidationStatus {
    Valid,
    Disabled,
    Expired,
    Invalid,
}

impl ApiKeyValidationStatus {
    pub fn to_error_code(&self) -> ErrorCode {
        match self {
            ApiKeyValidationStatus::Disabled => ErrorCode::ApiKeyDisabled,
            ApiKeyValidationStatus::Expired => ErrorCode::ApiKeyExpired,
            _ => ErrorCode::InvalidApiKey,
        }
    }

    pub fn to_error_code_enum(&self) -> ErrorCode {
        self.to_error_code()
    }
}

/// Check if a cached API key is valid (enabled and not expired)
/// Returns detailed validation status for better error messages
pub fn check_cached_key_status(cached: &ApiKeyCacheValue) -> ApiKeyValidationStatus {
    if !cached.enabled {
        return ApiKeyValidationStatus::Disabled;
    }
    match &cached.expires_at {
        Some(expires_at_str) => match chrono::DateTime::parse_from_rfc3339(expires_at_str) {
            Ok(expires_at) => {
                if Utc::now() > expires_at.with_timezone(&Utc) {
                    ApiKeyValidationStatus::Expired
                } else {
                    ApiKeyValidationStatus::Valid
                }
            }
            Err(_) => ApiKeyValidationStatus::Invalid,
        },
        None => ApiKeyValidationStatus::Valid,
    }
}

/// Check if a domain entity is valid (enabled and not expired)
/// Returns detailed validation status for better error messages
pub fn check_entity_status(api_key: &ClientApiKey) -> ApiKeyValidationStatus {
    if !api_key.enabled {
        return ApiKeyValidationStatus::Disabled;
    }
    match api_key.expires_at {
        Some(exp) if Utc::now() > exp => ApiKeyValidationStatus::Expired,
        _ => ApiKeyValidationStatus::Valid,
    }
}

/// Convert cached value to domain entity
pub fn cached_to_entity(cached: ApiKeyCacheValue) -> Result<ClientApiKey, String> {
    cached.try_into()
}
