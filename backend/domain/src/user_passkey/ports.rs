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
