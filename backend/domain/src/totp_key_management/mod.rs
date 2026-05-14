//! TOTP Key Management Domain Module
//!
//! This module provides realm-level TOTP key management functionality.
//! It defines the repository and service ports for managing realm TOTP encryption keys.
//!
//! Key features:
//! - Realm-level TOTP key storage (replaces environment variable approach)
//! - Key initialization for new realms
//! - Key retrieval for TOTP secret encryption/decryption
//!
//! Note: Key rotation is NOT implemented. The key_version field is reserved for future extension.

pub mod ports;
pub mod service;

pub use ports::{RealmTotpKeyRepository, RealmTotpKeyService, RealmTotpKeyVersion};
pub use service::RealmTotpKeyServiceImpl;
