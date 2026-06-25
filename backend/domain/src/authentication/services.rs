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
    #[tracing::instrument(
        // BE-D08 governance (§4.5/§5.4): request carries password (credential)
        // + email/username (PII); client_id/client_ip are identifiers;
        // self holds the session repo. Only the low-cardinality operation
        // type and db.system are recorded.
        skip(self, request, client_id, client_ip),
        fields(db.system = "postgres", db.operation = "auth_login")
    )]
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
            renewal_ttl_seconds: None,
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
    #[tracing::instrument(
        // BE-D08 governance (§4.5/§5.4): user carries identity/email;
        // client_id/client_ip are identifiers; self holds the session repo.
        // Only the low-cardinality operation type is recorded.
        skip(self, user, client_id, client_ip),
        fields(db.operation = "create_session")
    )]
    async fn create_session(
        &self,
        user: crate::user::entities::User,
        client_id: String,
        client_ip: String,
        ttl_seconds: u64,
        renewal_ttl_seconds: Option<u64>,
    ) -> Result<(Session, Identity), CoreError> {
        // Create session data first
        let session_data = SessionData {
            realm_id: user.realm_id.clone(),
            client_id: client_id.clone(),
            user_id: user.id.to_string(),
            client_ip,
            renewal_ttl_seconds,
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

    #[tracing::instrument(
        // BE-D08 governance (§4.5/§5.4): token is the session credential.
        skip(self, token),
        fields(db.operation = "logout")
    )]
    async fn logout(&self, token: String) -> Result<(), CoreError> {
        self.session_repository.delete_session(&token).await
    }

    #[tracing::instrument(
        // BE-D08 governance (§4.5/§5.4): token is the session credential.
        skip(self, _token),
        fields(db.operation = "verify_session")
    )]
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

    #[tracing::instrument(
        // BE-D08 governance (§4.5/§5.4): token is the session credential.
        skip(self, token),
        fields(db.operation = "refresh_session")
    )]
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
            renewal_ttl_seconds: session_data.renewal_ttl_seconds,
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

// BE-T03 governance tests (design §5.4 / §4.5).
//
// Covers: BE-D08 — `AuthenticationServiceImpl` login / create_session / logout
// / verify_session / refresh_session instrument skip correctness.
//
// WHY: login carries password + email/username; the session methods take the
// session `token` — a credential. If the `#[instrument]` macro ever stops
// skipping those, the credential/PII leaks into a span field. Source-scan
// baseline (design §6.1), anchored per method to the immediately-preceding
// `#[tracing::instrument(...)]`.
#[cfg(test)]
mod instrument_skip_tests {
    const SRC: &str = include_str!("services.rs");

    fn instrument_body_preceding(fn_name: &str) -> String {
        let needle = format!("fn {fn_name}");
        let fn_pos = SRC
            .find(&needle)
            .unwrap_or_else(|| panic!("fn {fn_name} not found in source"));
        let attr_start = SRC[..fn_pos]
            .rfind("#[tracing::instrument(")
            .unwrap_or_else(|| panic!("no #[tracing::instrument( preceding fn {fn_name}"));
        let body_start = attr_start + "#[tracing::instrument(".len();
        // Find the attribute close: the first line at/after body_start whose
        // trimmed content is exactly `)]`. This handles indented closes (e.g.
        // inside an `impl` block) and ignores inline `))]` sequences such as
        // `#[validate(length(...))]` that appear on struct fields.
        let tail = &SRC[body_start..];
        let mut consumed = 0usize;
        for line in tail.lines() {
            let prev = consumed;
            consumed += line.len() + 1; // +1 for the line separator
            if line.trim() == ")]" {
                return tail[..prev].to_string();
            }
        }
        panic!("unterminated #[tracing::instrument( for fn {fn_name}")
    }

    #[test]
    fn instrument_skip_auth_service_login_excludes_password_email() {
        let body = instrument_body_preceding("login");
        for required in ["request", "client_id", "client_ip"] {
            assert!(
                body.contains(required),
                "AuthenticationServiceImpl::login must skip `{required}`; body was:\n{body}"
            );
        }
        for banned in ["password", "email", "token", "secret"] {
            assert!(
                !body.contains(&format!("{banned} ="))
                    && !body.contains(&format!("fields({banned}")),
                "auth login span must not record a `{banned}` field; body was:\n{body}"
            );
        }
    }

    #[test]
    fn instrument_skip_auth_service_create_session_excludes_user_and_identifiers() {
        let body = instrument_body_preceding("create_session");
        for required in ["user", "client_id", "client_ip"] {
            assert!(
                body.contains(required),
                "create_session must skip `{required}`; body was:\n{body}"
            );
        }
        for banned in ["token", "password", "email", "secret"] {
            assert!(
                !body.contains(&format!("{banned} ="))
                    && !body.contains(&format!("fields({banned}")),
                "create_session span must not record a `{banned}` field; body was:\n{body}"
            );
        }
    }

    #[test]
    fn instrument_skip_auth_service_logout_excludes_session_token() {
        let body = instrument_body_preceding("logout");
        assert!(
            body.contains("token"),
            "logout must skip `token` (session credential); body was:\n{body}"
        );
    }

    #[test]
    fn instrument_skip_auth_service_verify_session_excludes_session_token() {
        let body = instrument_body_preceding("verify_session");
        assert!(
            body.contains("_token") || body.contains("token"),
            "verify_session must skip the session token; body was:\n{body}"
        );
    }

    #[test]
    fn instrument_skip_auth_service_refresh_session_excludes_session_token() {
        let body = instrument_body_preceding("refresh_session");
        assert!(
            body.contains("token"),
            "refresh_session must skip `token` (session credential); body was:\n{body}"
        );
    }
}
