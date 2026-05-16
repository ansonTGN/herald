use crate::authentication::Identity;
use crate::authentication::entities::{Session, SessionData};
use crate::common::entities::app_errors::CoreError;
use crate::user::entities::User;
use crate::user::value_objects::LoginRequest;
use std::future::Future;

#[cfg_attr(test, mockall::automock)]
pub trait SessionRepository: Send + Sync {
    fn store_session(
        &self,
        token: &str,
        data: SessionData,
        ttl_seconds: u64,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn load_session(
        &self,
        token: &str,
    ) -> impl Future<Output = Result<Option<Session>, CoreError>> + Send;

    /// Load session data directly (for internal use like refresh_session)
    fn load_session_data(
        &self,
        token: &str,
    ) -> impl Future<Output = Result<Option<SessionData>, CoreError>> + Send;

    fn delete_session(&self, token: &str) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn delete_user_sessions(
        &self,
        user_id: &str,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;
}

#[cfg_attr(test, mockall::automock)]
pub trait AuthenticationService: Send + Sync {
    fn login(
        &self,
        request: LoginRequest,
        client_id: String,
        client_ip: String,
    ) -> impl Future<Output = Result<(Session, Identity), CoreError>> + Send;

    /// Create a session for an authenticated user (after TOTP verification if enabled)
    ///
    /// This method creates a session without re-authenticating the user.
    /// It's used when the user has already passed password authentication
    /// and optional TOTP verification.
    ///
    /// # Arguments
    /// * `user` - The authenticated user entity
    /// * `client_id` - The client application ID from the login request
    /// * `client_ip` - The client IP address for session validation
    /// * `ttl_seconds` - Session time-to-live in seconds
    ///
    /// # Returns
    /// * `Session` - The created session
    /// * `Identity` - The user identity
    fn create_session(
        &self,
        user: User,
        client_id: String,
        client_ip: String,
        ttl_seconds: u64,
        renewal_ttl_seconds: Option<u64>,
    ) -> impl Future<Output = Result<(Session, Identity), CoreError>> + Send;

    fn logout(&self, token: String) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn verify_session(
        &self,
        token: String,
    ) -> impl Future<Output = Result<Identity, CoreError>> + Send;

    fn refresh_session(
        &self,
        token: String,
    ) -> impl Future<Output = Result<Session, CoreError>> + Send;
}
