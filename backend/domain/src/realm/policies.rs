// =============================================================================
// Realm Policy Trait
// =============================================================================
//
// 定义 Realm 管理相关的权限检查接口
//
// =============================================================================

use crate::authentication::Identity;
use crate::common::entities::app_errors::CoreError;
use crate::realm::Realm;

/// Realm Policy - Realm 管理权限检查
pub trait RealmPolicy: Send + Sync {
    /// 检查是否可以创建 Realm
    fn can_create_realm(
        &self,
        identity: &Identity,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;

    /// 检查是否可以查看 Realm 详情
    fn can_view_realm(
        &self,
        identity: &Identity,
        realm: &Realm,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;

    /// 检查是否可以更新 Realm
    fn can_update_realm(
        &self,
        identity: &Identity,
        realm: &Realm,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;

    /// 检查是否可以删除 Realm
    fn can_delete_realm(
        &self,
        identity: &Identity,
        realm: &Realm,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;

    /// 检查是否可以列出 Realms
    fn can_list_realms(
        &self,
        identity: &Identity,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;

    /// 检查是否是 Realm Admin
    fn can_admin_realm(
        &self,
        identity: &Identity,
        realm: &Realm,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;
}
