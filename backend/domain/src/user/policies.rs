// =============================================================================
// User Policy Trait
// =============================================================================
//
// 定义用户管理相关的权限检查接口
// 参考：FerrisKey ferriskey/ferriskey/core/src/domain/user/policies.rs
//
// 在 Service 层使用：
// ```rust
// if !self.user_policy.can_list_users(&identity, &realm).await? {
//     return Err(CoreError::Forbidden(...));
// }
// ```
//
// =============================================================================

use crate::authentication::Identity;
use crate::common::entities::app_errors::CoreError;
use crate::realm::Realm;

/// User Policy - 用户管理权限检查
pub trait UserPolicy: Send + Sync {
    /// 检查是否可以列出用户
    fn can_list_users(
        &self,
        identity: &Identity,
        realm: &Realm,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;

    /// 检查是否可以查看用户详情
    fn can_view_user(
        &self,
        identity: &Identity,
        realm: &Realm,
        user_id: &str,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;

    /// 检查是否可以创建用户
    fn can_create_user(
        &self,
        identity: &Identity,
        realm: &Realm,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;

    /// 检查是否可以更新用户
    fn can_update_user(
        &self,
        identity: &Identity,
        realm: &Realm,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;

    /// 检查是否可以删除用户
    fn can_delete_user(
        &self,
        identity: &Identity,
        realm: &Realm,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;

    /// 检查是否可以分配角色
    fn can_assign_roles(
        &self,
        identity: &Identity,
        realm: &Realm,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;
}
