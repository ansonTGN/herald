use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
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

    /// Client App to bind this API key to. Defaults to the built-in admin-api-client.
    pub client_app_id: Option<Uuid>,

    /// Optional role IDs to assign to this API key after creation (max 20).
    #[validate(length(max = 20))]
    pub role_ids: Option<Vec<Uuid>>,
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

    /// Client App this API key is bound to
    pub client_app_id: Option<Uuid>,

    /// Client App display name
    pub client_app_name: Option<String>,

    /// Whether the API key is enabled
    pub enabled: bool,

    /// Optional expiration time (ISO 8601)
    pub expires_at: Option<String>,

    /// Creation time (ISO 8601)
    pub created_at: String,

    /// Role binding failure after the key was created. The plaintext key is
    /// still returned so the caller never loses the one-time secret.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_binding_error: Option<String>,
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

    /// Client App this API key is bound to
    pub client_app_id: Option<Uuid>,

    /// Client App display name
    pub client_app_name: Option<String>,

    /// Whether the API key is enabled
    pub enabled: bool,

    /// Optional expiration time (ISO 8601)
    pub expires_at: Option<String>,

    /// Last usage time (ISO 8601)
    pub last_used_at: Option<String>,

    /// Creation time (ISO 8601)
    pub created_at: String,

    /// Roles assigned to this API key
    #[serde(default)]
    pub roles: Vec<ApiKeyRoleSummary>,
}

/// Role summary for API Key list display
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyRoleSummary {
    pub id: Uuid,
    pub name: String,
}

/// Response for API Key roles query
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyRolesResponse {
    pub roles: Vec<ApiKeyRoleDetail>,
}

/// Detailed role info for an API Key
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyRoleDetail {
    pub id: uuid::Uuid,
    pub name: String,
    pub description: Option<String>,
}

/// Request to replace API Key roles (empty array clears all roles; max 20).
#[derive(Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateApiKeyRolesRequest {
    #[validate(length(max = 20))]
    pub role_ids: Vec<uuid::Uuid>,
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
