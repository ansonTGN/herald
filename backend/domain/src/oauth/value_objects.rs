// OAuth value objects

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::oauth::entities::ProviderType;

/// OAuth user information from provider
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct OAuthUserInfo {
    pub provider_type: ProviderType,
    pub provider_user_id: String,
    pub email: String,
    pub verified: bool,
    pub avatar: Option<String>,
    pub name: Option<String>,
    pub union_id: Option<String>,
    pub open_id: Option<String>,
}

/// OAuth state stored in cookie
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OAuthState {
    /// Expiration timestamp
    pub exp: i64,
    /// CSRF token
    #[serde(rename = "csrf")]
    pub csrf_secret: String,
    /// PKCE code verifier
    #[serde(rename = "verifier")]
    pub pkce_code_verifier: String,
    /// Redirect URI after OAuth flow
    pub redirect_uri: Option<String>,
    /// Realm ID
    pub realm_id: Option<String>,
}

/// OAuth configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    #[serde(default)]
    pub scopes: Vec<String>,
}

/// Provider-specific OAuth configurations
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OAuthProviderConfigs {
    #[serde(default)]
    pub google: Option<OAuthConfig>,
    #[serde(default)]
    pub github: Option<OAuthConfig>,
    #[serde(default)]
    pub facebook: Option<OAuthConfig>,
    #[serde(default)]
    pub apple: Option<AppleOAuthConfig>,
}

/// Apple OAuth specific configuration (requires JWT signing)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppleOAuthConfig {
    pub client_id: String,
    pub key_id: String,
    pub team_id: String,
    pub private_key: String,
    pub redirect_uri: String,
}
