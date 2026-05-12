// Client API Key entities
//
// This module defines the domain entities for client API key management.
// Following six-sided architecture principles, this layer contains ZERO
// external dependencies (no sea_orm, redis, http clients).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Client API Key entity
///
/// Represents an API key used by client applications to authenticate
/// with the Herald system. API keys are bound to a specific realm and optionally
/// to a specific Client App for fine-grained access control.
///
/// # Lifecycle
/// 1. Created via `ClientApiKeyService::generate_api_key()` (UUID v7)
/// 2. Hashed with SHA-256 before storage
/// 3. Validated on each API request via O(1) hash-based lookup
/// 4. Usage statistics updated asynchronously
///
/// # Example
/// ```rust,no_run
/// use herald_core::domain::client_api_keys::entities::ClientApiKey;
/// use chrono::Utc;
///
/// let api_key = ClientApiKey {
///     id: "017f22e2-79b0-7cc3-98c4-dc0c0c07398f".to_string(),
///     name: "Production API Key".to_string(),
///     api_key_hash: "sha256:a1b2c3...".to_string(),
///     realm_id: "realm-123".to_string(),
///     client_app_id: Some("app-456".to_string()),
///     enabled: true,
///     expires_at: None,
///     created_at: Utc::now(),
///     last_used_at: None,
///     usage_count: 0,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientApiKey {
    /// Unique identifier (UUID v7, time-ordered)
    pub id: String,

    /// Human-readable name for the API key
    pub name: String,

    /// SHA-256 hash of the API key
    /// Format: "sha256:<hex>"
    pub api_key_hash: String,

    /// Realm this API key belongs to (for multi-tenant isolation)
    pub realm_id: String,

    /// Client App this API key belongs to (1:1 relationship, optional for backward compatibility)
    pub client_app_id: Option<uuid::Uuid>,

    /// Whether the API key is currently enabled
    pub enabled: bool,

    /// Optional expiration time (None = never expires)
    pub expires_at: Option<DateTime<Utc>>,

    /// Timestamp when the API key was created
    pub created_at: DateTime<Utc>,

    /// Timestamp of the last successful authentication (None = never used)
    pub last_used_at: Option<DateTime<Utc>>,

    /// Number of times this API key has been successfully used
    pub usage_count: i32,
}

impl ClientApiKey {
    /// Check if this API key is currently valid
    ///
    /// An API key is valid if:
    /// - It is enabled
    /// - It has not expired (expires_at is None or in the future)
    pub fn is_valid(&self) -> bool {
        if !self.enabled {
            return false;
        }

        if let Some(expires_at) = self.expires_at
            && Utc::now() > expires_at
        {
            return false;
        }

        true
    }
}

impl fmt::Display for ClientApiKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let app_info = self
            .client_app_id
            .as_ref()
            .map(|id| format!(", client_app_id={}", id))
            .unwrap_or_default();

        write!(
            f,
            "ClientApiKey(id={}, name={}, realm_id={}, enabled={}, usage_count={}{})",
            self.id, self.name, self.realm_id, self.enabled, self.usage_count, app_info
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_key_valid_when_enabled_and_not_expired() {
        let api_key = ClientApiKey {
            id: "test-id".to_string(),
            name: "Test Key".to_string(),
            api_key_hash: "hash".to_string(),
            realm_id: "realm-1".to_string(),
            client_app_id: None,
            enabled: true,
            expires_at: None,
            created_at: Utc::now(),
            last_used_at: None,
            usage_count: 0,
        };

        assert!(api_key.is_valid());
    }

    #[test]
    fn test_api_key_invalid_when_disabled() {
        let api_key = ClientApiKey {
            id: "test-id".to_string(),
            name: "Test Key".to_string(),
            api_key_hash: "hash".to_string(),
            realm_id: "realm-1".to_string(),
            client_app_id: None,
            enabled: false,
            expires_at: None,
            created_at: Utc::now(),
            last_used_at: None,
            usage_count: 0,
        };

        assert!(!api_key.is_valid());
    }

    #[test]
    fn test_api_key_invalid_when_expired() {
        let api_key = ClientApiKey {
            id: "test-id".to_string(),
            name: "Test Key".to_string(),
            api_key_hash: "hash".to_string(),
            realm_id: "realm-1".to_string(),
            client_app_id: None,
            enabled: true,
            expires_at: Some(Utc::now() - chrono::Duration::days(1)),
            created_at: Utc::now(),
            last_used_at: None,
            usage_count: 0,
        };

        assert!(!api_key.is_valid());
    }
}
