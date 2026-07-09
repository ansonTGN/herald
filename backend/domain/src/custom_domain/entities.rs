use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// A row of the `custom_domain_mapping` table, surfaced as a domain DTO.
///
/// This is the read shape returned by `CustomDomainMappingRepository` lookups.
/// `enabled` is the sole request-time effectiveness signal (design §5.1);
/// `cname_verified` / `tls_ready` / `status_checked_at` are surface-only status
/// fields for the realm admin config page and are NOT part of host→realm
/// resolution.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
pub struct MappingRow {
    /// Row UUID (uuidv7)
    pub id: Uuid,
    /// Realm this hostname maps to
    #[serde(rename = "realmId")]
    pub realm_id: String,
    /// Precise custom login hostname, globally unique
    pub hostname: String,
    /// Whether this mapping is currently published-effective
    pub enabled: bool,
    /// Whether the hostname's CNAME currently points to Herald's cname target
    #[serde(rename = "cnameVerified")]
    pub cname_verified: bool,
    /// Whether Caddy has issued On-Demand TLS for the hostname
    #[serde(rename = "tlsReady")]
    pub tls_ready: bool,
    /// Last time the CNAME/TLS status was probed
    #[serde(rename = "statusCheckedAt")]
    pub status_checked_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Creation timestamp
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last update timestamp
    #[serde(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
