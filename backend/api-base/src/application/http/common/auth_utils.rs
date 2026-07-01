// =============================================================================
// Authorization Utilities
// =============================================================================
//
// Shared authorization helper functions to reduce code duplication across
// HTTP handlers. These utilities provide consistent authorization checks
// with proper error handling and logging.
//
// =============================================================================

use crate::application::http::server::api_entities::ApiError;
use crate::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::authorization::permission_service::PermissionService;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SelfIdentity {
    identity: Identity,
    user_id: Uuid,
}

impl SelfIdentity {
    pub fn require(identity: Identity) -> Result<Self, ApiError> {
        if !identity.is_user() {
            return Err(ApiError::forbidden(
                "Access denied: authenticated user session required",
            ));
        }

        let user_id = Uuid::parse_str(&identity.user_id())
            .map_err(|_| ApiError::internal("Invalid user ID"))?;

        Ok(Self { identity, user_id })
    }

    pub fn identity(&self) -> &Identity {
        &self.identity
    }

    pub fn into_identity(self) -> Identity {
        self.identity
    }

    pub fn realm_id(&self) -> String {
        self.identity.realm_id()
    }

    pub fn user_id(&self) -> Uuid {
        self.user_id
    }

    pub fn user_id_string(&self) -> String {
        self.user_id.to_string()
    }
}

#[derive(Debug, Clone)]
pub struct AdminIdentity {
    identity: Identity,
    realm_id: String,
    user_id: Uuid,
}

impl AdminIdentity {
    pub fn require(identity: Identity, realm_id: &str, context: &str) -> Result<Self, ApiError> {
        if !identity.is_user() {
            return Err(ApiError::forbidden(format!(
                "Access denied: authenticated user session required for {}",
                context
            )));
        }

        if identity.realm_id() != realm_id {
            return Err(ApiError::forbidden(format!(
                "Access denied: cannot access {} from a different realm",
                context
            )));
        }

        let user_id = Uuid::parse_str(&identity.user_id())
            .map_err(|_| ApiError::internal("Invalid user ID"))?;

        Ok(Self {
            identity,
            realm_id: realm_id.to_string(),
            user_id,
        })
    }

    pub fn identity(&self) -> &Identity {
        &self.identity
    }

    pub fn into_identity(self) -> Identity {
        self.identity
    }

    pub fn realm_id(&self) -> &str {
        &self.realm_id
    }

    pub fn user_id(&self) -> Uuid {
        self.user_id
    }

    pub fn user_id_string(&self) -> String {
        self.user_id.to_string()
    }

    pub async fn require_permission(
        &self,
        state: &AppState,
        resource: &str,
        action: &str,
    ) -> Result<(), ApiError> {
        let allowed = state
            .permission_checker
            .check_permission(&self.realm_id, &self.user_id.to_string(), resource, action)
            .await
            .map_err(|e| {
                tracing::error!(
                    error = %e,
                    realm_id = %self.realm_id,
                    user_id = %self.user_id,
                    resource,
                    action,
                    "Permission check failed"
                );
                ApiError::internal("Failed to check permission")
            })?;

        if !allowed {
            return Err(ApiError::forbidden(format!(
                "Insufficient permissions: requires {resource}.{action}"
            )));
        }

        Ok(())
    }
}

/// Require an authenticated user session and verify realm access.
///
/// This helper ensures that:
/// - The identity represents an authenticated user (not a client/API key)
/// - The user has access to the specified realm
///
/// # Arguments
/// * `identity` - The authenticated identity
/// * `realm_id` - The realm ID to verify access to
/// * `context` - Context description for error messages (e.g., "purchase APIs", "WeChat orders")
///
/// # Returns
/// * `Ok(Uuid)` - The parsed user ID if authorized
/// * `Err(ApiError)` - Forbidden error if unauthorized
pub fn require_authenticated_user_in_realm(
    identity: &Identity,
    realm_id: &str,
    context: &str,
) -> Result<Uuid, ApiError> {
    if !identity.is_user() {
        return Err(ApiError::forbidden(format!(
            "Access denied: authenticated user session required for {}",
            context
        )));
    }

    if !identity.has_access_to_realm(realm_id) {
        return Err(ApiError::forbidden(format!(
            "Access denied: cannot access {} from a different realm",
            context
        )));
    }

    Uuid::parse_str(&identity.user_id()).map_err(|_| ApiError::internal("Invalid user ID"))
}

/// Require realm access for the current identity.
///
/// This helper verifies that the identity has access to the specified realm,
/// preventing cross-realm access attempts.
///
/// # Arguments
/// * `identity` - The authenticated identity
/// * `realm_id` - The realm ID to verify access to
/// * `action` - The action being performed (for error messages)
///
/// # Returns
/// * `Ok(())` - If the identity has access to the realm
/// * `Err(ApiError)` - Forbidden error if cross-realm access attempted
pub fn require_realm_access(
    identity: &Identity,
    realm_id: &str,
    action: &str,
) -> Result<(), ApiError> {
    if identity.realm_id() != realm_id {
        return Err(ApiError::forbidden(format!(
            "Access denied: cannot {} resources in a different realm",
            action
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use herald_core::domain::{
        authentication::Identity, client::entities::ClientApp, common::entities::generate_uuid_v7,
        user::entities::User,
    };

    fn create_test_user(user_id: &str, realm_id: &str) -> User {
        User {
            id: generate_uuid_v7(),
            realm_id: realm_id.to_string(),
            email: format!("{}@example.com", user_id),
            nickname: None,
            password_hash: None,
            provider_ids: vec![],
            status: herald_core::domain::user::entities::UserStatus::Normal,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn create_test_client(client_id: &str, realm_id: &str) -> ClientApp {
        ClientApp {
            id: generate_uuid_v7(),
            realm_id: realm_id.to_string(),
            client_id: client_id.to_string(),
            name: "Test Client".to_string(),
            description: None,
            redirect_uris: vec![],
            enabled: true,
            icon_url: None,
            session_ttl_seconds: 1800,
            session_renewal_ttl_seconds: None,
            client_secret: None,
            device_code_grant_enabled: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_require_realm_access_allows_same_realm() {
        let user = create_test_user("user123", "realm1");
        let identity = Identity::User(user);
        assert!(require_realm_access(&identity, "realm1", "view").is_ok());
    }

    #[test]
    fn test_require_realm_access_blocks_cross_realm() {
        let user = create_test_user("user123", "realm1");
        let identity = Identity::User(user);
        let result = require_realm_access(&identity, "realm2", "view");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("different realm"));
    }

    #[test]
    fn test_require_authenticated_user_in_realm_blocks_non_user() {
        let client = create_test_client("client123", "realm1");
        let identity = Identity::Client(client);
        let result = require_authenticated_user_in_realm(&identity, "realm1", "test action");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("authenticated user session required")
        );
    }

    #[test]
    fn test_require_authenticated_user_in_realm_blocks_cross_realm() {
        let user = create_test_user("user123", "realm1");
        let identity = Identity::User(user);
        let result = require_authenticated_user_in_realm(&identity, "realm2", "test action");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("different realm"));
    }

    #[test]
    fn test_require_authenticated_user_in_realm_returns_user_id() {
        let user_id = "123e4567-e89b-12d3-a456-426614174000";
        let user = create_test_user(user_id, "realm1");
        let expected_id = user.id; // Save the ID before moving user
        let identity = Identity::User(user);
        let result = require_authenticated_user_in_realm(&identity, "realm1", "test action");
        assert!(result.is_ok());
        // Verify we get back the same UUID that was assigned to the user
        assert_eq!(result.unwrap(), expected_id);
    }

    #[test]
    fn test_self_identity_requires_user() {
        let client = create_test_client("client123", "realm1");
        let result = SelfIdentity::require(Identity::Client(client));

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("authenticated user session required")
        );
    }

    #[test]
    fn test_self_identity_wraps_current_user() {
        let user = create_test_user("user123", "realm1");
        let expected_id = user.id;
        let result = SelfIdentity::require(Identity::User(user)).unwrap();

        assert_eq!(result.realm_id(), "realm1");
        assert_eq!(result.user_id(), expected_id);
    }

    #[test]
    fn test_admin_identity_requires_same_realm() {
        let user = create_test_user("user123", "realm1");
        let result = AdminIdentity::require(Identity::User(user), "realm2", "admin test");

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("different realm"));
    }

    #[test]
    fn test_admin_identity_wraps_same_realm_user() {
        let user = create_test_user("user123", "realm1");
        let expected_id = user.id;
        let result = AdminIdentity::require(Identity::User(user), "realm1", "admin test").unwrap();

        assert_eq!(result.realm_id(), "realm1");
        assert_eq!(result.user_id(), expected_id);
    }
}
