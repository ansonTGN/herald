// Constants for API Key domain module

/// API Key hash format prefix for SHA-256
pub const SHA256_HASH_PREFIX: &str = "sha256:";

/// Fixed salt for deterministic SHA-256 hashing
/// This enables O(1) database lookups by hash.
pub const API_KEY_SALT_V1: &str = "herald-api-key-salt-v1";

/// Returns the active API key salt for hashing.
///
/// Reads from the `HERALD_API_KEY_SALT` environment variable.
/// Falls back to the built-in `API_KEY_SALT_V1` constant if the variable
/// is not set.
pub fn get_active_api_key_salt() -> String {
    std::env::var("HERALD_API_KEY_SALT").unwrap_or_else(|_| API_KEY_SALT_V1.to_string())
}
