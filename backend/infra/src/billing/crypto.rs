//! Secret encryption/decryption module for billing configuration
//!
//! This module provides AES-256-GCM encryption for sensitive data like
//! API secrets and webhook secrets.

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use herald_domain::common::entities::app_errors::CoreError;
use once_cell::sync::Lazy;

/// Encryption key loaded from environment variable
/// Must be 32 bytes (256 bits) for AES-256-GCM
static ENCRYPTION_KEY: Lazy<Result<[u8; 32], String>> = Lazy::new(|| {
    let key_str = match std::env::var("ENCRYPTION_KEY") {
        Ok(v) => v,
        Err(_) => return Err("ENCRYPTION_KEY environment variable not set".to_string()),
    };

    // Decode base64 to get 32 bytes
    let key_bytes = match STANDARD.decode(&key_str) {
        Ok(b) => b,
        Err(e) => return Err(format!("ENCRYPTION_KEY must be valid base64: {e}")),
    };

    if key_bytes.len() != 32 {
        return Err(format!(
            "ENCRYPTION_KEY must be 32 bytes (256 bits), got {} bytes",
            key_bytes.len()
        ));
    }

    let mut key = [0u8; 32];
    key.copy_from_slice(&key_bytes);
    Ok(key)
});

/// Nonce size for AES-256-GCM (96 bits / 12 bytes)
const NONCE_SIZE: usize = 12;

/// Get the encryption key, returning a CoreError if not properly configured.
fn get_encryption_key() -> Result<&'static [u8; 32], CoreError> {
    ENCRYPTION_KEY
        .as_ref()
        .map_err(|e| CoreError::InternalServerError(e.clone()))
}

/// Encrypt a secret using AES-256-GCM
///
/// # Arguments
/// * `secret` - The plaintext secret to encrypt
///
/// # Returns
/// * `Result<String, CoreError>` - Base64-encoded encrypted secret
///   Format: base64(nonce || ciphertext || tag)
///
/// # Errors
/// * `CoreError::InternalServerError` - If encryption fails
///
/// # Example
/// ```rust
/// use crate::billing::encrypt_secret;
///
/// let encrypted = encrypt_secret("my-secret-key").unwrap();
/// println!("Encrypted: {}", encrypted);
/// ```
pub fn encrypt_secret(secret: &str) -> Result<String, CoreError> {
    let key = get_encryption_key()?;
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| {
        tracing::error!("Failed to create cipher: {}", e);
        CoreError::InternalServerError(format!("Cipher init failed: {}", e))
    })?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    // Encrypt the secret
    let ciphertext = cipher.encrypt(&nonce, secret.as_bytes()).map_err(|e| {
        tracing::error!("Failed to encrypt secret: {}", e);
        CoreError::InternalServerError(format!("Encryption failed: {}", e))
    })?;

    // Combine nonce + ciphertext and encode as base64
    let combined = [nonce.as_slice(), &ciphertext].concat();
    Ok(STANDARD.encode(combined))
}

/// Decrypt a secret that was encrypted with AES-256-GCM
///
/// # Arguments
/// * `encrypted_secret` - Base64-encoded encrypted secret
///
/// # Returns
/// * `Result<String, CoreError>` - Decrypted plaintext secret
///
/// # Errors
/// * `CoreError::InvalidWebhookSecret` - If decryption fails (wrong key or corrupted data)
/// * `CoreError::InternalServerError` - For other errors
///
/// # Example
/// ```rust
/// use crate::billing::{encrypt_secret, decrypt_secret};
///
/// let original = "my-secret-key";
/// let encrypted = encrypt_secret(original).unwrap();
/// let decrypted = decrypt_secret(&encrypted).unwrap();
/// assert_eq!(original, decrypted);
/// ```
pub fn decrypt_secret(encrypted_secret: &str) -> Result<String, CoreError> {
    // Decode base64
    let combined = STANDARD.decode(encrypted_secret).map_err(|e| {
        tracing::error!("Failed to decode base64 encrypted secret: {}", e);
        CoreError::InvalidWebhookSecret
    })?;

    // Validate minimum length (nonce + tag)
    if combined.len() < NONCE_SIZE + 16 {
        return Err(CoreError::InvalidWebhookSecret);
    }

    // Split nonce and ciphertext
    let (nonce_bytes, ciphertext) = combined.split_at(NONCE_SIZE);
    let nonce = Nonce::from_slice(nonce_bytes);

    // Create cipher from the encryption key
    let key = get_encryption_key()?;
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| {
        tracing::error!("Failed to create cipher: {}", e);
        CoreError::InternalServerError(format!("Cipher init failed: {}", e))
    })?;
    let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|e| {
        tracing::debug!("Failed to decrypt secret: {}", e);
        CoreError::InvalidWebhookSecret
    })?;

    // Convert bytes to string
    String::from_utf8(plaintext).map_err(|e| {
        tracing::error!("Decrypted data is not valid UTF-8: {}", e);
        CoreError::InvalidWebhookSecret
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::OnceLock;

    /// Set up a test encryption key before running any tests.
    /// Uses a well-known base64-encoded 32-byte key for deterministic testing.
    static TEST_KEY_SETUP: OnceLock<()> = OnceLock::new();

    fn ensure_test_key() {
        TEST_KEY_SETUP.get_or_init(|| {
            // base64 of 32 zero bytes (256 bits for AES-256)
            unsafe {
                std::env::set_var(
                    "ENCRYPTION_KEY",
                    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
                );
            }
        });
    }

    #[test]
    fn test_encrypt_decrypt_secret() {
        ensure_test_key();
        let original = "my-api-secret-key-12345";
        let encrypted = encrypt_secret(original).unwrap();

        // Verify it's not the same as original
        assert_ne!(original, encrypted);

        // Verify it's base64 encoded
        STANDARD.decode(&encrypted).unwrap();

        // Verify we can decrypt it back
        let decrypted = decrypt_secret(&encrypted).unwrap();
        assert_eq!(original, decrypted);
    }

    #[test]
    fn test_decrypt_invalid_base64() {
        ensure_test_key();
        let result = decrypt_secret("not-valid-base64!!!");

        // Should return InvalidWebhookSecret error
        match result {
            Err(CoreError::InvalidWebhookSecret) => (),
            _ => panic!("Expected InvalidWebhookSecret error, got: {:?}", result),
        }
    }

    #[test]
    fn test_decrypt_invalid_ciphertext() {
        ensure_test_key();
        // Create a base64 string that decodes to something too short
        let too_short = STANDARD.encode([0u8; 8]);

        let result = decrypt_secret(&too_short);

        // Should return InvalidWebhookSecret error
        match result {
            Err(CoreError::InvalidWebhookSecret) => (),
            _ => panic!("Expected InvalidWebhookSecret error, got: {:?}", result),
        }
    }

    #[test]
    fn test_encrypt_produces_different_ciphertext() {
        ensure_test_key();
        let secret = "same-secret";

        // Encrypting the same secret twice should produce different ciphertext
        // due to random nonce
        let enc1 = encrypt_secret(secret).unwrap();
        let enc2 = encrypt_secret(secret).unwrap();

        assert_ne!(enc1, enc2);

        // But both should decrypt to the same value
        assert_eq!(decrypt_secret(&enc1).unwrap(), secret);
        assert_eq!(decrypt_secret(&enc2).unwrap(), secret);
    }
}
