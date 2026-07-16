use crate::authentication::entities::{BrowserAccessTokenData, BrowserTokenSet, RefreshError};
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
    ) -> impl Future<Output = Result<BrowserTokenSet, CoreError>> + Send;

    fn create_first_party_token_family(
        &self,
        user: &User,
        client_app: &ClientApp,
    ) -> impl Future<Output = Result<BrowserTokenSet, CoreError>> + Send;

    fn refresh(
        &self,
        refresh_token: &str,
        client_app_id: Uuid,
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
}
