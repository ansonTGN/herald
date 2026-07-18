use crate::authentication::entities::{
    BrowserAccessTokenData, BrowserTokenSet, FamilyLifecycle, RefreshError, UserSessionSummary,
};
use crate::client::entities::ClientApp;
use crate::common::entities::app_errors::CoreError;
use crate::user::entities::User;
use std::future::Future;
use uuid::Uuid;

pub trait BrowserTokenService: Send + Sync {
    fn lookup_access_token(
        &self,
        access_token: &str,
    ) -> impl Future<Output = Result<Option<BrowserAccessTokenData>, CoreError>> + Send;

    fn create_token_family(
        &self,
        user: &User,
        client_app: &ClientApp,
        user_agent: Option<String>,
        client_ip: Option<String>,
    ) -> impl Future<Output = Result<BrowserTokenSet, CoreError>> + Send;

    fn create_first_party_token_family(
        &self,
        user: &User,
        client_app: &ClientApp,
        user_agent: Option<String>,
        client_ip: Option<String>,
    ) -> impl Future<Output = Result<BrowserTokenSet, CoreError>> + Send;

    fn refresh(
        &self,
        refresh_token: &str,
    ) -> impl Future<Output = Result<BrowserTokenSet, RefreshError>> + Send;

    fn revoke_family(&self, family_id: Uuid) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn revoke_client_families(
        &self,
        client_app_id: Uuid,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn revoke_user_families(
        &self,
        user_id: &str,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    /// List active session summaries for a user, filtering revoked/expired
    /// families (and opportunistically pruning stale family ids from the user
    /// set). Families created before the metadata index existed surface the
    /// meta-derived fields as `None`.
    fn list_user_sessions(
        &self,
        user_id: &str,
    ) -> impl Future<Output = Result<Vec<UserSessionSummary>, CoreError>> + Send;

    /// Returns the ownership + lifecycle status of a single family, read
    /// directly from the family record (`bt:fam:{familyId}`) without the
    /// active-only filtering applied by `list_user_sessions`.
    ///
    /// Used by the admin "revoke one session" guard (design kickoff-user
    /// §4.2.2) to distinguish a family that is absent / belongs to another
    /// user or realm (`Ok(None)` → caller returns 404, no cross-realm leak)
    /// from a family that belongs to the target user/realm but is already
    /// revoked or past its absolute expiry (returned `Some` with
    /// `revoked`/`expired` set → caller returns 204 idempotent no-op).
    ///
    /// `expired` is computed at read time against `absolute_expires_at_ts`.
    fn get_family_lifecycle(
        &self,
        family_id: Uuid,
    ) -> impl Future<Output = Result<Option<FamilyLifecycle>, CoreError>> + Send;
}
