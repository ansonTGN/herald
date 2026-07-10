use crate::common::entities::app_errors::CoreError;
use crate::user_passkey::entities::UserPasskeyCredential;
use chrono::{DateTime, Utc};
use std::future::Future;
use uuid::Uuid;

#[cfg_attr(test, mockall::automock)]
pub trait UserPasskeyRepository: Send + Sync {
    fn list_by_user(
        &self,
        realm_id: &str,
        user_id: Uuid,
    ) -> impl Future<Output = Result<Vec<UserPasskeyCredential>, CoreError>> + Send;

    fn find_by_credential_id(
        &self,
        realm_id: &str,
        credential_id: &[u8],
    ) -> impl Future<Output = Result<Option<UserPasskeyCredential>, CoreError>> + Send;

    fn insert(
        &self,
        credential: UserPasskeyCredential,
    ) -> impl Future<Output = Result<UserPasskeyCredential, CoreError>> + Send;

    fn rename(
        &self,
        realm_id: &str,
        user_id: Uuid,
        id: Uuid,
        nickname: &str,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn delete(
        &self,
        realm_id: &str,
        user_id: Uuid,
        id: Uuid,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn update_counter_and_used(
        &self,
        id: Uuid,
        counter: u64,
        user_verified: bool,
        used_at: DateTime<Utc>,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;
}

/// Realm-level Passkey policy resolved from `realm_config` (config_type='passkey').
///
/// Drives the per-realm ceremony options (user-verification requirement and
/// authenticator-attachment selection) applied at challenge-generation time.
#[derive(Debug, Clone, Default)]
pub struct PasskeyRealmPolicy {
    /// "preferred" | "required". "required" enforces UV at the ceremony level.
    pub user_verification: UserVerificationPolicy,
    /// When false, restrict registration/authentication to platform
    /// authenticators. When true, allow cross-platform authenticators.
    pub cross_platform_authenticator: bool,
}

/// User-verification requirement for the passkey ceremony.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum UserVerificationPolicy {
    /// UV preferred but not required (passkey-auth builder default).
    #[default]
    Preferred,
    /// UV strictly required; authenticators that cannot do UV are rejected.
    Required,
}

impl UserVerificationPolicy {
    pub fn is_required(&self) -> bool {
        matches!(self, Self::Required)
    }

    /// Parse from the realm config string value stored in `realm_config`.
    pub fn parse(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "required" => Self::Required,
            _ => Self::Preferred,
        }
    }
}

/// Reads the realm passkey policy so the ceremony can apply per-realm options.
#[cfg_attr(test, mockall::automock)]
pub trait PasskeyRealmConfigReader: Send + Sync {
    fn get_policy(
        &self,
        realm_id: &str,
    ) -> impl Future<Output = Result<PasskeyRealmPolicy, CoreError>> + Send;
}

#[cfg_attr(test, mockall::automock)]
pub trait PasskeyChallengeStore: Send + Sync {
    fn store(
        &self,
        token: &str,
        payload: &[u8],
        ttl_secs: u64,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn load(&self, token: &str) -> impl Future<Output = Result<Option<Vec<u8>>, CoreError>> + Send;

    fn delete(&self, token: &str) -> impl Future<Output = Result<(), CoreError>> + Send;
}
