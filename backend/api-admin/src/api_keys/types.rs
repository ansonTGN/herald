use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// Request to create a new API Key
#[derive(Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateApiKeyRequest {
    /// Human-readable name for the API key (1-100 characters)
    #[validate(length(min = 1, max = 100))]
    #[schema(example = "Production API Key")]
    pub name: String,

    /// Optional expiration time (ISO 8601 string)
    #[schema(example = "2026-12-31T23:59:59Z")]
    pub expires_at: Option<String>,
}

/// Response after creating an API Key (plaintext key shown once)
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateApiKeyResponse {
    /// API Key UUID
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Plaintext API key (shown only once, store securely)
    pub key: String,

    /// Realm this API key belongs to
    pub realm_id: String,

    /// Whether the API key is enabled
    pub enabled: bool,

    /// Optional expiration time (ISO 8601)
    pub expires_at: Option<String>,

    /// Creation time (ISO 8601)
    pub created_at: String,
}

/// API Key list item (never exposes hash or plaintext)
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyListItem {
    /// API Key UUID
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Realm this API key belongs to
    pub realm_id: String,

    /// Whether the API key is enabled
    pub enabled: bool,

    /// Optional expiration time (ISO 8601)
    pub expires_at: Option<String>,

    /// Last usage time (ISO 8601)
    pub last_used_at: Option<String>,

    /// Total usage count
    pub usage_count: i32,

    /// Creation time (ISO 8601)
    pub created_at: String,
}

/// Request to update an existing API Key
#[derive(Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateApiKeyRequest {
    /// Human-readable name (1-100 characters)
    #[validate(length(min = 1, max = 100))]
    pub name: Option<String>,

    /// Whether the API key is enabled
    pub enabled: Option<bool>,

    /// Optional expiration time (ISO 8601 string, or null to clear)
    pub expires_at: Option<Option<String>>,
}

/// Response after rotating an API Key (plaintext key shown once)
pub type RotateApiKeyResponse = CreateApiKeyResponse;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListQuery {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
}

fn default_page() -> i64 {
    0
}

fn default_page_size() -> i64 {
    20
}
