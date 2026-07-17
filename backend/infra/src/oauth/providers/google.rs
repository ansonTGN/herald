// Google OAuth provider implementation

use herald_domain::common::entities::app_errors::CoreError;
use herald_domain::oauth::{
    entities::ProviderType,
    http_client::{HttpClient, HttpClientRequestBuilder, HttpMethod},
    ports::OAuthProviderHandler,
    value_objects::{OAuthConfig, OAuthUserInfo},
};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, RedirectUrl, Scope, TokenResponse,
    TokenUrl, basic::BasicClient,
};
use serde::Deserialize;

pub struct GoogleOAuthProvider;

impl GoogleOAuthProvider {
    const AUTH_URL: &'static str = "https://accounts.google.com/o/oauth2/auth";
    const TOKEN_URL: &'static str = "https://accounts.google.com/o/oauth2/token";
    const USER_INFO_URL: &'static str = "https://www.googleapis.com/oauth2/v1/userinfo";

    /// Production Google JWKS endpoint. Public so handlers in other crates
    /// can inject it as the `jwks_url` argument to
    /// [`verify_google_id_token`]. Tests inject a wiremock URL instead.
    pub const GOOGLE_JWKS_URL: &'static str = "https://www.googleapis.com/oauth2/v3/certs";

    /// Accepted Google ID Token issuers (Google OIDC serves both forms).
    const ISSUERS: [&'static str; 2] = ["accounts.google.com", "https://accounts.google.com"];
}

// `pub` (not `pub(crate)`) because this is the return type of the cross-crate
// `pub async fn verify_google_id_token`; `pub(crate)` would trigger E0446
// (private type in public interface).
#[derive(Debug, Deserialize)]
pub struct GoogleIdTokenClaims {
    pub sub: String,
    pub iss: String,
    /// Must equal the Realm's Google `client_id`; enforced via `Validation::set_audience`.
    pub aud: String,
    pub exp: u64,
    pub email: Option<String>,
    /// Google may serialize this as `true`/`false` (bool) or `"true"`/`"false"` (string).
    pub email_verified: Option<StringOrBool>,
    pub name: Option<String>,
    pub picture: Option<String>,
}

/// Untagged enum to tolerate Google's `email_verified` being either a bool or
/// a string. Public because the handler pattern-matches its variants across
/// crates.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum StringOrBool {
    Bool(bool),
    Str(String),
}

// JWKS response types; private, only used inside `verify_google_id_token`.
#[derive(Debug, Deserialize)]
struct GoogleJwks {
    keys: Vec<GoogleJwk>,
}

#[derive(Debug, Deserialize)]
struct GoogleJwk {
    kid: String,
    n: String,
    e: String,
}

#[derive(Deserialize)]
struct GoogleUserInfo {
    id: String,
    email: String,
    verified_email: bool,
    picture: Option<String>,
    name: Option<String>,
}

impl OAuthProviderHandler for GoogleOAuthProvider {
    fn provider_type(&self) -> &'static str {
        "google"
    }

    fn display_name(&self) -> &'static str {
        "Google"
    }

    fn get_auth_url(&self, state: &str, config: &OAuthConfig) -> Result<String, CoreError> {
        let client = BasicClient::new(ClientId::new(config.client_id.clone()))
            .set_client_secret(ClientSecret::new(config.client_secret.clone()))
            .set_auth_uri(AuthUrl::new(Self::AUTH_URL.to_string())?)
            .set_token_uri(TokenUrl::new(Self::TOKEN_URL.to_string())?)
            .set_redirect_uri(RedirectUrl::new(config.redirect_uri.clone())?);

        let (auth_url, _csrf_token) = client
            .authorize_url(|| oauth2::CsrfToken::new(state.to_string()))
            .add_scopes([
                Scope::new("https://www.googleapis.com/auth/userinfo.profile".to_string()),
                Scope::new("https://www.googleapis.com/auth/userinfo.email".to_string()),
            ])
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
            let client = BasicClient::new(ClientId::new(config.client_id.clone()))
                .set_client_secret(ClientSecret::new(config.client_secret.clone()))
                .set_auth_uri(AuthUrl::new(Self::AUTH_URL.to_string())?)
                .set_token_uri(TokenUrl::new(Self::TOKEN_URL.to_string())?)
                .set_redirect_uri(RedirectUrl::new(config.redirect_uri.clone())?);

            let oauth_http_client = oauth2::reqwest::ClientBuilder::new()
                .redirect(oauth2::reqwest::redirect::Policy::none())
                .build()
                .map_err(|e| {
                    CoreError::InternalServerError(format!(
                        "Failed to build OAuth HTTP client: {}",
                        e
                    ))
                })?;

            let token_result = client
                .exchange_code(AuthorizationCode::new(code))
                .request_async(&oauth_http_client)
                .await
                .map_err(|e| CoreError::BadRequest(format!("Token exchange failed: {}", e)))?;

            let access_token = token_result.access_token().secret();

            // Get user info using the HTTP client abstraction
            let response = http_client
                .request(
                    HttpClientRequestBuilder::new(Self::USER_INFO_URL, HttpMethod::Get)
                        .bearer_auth(access_token)
                        .build(),
                )
                .await?;

            if !response.is_success() {
                let status_code = response.status_code;
                let response_body = response.body_as_string().unwrap_or_default();
                return Err(CoreError::InternalServerError(format!(
                    "Failed to get user info from Google: status={}, body={}",
                    status_code, response_body
                )));
            }

            let response_body = response.body_as_string()?;
            let user_info: GoogleUserInfo = serde_json::from_str(&response_body).map_err(|e| {
                CoreError::InternalServerError(format!("Failed to parse user info: {}", e))
            })?;

            if !user_info.verified_email {
                return Err(CoreError::BadRequest("Email not verified".to_string()));
            }

            Ok(OAuthUserInfo {
                provider_type: ProviderType::Google,
                provider_user_id: user_info.id.clone(),
                email: user_info.email,
                verified: user_info.verified_email,
                avatar: user_info.picture,
                name: user_info.name,
                union_id: None, // Google doesn't provide UnionID
                open_id: Some(user_info.id),
            })
        }
    }
}

/// Verify a Google ID Token (JWT) issued by Google One Tap / GIS.
///
/// Verifies signature (via Google JWKS), `aud` (must equal `client_id`),
/// `iss` (one of [`GoogleOAuthProvider::ISSUERS`]) and `exp`. Returns the
/// decoded claims on success. `email_verified` judgement is left to the
/// caller (handler) — this function does not hardcode a "reject unverified"
/// policy.
///
/// `jwks_url` is a parameter so tests can inject a wiremock URL; production
/// callers pass [`GoogleOAuthProvider::GOOGLE_JWKS_URL`].
pub async fn verify_google_id_token<H>(
    id_token: &str,
    client_id: &str,
    http_client: &H,
    jwks_url: &str,
) -> Result<GoogleIdTokenClaims, CoreError>
where
    H: HttpClient + Send + Sync,
{
    let header = decode_header(id_token)
        .map_err(|e| CoreError::BadRequest(format!("Invalid Google id_token header: {}", e)))?;

    let kid = header
        .kid
        .ok_or_else(|| CoreError::BadRequest("Google id_token missing kid".to_string()))?;

    let keys_response = http_client.get(jwks_url).await?;
    if !keys_response.is_success() {
        let status_code = keys_response.status_code;
        let response_body = keys_response.body_as_string().unwrap_or_default();
        return Err(CoreError::InternalServerError(format!(
            "Failed to get Google public keys: status={}, body={}",
            status_code, response_body
        )));
    }

    let keys_body = keys_response.body_as_string()?;
    let jwks: GoogleJwks = serde_json::from_str(&keys_body).map_err(|e| {
        CoreError::InternalServerError(format!("Failed to parse Google public keys: {}", e))
    })?;

    let jwk = jwks
        .keys
        .into_iter()
        .find(|key| key.kid == kid)
        .ok_or_else(|| CoreError::BadRequest("No matching Google public key".to_string()))?;

    let decoding_key = DecodingKey::from_rsa_components(&jwk.n, &jwk.e)
        .map_err(|e| CoreError::InternalServerError(format!("Invalid Google public key: {}", e)))?;

    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[client_id]);
    validation.set_issuer(&GoogleOAuthProvider::ISSUERS);

    let token_data = decode::<GoogleIdTokenClaims>(id_token, &decoding_key, &validation)
        .map_err(|e| CoreError::BadRequest(format!("Invalid Google id_token: {}", e)))?;

    Ok(token_data.claims)
}
