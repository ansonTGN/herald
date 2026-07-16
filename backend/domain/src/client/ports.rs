use crate::authentication::Identity;
use crate::client::{
    entities::ClientApp,
    value_objects::{CreateClientAppRequest, UpdateClientAppRequest},
};
use crate::common::entities::app_errors::CoreError;
use std::future::Future;
use uuid::Uuid;

#[cfg_attr(test, mockall::automock)]
pub trait ClientRepository: Send + Sync {
    fn create_client_app(
        &self,
        request: CreateClientAppRequest,
    ) -> impl Future<Output = Result<ClientApp, CoreError>> + Send;

    fn get_client_app_by_id(
        &self,
        id: Uuid,
    ) -> impl Future<Output = Result<ClientApp, CoreError>> + Send;

    fn get_client_app_by_client_id(
        &self,
        realm_id: &str,
        client_id: &str,
    ) -> impl Future<Output = Result<ClientApp, CoreError>> + Send;

    fn list_client_apps(
        &self,
        realm_id: &str,
    ) -> impl Future<Output = Result<Vec<ClientApp>, CoreError>> + Send;

    fn list_client_apps_paginated(
        &self,
        realm_id: &str,
        page: u64,
        page_size: u64,
    ) -> impl Future<Output = Result<(Vec<ClientApp>, u64), CoreError>> + Send;

    fn update_client_app(
        &self,
        id: Uuid,
        request: UpdateClientAppRequest,
    ) -> impl Future<Output = Result<ClientApp, CoreError>> + Send;

    fn delete_client_app(&self, id: Uuid) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn set_first_party(
        &self,
        id: Uuid,
        is_first_party: bool,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;
}

#[cfg_attr(test, mockall::automock)]
pub trait ClientService: Send + Sync {
    fn create_client_app(
        &self,
        identity: Identity,
        request: CreateClientAppRequest,
    ) -> impl Future<Output = Result<ClientApp, CoreError>> + Send;

    fn get_client_app(
        &self,
        identity: Identity,
        id: Uuid,
    ) -> impl Future<Output = Result<ClientApp, CoreError>> + Send;

    fn get_client_app_by_client_id(
        &self,
        realm_id: &str,
        client_id: &str,
    ) -> impl Future<Output = Result<ClientApp, CoreError>> + Send;

    fn list_client_apps(
        &self,
        identity: Identity,
        realm_id: String,
    ) -> impl Future<Output = Result<Vec<ClientApp>, CoreError>> + Send;

    fn list_client_apps_paginated(
        &self,
        identity: Identity,
        realm_id: String,
        page: u64,
        page_size: u64,
    ) -> impl Future<Output = Result<(Vec<ClientApp>, u64), CoreError>> + Send;

    fn update_client_app(
        &self,
        identity: Identity,
        id: Uuid,
        request: UpdateClientAppRequest,
    ) -> impl Future<Output = Result<ClientApp, CoreError>> + Send;

    fn delete_client_app(
        &self,
        identity: Identity,
        id: Uuid,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;
}
