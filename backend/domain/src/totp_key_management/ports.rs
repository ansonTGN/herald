use crate::common::entities::app_errors::CoreError;
use std::future::Future;

/// Realm TOTP key version information
///
/// This struct represents metadata about a realm's TOTP encryption key.
/// Note: Key rotation is NOT implemented. This struct is reserved for future extension.
#[derive(Debug, Clone)]
pub struct RealmTotpKeyVersion {
    /// Key version number (currently fixed at 1)
    pub version: i32,
    /// When this key was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Whether this key is currently active
    pub enabled: bool,
}

/// Realm TOTP key management Repository Trait
///
/// This trait defines the interface for storing and retrieving realm-level TOTP encryption keys.
/// Keys are stored in the realm_config table with config_type='totp_key'.
#[cfg_attr(test, mockall::automock)]
pub trait RealmTotpKeyRepository: Send + Sync {
    /// Get the active TOTP key for a realm
    ///
    /// Returns the 32-byte encryption key if one exists and is enabled.
    /// The key is stored in base64-encoded form in the database.
    ///
    /// # Arguments
    /// * `realm_id` - The realm identifier
    ///
    /// # Returns
    /// * `Some([u8; 32])` - The active 32-byte AES-256 key
    /// * `None` - No active key found for the realm
    fn get_active_key(
        &self,
        realm_id: &str,
    ) -> impl Future<Output = Result<Option<[u8; 32]>, CoreError>> + Send;

    /// Create a new TOTP key for a realm
    ///
    /// Creates a new encryption key entry in the realm_config table.
    /// The key is stored with:
    /// - config_type: 'totp_key'
    /// - config_key: 'version_1'
    /// - is_secret: true
    /// - enabled: true
    ///
    /// # Arguments
    /// * `realm_id` - The realm identifier
    /// * `key` - The 32-byte AES-256 encryption key
    ///
    /// # Returns
    /// * `Ok(())` - Key created successfully
    /// * `Err(CoreError)` - Failed to create key
    fn create_key(
        &self,
        realm_id: &str,
        key: [u8; 32],
    ) -> impl Future<Output = Result<(), CoreError>> + Send;
}

/// Realm TOTP key management Service Trait
///
/// This trait defines the interface for initializing and managing realm TOTP keys.
/// The service coordinates key generation and storage.
#[cfg_attr(test, mockall::automock)]
pub trait RealmTotpKeyService: Send + Sync {
    /// Initialize TOTP key for a realm
    ///
    /// Generates a new 32-byte random key and stores it in the realm_config table.
    /// This should be called when a new realm is created.
    ///
    /// Note: Key rotation is NOT implemented. This method only creates the initial key.
    ///
    /// # Arguments
    /// * `realm_id` - The realm identifier
    ///
    /// # Returns
    /// * `Ok(())` - Key initialized successfully
    /// * `Err(CoreError)` - Failed to initialize key
    fn init_realm_key(&self, realm_id: &str) -> impl Future<Output = Result<(), CoreError>> + Send;
}
