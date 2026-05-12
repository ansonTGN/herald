// Facebook OAuth provider implementation

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

pub struct FacebookOAuthProvider;

impl FacebookOAuthProvider {
    const AUTH_URL: &'static str = "https://www.facebook.com/v18.0/dialog/oauth";
    const TOKEN_URL: &'static str = "https://graph.facebook.com/v18.0/oauth/access_token";
    const USER_API_URL: &'static str = "https://graph.facebook.com/me";
}

#[derive(Deserialize)]
struct FacebookPictureData {
    url: String,
}

#[derive(Deserialize)]
struct FacebookPicture {
    data: FacebookPictureData,
}

#[derive(Deserialize)]
struct FacebookUser {
    id: String,
    email: String,
    name: Option<String>,
    picture: Option<FacebookPicture>,
}

impl OAuthProviderHandler for FacebookOAuthProvider {
    fn provider_type(&self) -> &'static str {
        "facebook"
    }

    fn display_name(&self) -> &'static str {
        "Facebook"
    }

    fn get_auth_url(&self, state: &str, config: &OAuthConfig) -> Result<String, CoreError> {
        let client = BasicClient::new(ClientId::new(config.client_id.clone()))
            .set_client_secret(ClientSecret::new(config.client_secret.clone()))
            .set_auth_uri(AuthUrl::new(Self::AUTH_URL.to_string())?)
            .set_token_uri(TokenUrl::new(Self::TOKEN_URL.to_string())?)
            .set_redirect_uri(RedirectUrl::new(config.redirect_uri.clone())?);

        let (auth_url, _csrf_token) = client
            .authorize_url(|| oauth2::CsrfToken::new(state.to_string()))
            .add_scopes([Scope::new("email".to_string())])
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
            let user_info_url = format!("{}?fields=id,email,name,picture", Self::USER_API_URL);

            let response = http_client.get(&user_info_url).await?;

            if !response.is_success() {
                return Err(CoreError::InternalServerError(
                    "Failed to get user info from Facebook".to_string(),
                ));
            }

            let response_body = response.body_as_string()?;
            let facebook_user: FacebookUser =
                serde_json::from_str(&response_body).map_err(|e| {
                    CoreError::InternalServerError(format!("Failed to parse user info: {}", e))
                })?;

            Ok(OAuthUserInfo {
                provider_type: ProviderType::Facebook,
                provider_user_id: facebook_user.id.clone(),
                email: facebook_user.email,
                verified: true, // Facebook OAuth provides verified emails
                avatar: facebook_user.picture.map(|p| p.data.url),
                name: facebook_user.name,
                union_id: None, // Facebook doesn't provide UnionID
                open_id: Some(facebook_user.id),
            })
        }
    }
}
