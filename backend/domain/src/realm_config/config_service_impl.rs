// Realm configuration service implementation

use crate::{
    authentication::Identity,
    common::{
        entities::app_errors::CoreError,
        policies::{RealmConfigPolicy, ensure_policy},
    },
    realm_config::{
        entities::{BatchUpsertRealmConfigRequest, RealmConfig, UpsertRealmConfigRequest},
        ports::RealmConfigRepository,
        service::RealmConfigService,
    },
};
use std::future::Future;
use std::sync::Arc;

/// Realm configuration service implementation
pub struct RealmConfigServiceImpl<R, P>
where
    R: RealmConfigRepository,
    P: RealmConfigPolicy,
{
    config_repository: Arc<R>,
    policy: Arc<P>,
}

impl<R, P> RealmConfigServiceImpl<R, P>
where
    R: RealmConfigRepository,
    P: RealmConfigPolicy,
{
    pub fn new(config_repository: Arc<R>, policy: Arc<P>) -> Self {
        Self {
            config_repository,
            policy,
        }
    }
}

impl<R, P> RealmConfigService for RealmConfigServiceImpl<R, P>
where
    R: RealmConfigRepository,
    P: RealmConfigPolicy,
{
    fn upsert_config(
        &self,
        identity: Identity,
        realm_id: String,
        request: UpsertRealmConfigRequest,
    ) -> impl Future<Output = Result<RealmConfig, CoreError>> + Send {
        let repo = self.config_repository.clone();
        let policy = self.policy.clone();

        async move {
            // Policy check - 配置管理需要更新权限（使用具体方法 + ensure_policy）
            ensure_policy(
                policy.can_update_config(identity.clone()).await,
                "Insufficient permissions to update config",
            )?;

            // CRITICAL: Realm boundary check - prevent cross-realm config access
            if !identity.has_access_to_realm(&realm_id) {
                return Err(CoreError::Forbidden(
                    "Access denied: cannot modify config in a different realm".to_string(),
                ));
            }

            repo.upsert(&realm_id, request).await
        }
    }

    fn batch_upsert_configs(
        &self,
        identity: Identity,
        realm_id: String,
        request: BatchUpsertRealmConfigRequest,
    ) -> impl Future<Output = Result<Vec<RealmConfig>, CoreError>> + Send {
        let repo = self.config_repository.clone();
        let policy = self.policy.clone();

        async move {
            // Policy check - 配置管理需要更新权限
            ensure_policy(
                policy.can_update_config(identity.clone()).await,
                "Insufficient permissions to update config",
            )?;

            // CRITICAL: Realm boundary check - prevent cross-realm config access
            if !identity.has_access_to_realm(&realm_id) {
                return Err(CoreError::Forbidden(
                    "Access denied: cannot modify configs in a different realm".to_string(),
                ));
            }

            repo.batch_upsert(&realm_id, request.configs).await
        }
    }

    fn get_config(
        &self,
        identity: Identity,
        realm_id: String,
        config_type: String,
        config_key: String,
    ) -> impl Future<Output = Result<Option<RealmConfig>, CoreError>> + Send {
        let repo = self.config_repository.clone();
        let policy = self.policy.clone();

        async move {
            // Policy check - 读取配置需要读取权限
            ensure_policy(
                policy.can_read_config(identity.clone()).await,
                "Insufficient permissions to read config",
            )?;

            // CRITICAL: Realm boundary check - prevent cross-realm config access
            if !identity.has_access_to_realm(&realm_id) {
                return Err(CoreError::Forbidden(
                    "Access denied: cannot read config from a different realm".to_string(),
                ));
            }

            repo.get(realm_id, config_type, config_key).await
        }
    }

    fn get_configs_by_type(
        &self,
        identity: Identity,
        realm_id: String,
        config_type: String,
    ) -> impl Future<Output = Result<Vec<RealmConfig>, CoreError>> + Send {
        let repo = self.config_repository.clone();
        let policy = self.policy.clone();

        async move {
            // Policy check - 读取配置需要读取权限
            ensure_policy(
                policy.can_read_config(identity.clone()).await,
                "Insufficient permissions to read config",
            )?;

            // CRITICAL: Realm boundary check - prevent cross-realm config access
            if !identity.has_access_to_realm(&realm_id) {
                return Err(CoreError::Forbidden(
                    "Access denied: cannot read configs from a different realm".to_string(),
                ));
            }

            repo.get_by_type(realm_id, config_type).await
        }
    }

    fn get_all_configs(
        &self,
        identity: Identity,
        realm_id: String,
    ) -> impl Future<Output = Result<Vec<RealmConfig>, CoreError>> + Send {
        let repo = self.config_repository.clone();
        let policy = self.policy.clone();

        async move {
            // Policy check - 读取配置需要读取权限
            ensure_policy(
                policy.can_read_config(identity.clone()).await,
                "Insufficient permissions to read config",
            )?;

            // CRITICAL: Realm boundary check - prevent cross-realm config access
            if !identity.has_access_to_realm(&realm_id) {
                return Err(CoreError::Forbidden(
                    "Access denied: cannot read configs from a different realm".to_string(),
                ));
            }

            repo.get_all(realm_id).await
        }
    }

    fn delete_config(
        &self,
        identity: Identity,
        realm_id: String,
        config_type: String,
        config_key: String,
    ) -> impl Future<Output = Result<(), CoreError>> + Send {
        let repo = self.config_repository.clone();
        let policy = self.policy.clone();

        async move {
            // Policy check - 删除配置需要删除权限
            ensure_policy(
                policy.can_delete_config(identity.clone()).await,
                "Insufficient permissions to delete config",
            )?;

            // CRITICAL: Realm boundary check - prevent cross-realm config access
            if !identity.has_access_to_realm(&realm_id) {
                return Err(CoreError::Forbidden(
                    "Access denied: cannot delete config from a different realm".to_string(),
                ));
            }

            repo.delete(realm_id, config_type, config_key).await
        }
    }
}

impl<R, P> std::fmt::Debug for RealmConfigServiceImpl<R, P>
where
    R: RealmConfigRepository,
    P: RealmConfigPolicy,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RealmConfigServiceImpl").finish()
    }
}
