use chrono::{Duration, Utc};
use std::convert::TryFrom;
use std::sync::Arc;

use crate::security_constants::DEFAULT_SESSION_TTL_SECONDS;
use crate::user::value_objects::LoginRequest;
use crate::{
    authentication::{
        Identity,
        entities::{Session, SessionData},
        ports::{AuthenticationService, SessionRepository},
    },
    common::entities::app_errors::CoreError,
    user::ports::UserService,
};

pub struct AuthenticationServiceImpl<U, S>
where
    U: UserService,
    S: SessionRepository,
{
    user_service: Arc<U>,
    session_repository: Arc<S>,
}

impl<U, S> AuthenticationServiceImpl<U, S>
where
    U: UserService,
    S: SessionRepository,
{
    pub fn new(user_service: Arc<U>, session_repository: Arc<S>) -> Self {
        Self {
            user_service,
            session_repository,
        }
    }

    fn generate_token(realm_id: &str, user_id: &str) -> String {
        // Generate session token in format: {realm_id}_{user_id}_{timestamp}
        let timestamp = Utc::now().timestamp();
        format!("{}_{}_{}", realm_id, user_id, timestamp)
    }
}

impl<U, S> AuthenticationService for AuthenticationServiceImpl<U, S>
where
    U: UserService,
    S: SessionRepository,
{
    async fn login(
        &self,
        request: LoginRequest,
        client_id: String,
        client_ip: String,
    ) -> Result<(Session, Identity), CoreError> {
        // Authenticate user
        let user = self.user_service.login(request).await?;

        // Create session data first
        let session_data = SessionData {
            realm_id: user.realm_id.clone(),
            client_id: client_id.clone(),
            user_id: user.id.to_string(),
            client_ip,
        };

        // Generate session token
        let token = Self::generate_token(&session_data.realm_id, &session_data.user_id);
        let expires_at = Utc::now()
            + Duration::seconds(
                i64::try_from(DEFAULT_SESSION_TTL_SECONDS).unwrap_or_else(|_| {
                    tracing::error!(
                        "DEFAULT_SESSION_TTL_SECONDS {} exceeds i64::MAX",
                        DEFAULT_SESSION_TTL_SECONDS
                    );
                    i64::MAX // Use maximum value as fallback
                }),
            );

        // Store session
        self.session_repository
            .store_session(&token, session_data.clone(), DEFAULT_SESSION_TTL_SECONDS)
            .await?;

        // Create session and identity
        let session = Session::new(token.clone(), session_data.clone(), expires_at);
        let identity = Identity::User(user);

        Ok((session, identity))
    }

    /// Create a session for an authenticated user
    async fn create_session(
        &self,
        user: crate::user::entities::User,
        client_id: String,
        client_ip: String,
        ttl_seconds: u64,
    ) -> Result<(Session, Identity), CoreError> {
        // Create session data first
        let session_data = SessionData {
            realm_id: user.realm_id.clone(),
            client_id: client_id.clone(),
            user_id: user.id.to_string(),
            client_ip,
        };

        // Generate session token
        let token = Self::generate_token(&session_data.realm_id, &session_data.user_id);
        let expires_at = Utc::now()
            + Duration::seconds(i64::try_from(ttl_seconds).unwrap_or_else(|_| {
                tracing::error!("ttl_seconds {} exceeds i64::MAX", ttl_seconds);
                i64::MAX // Use maximum value as fallback
            }));

        // Store session
        self.session_repository
            .store_session(&token, session_data.clone(), ttl_seconds)
            .await?;

        // Create session and identity
        let session = Session::new(token.clone(), session_data.clone(), expires_at);
        let identity = Identity::User(user);

        Ok((session, identity))
    }

    async fn logout(&self, token: String) -> Result<(), CoreError> {
        self.session_repository.delete_session(&token).await
    }

    async fn verify_session(
        &self,
        _token: String,
    ) -> Result<crate::authentication::Identity, CoreError> {
        // DEPRECATED: Use identity_middleware instead
        // This method is kept for backward compatibility but should not be used
        Err(CoreError::InternalServerError(
            "verify_session is deprecated. Use identity_middleware::reconstruct_identity instead"
                .to_string(),
        ))
    }

    async fn refresh_session(&self, token: String) -> Result<Session, CoreError> {
        let session = self
            .session_repository
            .load_session(&token)
            .await?
            .ok_or(CoreError::Unauthorized)?;

        if session.is_expired() {
            return Err(CoreError::Unauthorized);
        }

        // Load session data to preserve client_ip
        let session_data = self
            .session_repository
            .load_session_data(&token)
            .await?
            .ok_or(CoreError::Unauthorized)?;

        // Delete old session
        self.session_repository.delete_session(&token).await?;

        // Generate new token
        let new_token = Self::generate_token(&session_data.realm_id, &session_data.user_id);
        let expires_at = Utc::now()
            + Duration::seconds(
                i64::try_from(DEFAULT_SESSION_TTL_SECONDS)
                    .expect("DEFAULT_SESSION_TTL_SECONDS exceeds i64::MAX"),
            );

        // Create new session with preserved client_ip
        let new_session_data = SessionData {
            realm_id: session_data.realm_id.clone(),
            client_id: session_data.client_id.clone(),
            user_id: session_data.user_id.clone(),
            client_ip: session_data.client_ip.clone(),
        };

        self.session_repository
            .store_session(
                &new_token,
                new_session_data.clone(),
                DEFAULT_SESSION_TTL_SECONDS,
            )
            .await?;

        Ok(Session::new(new_token, new_session_data, expires_at))
    }
}

impl<U, S> std::fmt::Debug for AuthenticationServiceImpl<U, S>
where
    U: UserService,
    S: SessionRepository,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthenticationServiceImpl").finish()
    }
}
