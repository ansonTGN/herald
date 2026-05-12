// Herald API OAuth Module
// OAuth provider authentication handlers

pub mod authorize;
pub mod callback;
pub mod config;
pub mod helper;
pub mod login;
pub mod wechat;
pub mod wechat_miniprogram;

pub use authorize::*;
pub use callback::*;
pub use config::*;
pub use login::*;
pub use wechat::*;
pub use wechat_miniprogram::*;

/// OpenAPI specification for OAuth module
#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
        crate::authorize::oauth_authorize,
        crate::login::oauth_login,
        crate::callback::oauth_callback,
        crate::config::list_oauth_configs,
        crate::config::create_oauth_config,
        crate::config::get_oauth_config,
        crate::config::update_oauth_config,
        crate::config::delete_oauth_config,
        crate::wechat::wechat_login,
        crate::wechat::wechat_callback,
        crate::wechat_miniprogram::wechat_miniprogram_login,
    ),
    components(schemas(
        crate::login::OAuthLoginRequest,
        crate::login::OAuthLoginResponse,
        crate::callback::OAuthCallbackResponse,
        crate::authorize::AuthorizeQueryParams,
        crate::config::CreateOAuthConfigRequest,
        crate::config::UpdateOAuthConfigRequest,
        crate::config::OAuthConfigResponse,
        crate::wechat::WeChatAuthUrlResponse,
        crate::wechat_miniprogram::WeChatMiniProgramLoginRequest,
        crate::wechat_miniprogram::WeChatMiniProgramLoginResponse,
    ))
)]
pub struct ApiDoc;
