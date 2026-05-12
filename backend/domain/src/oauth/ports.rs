// OAuth ports and traits

use std::future::Future;
use uuid::Uuid;

use crate::common::entities::app_errors::CoreError;
use crate::oauth::entities::{
    OAuthProvider, OAuthProviderConfig, UpdateOAuthProviderConfigRequest,
};
use crate::oauth::http_client::HttpClient;
use crate::oauth::value_objects::{OAuthConfig, OAuthProviderConfigs, OAuthUserInfo};

/// OAuth provider trait
pub trait OAuthProviderHandler: Send + Sync {
    /// Get the provider type name
    fn provider_type(&self) -> &'static str;

    /// Get display name
    fn display_name(&self) -> &'static str;

    /// Get authorization URL
    fn get_auth_url(&self, state: &str, config: &OAuthConfig) -> Result<String, CoreError>;

    /// Exchange code for access token and get user info
    fn exchange_code_and_get_user<H>(
        &self,
        code: String,
        config: &OAuthConfig,
        http_client: &H,
    ) -> impl Future<Output = Result<OAuthUserInfo, CoreError>> + Send
    where
        H: HttpClient + Send + Sync;
}

/// OAuth repository trait (for user-linked OAuth accounts)
pub trait OAuthRepository: Send + Sync {
    fn find_by_provider_and_open_id(
        &self,
        realm_id: &str,
        provider_type: &str,
        open_id: &str,
    ) -> impl Future<Output = Result<OAuthProvider, CoreError>> + Send;

    fn find_by_union_id(
        &self,
        realm_id: &str,
        union_id: &str,
    ) -> impl Future<Output = Result<OAuthProvider, CoreError>> + Send;

    fn find_by_user_id(
        &self,
        user_id: Uuid,
    ) -> impl Future<Output = Result<Vec<OAuthProvider>, CoreError>> + Send;

    fn create_provider(
        &self,
        provider: OAuthProvider,
    ) -> impl Future<Output = Result<OAuthProvider, CoreError>> + Send;

    fn link_provider_to_user(
        &self,
        user_id: Uuid,
        provider_id: Uuid,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn unlink_provider_from_user(
        &self,
        user_id: Uuid,
        provider_id: Uuid,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;
}

/// OAuth configuration repository trait (for per-realm provider configs)
pub trait OAuthConfigRepository: Send + Sync {
    /// Get OAuth provider config by realm and provider type
    fn get_config(
        &self,
        realm_id: &str,
        provider_type: &str,
    ) -> impl Future<Output = Result<OAuthProviderConfig, CoreError>> + Send;

    /// Get OAuth provider config by ID
    fn get_config_by_id(
        &self,
        id: Uuid,
    ) -> impl Future<Output = Result<OAuthProviderConfig, CoreError>> + Send;

    /// List all OAuth provider configs for a realm
    fn list_configs(
        &self,
        realm_id: &str,
    ) -> impl Future<Output = Result<Vec<OAuthProviderConfig>, CoreError>> + Send;

    /// List only enabled OAuth provider configs for a realm
    fn list_enabled_configs(
        &self,
        realm_id: &str,
    ) -> impl Future<Output = Result<Vec<OAuthProviderConfig>, CoreError>> + Send;

    /// Create OAuth provider config
    fn create_config(
        &self,
        config: OAuthProviderConfig,
    ) -> impl Future<Output = Result<OAuthProviderConfig, CoreError>> + Send;

    /// Update OAuth provider config
    fn update_config(
        &self,
        id: Uuid,
        request: UpdateOAuthProviderConfigRequest,
    ) -> impl Future<Output = Result<OAuthProviderConfig, CoreError>> + Send;

    /// Delete OAuth provider config
    fn delete_config(&self, id: Uuid) -> impl Future<Output = Result<(), CoreError>> + Send;
}

/// OAuth configuration trait (legacy, for file-based config)
pub trait OAuthConfigService: Send + Sync {
    fn get_provider_configs(&self) -> &OAuthProviderConfigs;

    fn get_provider_config(&self, provider_type: &str) -> Option<OAuthConfig>;
}
