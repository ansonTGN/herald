// WeChat OAuth provider implementation (website application login)

use herald_domain::common::entities::app_errors::CoreError;
use herald_domain::oauth::{
    entities::ProviderType,
    http_client::HttpClient,
    ports::OAuthProviderHandler,
    value_objects::{OAuthConfig, OAuthUserInfo},
};
use reqwest::Url;
use serde::Deserialize;
use urlencoding::encode;

pub struct WeChatOAuthProvider;

impl WeChatOAuthProvider {
    pub const AUTH_URL: &'static str = "https://open.weixin.qq.com/connect/qrconnect";
    pub const TOKEN_URL: &'static str = "https://api.weixin.qq.com/sns/oauth2/access_token";
    pub const USER_INFO_URL: &'static str = "https://api.weixin.qq.com/sns/userinfo";
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct WeChatTokenResponse {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: String,
    pub expires_in: i64,
    pub openid: String,
    pub scope: String,
    pub unionid: Option<String>,
    // Error fields - only present when API returns error
    #[serde(default)]
    pub errcode: Option<i32>,
    #[serde(default)]
    pub errmsg: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct WeChatUserInfo {
    pub openid: String,
    pub nickname: String,
    pub sex: i32,
    pub province: String,
    pub city: String,
    pub country: String,
    pub headimgurl: String,
    pub privilege: Vec<String>,
    pub unionid: Option<String>,
}

impl OAuthProviderHandler for WeChatOAuthProvider {
    fn provider_type(&self) -> &'static str {
        "wechat"
    }

    fn display_name(&self) -> &'static str {
        "WeChat"
    }

    fn get_auth_url(&self, state: &str, config: &OAuthConfig) -> Result<String, CoreError> {
        // WeChat QRconnect API doesn't follow OAuth2 standard exactly
        // We construct the URL manually using the url crate
        let mut url = Url::parse(Self::AUTH_URL)?;

        url.query_pairs_mut()
            .append_pair("appid", &config.client_id)
            .append_pair("redirect_uri", &config.redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", "snsapi_login")
            .append_pair("state", state);

        Ok(format!("{}#wechat_redirect", url))
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
            // Step 1: Exchange code for access_token
            let token_url = format!(
                "{}?appid={}&secret={}&code={}&grant_type=authorization_code",
                Self::TOKEN_URL,
                encode(&config.client_id),
                encode(&config.client_secret),
                encode(&code)
            );

            let token_response = http_client.get(&token_url).await?;

            if !token_response.is_success() {
                return Err(CoreError::InternalServerError(
                    "Token request failed".to_string(),
                ));
            }

            let response_body = token_response.body_as_string()?;
            let token_data: WeChatTokenResponse =
                serde_json::from_str(&response_body).map_err(|e| {
                    CoreError::InternalServerError(format!("Failed to parse token response: {}", e))
                })?;

            // Check for WeChat API errors
            if let Some(errcode) = token_data.errcode {
                let error_msg = token_data
                    .errmsg
                    .unwrap_or_else(|| "Unknown error".to_string());
                return Err(match errcode {
                    -1 => {
                        CoreError::InternalServerError(format!("WeChat API error: {}", error_msg))
                    }
                    40001 => CoreError::BadRequest("Invalid appsecret".to_string()),
                    40029 => CoreError::BadRequest("Invalid code".to_string()),
                    40163 => CoreError::BadRequest("Code has been used".to_string()),
                    42002 => CoreError::BadRequest("Code expired".to_string()),
                    _ => CoreError::BadRequest(format!("WeChat error {}: {}", errcode, error_msg)),
                });
            }

            // Step 2: Get user info using access_token
            let user_info_url = format!(
                "{}?access_token={}&openid={}&lang=zh_CN",
                Self::USER_INFO_URL,
                encode(&token_data.access_token),
                encode(&token_data.openid)
            );

            let user_response = http_client.get(&user_info_url).await?;

            if !user_response.is_success() {
                return Err(CoreError::InternalServerError(
                    "Failed to get user info from WeChat".to_string(),
                ));
            }

            let response_body = user_response.body_as_string()?;
            let user_data: WeChatUserInfo = serde_json::from_str(&response_body).map_err(|e| {
                CoreError::InternalServerError(format!("Failed to parse user info: {}", e))
            })?;

            // Generate placeholder email (WeChat doesn't provide real email)
            // Priority: unionid > openid
            let id_for_email = user_data
                .unionid
                .as_ref()
                .or(Some(&token_data.openid))
                .ok_or_else(|| CoreError::InternalServerError("Missing openid".to_string()))?;

            let placeholder_email = format!("{}@wechat.placeholder", id_for_email);

            Ok(OAuthUserInfo {
                provider_type: ProviderType::WeChat,
                provider_user_id: user_data.openid.clone(),
                email: placeholder_email,
                verified: false, // Placeholder email is not verified
                avatar: Some(user_data.headimgurl),
                name: Some(user_data.nickname),
                union_id: user_data.unionid,
                open_id: Some(user_data.openid),
            })
        }
    }
}
