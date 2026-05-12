use crate::common::entities::app_errors::CoreError;
use crate::totp_key_management::ports::{RealmTotpKeyRepository, RealmTotpKeyService};
use rand::RngCore;
use std::sync::Arc;

/// Realm TOTP key management Service implementation
///
/// This service manages realm-level TOTP encryption keys.
/// It generates random keys and stores them via the repository.
///
/// Note: Key rotation is NOT implemented. The key_version field is reserved for future extension.
pub struct RealmTotpKeyServiceImpl<RKR>
where
    RKR: RealmTotpKeyRepository,
{
    key_repository: Arc<RKR>,
}

impl<RKR> RealmTotpKeyServiceImpl<RKR>
where
    RKR: RealmTotpKeyRepository,
{
    /// Create a new RealmTotpKeyService instance
    ///
    /// # Arguments
    /// * `key_repository` - Repository for storing/retrieving realm keys
    pub fn new(key_repository: Arc<RKR>) -> Self {
        Self { key_repository }
    }
}

impl<RKR> RealmTotpKeyService for RealmTotpKeyServiceImpl<RKR>
where
    RKR: RealmTotpKeyRepository,
{
    async fn init_realm_key(&self, realm_id: &str) -> Result<(), CoreError> {
        // Generate 32 bytes of random data for AES-256 key
        let mut realm_key = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut realm_key);

        // Create the key (version fixed at 1)
        self.key_repository.create_key(realm_id, realm_key).await?;

        tracing::info!(
            realm_id = %realm_id,
            "TOTP realm key initialized (version: 1)"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::totp_key_management::ports::MockRealmTotpKeyRepository;
    use std::sync::Arc;

    // =========================================================================
    // Unit Tests: RealmTotpKeyServiceImpl
    // =========================================================================

    #[test]
    fn test_unit_realm_totp_key_service_new() {
        let _mock_repo = Arc::new(MockRealmTotpKeyRepository::new());
        let _service = RealmTotpKeyServiceImpl::new(_mock_repo);

        // Service is created successfully
        // (Can't inspect internal field due to privacy, but we can verify it exists)
    }
}
