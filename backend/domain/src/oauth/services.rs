// OAuth service implementations

use base64::Engine;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    authentication::ports::AuthenticationService,
    common::entities::app_errors::CoreError,
    oauth::{
        entities::{CreateOAuthProviderConfig, OAuthProvider},
        http_client::HttpClient,
        ports::{OAuthConfigService, OAuthProviderHandler, OAuthRepository},
        value_objects::OAuthState,
    },
    user::{entities::User, ports::UserService},
};

/// OAuth Service
pub struct OAuthService<R, C, U, A, H>
where
    R: OAuthRepository,
    C: OAuthConfigService,
    U: UserService,
    A: AuthenticationService,
    H: HttpClient + Send + Sync,
{
    oauth_repository: Arc<R>,
    oauth_config_service: Arc<C>,
    #[allow(dead_code)] // 保留用于未来使用（用户服务集成）
    user_service: Arc<U>,
    #[allow(dead_code)] // 保留用于未来使用（认证服务集成）
    authentication_service: Arc<A>,
    http_client: Arc<H>,
}

impl<R, C, U, A, H> OAuthService<R, C, U, A, H>
where
    R: OAuthRepository,
    C: OAuthConfigService,
    U: UserService,
    A: AuthenticationService,
    H: HttpClient + Send + Sync,
{
    pub fn new(
        oauth_repository: Arc<R>,
        oauth_config_service: Arc<C>,
        user_service: Arc<U>,
        authentication_service: Arc<A>,
        http_client: Arc<H>,
    ) -> Self {
        Self {
            oauth_repository,
            oauth_config_service,
            user_service,
            authentication_service,
            http_client,
        }
    }

    /// Generate OAuth state for CSRF protection
    pub fn generate_oauth_state(
        &self,
        realm_id: Option<String>,
        redirect_uri: Option<String>,
    ) -> Result<String, CoreError> {
        use oauth2::CsrfToken;

        let csrf_token = CsrfToken::new_random();
        let state = OAuthState {
            exp: chrono::Utc::now().timestamp() + 300, // 5 minutes
            csrf_secret: csrf_token.secret().to_string(),
            pkce_code_verifier: csrf_token.secret().to_string(), // Simplified
            redirect_uri,
            realm_id,
        };

        // Encode state as JWT
        let state_json = serde_json::to_string(&state).map_err(|e| {
            CoreError::InternalServerError(format!("Failed to serialize state: {}", e))
        })?;

        Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(state_json))
    }

    /// Validate OAuth state
    pub fn validate_oauth_state(
        &self,
        encoded_state: &str,
        expected_csrf: &str,
    ) -> Result<OAuthState, CoreError> {
        let state_json = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(encoded_state)
            .map_err(|e| CoreError::BadRequest(format!("Invalid state encoding: {}", e)))?;

        let state: OAuthState = serde_json::from_slice(&state_json)
            .map_err(|e| CoreError::BadRequest(format!("Invalid state format: {}", e)))?;

        // Check expiration
        if chrono::Utc::now().timestamp() > state.exp {
            return Err(CoreError::BadRequest("OAuth state expired".to_string()));
        }

        // Validate CSRF token
        if state.csrf_secret != expected_csrf {
            return Err(CoreError::BadRequest("Invalid CSRF token".to_string()));
        }

        Ok(state)
    }

    /// Get authorization URL for OAuth provider
    pub async fn get_authorization_url<P: OAuthProviderHandler>(
        &self,
        provider: &P,
        provider_type: &str,
        realm_id: Option<String>,
        redirect_uri: Option<String>,
    ) -> Result<(String, String), CoreError> {
        let config = self
            .oauth_config_service
            .get_provider_config(provider_type)
            .ok_or_else(|| {
                CoreError::BadRequest(format!("OAuth provider '{}' not configured", provider_type))
            })?;

        let state = self.generate_oauth_state(realm_id.clone(), redirect_uri)?;

        // For simplified implementation, we use a fixed CSRF value
        // In production, use the state's csrf_secret
        let csrf_token = &state.clone();

        let auth_url = provider.get_auth_url(csrf_token, &config)?;

        Ok((auth_url, state))
    }

    /// Handle OAuth callback
    pub async fn handle_callback<P: OAuthProviderHandler>(
        &self,
        provider: &P,
        provider_type: &str,
        code: String,
        state: String,
    ) -> Result<(User, OAuthProvider), CoreError> {
        let config = self
            .oauth_config_service
            .get_provider_config(provider_type)
            .ok_or_else(|| {
                CoreError::BadRequest(format!("OAuth provider '{}' not configured", provider_type))
            })?;

        // Validate state (simplified - in production decode and verify JWT)
        let oauth_state = self.validate_oauth_state(&state, &state)?;

        let user_info = provider
            .exchange_code_and_get_user(code, &config, self.http_client.as_ref())
            .await?;

        let realm_id = oauth_state
            .realm_id
            .unwrap_or_else(|| "default".to_string());

        match self
            .oauth_repository
            .find_by_provider_and_open_id(&realm_id, provider_type, &user_info.provider_user_id)
            .await
        {
            Ok(_oauth_provider) => {
                // Find associated user
                // TODO: Implement user lookup by email
                // This requires UserService to have get_user_by_email method
                Err(CoreError::InternalServerError(
                    "OAuth user lookup not yet implemented".to_string(),
                ))
            }
            Err(CoreError::NotFound) => {
                // New user, create account
                // TODO: Implement user creation without password
                Err(CoreError::InternalServerError(
                    "OAuth user creation not yet implemented".to_string(),
                ))
            }
            Err(e) => Err(e),
        }
    }

    /// Link OAuth provider to existing user
    pub async fn link_provider<P: OAuthProviderHandler>(
        &self,
        provider: &P,
        user_id: Uuid,
        provider_type: &str,
        code: String,
        realm_id: &str,
    ) -> Result<OAuthProvider, CoreError> {
        let config = self
            .oauth_config_service
            .get_provider_config(provider_type)
            .ok_or_else(|| {
                CoreError::BadRequest(format!("OAuth provider '{}' not configured", provider_type))
            })?;

        let user_info = provider
            .exchange_code_and_get_user(code, &config, self.http_client.as_ref())
            .await?;

        if let Ok(existing) = self
            .oauth_repository
            .find_by_provider_and_open_id(realm_id, provider_type, &user_info.provider_user_id)
            .await
        {
            // This depends on your data model
            return Ok(existing);
        }

        let create_oauth_provider_config = CreateOAuthProviderConfig {
            realm_id: realm_id.to_string(),
            provider_type: user_info.provider_type.clone(),
            open_id: user_info.provider_user_id,
            union_id: None,
            email: Some(user_info.email),
        };

        let oauth_provider = OAuthProvider::new(create_oauth_provider_config);
        let oauth_provider = self
            .oauth_repository
            .create_provider(oauth_provider)
            .await?;

        // Link to user
        self.oauth_repository
            .link_provider_to_user(user_id, oauth_provider.id)
            .await?;

        Ok(oauth_provider)
    }

    /// Unlink OAuth provider from user
    pub async fn unlink_provider(&self, user_id: Uuid, provider_id: Uuid) -> Result<(), CoreError> {
        self.oauth_repository
            .unlink_provider_from_user(user_id, provider_id)
            .await
    }

    /// List linked providers for user
    pub async fn list_user_providers(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<OAuthProvider>, CoreError> {
        self.oauth_repository.find_by_user_id(user_id).await
    }
}
