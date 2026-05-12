use super::entities::{BatchUpsertRealmConfigRequest, RealmConfig, UpsertRealmConfigRequest};
use crate::authentication::Identity;
use crate::common::entities::app_errors::CoreError;
use std::future::Future;

/// Realm 配置服务接口
#[cfg_attr(test, mockall::automock)]
pub trait RealmConfigService: Send + Sync {
    /// 创建或更新单个配置
    fn upsert_config(
        &self,
        identity: Identity,
        realm_id: String,
        request: UpsertRealmConfigRequest,
    ) -> impl Future<Output = Result<RealmConfig, CoreError>> + Send;

    /// 批量创建或更新配置
    fn batch_upsert_configs(
        &self,
        identity: Identity,
        realm_id: String,
        request: BatchUpsertRealmConfigRequest,
    ) -> impl Future<Output = Result<Vec<RealmConfig>, CoreError>> + Send;

    /// 获取指定 realm 的单个配置
    fn get_config(
        &self,
        identity: Identity,
        realm_id: String,
        config_type: String,
        config_key: String,
    ) -> impl Future<Output = Result<Option<RealmConfig>, CoreError>> + Send;

    /// 获取指定 realm 的指定类型的所有配置
    fn get_configs_by_type(
        &self,
        identity: Identity,
        realm_id: String,
        config_type: String,
    ) -> impl Future<Output = Result<Vec<RealmConfig>, CoreError>> + Send;

    /// 获取指定 realm 的所有配置
    fn get_all_configs(
        &self,
        identity: Identity,
        realm_id: String,
    ) -> impl Future<Output = Result<Vec<RealmConfig>, CoreError>> + Send;

    /// 删除指定配置
    fn delete_config(
        &self,
        identity: Identity,
        realm_id: String,
        config_type: String,
        config_key: String,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;
}
