// Lightweight principal reference for authorization callers.
//
// PrincipalRef is the minimal authorization subject shape needed by the
// RBAC system. It is derived from `Identity` and never persisted directly;
// the backing entity (User or ClientApiKey) remains the source of truth.

/// Principal type constants used across the authorization system.
///
/// Use these instead of bare string literals to avoid typos and ensure consistency.
pub mod principal_types {
    /// A regular user principal.
    pub const USER: &str = "user";
    /// An API key principal (ThirdParty identity).
    pub const API_KEY: &str = "api_key";
    /// An OAuth client principal.
    pub const CLIENT: &str = "client";
}

/// Authorization subject representing an authenticated caller.
///
/// Users map to `principal_type = principal_types::USER` with `principal_id = user_id`.
/// API keys map to `principal_type = principal_types::API_KEY` with `principal_id = client_api_keys.id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrincipalRef {
    /// Principal type (use `principal_types::*` constants)
    pub principal_type: &'static str,
    /// User ID or ClientApiKey ID
    pub principal_id: String,
    /// Realm this principal belongs to
    pub realm_id: String,
}

impl std::fmt::Display for PrincipalRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PrincipalRef(type={}, id={}, realm={})",
            self.principal_type, self.principal_id, self.realm_id
        )
    }
}
