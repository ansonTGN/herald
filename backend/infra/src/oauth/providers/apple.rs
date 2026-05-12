// Apple OAuth provider implementation (Sign in with Apple)

use herald_domain::common::entities::app_errors::CoreError;
use herald_domain::oauth::{
    entities::ProviderType,
    http_client::HttpClient,
    ports::OAuthProviderHandler,
    value_objects::{OAuthConfig, OAuthUserInfo},
};
use oauth2::{AuthUrl, ClientId, RedirectUrl, Scope, basic::BasicClient};
use serde::Deserialize;
use urlencoding::encode;

#[cfg(test)]
use serde::Serialize;

#[cfg(test)]
use std::time::{SystemTime, UNIX_EPOCH};

pub struct AppleOAuthProvider;

impl AppleOAuthProvider {
    const AUTH_URL: &'static str = "https://appleid.apple.com/auth/authorize";
    const TOKEN_URL: &'static str = "https://appleid.apple.com/auth/token";
}

// Apple's token response
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct AppleTokenResponse {
    access_token: String,
    token_type: String,
    expires_in: u64,
    refresh_token: Option<String>,
    id_token: String,
}

// JWT claims for Apple client secret
#[cfg(test)]
#[derive(Debug, Serialize)]
struct AppleClientSecretClaims {
    iss: String, // team_id
    sub: String, // client_id
    aud: String, // "https://appleid.apple.com"
    iat: u64,
    exp: u64,
}

// Decoded ID token claims
#[derive(Debug, Deserialize)]
struct AppleIdTokenClaims {
    sub: String, // Apple's unique user ID
    email: Option<String>,
    email_verified: Option<String>,
}

impl OAuthProviderHandler for AppleOAuthProvider {
    fn provider_type(&self) -> &'static str {
        "apple"
    }

    fn display_name(&self) -> &'static str {
        "Apple"
    }

    fn get_auth_url(&self, state: &str, config: &OAuthConfig) -> Result<String, CoreError> {
        let client = BasicClient::new(ClientId::new(config.client_id.clone()))
            .set_auth_uri(AuthUrl::new(Self::AUTH_URL.to_string())?)
            .set_redirect_uri(RedirectUrl::new(config.redirect_uri.clone())?);

        let (auth_url, _csrf_token) = client
            .authorize_url(|| oauth2::CsrfToken::new(state.to_string()))
            .add_scopes([
                Scope::new("name".to_string()),
                Scope::new("email".to_string()),
            ])
            .set_response_type(&oauth2::ResponseType::new("code id_token".to_string()))
            .url();

        Ok(auth_url.to_string())
    }

    #[allow(clippy::manual_async_fn)]
    fn exchange_code_and_get_user<H>(
        &self,
        code: String,
        config: &OAuthConfig,
        http_client: &H,
    ) -> impl Future<Output = Result<OAuthUserInfo, CoreError>> + Send
    where
        H: HttpClient + Send + Sync,
    {
        async move {
            // Note: For Apple, you need to pass AppleOAuthConfig instead of OAuthConfig
            // This is a simplified implementation that requires the client_secret parameter
            // to be generated externally using JWT with the private key

            // Exchange code for tokens
            let token_url = format!(
                "{}?client_id={}&client_secret={}&code={}&grant_type=authorization_code&redirect_uri={}",
                Self::TOKEN_URL,
                encode(&config.client_id),
                encode(&config.client_secret),
                encode(&code),
                encode(&config.redirect_uri)
            );

            let response = http_client
                .post(&token_url, Vec::new()) // Empty body for URL parameter requests
                .await?;

            if !response.is_success() {
                let error_text = response
                    .body_as_string()
                    .unwrap_or_else(|_| "Unknown error".to_string());
                return Err(CoreError::BadRequest(format!(
                    "Token exchange failed: {}",
                    error_text
                )));
            }

            let response_body = response.body_as_string()?;
            let token_response: AppleTokenResponse =
                serde_json::from_str(&response_body).map_err(|e| {
                    CoreError::InternalServerError(format!("Failed to parse token response: {}", e))
                })?;

            // Decode the id_token (JWT) to get user info
            // For production, you should verify the signature using Apple's public keys
            let id_token_parts: Vec<&str> = token_response.id_token.split('.').collect();
            if id_token_parts.len() != 3 {
                return Err(CoreError::InternalServerError(
                    "Invalid id_token format".to_string(),
                ));
            }

            // Decode payload (middle part of JWT)
            let payload = base64_url_decode(id_token_parts[1]);
            let claims: AppleIdTokenClaims = serde_json::from_slice(&payload).map_err(|e| {
                CoreError::InternalServerError(format!("Failed to decode id_token: {}", e))
            })?;

            let email = claims.email.ok_or_else(|| {
                CoreError::BadRequest(
                    "Email not provided by Apple. User may have chosen to hide email.".to_string(),
                )
            })?;

            let verified = claims.email_verified.as_deref() == Some("true");

            Ok(OAuthUserInfo {
                provider_type: ProviderType::Apple,
                provider_user_id: claims.sub.clone(),
                email,
                verified,
                avatar: None,   // Apple doesn't provide avatar
                name: None, // Name is only provided on first login, would need to handle user session
                union_id: None, // Apple doesn't provide UnionID
                open_id: Some(claims.sub),
            })
        }
    }
}

// Helper function for base64 URL decoding
fn base64_url_decode(input: &str) -> Vec<u8> {
    use base64::Engine;

    let mut input = input.replace('-', "+").replace('_', "/");
    while !input.len().is_multiple_of(4) {
        input.push('=');
    }
    base64::engine::general_purpose::STANDARD
        .decode(&input)
        .unwrap_or_else(|_| Vec::new())
}

// Helper to generate Apple client secret JWT (if needed)
// Note: This requires the 'jsonwebtoken' crate and p8 private key
#[cfg(test)] // 保留用于测试环境
pub fn generate_apple_client_secret(
    client_id: &str,
    team_id: &str,
    key_id: &str,
    private_key_pem: &str,
) -> Result<String, CoreError> {
    use jsonwebtoken::{EncodingKey, Header, encode};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| CoreError::InternalServerError(format!("Time error: {}", e)))?
        .as_secs();

    let claims = AppleClientSecretClaims {
        iss: team_id.to_string(),
        sub: client_id.to_string(),
        aud: "https://appleid.apple.com".to_string(),
        iat: now,
        exp: now + 86400, // 24 hours
    };

    let header = Header {
        kid: Some(key_id.to_string()),
        ..Default::default()
    };

    let encoding_key = EncodingKey::from_rsa_pem(private_key_pem.as_bytes())
        .map_err(|e| CoreError::InternalServerError(format!("Invalid private key: {}", e)))?;

    encode(&header, &claims, &encoding_key)
        .map_err(|e| CoreError::InternalServerError(format!("Failed to encode JWT: {}", e)))
}
