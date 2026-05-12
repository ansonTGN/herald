use crate::common::entities::app_errors::CoreError;
use crate::user_totp::entities::{
    BackupCodeStats, RealmTotpConfig, RealmTotpStatistics, UserTotpBackupCode, UserTotpConfig,
};
use std::future::Future;
use uuid::Uuid;

// ============================================================================
// Repository Ports (Traits)
// ============================================================================

#[cfg_attr(test, mockall::automock)]
pub trait UserTotpRepository: Send + Sync {
    /// Create a new TOTP configuration for a user
    fn create_config(
        &self,
        config: UserTotpConfig,
    ) -> impl Future<Output = Result<UserTotpConfig, CoreError>> + Send;

    /// Get TOTP configuration by user ID
    fn get_config_by_user_id(
        &self,
        user_id: Uuid,
    ) -> impl Future<Output = Result<Option<UserTotpConfig>, CoreError>> + Send;

    /// Get TOTP configuration by ID
    fn get_config_by_id(
        &self,
        config_id: Uuid,
    ) -> impl Future<Output = Result<UserTotpConfig, CoreError>> + Send;

    /// Update TOTP configuration
    fn update_config(
        &self,
        config: UserTotpConfig,
    ) -> impl Future<Output = Result<UserTotpConfig, CoreError>> + Send;

    /// Delete TOTP configuration (cascade deletes backup codes)
    fn delete_config(&self, user_id: Uuid) -> impl Future<Output = Result<(), CoreError>> + Send;

    /// Create backup codes for a TOTP configuration
    fn create_backup_codes(
        &self,
        codes: Vec<UserTotpBackupCode>,
    ) -> impl Future<Output = Result<Vec<UserTotpBackupCode>, CoreError>> + Send;

    /// Get backup codes for a TOTP configuration
    fn get_backup_codes(
        &self,
        config_id: Uuid,
    ) -> impl Future<Output = Result<Vec<UserTotpBackupCode>, CoreError>> + Send;

    /// Find an unused backup code by hash
    fn find_unused_backup_code(
        &self,
        config_id: Uuid,
        code_hash: &str,
    ) -> impl Future<Output = Result<Option<UserTotpBackupCode>, CoreError>> + Send;

    /// Mark backup code as used
    fn mark_backup_code_used(
        &self,
        code_id: i64,
    ) -> impl Future<Output = Result<UserTotpBackupCode, CoreError>> + Send;

    /// Delete all backup codes for a configuration
    fn delete_backup_codes(
        &self,
        config_id: Uuid,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    /// Get backup code statistics
    fn get_backup_code_stats(
        &self,
        config_id: Uuid,
    ) -> impl Future<Output = Result<BackupCodeStats, CoreError>> + Send;
}

#[cfg_attr(test, mockall::automock)]
pub trait RealmTotpConfigRepository: Send + Sync {
    /// Get TOTP configuration for a realm
    fn get_realm_totp_config(
        &self,
        realm_id: &str,
    ) -> impl Future<Output = Result<Option<RealmTotpConfig>, CoreError>> + Send;

    /// Update realm TOTP configuration
    fn upsert_realm_totp_config(
        &self,
        realm_id: &str,
        config: RealmTotpConfig,
    ) -> impl Future<Output = Result<RealmTotpConfig, CoreError>> + Send;

    /// Get TOTP statistics for a realm
    fn get_realm_totp_statistics(
        &self,
        realm_id: &str,
    ) -> impl Future<Output = Result<RealmTotpStatistics, CoreError>> + Send;
}
