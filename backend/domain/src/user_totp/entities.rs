use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// User TOTP configuration domain entity
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
pub struct UserTotpConfig {
    pub id: Uuid,
    pub user_id: Uuid,
    pub realm_id: String,
    /// Encrypted TOTP secret key (AES-256-GCM)
    pub secret_hash: String,
    /// Version of realm TOTP key used for encryption
    ///
    /// Note: Key rotation is NOT implemented. This field is reserved for future extension.
    /// Currently fixed at 1 for all user TOTP configurations.
    pub key_version: i32,
    pub enabled: bool,
    pub verified_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl UserTotpConfig {
    pub fn new(user_id: Uuid, realm_id: String, secret_hash: String, key_version: i32) -> Self {
        let now = Utc::now();
        Self {
            id: crate::common::generate_uuid_v7(),
            user_id,
            realm_id,
            secret_hash,
            key_version, // Note: Fixed at 1, reserved for future key rotation
            enabled: false,
            verified_at: None,
            last_used_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn enable(&mut self) {
        self.enabled = true;
        self.verified_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn disable(&mut self) {
        self.enabled = false;
        self.updated_at = Utc::now();
    }

    pub fn update_last_used(&mut self) {
        self.last_used_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn regenerate_secret(&mut self, new_secret_hash: String) {
        self.secret_hash = new_secret_hash;
        self.enabled = false;
        self.verified_at = None;
        self.last_used_at = None;
        self.updated_at = Utc::now();
    }
}

/// User TOTP backup code domain entity
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
pub struct UserTotpBackupCode {
    pub id: i64,
    pub user_totp_config_id: Uuid,
    /// bcrypt hashed backup code
    pub code_hash: String,
    pub used: bool,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl UserTotpBackupCode {
    pub fn new(user_totp_config_id: Uuid, code_hash: String) -> Self {
        Self {
            id: 0, // Set by database
            user_totp_config_id,
            code_hash,
            used: false,
            used_at: None,
            created_at: Utc::now(),
        }
    }

    pub fn mark_as_used(&mut self) {
        self.used = true;
        self.used_at = Some(Utc::now());
    }
}

/// TOTP setup response (shown to user during setup)
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct TotpSetupResponse {
    pub secret: String,
    pub qr_code_url: String,
    pub backup_codes: Vec<String>,
    pub temp_token: String,
}

/// TOTP status response
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct TotpStatusResponse {
    pub enabled: bool,
    pub enabled_at: Option<DateTime<Utc>>,
    pub last_verified_at: Option<DateTime<Utc>>,
    pub backup_codes: BackupCodeStats,
}

/// Backup code statistics
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct BackupCodeStats {
    pub total: i32,
    pub remaining: i32,
    pub used: i32,
}

/// Realm TOTP configuration
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct RealmTotpConfig {
    pub enabled: bool,
    pub force_enabled: bool,
}

/// Realm TOTP statistics
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct RealmTotpStatistics {
    pub total_users: i64,
    pub totp_enabled_users: i64,
    pub totp_disabled_users: i64,
    pub enablement_rate: f64,
}
