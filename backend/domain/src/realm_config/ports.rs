use super::entities::{RealmConfig, UpsertRealmConfigRequest};
use crate::common::entities::app_errors::CoreError;
use std::future::Future;

/// Realm 配置仓库接口
#[cfg_attr(test, mockall::automock)]
pub trait RealmConfigRepository: Send + Sync {
    /// 创建或更新配置（upsert 操作）
    fn upsert(
        &self,
        realm_id: &str,
        request: UpsertRealmConfigRequest,
    ) -> impl Future<Output = Result<RealmConfig, CoreError>> + Send;

    /// 批量创建或更新配置
    fn batch_upsert(
        &self,
        realm_id: &str,
        requests: Vec<UpsertRealmConfigRequest>,
    ) -> impl Future<Output = Result<Vec<RealmConfig>, CoreError>> + Send;

    /// 获取指定 realm 的单个配置
    fn get(
        &self,
        realm_id: String,
        config_type: String,
        config_key: String,
    ) -> impl Future<Output = Result<Option<RealmConfig>, CoreError>> + Send;

    /// 获取指定 realm 的指定类型的所有配置
    fn get_by_type(
        &self,
        realm_id: String,
        config_type: String,
    ) -> impl Future<Output = Result<Vec<RealmConfig>, CoreError>> + Send;

    /// 获取指定 realm 的所有配置
    fn get_all(
        &self,
        realm_id: String,
    ) -> impl Future<Output = Result<Vec<RealmConfig>, CoreError>> + Send;

    /// 删除指定配置
    fn delete(
        &self,
        realm_id: String,
        config_type: String,
        config_key: String,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    /// 检查配置是否存在
    fn exists(
        &self,
        realm_id: String,
        config_type: String,
        config_key: String,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;
}
