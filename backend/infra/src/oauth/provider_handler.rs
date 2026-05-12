// OAuth Provider Handler - concrete provider dispatch
// Moved from domain/oauth/services.rs to eliminate domain -> infrastructure dependency

use crate::oauth::providers::{
    apple::AppleOAuthProvider, facebook::FacebookOAuthProvider, github::GitHubOAuthProvider,
    google::GoogleOAuthProvider, wechat::WeChatOAuthProvider,
    wechat_miniprogram::WeChatMiniProgramProvider,
};
use herald_domain::common::entities::app_errors::CoreError;
use herald_domain::oauth::{
    http_client::HttpClient,
    ports::OAuthProviderHandler,
    value_objects::{OAuthConfig, OAuthUserInfo},
};

/// Enum to represent different OAuth providers (avoids dyn compatibility issues)
///
/// NOTE: Uses concrete infrastructure provider types instead of domain trait abstractions.
/// This is a technical compromise because:
/// 1. Provider implementations contain external client dependencies (reqwest, oauth2)
/// 2. Moving provider logic to domain layer would violate dependency rules
/// 3. Using dyn traits would complicate the async handling
pub enum ProviderHandler {
    Google(GoogleOAuthProvider),
    GitHub(GitHubOAuthProvider),
    Facebook(FacebookOAuthProvider),
    Apple(AppleOAuthProvider),
    WeChat(WeChatOAuthProvider),
    WeChatMiniProgram(WeChatMiniProgramProvider),
}

// Implement OAuthProviderHandler for the enum by delegating to each variant
impl OAuthProviderHandler for ProviderHandler {
    fn provider_type(&self) -> &'static str {
        match self {
            ProviderHandler::Google(p) => p.provider_type(),
            ProviderHandler::GitHub(p) => p.provider_type(),
            ProviderHandler::Facebook(p) => p.provider_type(),
            ProviderHandler::Apple(p) => p.provider_type(),
            ProviderHandler::WeChat(p) => p.provider_type(),
            ProviderHandler::WeChatMiniProgram(p) => p.provider_type(),
        }
    }

    fn display_name(&self) -> &'static str {
        match self {
            ProviderHandler::Google(p) => p.display_name(),
            ProviderHandler::GitHub(p) => p.display_name(),
            ProviderHandler::Facebook(p) => p.display_name(),
            ProviderHandler::Apple(p) => p.display_name(),
            ProviderHandler::WeChat(p) => p.display_name(),
            ProviderHandler::WeChatMiniProgram(p) => p.display_name(),
        }
    }

    fn get_auth_url(&self, state: &str, config: &OAuthConfig) -> Result<String, CoreError> {
        match self {
            ProviderHandler::Google(p) => p.get_auth_url(state, config),
            ProviderHandler::GitHub(p) => p.get_auth_url(state, config),
            ProviderHandler::Facebook(p) => p.get_auth_url(state, config),
            ProviderHandler::Apple(p) => p.get_auth_url(state, config),
            ProviderHandler::WeChat(p) => p.get_auth_url(state, config),
            ProviderHandler::WeChatMiniProgram(p) => p.get_auth_url(state, config),
        }
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
        async fn execute<H>(
            handler: &ProviderHandler,
            code: String,
            config: &OAuthConfig,
            http_client: &H,
        ) -> Result<OAuthUserInfo, CoreError>
        where
            H: HttpClient + Send + Sync,
        {
            match handler {
                ProviderHandler::Google(p) => {
                    p.exchange_code_and_get_user(code, config, http_client)
                        .await
                }
                ProviderHandler::GitHub(p) => {
                    p.exchange_code_and_get_user(code, config, http_client)
                        .await
                }
                ProviderHandler::Facebook(p) => {
                    p.exchange_code_and_get_user(code, config, http_client)
                        .await
                }
                ProviderHandler::Apple(p) => {
                    p.exchange_code_and_get_user(code, config, http_client)
                        .await
                }
                ProviderHandler::WeChat(p) => {
                    p.exchange_code_and_get_user(code, config, http_client)
                        .await
                }
                ProviderHandler::WeChatMiniProgram(p) => {
                    p.exchange_code_and_get_user(code, config, http_client)
                        .await
                }
            }
        }
        async move { execute(self, code, config, http_client).await }
    }
}

/// Create a provider handler by type string
pub fn create_provider_handler(provider_type: &str) -> Result<ProviderHandler, CoreError> {
    match provider_type {
        "google" => Ok(ProviderHandler::Google(GoogleOAuthProvider)),
        "github" => Ok(ProviderHandler::GitHub(GitHubOAuthProvider)),
        "facebook" => Ok(ProviderHandler::Facebook(FacebookOAuthProvider)),
        "apple" => Ok(ProviderHandler::Apple(AppleOAuthProvider)),
        "wechat" => Ok(ProviderHandler::WeChat(WeChatOAuthProvider)),
        "wechat_miniprogram" => Ok(ProviderHandler::WeChatMiniProgram(
            WeChatMiniProgramProvider,
        )),
        _ => Err(CoreError::BadRequest(format!(
            "Unsupported OAuth provider: {}",
            provider_type
        ))),
    }
}
