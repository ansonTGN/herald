use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use std::sync::Arc;

use crate::authorization::RedisPermissionChecker;
use herald_domain::authorization::permission_service::PermissionService;
use herald_domain::common::entities::app_errors::CoreError;
use herald_domain::rbac_init::{CreateRolePolicyRequest, RolePolicyRepository};
use herald_entity::role_policies;

/// PostgreSQL implementation of RolePolicyRepository
pub struct PostgresRolePolicyRepository {
    db: Arc<DatabaseConnection>,
    permission_checker: Arc<RedisPermissionChecker>,
}

impl PostgresRolePolicyRepository {
    pub fn new(
        db: Arc<DatabaseConnection>,
        permission_checker: Arc<RedisPermissionChecker>,
    ) -> Self {
        Self {
            db,
            permission_checker,
        }
    }
}

impl RolePolicyRepository for PostgresRolePolicyRepository {
    async fn create_policy(&self, request: CreateRolePolicyRequest) -> Result<(), CoreError> {
        if let Ok(Some(())) = self
            .find_policy(request.role_id, &request.resource, &request.action)
            .await
        {
            tracing::info!(
                "Policy already exists: role_id={}, resource={}, action={}",
                request.role_id,
                request.resource,
                request.action
            );
            // Still invalidate cache to ensure consistency
            self.permission_checker
                .invalidate_realm_cache(&request.realm_id)
                .await
                .map_err(|e| {
                    CoreError::InternalServerError(format!("Failed to invalidate cache: {}", e))
                })?;
            return Ok(());
        }

        let policy = role_policies::ActiveModel {
            id: Set(uuid::Uuid::now_v7()),
            realm_id: Set(request.realm_id.clone()),
            role_id: Set(request.role_id),
            resource: Set(request.resource),
            action: Set(request.action),
            created_at: Set(chrono::Utc::now().into()),
        };

        policy.insert(&*self.db).await.map_err(|e| {
            CoreError::InternalServerError(format!("Failed to create policy: {}", e))
        })?;

        // Invalidate cache after creating policy
        self.permission_checker
            .invalidate_realm_cache(&request.realm_id)
            .await
            .map_err(|e| {
                CoreError::InternalServerError(format!("Failed to invalidate cache: {}", e))
            })?;

        Ok(())
    }

    async fn invalidate_realm_cache(&self, realm_id: &str) -> Result<(), CoreError> {
        self.permission_checker
            .invalidate_realm_cache(realm_id)
            .await
            .map_err(|e| {
                CoreError::InternalServerError(format!("Failed to invalidate cache: {}", e))
            })?;

        Ok(())
    }
}

impl PostgresRolePolicyRepository {
    /// Find policy by role_id, resource, and action (for idempotency check)
    async fn find_policy(
        &self,
        role_id: uuid::Uuid,
        resource: &str,
        action: &str,
    ) -> Result<Option<()>, CoreError> {
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

        let exists = role_policies::Entity::find()
            .filter(role_policies::Column::RoleId.eq(role_id))
            .filter(role_policies::Column::Resource.eq(resource))
            .filter(role_policies::Column::Action.eq(action))
            .one(&*self.db)
            .await
            .map_err(|e| {
                CoreError::InternalServerError(format!("Failed to check policy existence: {}", e))
            })?;

        Ok(if exists.is_some() { Some(()) } else { None })
    }
}
