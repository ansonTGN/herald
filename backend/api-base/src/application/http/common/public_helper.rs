// Public realm configuration helpers
//
// This module provides read-only helper functions for public/unauthenticated endpoints
// that need to query realm configuration without authentication or policy checks.

use crate::application::http::server::api_entities::ApiError;
use sqlx::PgPool;

fn public_config_query_error(message: &'static str) -> ApiError {
    ApiError::internal(message)
}

/// Query a single config value from realm_config table
///
/// This is a lightweight helper for public endpoints that don't require authentication.
/// For authenticated endpoints, use RealmConfigService instead.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `realm_id` - The realm ID
/// * `config_type` - The config type (e.g., "registration", "turnstile")
/// * `config_key` - The config key (e.g., "allowed", "site_secret")
///
/// # Returns
///
/// The config value if found and enabled, None otherwise
pub async fn query_config_value(
    pool: &PgPool,
    realm_id: &str,
    config_type: &str,
    config_key: &str,
) -> Result<Option<String>, ApiError> {
    let value = sqlx::query_as::<_, (String,)>(
        "SELECT config_value FROM realm_config
         WHERE realm_id = $1 AND config_type = $2 AND config_key = $3 AND enabled = true",
    )
    .bind(realm_id)
    .bind(config_type)
    .bind(config_key)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!(
            realm_id = %realm_id,
            config_type = %config_type,
            config_key = %config_key,
            error = %e,
            "Failed to query realm config"
        );
        public_config_query_error("Failed to query realm config")
    })?;

    Ok(value.map(|(v,)| v))
}

/// Query a single config value with metadata from realm_config table
///
/// This is a lightweight helper for public endpoints that need both value and metadata.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `realm_id` - The realm ID
/// * `config_type` - The config type (e.g., "turnstile")
/// * `config_key` - The config key (e.g., "site_secret")
///
/// # Returns
///
/// The config value and metadata if found and enabled, None otherwise
pub async fn query_config_with_metadata(
    pool: &PgPool,
    realm_id: &str,
    config_type: &str,
    config_key: &str,
) -> Result<Option<(String, Option<serde_json::Value>)>, ApiError> {
    let row = sqlx::query_as::<_, (String, Option<serde_json::Value>)>(
        "SELECT config_value, metadata FROM realm_config
         WHERE realm_id = $1 AND config_type = $2 AND config_key = $3 AND enabled = true",
    )
    .bind(realm_id)
    .bind(config_type)
    .bind(config_key)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!(
            realm_id = %realm_id,
            config_type = %config_type,
            config_key = %config_key,
            error = %e,
            "Failed to query realm config with metadata"
        );
        public_config_query_error("Failed to query realm config")
    })?;

    Ok(row)
}

/// Parse a string value into a boolean
///
/// Supports: true/false, 1/0, yes/no (case-insensitive).
/// Returns None for unrecognized values.
pub fn parse_bool(value: &str) -> Option<bool> {
    match value.to_lowercase().as_str() {
        "true" | "1" | "yes" => Some(true),
        "false" | "0" | "no" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    #[test]
    fn test_public_config_query_error_is_generic() {
        let err = public_config_query_error("Failed to query realm config");
        let response = err.into_response();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
