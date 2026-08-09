use crate::common::entities::app_errors::CoreError;
use std::future::Future;
use uuid::Uuid;

pub mod services;

pub use services::RealmInitializationServiceImpl;

/// Realm RBAC初始化请求
#[derive(Debug, Clone)]
pub struct RealmRBACInitRequest {
    pub realm_id: String,
    pub admin_web_console_client_id: String,
}

/// 角色策略创建请求
#[derive(Debug, Clone)]
pub struct CreateRolePolicyRequest {
    pub realm_id: String,
    pub role_id: Uuid,
    pub resource: String,
    pub action: String,
}

/// 角色策略仓库 - Domain层定义的trait
#[cfg_attr(test, mockall::automock)]
pub trait RolePolicyRepository: Send + Sync {
    /// 创建角色策略
    fn create_policy(
        &self,
        request: CreateRolePolicyRequest,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    /// 失效realm缓存（策略变更后调用）
    fn invalidate_realm_cache(
        &self,
        realm_id: &str,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;
}

/// Realm初始化Service Trait
#[cfg_attr(test, mockall::automock)]
pub trait RealmInitializationService: Send + Sync {
    /// 初始化Realm的默认RBAC配置
    /// 包括：角色定义、权限定义、角色权限关联、策略
    fn init_default_rbac(
        &self,
        request: RealmRBACInitRequest,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;
}
