// Google OAuth provider implementation

use herald_domain::common::entities::app_errors::CoreError;
use herald_domain::oauth::{
    entities::ProviderType,
    http_client::HttpClient,
    ports::OAuthProviderHandler,
    value_objects::{OAuthConfig, OAuthUserInfo},
};
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

            let _access_token = token_result.access_token().secret();

            // Get user info using the HTTP client abstraction
            let response = http_client.get(Self::USER_INFO_URL).await?;

            if !response.is_success() {
                return Err(CoreError::InternalServerError(
                    "Failed to get user info from Google".to_string(),
                ));
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
