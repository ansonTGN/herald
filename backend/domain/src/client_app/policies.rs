// =============================================================================
// Client App Policy Trait
// =============================================================================
//
// 定义 Client App 管理相关的权限检查接口
//
// =============================================================================

use crate::authentication::Identity;
use crate::common::entities::app_errors::CoreError;
use crate::realm::Realm;

/// Client App Policy - Client App 管理权限检查
pub trait ClientAppPolicy: Send + Sync {
    /// 检查是否可以列出 Client Apps
    fn can_list_client_apps(
        &self,
        identity: &Identity,
        realm: &Realm,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;

    /// 检查是否可以查看 Client App 详情
    fn can_view_client_app(
        &self,
        identity: &Identity,
        realm: &Realm,
        client_id: &str,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;

    /// 检查是否可以创建 Client App
    fn can_create_client_app(
        &self,
        identity: &Identity,
        realm: &Realm,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;

    /// 检查是否可以更新 Client App
    fn can_update_client_app(
        &self,
        identity: &Identity,
        realm: &Realm,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;

    /// 检查是否可以删除 Client App
    fn can_delete_client_app(
        &self,
        identity: &Identity,
        realm: &Realm,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;
}
