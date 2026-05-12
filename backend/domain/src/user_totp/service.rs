use crate::common::entities::app_errors::CoreError;
use crate::user_totp::entities::{
    BackupCodeStats, TotpSetupResponse, TotpStatusResponse, UserTotpBackupCode, UserTotpConfig,
};
use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use bcrypt::{DEFAULT_COST, hash, verify};
use rand::Rng;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use totp_lite::{Sha256, totp_custom};

/// TOTP verification result
#[derive(Debug, PartialEq)]
pub enum TotpVerificationResult {
    /// Code is valid and not reused
    Valid,
    /// Code has expired (outside valid time window)
    Expired,
    /// Code was already used (replay attack)
    Replay,
}

/// TOTP service - Pure business logic with zero external dependencies
pub struct UserTotpService;

impl UserTotpService {
    /// Generate TOTP secret key (Base32 encoded, 20 bytes)
    pub fn generate_secret() -> String {
        let secret: [u8; 20] = rand::random();
        base32::encode(base32::Alphabet::Rfc4648 { padding: true }, &secret)
    }

    /// Generate backup recovery codes (10 codes, 6 characters each)
    pub fn generate_backup_codes() -> Vec<String> {
        const CHARSET: &[u8] = b"0123456789";
        (0..10)
            .map(|_| {
                (0..6)
                    .map(|_| {
                        let idx = rand::thread_rng().gen_range(0..CHARSET.len());
                        CHARSET[idx] as char
                    })
                    .collect()
            })
            .collect()
    }

    /// Verify TOTP code (supports ±1 time step, 60-second window)
    pub fn verify_totp(secret: &str, code: &str) -> Result<bool, CoreError> {
        // Decode Base32 secret
        let secret_bytes = base32::decode(base32::Alphabet::Rfc4648 { padding: true }, secret)
            .ok_or(CoreError::InternalServerError(
                "Invalid TOTP secret format".to_string(),
            ))?;

        // Get current time
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| CoreError::InternalServerError(format!("System time error: {}", e)))?
            .as_secs();

        // Check current step and ±1 step (30-second steps)
        for offset in -1i64..=1 {
            let time = (current_time as i64 + offset * 30) as u64;
            let expected_code = totp_custom::<Sha256>(30, 6, &secret_bytes, time);
            if expected_code == code {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Verify TOTP code with replay protection and proper expiration
    ///
    /// This function implements strict TOTP verification to prevent replay attacks
    /// and properly reject expired codes.
    ///
    /// # Arguments
    /// * `secret` - The TOTP secret key (Base32 encoded)
    /// * `code` - The TOTP code to verify (6 digits)
    /// * `last_code_data` - Optional string containing "code:timestamp" of the last successful verification
    ///
    /// # Returns
    /// * `TotpVerificationResult::Valid` - Code is valid and not reused
    /// * `TotpVerificationResult::Expired` - Code has expired (outside valid time window)
    /// * `TotpVerificationResult::Replay` - Code was already used
    pub fn verify_totp_with_replay_protection(
        secret: &str,
        code: &str,
        last_code_data: Option<&str>,
    ) -> Result<TotpVerificationResult, CoreError> {
        // Decode Base32 secret
        let secret_bytes = base32::decode(base32::Alphabet::Rfc4648 { padding: true }, secret)
            .ok_or(CoreError::InternalServerError(
                "Invalid TOTP secret format".to_string(),
            ))?;

        // Get current time
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| CoreError::InternalServerError(format!("System time error: {}", e)))?
            .as_secs();

        // Check for replay attack: if the same code was already used
        if let Some(last_data) = last_code_data
            && let Some((last_code, last_timestamp_str)) = last_data.split_once(':')
            && last_code == code
        {
            // Same code was used before - check if it's still within the same time step
            if let Ok(last_timestamp) = last_timestamp_str.parse::<u64>() {
                // Calculate time steps
                let last_step = last_timestamp / 30;
                let current_step = current_time / 30;

                // If same code from same or previous step, it's a replay attack
                if last_step >= current_step.saturating_sub(1) {
                    tracing::debug!(
                        code = %code,
                        last_step = last_step,
                        current_step = current_step,
                        "Replay attack detected"
                    );
                    return Ok(TotpVerificationResult::Replay);
                }
            }
        }

        // Verify code against current and next time steps only (NOT previous step)
        // This ensures codes older than 30 seconds are rejected
        let mut is_valid = false;

        for offset in 0i64..=1 {
            let time = (current_time as i64 + offset * 30) as u64;
            let expected_code = totp_custom::<Sha256>(30, 6, &secret_bytes, time);
            if expected_code == code {
                is_valid = true;
                break;
            }
        }

        if !is_valid {
            // Code doesn't match current or next step
            // It might match the previous step, but we explicitly reject those
            return Ok(TotpVerificationResult::Expired);
        }

        // Additional check: if the code matches the previous step, it's expired
        let previous_time = (current_time as i64 - 30) as u64;
        let previous_code = totp_custom::<Sha256>(30, 6, &secret_bytes, previous_time);
        if previous_code == code {
            tracing::debug!(
                code = %code,
                "Code expired (matches previous time step)"
            );
            return Ok(TotpVerificationResult::Expired);
        }

        Ok(TotpVerificationResult::Valid)
    }

    /// Encrypt TOTP secret using AES-256-GCM
    pub fn encrypt_secret(secret: &str) -> Result<String, CoreError> {
        // Get encryption key from environment variable
        let key_bytes = env::var("TOTP_SECRET_KEY")
            .map_err(|_| {
                CoreError::InternalServerError(
                    "TOTP_SECRET_KEY environment variable not set".to_string(),
                )
            })?
            .as_bytes()
            .to_vec();

        // Ensure key is exactly 32 bytes (AES-256)
        let key = if key_bytes.len() >= 32 {
            &key_bytes[..32]
        } else {
            return Err(CoreError::InternalServerError(
                "TOTP_SECRET_KEY must be at least 32 bytes".to_string(),
            ));
        };

        let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| {
            CoreError::InternalServerError("Failed to create AES cipher".to_string())
        })?;

        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher
            .encrypt(&nonce, secret.as_bytes())
            .map_err(|_| CoreError::InternalServerError("Failed to encrypt secret".to_string()))?;

        // Format: nonce:ciphertext (both hex-encoded)
        Ok(format!(
            "{}:{}",
            hex::encode(nonce),
            hex::encode(ciphertext)
        ))
    }

    /// Decrypt TOTP secret using AES-256-GCM
    pub fn decrypt_secret(encrypted: &str) -> Result<String, CoreError> {
        // Get encryption key from environment variable
        let key_bytes = env::var("TOTP_SECRET_KEY")
            .map_err(|_| {
                CoreError::InternalServerError(
                    "TOTP_SECRET_KEY environment variable not set".to_string(),
                )
            })?
            .as_bytes()
            .to_vec();

        // Ensure key is exactly 32 bytes (AES-256)
        let key = if key_bytes.len() >= 32 {
            &key_bytes[..32]
        } else {
            return Err(CoreError::InternalServerError(
                "TOTP_SECRET_KEY must be at least 32 bytes".to_string(),
            ));
        };

        // Parse encrypted format: nonce:ciphertext
        let parts: Vec<&str> = encrypted.split(':').collect();
        if parts.len() != 2 {
            return Err(CoreError::InternalServerError(
                "Invalid encrypted secret format".to_string(),
            ));
        }

        let nonce_bytes = hex::decode(parts[0])
            .map_err(|_| CoreError::InternalServerError("Invalid nonce encoding".to_string()))?;
        let ciphertext = hex::decode(parts[1]).map_err(|_| {
            CoreError::InternalServerError("Invalid ciphertext encoding".to_string())
        })?;

        let nonce = Nonce::from_slice(&nonce_bytes);
        let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| {
            CoreError::InternalServerError("Failed to create AES cipher".to_string())
        })?;

        let plaintext = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|_| CoreError::InternalServerError("Failed to decrypt secret".to_string()))?;

        String::from_utf8(plaintext).map_err(|_| {
            CoreError::InternalServerError("Decrypted data is not valid UTF-8".to_string())
        })
    }

    /// Hash backup code using bcrypt
    pub fn hash_backup_code(code: &str) -> Result<String, CoreError> {
        hash(code, DEFAULT_COST)
            .map_err(|_| CoreError::InternalServerError("Failed to hash backup code".to_string()))
    }

    /// Verify backup code against hash
    pub fn verify_backup_code(code: &str, hash: &str) -> Result<bool, CoreError> {
        verify(code, hash)
            .map_err(|_| CoreError::InternalServerError("Failed to verify backup code".to_string()))
    }

    /// Generate QR code URL for TOTP apps
    pub fn generate_qr_code_url(secret: &str, account_email: &str, issuer: &str) -> String {
        format!(
            "otpauth://totp/{}:{}?secret={}&issuer={}&algorithm=SHA256&digits=6&period=30",
            issuer, account_email, secret, issuer
        )
    }

    /// Create TOTP setup response
    pub fn create_setup_response(
        secret: String,
        backup_codes: Vec<String>,
        account_email: &str,
        realm_id: &str,
        temp_token: String,
    ) -> TotpSetupResponse {
        let qr_code_url = Self::generate_qr_code_url(&secret, account_email, realm_id);

        TotpSetupResponse {
            secret,
            qr_code_url,
            backup_codes,
            temp_token,
        }
    }

    /// Create TOTP status response
    pub fn create_status_response(
        config: &UserTotpConfig,
        backup_code_stats: BackupCodeStats,
    ) -> TotpStatusResponse {
        TotpStatusResponse {
            enabled: config.enabled,
            enabled_at: config.verified_at,
            last_verified_at: config.last_used_at,
            backup_codes: backup_code_stats,
        }
    }

    /// Verify TOTP code or backup code during login
    ///
    /// This is the main business logic for TOTP verification during the login process.
    /// It handles both TOTP code verification (with replay protection) and backup code verification.
    ///
    /// # Arguments
    /// * `totp_config` - The user's TOTP configuration
    /// * `code` - Optional TOTP code (6 digits)
    /// * `backup_code` - Optional backup code (6 alphanumeric characters)
    /// * `backup_codes` - List of unused backup codes from the database
    /// * `last_code_data` - Optional string containing "code:timestamp" of the last successful verification
    ///
    /// # Returns
    /// * `Ok(TotpVerificationResult::Valid)` - Verification succeeded
    /// * `Ok(TotpVerificationResult::Expired)` - TOTP code expired
    /// * `Ok(TotpVerificationResult::Replay)` - TOTP code was reused
    /// * `Ok(None)` - Backup code verification succeeded (returns the code ID to mark as used)
    /// * `Err(CoreError)` - Verification error (invalid backup code, missing parameters, etc.)
    pub fn verify_totp_or_backup_code(
        totp_config: &UserTotpConfig,
        code: Option<String>,
        backup_code: Option<String>,
        backup_codes: Vec<UserTotpBackupCode>,
        last_code_data: Option<&str>,
    ) -> Result<TotpVerificationResultWithBackup, CoreError> {
        // Validate that either code or backup_code is provided
        if code.is_none() && backup_code.is_none() {
            return Err(CoreError::BadRequest(
                "Either code or backup_code must be provided".to_string(),
            ));
        }

        // Verify TOTP code or backup code
        if let Some(code) = code {
            // Verify TOTP code with replay protection
            let secret = Self::decrypt_secret(&totp_config.secret_hash)?;

            // Verify the code with replay protection
            let verification_result =
                Self::verify_totp_with_replay_protection(&secret, &code, last_code_data)?;

            match verification_result {
                TotpVerificationResult::Valid => Ok(TotpVerificationResultWithBackup::Valid),
                TotpVerificationResult::Expired => Ok(TotpVerificationResultWithBackup::Expired),
                TotpVerificationResult::Replay => Ok(TotpVerificationResultWithBackup::Replay),
            }
        } else if let Some(backup_code) = backup_code {
            // Verify backup code
            // Note: bcrypt produces different hashes for the same plaintext, so we need to verify against all unused codes
            let unused_codes = backup_codes
                .into_iter()
                .filter(|code| !code.used)
                .collect::<Vec<_>>();

            let mut verified_code_id: Option<i64> = None;
            for backup in &unused_codes {
                let verified = Self::verify_backup_code(&backup_code, &backup.code_hash)?;
                if verified {
                    verified_code_id = Some(backup.id);
                    break;
                }
            }

            if let Some(code_id) = verified_code_id {
                Ok(TotpVerificationResultWithBackup::BackupCodeUsed(code_id))
            } else {
                Ok(TotpVerificationResultWithBackup::Expired)
            }
        } else {
            Err(CoreError::BadRequest(
                "Either code or backup_code must be provided".to_string(),
            ))
        }
    }
}

/// Extended verification result that includes backup code ID
#[derive(Debug, PartialEq)]
pub enum TotpVerificationResultWithBackup {
    /// TOTP code is valid and not reused
    Valid,
    /// TOTP code has expired (outside valid time window)
    Expired,
    /// TOTP code was already used (replay attack)
    Replay,
    /// Backup code was used successfully (contains the backup code ID)
    BackupCodeUsed(i64),
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    // =========================================================================
    // Unit Tests: TOTP Secret Generation
    // =========================================================================

    #[test]
    fn test_unit_generate_secret() {
        let secret = UserTotpService::generate_secret();
        assert!(!secret.is_empty());
        assert!(secret.len() >= 26); // Base32 encoding of 20 bytes
        // Verify it's valid Base32
        assert!(secret.chars().all(|c| c.is_alphanumeric() || c == '='));
    }

    #[test]
    fn test_unit_generate_secret_unique() {
        let secret1 = UserTotpService::generate_secret();
        let secret2 = UserTotpService::generate_secret();
        assert_ne!(secret1, secret2, "Secrets should be unique");
    }

    // =========================================================================
    // Unit Tests: Backup Code Generation
    // =========================================================================

    #[test]
    fn test_unit_generate_backup_codes() {
        let codes = UserTotpService::generate_backup_codes();
        assert_eq!(codes.len(), 10);
        for code in codes {
            assert_eq!(code.len(), 6);
            assert!(code.chars().all(|c| c.is_alphanumeric()));
            assert!(
                code.chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
            );
        }
    }

    #[test]
    fn test_unit_generate_backup_codes_unique() {
        let codes1 = UserTotpService::generate_backup_codes();
        let codes2 = UserTotpService::generate_backup_codes();

        // Check all codes are unique within a batch
        let unique_codes1: std::collections::HashSet<_> = codes1.iter().collect();
        assert_eq!(unique_codes1.len(), 10, "All backup codes should be unique");

        // Check different batches are different
        assert_ne!(codes1, codes2, "Backup code batches should be unique");
    }

    // =========================================================================
    // Unit Tests: Backup Code Hashing and Verification
    // =========================================================================

    #[test]
    fn test_unit_hash_backup_code() {
        let code = "ABC123";
        let hash = UserTotpService::hash_backup_code(code).unwrap();
        assert!(!hash.is_empty());
        assert_ne!(hash, code);
        // bcrypt hashes start with $2b$
        assert!(
            hash.starts_with("$2b$"),
            "bcrypt hash should start with $2b$"
        );
    }

    #[test]
    fn test_unit_verify_backup_code() {
        let code = "ABC123";
        let hash = UserTotpService::hash_backup_code(code).unwrap();
        let result = UserTotpService::verify_backup_code(code, &hash).unwrap();
        assert!(result);
    }

    #[test]
    fn test_unit_verify_backup_code_wrong() {
        let code = "ABC123";
        let wrong_code = "XYZ789";
        let hash = UserTotpService::hash_backup_code(code).unwrap();
        let result = UserTotpService::verify_backup_code(wrong_code, &hash).unwrap();
        assert!(!result);
    }

    // =========================================================================
    // Unit Tests: TOTP Verification
    // =========================================================================

    #[test]
    fn test_unit_verify_totp_current_step() {
        // Generate a secret
        let secret = UserTotpService::generate_secret();

        // Calculate current TOTP code
        let secret_bytes =
            base32::decode(base32::Alphabet::Rfc4648 { padding: true }, &secret).unwrap();
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let expected_code =
            totp_lite::totp_custom::<totp_lite::Sha256>(30, 6, &secret_bytes, current_time);

        // Verify should succeed
        let result = UserTotpService::verify_totp(&secret, &expected_code).unwrap();
        assert!(result);
    }

    #[test]
    fn test_unit_verify_totp_previous_step() {
        let secret = UserTotpService::generate_secret();
        let secret_bytes =
            base32::decode(base32::Alphabet::Rfc4648 { padding: true }, &secret).unwrap();

        // Calculate previous step code (30 seconds ago)
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let previous_time = current_time - 30;
        let previous_code =
            totp_lite::totp_custom::<totp_lite::Sha256>(30, 6, &secret_bytes, previous_time);

        // Verify should succeed (supports time drift)
        let result = UserTotpService::verify_totp(&secret, &previous_code).unwrap();
        assert!(result);
    }

    #[test]
    fn test_unit_verify_totp_next_step() {
        let secret = UserTotpService::generate_secret();
        let secret_bytes =
            base32::decode(base32::Alphabet::Rfc4648 { padding: true }, &secret).unwrap();

        // Calculate next step code (30 seconds ahead)
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let next_time = current_time + 30;
        let next_code =
            totp_lite::totp_custom::<totp_lite::Sha256>(30, 6, &secret_bytes, next_time);

        // Verify should succeed (supports time drift)
        let result = UserTotpService::verify_totp(&secret, &next_code).unwrap();
        assert!(result);
    }

    #[test]
    fn test_unit_verify_totp_wrong_code() {
        let secret = UserTotpService::generate_secret();
        let wrong_code = "000000";

        let result = UserTotpService::verify_totp(&secret, wrong_code).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_unit_verify_totp_invalid_secret() {
        let invalid_secret = "INVALID@#$!";
        let code = "123456";

        let result = UserTotpService::verify_totp(invalid_secret, code);
        assert!(result.is_err(), "Should return error for invalid secret");
    }

    // =========================================================================
    // Unit Tests: TOTP Verification with Replay Protection
    // =========================================================================

    #[test]
    fn test_unit_verify_totp_with_replay_protection_valid_current() {
        let secret = UserTotpService::generate_secret();

        // Calculate current TOTP code
        let secret_bytes =
            base32::decode(base32::Alphabet::Rfc4648 { padding: true }, &secret).unwrap();
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let current_code =
            totp_lite::totp_custom::<totp_lite::Sha256>(30, 6, &secret_bytes, current_time);

        // Verify with no previous code - should succeed
        let result =
            UserTotpService::verify_totp_with_replay_protection(&secret, &current_code, None)
                .unwrap();
        assert_eq!(result, TotpVerificationResult::Valid);
    }

    #[test]
    fn test_unit_verify_totp_with_replay_protection_valid_next_step() {
        let secret = UserTotpService::generate_secret();
        let secret_bytes =
            base32::decode(base32::Alphabet::Rfc4648 { padding: true }, &secret).unwrap();

        // Calculate next step code (30 seconds ahead)
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let next_time = current_time + 30;
        let next_code =
            totp_lite::totp_custom::<totp_lite::Sha256>(30, 6, &secret_bytes, next_time);

        // Verify with no previous code - should succeed
        let result =
            UserTotpService::verify_totp_with_replay_protection(&secret, &next_code, None).unwrap();
        assert_eq!(result, TotpVerificationResult::Valid);
    }

    #[test]
    fn test_unit_verify_totp_with_replay_protection_expired_previous() {
        let secret = UserTotpService::generate_secret();
        let secret_bytes =
            base32::decode(base32::Alphabet::Rfc4648 { padding: true }, &secret).unwrap();

        // Calculate previous step code (30 seconds ago)
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let previous_time = current_time - 30;
        let previous_code =
            totp_lite::totp_custom::<totp_lite::Sha256>(30, 6, &secret_bytes, previous_time);

        // Verify previous step code - should be rejected as expired
        let result =
            UserTotpService::verify_totp_with_replay_protection(&secret, &previous_code, None)
                .unwrap();
        assert_eq!(result, TotpVerificationResult::Expired);
    }

    #[test]
    fn test_unit_verify_totp_with_replay_protection_replay_attack() {
        let secret = UserTotpService::generate_secret();
        let secret_bytes =
            base32::decode(base32::Alphabet::Rfc4648 { padding: true }, &secret).unwrap();

        // Calculate current TOTP code
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let current_code =
            totp_lite::totp_custom::<totp_lite::Sha256>(30, 6, &secret_bytes, current_time);

        // First verification should succeed
        let result1 =
            UserTotpService::verify_totp_with_replay_protection(&secret, &current_code, None)
                .unwrap();
        assert_eq!(result1, TotpVerificationResult::Valid);

        // Create last_code_data for replay attack detection
        let last_code_data = format!("{}:{}", current_code, current_time);

        // Second verification of same code should be rejected (replay attack)
        let result2 = UserTotpService::verify_totp_with_replay_protection(
            &secret,
            &current_code,
            Some(&last_code_data),
        )
        .unwrap();
        assert_eq!(result2, TotpVerificationResult::Replay);
    }

    #[test]
    fn test_unit_verify_totp_with_replay_protection_replay_old_code() {
        let secret = UserTotpService::generate_secret();
        let secret_bytes =
            base32::decode(base32::Alphabet::Rfc4648 { padding: true }, &secret).unwrap();

        // Calculate previous step code (30 seconds ago - one time step before current)
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let old_time = current_time - 30;
        let old_code = totp_lite::totp_custom::<totp_lite::Sha256>(30, 6, &secret_bytes, old_time);

        // Create last_code_data for old code
        let last_code_data = format!("{}:{}", old_code, old_time);

        // Try to verify the old code again - should be rejected as replay
        let result = UserTotpService::verify_totp_with_replay_protection(
            &secret,
            &old_code,
            Some(&last_code_data),
        )
        .unwrap();
        assert_eq!(result, TotpVerificationResult::Replay);
    }

    #[test]
    fn test_unit_verify_totp_with_replay_protection_wrong_code() {
        let secret = UserTotpService::generate_secret();
        let wrong_code = "000000";

        // Verify wrong code - should be rejected as expired (doesn't match any valid step)
        let result =
            UserTotpService::verify_totp_with_replay_protection(&secret, wrong_code, None).unwrap();
        assert_eq!(result, TotpVerificationResult::Expired);
    }

    // =========================================================================
    // Unit Tests: Secret Encryption/Decryption
    // =========================================================================

    #[test]
    fn test_unit_encrypt_decrypt_secret() {
        // Set test environment variable
        unsafe {
            std::env::set_var("TOTP_SECRET_KEY", "test_key_32_bytes_long_1234567890");
        }

        let secret = "JBSWY3DPEHPK3PXP";

        // Encrypt
        let encrypted = UserTotpService::encrypt_secret(secret).unwrap();
        assert_ne!(encrypted, secret);
        assert!(
            encrypted.contains(':'),
            "Encrypted format should contain ':' separator"
        );

        // Decrypt
        let decrypted = UserTotpService::decrypt_secret(&encrypted).unwrap();
        assert_eq!(decrypted, secret);
    }

    #[test]
    fn test_unit_encrypt_secret_missing_key() {
        // Save current value
        let original_value = std::env::var("TOTP_SECRET_KEY").ok();

        // Remove environment variable
        unsafe {
            std::env::remove_var("TOTP_SECRET_KEY");
        }

        let secret = "JBSWY3DPEHPK3PXP";
        let result = UserTotpService::encrypt_secret(secret);

        assert!(
            result.is_err(),
            "Should return error when TOTP_SECRET_KEY is not set"
        );

        // Restore original value
        if let Some(val) = original_value {
            unsafe {
                std::env::set_var("TOTP_SECRET_KEY", val);
            }
        }
    }

    #[test]
    fn test_unit_decrypt_secret_invalid_format() {
        // Ensure environment variable is set
        unsafe {
            std::env::set_var("TOTP_SECRET_KEY", "test_key_32_bytes_long_1234567890");
        }

        let invalid_encrypted = "invalid_format";
        let result = UserTotpService::decrypt_secret(invalid_encrypted);

        assert!(
            result.is_err(),
            "Should return error for invalid encrypted format"
        );
    }

    // =========================================================================
    // Unit Tests: QR Code URL Generation
    // =========================================================================

    #[test]
    fn test_unit_generate_qr_code_url() {
        let secret = "JBSWY3DPEHPK3PXP";
        let url = UserTotpService::generate_qr_code_url(secret, "user@example.com", "Herald");

        assert!(url.contains(secret));
        assert!(url.contains("user@example.com"));
        assert!(url.contains("otpauth://totp/"));
        assert!(url.contains("issuer=Herald"));
        assert!(url.contains("algorithm=SHA256"));
        assert!(url.contains("digits=6"));
        assert!(url.contains("period=30"));
    }

    // =========================================================================
    // Unit Tests: Setup and Status Responses
    // =========================================================================

    #[test]
    fn test_unit_create_setup_response() {
        let secret = "JBSWY3DPEHPK3PXP";
        let backup_codes = vec!["ABC123".to_string(), "DEF456".to_string()];
        let temp_token = "temp_token_123";

        let response = UserTotpService::create_setup_response(
            secret.to_string(),
            backup_codes.clone(),
            "user@example.com",
            "test-realm",
            temp_token.to_string(),
        );

        assert_eq!(response.secret, secret);
        assert!(response.qr_code_url.contains(secret));
        assert_eq!(response.backup_codes, backup_codes);
        assert_eq!(response.temp_token, temp_token);
    }

    #[test]
    fn test_unit_create_status_response() {
        let config = UserTotpConfig {
            id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            realm_id: "test-realm".to_string(),
            secret_hash: "encrypted_secret".to_string(),
            key_version: 1,
            enabled: true,
            verified_at: Some(chrono::Utc::now()),
            last_used_at: Some(chrono::Utc::now()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let backup_stats = BackupCodeStats {
            total: 10,
            remaining: 7,
            used: 3,
        };

        let response = UserTotpService::create_status_response(&config, backup_stats.clone());

        assert!(response.enabled);
        assert!(response.enabled_at.is_some());
        assert!(response.last_verified_at.is_some());
        assert_eq!(response.backup_codes.total, 10);
        assert_eq!(response.backup_codes.remaining, 7);
        assert_eq!(response.backup_codes.used, 3);
    }

    // =========================================================================
    // Unit Tests: TOTP or Backup Code Verification
    // =========================================================================

    #[test]
    fn test_unit_verify_totp_or_backup_code_totp_valid() {
        unsafe {
            std::env::set_var("TOTP_SECRET_KEY", "test_key_32_bytes_long_1234567890");
        }

        let secret = UserTotpService::generate_secret();
        let encrypted = UserTotpService::encrypt_secret(&secret).unwrap();

        let config = UserTotpConfig {
            id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            realm_id: "test-realm".to_string(),
            secret_hash: encrypted,
            key_version: 1,
            enabled: true,
            verified_at: Some(chrono::Utc::now()),
            last_used_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        // Calculate current TOTP code
        let secret_bytes =
            base32::decode(base32::Alphabet::Rfc4648 { padding: true }, &secret).unwrap();
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let current_code =
            totp_lite::totp_custom::<totp_lite::Sha256>(30, 6, &secret_bytes, current_time);

        let result = UserTotpService::verify_totp_or_backup_code(
            &config,
            Some(current_code.to_string()),
            None,
            Vec::new(),
            None,
        )
        .unwrap();

        assert_eq!(result, TotpVerificationResultWithBackup::Valid);
    }

    #[test]
    fn test_unit_verify_totp_or_backup_code_totp_expired() {
        unsafe {
            std::env::set_var("TOTP_SECRET_KEY", "test_key_32_bytes_long_1234567890");
        }

        let secret = UserTotpService::generate_secret();
        let encrypted = UserTotpService::encrypt_secret(&secret).unwrap();

        let config = UserTotpConfig {
            id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            realm_id: "test-realm".to_string(),
            secret_hash: encrypted,
            key_version: 1,
            enabled: true,
            verified_at: Some(chrono::Utc::now()),
            last_used_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        // Use wrong code
        let wrong_code = "000000";
        let result = UserTotpService::verify_totp_or_backup_code(
            &config,
            Some(wrong_code.to_string()),
            None,
            Vec::new(),
            None,
        )
        .unwrap();

        assert_eq!(result, TotpVerificationResultWithBackup::Expired);
    }

    #[test]
    fn test_unit_verify_totp_or_backup_code_replay_attack() {
        unsafe {
            std::env::set_var("TOTP_SECRET_KEY", "test_key_32_bytes_long_1234567890");
        }

        let secret = UserTotpService::generate_secret();
        let encrypted = UserTotpService::encrypt_secret(&secret).unwrap();

        let config = UserTotpConfig {
            id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            realm_id: "test-realm".to_string(),
            secret_hash: encrypted,
            key_version: 1,
            enabled: true,
            verified_at: Some(chrono::Utc::now()),
            last_used_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        // Calculate current TOTP code
        let secret_bytes =
            base32::decode(base32::Alphabet::Rfc4648 { padding: true }, &secret).unwrap();
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let current_code =
            totp_lite::totp_custom::<totp_lite::Sha256>(30, 6, &secret_bytes, current_time);

        // Create last_code_data for replay attack detection
        let last_code_data = format!("{}:{}", current_code, current_time);

        // Try to verify the same code again - should be rejected as replay
        let result = UserTotpService::verify_totp_or_backup_code(
            &config,
            Some(current_code.to_string()),
            None,
            Vec::new(),
            Some(&last_code_data),
        )
        .unwrap();

        assert_eq!(result, TotpVerificationResultWithBackup::Replay);
    }

    #[test]
    fn test_unit_verify_totp_or_backup_code_backup_code_valid() {
        let backup_code = "ABC123";
        let hash = UserTotpService::hash_backup_code(backup_code).unwrap();

        let backup_entry = UserTotpBackupCode {
            id: 1,
            user_totp_config_id: Uuid::now_v7(),
            code_hash: hash,
            used: false,
            used_at: None,
            created_at: chrono::Utc::now(),
        };

        let config = UserTotpConfig {
            id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            realm_id: "test-realm".to_string(),
            secret_hash: "encrypted_secret".to_string(),
            key_version: 1,
            enabled: true,
            verified_at: Some(chrono::Utc::now()),
            last_used_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let result = UserTotpService::verify_totp_or_backup_code(
            &config,
            None,
            Some(backup_code.to_string()),
            vec![backup_entry],
            None,
        )
        .unwrap();

        assert_eq!(result, TotpVerificationResultWithBackup::BackupCodeUsed(1));
    }

    #[test]
    fn test_unit_verify_totp_or_backup_code_backup_code_invalid() {
        let backup_code = "ABC123";
        let hash = UserTotpService::hash_backup_code(backup_code).unwrap();

        let backup_entry = UserTotpBackupCode {
            id: 1,
            user_totp_config_id: Uuid::now_v7(),
            code_hash: hash,
            used: false,
            used_at: None,
            created_at: chrono::Utc::now(),
        };

        let config = UserTotpConfig {
            id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            realm_id: "test-realm".to_string(),
            secret_hash: "encrypted_secret".to_string(),
            key_version: 1,
            enabled: true,
            verified_at: Some(chrono::Utc::now()),
            last_used_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let result = UserTotpService::verify_totp_or_backup_code(
            &config,
            None,
            Some("WRONG_CODE".to_string()),
            vec![backup_entry],
            None,
        );

        assert_eq!(result.unwrap(), TotpVerificationResultWithBackup::Expired);
    }

    #[test]
    fn test_unit_verify_totp_or_backup_code_neither_code_nor_backup() {
        let config = UserTotpConfig {
            id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            realm_id: "test-realm".to_string(),
            secret_hash: "encrypted_secret".to_string(),
            key_version: 1,
            enabled: true,
            verified_at: Some(chrono::Utc::now()),
            last_used_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let result =
            UserTotpService::verify_totp_or_backup_code(&config, None, None, Vec::new(), None);

        assert!(result.is_err());
    }

    #[test]
    fn test_unit_verify_totp_or_backup_code_backup_code_ignored_used() {
        let backup_code = "ABC123";
        let hash = UserTotpService::hash_backup_code(backup_code).unwrap();

        // Create both used and unused backup codes
        let used_backup = UserTotpBackupCode {
            id: 1,
            user_totp_config_id: Uuid::now_v7(),
            code_hash: hash.clone(),
            used: true, // Already used
            used_at: None,
            created_at: chrono::Utc::now(),
        };

        let unused_backup = UserTotpBackupCode {
            id: 2,
            user_totp_config_id: Uuid::now_v7(),
            code_hash: UserTotpService::hash_backup_code("DEF456").unwrap(),
            used: false,
            used_at: None,
            created_at: chrono::Utc::now(),
        };

        let config = UserTotpConfig {
            id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            realm_id: "test-realm".to_string(),
            secret_hash: "encrypted_secret".to_string(),
            key_version: 1,
            enabled: true,
            verified_at: Some(chrono::Utc::now()),
            last_used_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        // Try to use an already used backup code
        let result = UserTotpService::verify_totp_or_backup_code(
            &config,
            None,
            Some(backup_code.to_string()),
            vec![used_backup, unused_backup],
            None,
        );

        assert_eq!(result.unwrap(), TotpVerificationResultWithBackup::Expired);
    }
}
