// OAuth provider configuration service

use std::sync::Arc;
use uuid::Uuid;

use crate::{
    audit::{
        AuditAction, AuditCategory, AuditContext, AuditEventRepository, AuditResult,
        AuditTargetType, NewAuditEvent,
    },
    authentication::Identity,
    common::{
        entities::app_errors::CoreError,
        policies::{OAuthConfigPolicy, ensure_policy},
    },
    oauth::{
        entities::{
            CreateOAuthProviderConfigRequest, OAuthProviderConfig, UpdateOAuthProviderConfigRequest,
        },
        ports::OAuthConfigRepository,
    },
};

/// OAuth provider configuration service
pub struct OAuthConfigService<R, P, A>
where
    R: OAuthConfigRepository,
    P: OAuthConfigPolicy,
    A: AuditEventRepository + 'static,
{
    config_repository: Arc<R>,
    policy: Arc<P>,
    audit_event_repository: Arc<A>,
}

impl<R, P, A> OAuthConfigService<R, P, A>
where
    R: OAuthConfigRepository,
    P: OAuthConfigPolicy,
    A: AuditEventRepository + 'static,
{
    pub fn new(config_repository: Arc<R>, policy: Arc<P>, audit_event_repository: Arc<A>) -> Self {
        Self {
            config_repository,
            policy,
            audit_event_repository,
        }
    }

    async fn record_audit(
        &self,
        ctx: &AuditContext,
        config: &OAuthProviderConfig,
        action: AuditAction,
    ) {
        if let Err(e) = self
            .audit_event_repository
            .create(NewAuditEvent {
                realm_id: config.realm_id.clone(),
                category: AuditCategory::OAuth,
                action,
                actor_id: ctx.actor_id.clone(),
                actor_type: ctx.actor_type,
                actor_name: ctx.actor_name.clone(),
                target_type: AuditTargetType::OAuthConfig,
                target_id: config.id.to_string(),
                target_name: Some(config.provider_type.as_str().to_string()),
                result: AuditResult::Success,
                details: Some(serde_json::json!({
                    "provider_type": config.provider_type.as_str(),
                    "enabled": config.enabled,
                })),
                ip_address: ctx.ip_address.clone(),
                user_agent: ctx.user_agent.clone(),
                trace_id: ctx.trace_id.clone(),
            })
            .await
        {
            tracing::warn!(error = %e, "Failed to record oauth config audit event");
        }
    }

    /// Create OAuth provider configuration
    ///
    /// # Arguments
    ///
    /// * `identity` - The caller's identity (must have admin permissions)
    /// * `request` - OAuth configuration details
    ///
    /// # Returns
    ///
    /// Returns the created `OAuthProviderConfig`
    ///
    /// # Errors
    ///
    /// Returns `CoreError::Forbidden` if the caller lacks permission
    /// Returns `CoreError::BadRequest` if the request data is invalid
    /// Returns `CoreError::Conflict` if a provider with the same type already exists
    /// Returns `CoreError::Database` if database operation fails
    pub async fn create_config(
        &self,
        identity: Identity,
        ctx: AuditContext,
        request: CreateOAuthProviderConfigRequest,
    ) -> Result<OAuthProviderConfig, CoreError> {
        // Policy check（使用具体方法 + ensure_policy）
        ensure_policy(
            self.policy.can_create_config(identity.clone()).await,
            "Insufficient permissions to create oauth config",
        )?;

        // CRITICAL: Realm boundary check - prevent cross-realm config creation
        if !identity.has_access_to_realm(&request.realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot create oauth config in a different realm".to_string(),
            ));
        }

        if self
            .config_repository
            .get_config(&request.realm_id, request.provider_type.as_str())
            .await
            .is_ok()
        {
            return Err(CoreError::Conflict(
                "OAuth provider config already exists for this realm".to_string(),
            ));
        }

        let config = OAuthProviderConfig::new(request)?;
        let created = self.config_repository.create_config(config).await?;
        self.record_audit(&ctx, &created, AuditAction::OAuthConfigCreate)
            .await;
        Ok(created)
    }

    /// Get OAuth provider configuration by realm and provider type
    ///
    /// # Arguments
    ///
    /// * `identity` - The caller's identity
    /// * `realm_id` - The realm ID
    /// * `provider_type` - The OAuth provider type (e.g., "google", "github")
    ///
    /// # Returns
    ///
    /// Returns the `OAuthProviderConfig` if found
    ///
    /// # Errors
    ///
    /// Returns `CoreError::Forbidden` if the caller lacks permission
    /// Returns `CoreError::NotFound` if the config does not exist
    /// Returns `CoreError::Database` if database operation fails
    pub async fn get_config(
        &self,
        identity: Identity,
        realm_id: &str,
        provider_type: &str,
    ) -> Result<OAuthProviderConfig, CoreError> {
        // Policy check
        ensure_policy(
            self.policy.can_read_config(identity.clone()).await,
            "Insufficient permissions to read oauth config",
        )?;

        // CRITICAL: Realm boundary check - prevent cross-realm config access
        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot read oauth config from a different realm".to_string(),
            ));
        }

        self.config_repository
            .get_config(realm_id, provider_type)
            .await
    }

    /// List all OAuth provider configurations for a realm
    ///
    /// # Arguments
    ///
    /// * `identity` - The caller's identity
    /// * `realm_id` - The realm ID
    ///
    /// # Returns
    ///
    /// Returns a vector of all `OAuthProviderConfig` for the realm
    ///
    /// # Errors
    ///
    /// Returns `CoreError::Forbidden` if the caller lacks permission
    /// Returns `CoreError::Database` if database operation fails
    pub async fn list_configs(
        &self,
        identity: Identity,
        realm_id: &str,
    ) -> Result<Vec<OAuthProviderConfig>, CoreError> {
        // Policy check
        ensure_policy(
            self.policy.can_list_configs(identity.clone()).await,
            "Insufficient permissions to list oauth configs",
        )?;

        // CRITICAL: Realm boundary check - prevent cross-realm config access
        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot list oauth configs from a different realm".to_string(),
            ));
        }

        self.config_repository.list_configs(realm_id).await
    }

    /// Update OAuth provider configuration
    ///
    /// # Arguments
    ///
    /// * `identity` - The caller's identity (must have admin permissions)
    /// * `id` - The UUID of the config to update
    /// * `request` - Updated configuration details
    ///
    /// # Returns
    ///
    /// Returns the updated `OAuthProviderConfig`
    ///
    /// # Errors
    ///
    /// Returns `CoreError::Forbidden` if the caller lacks permission
    /// Returns `CoreError::NotFound` if the config does not exist
    /// Returns `CoreError::BadRequest` if the request data is invalid
    /// Returns `CoreError::Database` if database operation fails
    pub async fn update_config(
        &self,
        identity: Identity,
        ctx: AuditContext,
        id: Uuid,
        request: UpdateOAuthProviderConfigRequest,
    ) -> Result<OAuthProviderConfig, CoreError> {
        // Policy check
        ensure_policy(
            self.policy.can_update_config(identity.clone()).await,
            "Insufficient permissions to update oauth config",
        )?;

        // Get config and check realm boundary
        let config = self.config_repository.get_config_by_id(id).await?;

        // CRITICAL: Realm boundary check - prevent cross-realm config access
        if !identity.has_access_to_realm(&config.realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot update oauth config from a different realm".to_string(),
            ));
        }

        let updated = self.config_repository.update_config(id, request).await?;
        self.record_audit(&ctx, &updated, AuditAction::OAuthConfigUpdate)
            .await;
        Ok(updated)
    }

    /// Delete OAuth provider configuration
    ///
    /// # Arguments
    ///
    /// * `identity` - The caller's identity (must have admin permissions)
    /// * `id` - The UUID of the config to delete
    ///
    /// # Errors
    ///
    /// Returns `CoreError::Forbidden` if the caller lacks permission
    /// Returns `CoreError::NotFound` if the config does not exist
    /// Returns `CoreError::Database` if database operation fails
    pub async fn delete_config(
        &self,
        identity: Identity,
        ctx: AuditContext,
        id: Uuid,
    ) -> Result<(), CoreError> {
        // Policy check
        ensure_policy(
            self.policy.can_delete_config(identity.clone()).await,
            "Insufficient permissions to delete oauth config",
        )?;

        // Get config and check realm boundary
        let config = self.config_repository.get_config_by_id(id).await?;

        // CRITICAL: Realm boundary check - prevent cross-realm config access
        if !identity.has_access_to_realm(&config.realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot delete oauth config from a different realm".to_string(),
            ));
        }

        self.config_repository.delete_config(id).await?;
        self.record_audit(&ctx, &config, AuditAction::OAuthConfigDelete)
            .await;
        Ok(())
    }

    /// Get enabled OAuth providers for a realm
    ///
    /// # Arguments
    ///
    /// * `realm_id` - The realm ID
    ///
    /// # Returns
    ///
    /// Returns a vector of enabled `OAuthProviderConfig`
    ///
    /// # Errors
    ///
    /// Returns `CoreError::Database` if database operation fails
    pub async fn list_enabled_providers(
        &self,
        realm_id: &str,
    ) -> Result<Vec<OAuthProviderConfig>, CoreError> {
        self.config_repository.list_enabled_configs(realm_id).await
    }
}

impl<R, P, A> std::fmt::Debug for OAuthConfigService<R, P, A>
where
    R: OAuthConfigRepository,
    P: OAuthConfigPolicy,
    A: AuditEventRepository + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OAuthConfigService").finish()
    }
}
