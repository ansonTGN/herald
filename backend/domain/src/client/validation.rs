//! Client App validation utilities
//!
//! This module provides validation functions for Client App settings,
//! including redirect URI validation and session configuration validation.

use crate::common::entities::app_errors::CoreError;
use url::Url;

const MAX_SESSION_TTL_SECONDS: i32 = 86_400;

/// Validates a redirect URI according to security requirements
///
/// # Rules
/// - Must be a valid URL
/// - Must NOT use `javascript:` protocol (security risk)
/// - Must NOT be a protocol-relative URL (starts with `//`)
/// - Production environments require HTTPS
///
/// # Arguments
/// * `uri` - The redirect URI to validate
/// * `is_development` - Whether this is a development environment (allows HTTP)
///
/// # Returns
/// * `Ok(())` if the URI is valid
/// * `Err(CoreError)` if validation fails
pub fn validate_redirect_uri(uri: &str, is_development: bool) -> Result<(), CoreError> {
    // Check for protocol-relative URLs (security risk)
    if uri.starts_with("//") {
        return Err(CoreError::BadRequest(
            "Protocol-relative URLs (starting with //) are not allowed for redirect URIs"
                .to_string(),
        ));
    }

    // Check for javascript: protocol (security risk)
    if uri.to_lowercase().starts_with("javascript:") {
        return Err(CoreError::BadRequest(
            "javascript: protocol is not allowed for redirect URIs".to_string(),
        ));
    }

    // Parse and validate the URL
    let url = Url::parse(uri)
        .map_err(|_| CoreError::BadRequest(format!("Invalid redirect URI: {}", uri)))?;

    // Ensure the URL has a scheme
    if url.scheme().is_empty() {
        return Err(CoreError::BadRequest(
            "Redirect URI must have a scheme (http:// or https://)".to_string(),
        ));
    }

    // Enforce HTTPS in production
    if !is_development && url.scheme() != "https" {
        return Err(CoreError::BadRequest(
            "Redirect URIs must use HTTPS in production environments".to_string(),
        ));
    }

    // Only allow http and https schemes
    match url.scheme() {
        "http" | "https" => Ok(()),
        _ => Err(CoreError::BadRequest(format!(
            "Unsupported URL scheme: {}. Only http and https are allowed",
            url.scheme()
        ))),
    }
}

/// Validates a list of redirect URIs
///
/// # Rules
/// - Must contain at least one URI
/// - Each URI must pass `validate_redirect_uri`
///
/// # Arguments
/// * `uris` - The list of redirect URIs to validate
/// * `is_development` - Whether this is a development environment
///
/// # Returns
/// * `Ok(())` if all URIs are valid
/// * `Err(CoreError)` if validation fails
pub fn validate_redirect_uris(uris: &[String], is_development: bool) -> Result<(), CoreError> {
    // Must have at least one redirect URI
    if uris.is_empty() {
        return Err(CoreError::BadRequest(
            "At least one redirect URI is required".to_string(),
        ));
    }

    // Validate each URI
    for uri in uris {
        validate_redirect_uri(uri, is_development)
            .map_err(|e| CoreError::BadRequest(format!("Invalid redirect URI '{}': {}", uri, e)))?;
    }

    Ok(())
}

/// Validates session TTL configuration
///
/// # Rules
/// - Session TTL must be at least 60 seconds
/// - Session renewal TTL (if provided) must be at least 60 seconds
///
/// # Arguments
/// * `session_ttl_seconds` - The initial session TTL in seconds
/// * `session_renewal_ttl_seconds` - The optional session renewal TTL in seconds
///
/// # Returns
/// * `Ok(())` if the configuration is valid
/// * `Err(CoreError)` if validation fails
pub fn validate_session_config(
    session_ttl_seconds: i32,
    session_renewal_ttl_seconds: Option<i32>,
) -> Result<(), CoreError> {
    // Validate initial session TTL
    if session_ttl_seconds < 60 {
        return Err(CoreError::BadRequest(format!(
            "Session TTL must be at least 60 seconds, got {}",
            session_ttl_seconds
        )));
    }
    if session_ttl_seconds > MAX_SESSION_TTL_SECONDS {
        return Err(CoreError::BadRequest(format!(
            "Session TTL must be at most {} seconds, got {}",
            MAX_SESSION_TTL_SECONDS, session_ttl_seconds
        )));
    }

    // Validate renewal TTL if provided
    if let Some(renewal_ttl) = session_renewal_ttl_seconds {
        if renewal_ttl < 60 {
            return Err(CoreError::BadRequest(format!(
                "Session renewal TTL must be at least 60 seconds, got {}",
                renewal_ttl
            )));
        }
        if renewal_ttl > MAX_SESSION_TTL_SECONDS {
            return Err(CoreError::BadRequest(format!(
                "Session renewal TTL must be at most {} seconds, got {}",
                MAX_SESSION_TTL_SECONDS, renewal_ttl
            )));
        }
        if renewal_ttl >= session_ttl_seconds {
            return Err(CoreError::BadRequest(
                "Session renewal TTL must be less than session TTL".to_string(),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_redirect_uri_valid_https() {
        assert!(validate_redirect_uri("https://example.com/callback", false).is_ok());
    }

    #[test]
    fn test_validate_redirect_uri_valid_http_development() {
        assert!(validate_redirect_uri("http://localhost:3000/callback", true).is_ok());
    }

    #[test]
    fn test_validate_redirect_uri_http_rejected_in_production() {
        assert!(validate_redirect_uri("http://example.com/callback", false).is_err());
    }

    #[test]
    fn test_validate_redirect_uri_javascript_protocol_rejected() {
        assert!(validate_redirect_uri("javascript:alert('xss')", true).is_err());
    }

    #[test]
    fn test_validate_redirect_uri_protocol_relative_rejected() {
        assert!(validate_redirect_uri("//evil.com/callback", true).is_err());
    }

    #[test]
    fn test_validate_redirect_uri_invalid_url() {
        assert!(validate_redirect_uri("not-a-url", true).is_err());
    }

    #[test]
    fn test_validate_redirect_uris_requires_at_least_one() {
        assert!(validate_redirect_uris(&[], true).is_err());
    }

    #[test]
    fn test_validate_redirect_uris_valid_list() {
        let uris = vec![
            "https://example.com/callback".to_string(),
            "https://app.example.com/auth".to_string(),
        ];
        assert!(validate_redirect_uris(&uris, false).is_ok());
    }

    #[test]
    fn test_validate_session_config_valid() {
        assert!(validate_session_config(1800, Some(3600)).is_ok());
    }

    #[test]
    fn test_validate_session_config_ttl_too_short() {
        assert!(validate_session_config(30, None).is_err());
    }

    #[test]
    fn test_validate_session_config_renewal_ttl_too_short() {
        assert!(validate_session_config(1800, Some(30)).is_err());
    }

    #[test]
    fn test_validate_session_config_no_renewal() {
        assert!(validate_session_config(1800, None).is_ok());
    }
}
