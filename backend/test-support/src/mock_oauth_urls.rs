// =============================================================================
// Mock OAuth URL Constants for Testing
// =============================================================================
//
// This module provides mock OAuth server URLs for testing purposes only.
// These URLs point to the Beeceptor mock server that simulates OAuth providers.
//
// ## Usage
//
// Import these constants in test code to configure OAuth providers with mock URLs.
//
// ```rust
// use herald_test_support::mock_oauth_urls::{mock_auth_url, mock_token_url};
//
// let auth_url = mock_auth_url(); // "https://oauth-mock.mock.beeceptor.com/oauth/authorize"
// let token_url = mock_token_url("google"); // "https://oauth-mock.mock.beeceptor.com/oauth/token/google"
// ```
//
// ## Architecture
//
// - Mock URLs are defined as test-only constants
// - No configuration file dependency
// - Production code is not affected
// - Easy to maintain and update
//
// ## Reference
//
// - OAuth Mock specification: `.ai/future/third.md`
// - OAuth implementation: `api/src/application/http/oauth/`

/// Mock OAuth server base URL for testing
///
/// This URL points to the Beeceptor mock server that simulates OAuth providers.
/// It is used only in tests to avoid dependency on real OAuth providers.
pub const MOCK_OAUTH_BASE_URL: &str = "https://oauth-mock.mock.beeceptor.com";

/// Get the mock OAuth authorization URL
///
/// Returns the authorization endpoint URL for the mock OAuth server.
/// This URL is used in the OAuth authorization flow.
///
/// # Example
/// ```rust
/// # use herald_test_support::mock_oauth_urls::mock_auth_url;
/// let url = mock_auth_url();
/// assert_eq!(url, "https://oauth-mock.mock.beeceptor.com/oauth/authorize");
/// ```
pub fn mock_auth_url() -> String {
    format!("{}/oauth/authorize", MOCK_OAUTH_BASE_URL)
}

/// Get the mock OAuth token URL for a specific provider
///
/// Returns the token endpoint URL for the mock OAuth server.
/// This URL is used to exchange authorization codes for access tokens.
///
/// # Arguments
/// * `provider` - OAuth provider type (e.g., "google", "github", "wechat")
///
/// # Example
/// ```rust
/// # use herald_test_support::mock_oauth_urls::mock_token_url;
/// let google_token_url = mock_token_url("google");
/// assert_eq!(google_token_url, "https://oauth-mock.mock.beeceptor.com/oauth/token/google");
///
/// let github_token_url = mock_token_url("github");
/// assert_eq!(github_token_url, "https://oauth-mock.mock.beeceptor.com/oauth/token/github");
/// ```
pub fn mock_token_url(provider: &str) -> String {
    format!("{}/oauth/token/{}", MOCK_OAUTH_BASE_URL, provider)
}

/// Get the mock OAuth userinfo URL for a specific provider
///
/// Returns the userinfo endpoint URL for the mock OAuth server.
/// This URL is used to retrieve user information from the OAuth provider.
///
/// # Arguments
/// * `provider` - OAuth provider type (e.g., "google", "github", "wechat")
///
/// # Example
/// ```rust
/// # use herald_test_support::mock_oauth_urls::mock_userinfo_url;
/// let google_userinfo_url = mock_userinfo_url("google");
/// assert_eq!(google_userinfo_url, "https://oauth-mock.mock.beeceptor.com/userinfo/google");
///
/// let github_userinfo_url = mock_userinfo_url("github");
/// assert_eq!(github_userinfo_url, "https://oauth-mock.mock.beeceptor.com/userinfo/github");
/// ```
pub fn mock_userinfo_url(provider: &str) -> String {
    format!("{}/userinfo/{}", MOCK_OAUTH_BASE_URL, provider)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_auth_url() {
        let url = mock_auth_url();
        assert_eq!(url, "https://oauth-mock.mock.beeceptor.com/oauth/authorize");
    }

    #[test]
    fn test_mock_token_url() {
        assert_eq!(
            mock_token_url("google"),
            "https://oauth-mock.mock.beeceptor.com/oauth/token/google"
        );
        assert_eq!(
            mock_token_url("github"),
            "https://oauth-mock.mock.beeceptor.com/oauth/token/github"
        );
        assert_eq!(
            mock_token_url("wechat"),
            "https://oauth-mock.mock.beeceptor.com/oauth/token/wechat"
        );
    }

    #[test]
    fn test_mock_userinfo_url() {
        assert_eq!(
            mock_userinfo_url("google"),
            "https://oauth-mock.mock.beeceptor.com/userinfo/google"
        );
        assert_eq!(
            mock_userinfo_url("github"),
            "https://oauth-mock.mock.beeceptor.com/userinfo/github"
        );
        assert_eq!(
            mock_userinfo_url("wechat"),
            "https://oauth-mock.mock.beeceptor.com/userinfo/wechat"
        );
    }
}
