// Constants for API Key domain module

/// API Key hash format prefix for SHA-256
pub const SHA256_HASH_PREFIX: &str = "sha256:";

/// Fixed salt for deterministic SHA-256 hashing
/// This enables O(1) database lookups by hash.
pub const API_KEY_SALT_V1: &str = "herald-api-key-salt-v1";
