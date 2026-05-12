// GitHub OAuth provider implementation

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

pub struct GitHubOAuthProvider;

impl GitHubOAuthProvider {
    const AUTH_URL: &'static str = "https://github.com/login/oauth/authorize";
    const TOKEN_URL: &'static str = "https://github.com/login/oauth/access_token";
    const USER_API_URL: &'static str = "https://api.github.com/user";
    const USER_EMAILS_URL: &'static str = "https://api.github.com/user/emails";
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct GitHubUser {
    id: i64,
    email: Option<String>,
    login: String,
    avatar_url: Option<String>,
    name: Option<String>,
}

#[derive(Deserialize)]
struct GitHubEmail {
    email: String,
    primary: bool,
    verified: bool,
}

impl OAuthProviderHandler for GitHubOAuthProvider {
    fn provider_type(&self) -> &'static str {
        "github"
    }

    fn display_name(&self) -> &'static str {
        "GitHub"
    }

    fn get_auth_url(&self, state: &str, config: &OAuthConfig) -> Result<String, CoreError> {
        let client = BasicClient::new(ClientId::new(config.client_id.clone()))
            .set_client_secret(ClientSecret::new(config.client_secret.clone()))
            .set_auth_uri(AuthUrl::new(Self::AUTH_URL.to_string())?)
            .set_token_uri(TokenUrl::new(Self::TOKEN_URL.to_string())?)
            .set_redirect_uri(RedirectUrl::new(config.redirect_uri.clone())?);

        let (auth_url, _csrf_token) = client
            .authorize_url(|| oauth2::CsrfToken::new(state.to_string()))
            .add_scopes([Scope::new("user:email".to_string())])
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
            let user_response = http_client.get(Self::USER_API_URL).await?;

            if !user_response.is_success() {
                return Err(CoreError::InternalServerError(
                    "Failed to get user info from GitHub".to_string(),
                ));
            }

            let response_body = user_response.body_as_string()?;
            let github_user: GitHubUser = serde_json::from_str(&response_body).map_err(|e| {
                CoreError::InternalServerError(format!("Failed to parse user info: {}", e))
            })?;

            // Get email if not provided in user info
            let (email, verified) = if let Some(email) = github_user.email {
                (email, true) // Email from user API is primary email
            } else {
                // Fetch emails separately
                let emails_response = http_client.get(Self::USER_EMAILS_URL).await?;

                let response_body = emails_response.body_as_string()?;
                let emails: Vec<GitHubEmail> =
                    serde_json::from_str(&response_body).map_err(|e| {
                        CoreError::InternalServerError(format!("Failed to parse emails: {}", e))
                    })?;

                let primary_email = emails.iter().find(|e| e.primary).or_else(|| emails.first());

                match primary_email {
                    Some(email) => (email.email.clone(), email.verified),
                    None => return Err(CoreError::BadRequest("No email found".to_string())),
                }
            };

            Ok(OAuthUserInfo {
                provider_type: ProviderType::GitHub,
                provider_user_id: github_user.id.to_string(),
                email,
                verified,
                avatar: github_user.avatar_url,
                name: github_user.name,
                union_id: None, // GitHub doesn't provide UnionID
                open_id: Some(github_user.id.to_string()),
            })
        }
    }
}
