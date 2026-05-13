// Apple OAuth provider implementation (Sign in with Apple)

use herald_domain::common::entities::app_errors::CoreError;
use herald_domain::oauth::{
    entities::ProviderType,
    http_client::{HttpClient, HttpClientRequestBuilder, HttpMethod},
    ports::OAuthProviderHandler,
    value_objects::{OAuthConfig, OAuthUserInfo},
};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
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
    const JWKS_URL: &'static str = "https://appleid.apple.com/auth/keys";
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
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct AppleIdTokenClaims {
    sub: String, // Apple's unique user ID
    iss: String,
    aud: String,
    exp: u64,
    email: Option<String>,
    email_verified: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AppleJwks {
    keys: Vec<AppleJwk>,
}

#[derive(Debug, Deserialize)]
struct AppleJwk {
    kid: String,
    n: String,
    e: String,
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
            .add_extra_param("response_mode", "form_post")
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
            let token_body = format!(
                "client_id={}&client_secret={}&code={}&grant_type=authorization_code&redirect_uri={}",
                encode(&config.client_id),
                encode(&config.client_secret),
                encode(&code),
                encode(&config.redirect_uri)
            );

            let response = http_client
                .request(
                    HttpClientRequestBuilder::new(Self::TOKEN_URL, HttpMethod::Post)
                        .header("Content-Type", "application/x-www-form-urlencoded")
                        .header("Accept", "application/json")
                        .body(token_body.into_bytes())
                        .build(),
                )
                .await?;

            if !response.is_success() {
                let status_code = response.status_code;
                let error_text = response
                    .body_as_string()
                    .unwrap_or_else(|_| "Unknown error".to_string());
                return Err(CoreError::BadRequest(format!(
                    "Token exchange failed: status={}, body={}",
                    status_code, error_text
                )));
            }

            let response_body = response.body_as_string()?;
            let token_response: AppleTokenResponse =
                serde_json::from_str(&response_body).map_err(|e| {
                    CoreError::InternalServerError(format!("Failed to parse token response: {}", e))
                })?;

            let claims =
                verify_apple_id_token(&token_response.id_token, &config.client_id, http_client)
                    .await?;

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

async fn verify_apple_id_token<H>(
    id_token: &str,
    client_id: &str,
    http_client: &H,
) -> Result<AppleIdTokenClaims, CoreError>
where
    H: HttpClient + Send + Sync,
{
    let header = decode_header(id_token)
        .map_err(|e| CoreError::BadRequest(format!("Invalid Apple id_token header: {}", e)))?;

    let kid = header
        .kid
        .ok_or_else(|| CoreError::BadRequest("Apple id_token missing kid".to_string()))?;

    let keys_response = http_client.get(AppleOAuthProvider::JWKS_URL).await?;
    if !keys_response.is_success() {
        let status_code = keys_response.status_code;
        let response_body = keys_response.body_as_string().unwrap_or_default();
        return Err(CoreError::InternalServerError(format!(
            "Failed to get Apple public keys: status={}, body={}",
            status_code, response_body
        )));
    }

    let keys_body = keys_response.body_as_string()?;
    let jwks: AppleJwks = serde_json::from_str(&keys_body).map_err(|e| {
        CoreError::InternalServerError(format!("Failed to parse Apple public keys: {}", e))
    })?;

    let jwk = jwks
        .keys
        .into_iter()
        .find(|key| key.kid == kid)
        .ok_or_else(|| CoreError::BadRequest("No matching Apple public key".to_string()))?;

    let decoding_key = DecodingKey::from_rsa_components(&jwk.n, &jwk.e)
        .map_err(|e| CoreError::InternalServerError(format!("Invalid Apple public key: {}", e)))?;

    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[client_id]);
    validation.set_issuer(&["https://appleid.apple.com"]);

    let token_data = decode::<AppleIdTokenClaims>(id_token, &decoding_key, &validation)
        .map_err(|e| CoreError::BadRequest(format!("Invalid Apple id_token: {}", e)))?;

    Ok(token_data.claims)
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
