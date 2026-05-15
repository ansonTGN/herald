use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// Custom validator for client_id (alphanumeric, hyphen, underscore only)
fn validate_client_id(client_id: &str) -> Result<(), validator::ValidationError> {
    // Allow alphanumeric characters, hyphens, and underscores (URL-safe)
    if client_id
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        Ok(())
    } else {
        Err(validator::ValidationError::new("invalid client_id format"))
    }
}

/// Request to create a new OAuth client application
#[derive(Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ClientAppCreateRequest {
    /// OAuth client identifier (3-36 characters)
    ///
    /// Used as the client_id in OAuth flows. Must be unique within the realm.
    /// Can contain alphanumeric characters only.
    #[validate(length(min = 3, max = 36))]
    #[validate(custom(function = "validate_client_id"))]
    #[schema(example = "mywebapp")]
    pub client_id: String,

    /// Human-readable application name (1-100 characters)
    ///
    /// Display name shown to users in authorization screens and admin panels.
    #[validate(length(min = 1, max = 100))]
    #[schema(example = "My Web Application")]
    pub name: String,

    /// Detailed description of the client application (max 500 characters)
    ///
    /// Optional description explaining the purpose and functionality of the application.
    #[validate(length(max = 500))]
    #[schema(example = "Internal admin dashboard for organization management")]
    pub description: Option<String>,

    /// Allowed OAuth redirect URIs
    ///
    /// List of valid redirect URIs for OAuth flows. Must be valid HTTPS URLs
    /// (http://localhost is allowed for development). Users will only be redirected
    /// to these URIs after authentication.
    #[schema(example = json!(["https://example.com/callback", "http://localhost:3000/auth/callback"]))]
    pub redirect_uris: Option<Vec<String>>,

    /// Whether this client application is active
    ///
    /// When set to false, the client cannot be used for new OAuth flows.
    #[schema(example = "true")]
    pub enabled: Option<bool>,

    /// URL to client application icon (favicon, logo)
    ///
    /// Optional URL to an image file that will be displayed as the app icon.
    /// Should be a valid HTTPS URL to an image resource.
    #[schema(example = "https://example.com/logo.png")]
    pub icon_url: Option<String>,

    /// Session time-to-live in seconds
    ///
    /// After this period, the user must re-authenticate. Sessions are invalidated
    /// after this duration expires. If not set, a default value will be used.
    #[schema(example = "3600")]
    pub session_ttl_seconds: Option<i32>,

    /// Session renewal time-to-live in seconds
    ///
    /// If set, sessions can be renewed (refreshed) within this window without
    /// requiring full re-authentication. Allows for "sliding session" behavior.
    /// Must be greater than session_ttl_seconds if set.
    #[schema(example = "86400")]
    pub session_renewal_ttl_seconds: Option<i32>,

    pub device_code_grant_enabled: Option<bool>,
}

// Client ID regex: alphanumeric only
// Allows letters and numbers (e.g., "testclient123")

/// Request to update an existing OAuth client application
///
/// All fields are optional - only provided fields will be updated.
#[derive(Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ClientAppUpdateRequest {
    /// Client application name (1-36 characters)
    ///
    /// Updates the display name shown to users in authorization screens.
    #[validate(length(min = 1, max = 36))]
    #[schema(example = "Updated Web App")]
    pub name: Option<String>,

    /// Detailed description of the client application (max 255 characters)
    ///
    /// Updates the description explaining the purpose of the application.
    #[validate(length(max = 255))]
    #[schema(example = "Updated admin dashboard with new features")]
    pub description: Option<String>,

    /// Allowed OAuth redirect URIs
    ///
    /// Updates the list of valid redirect URIs for OAuth flows. Must be valid HTTPS URLs
    /// (http://localhost is allowed for development). Users will only be redirected
    /// to these URIs after authentication.
    #[schema(example = json!(["https://example.com/callback", "https://app.example.com/auth/callback"]))]
    pub redirect_uris: Option<Vec<String>>,

    /// Whether this client application is active
    ///
    /// When set to false, the client cannot be used for new OAuth flows.
    #[schema(example = "true")]
    pub enabled: Option<bool>,

    /// URL to client application icon (favicon, logo)
    ///
    /// Updates the URL to an image file that will be displayed as the app icon.
    /// Should be a valid HTTPS URL to an image resource.
    #[schema(example = "https://example.com/new-logo.png")]
    pub icon_url: Option<String>,

    /// Session time-to-live in seconds
    ///
    /// Updates the session duration. After this period, the user must re-authenticate.
    #[schema(example = "7200")]
    pub session_ttl_seconds: Option<i32>,

    /// Session renewal time-to-live in seconds
    ///
    /// Updates the session renewal window. If set, sessions can be renewed within this
    /// window without requiring full re-authentication.
    #[schema(example = "172800")]
    pub session_renewal_ttl_seconds: Option<i32>,

    /// Regenerate client secret
    ///
    /// Set to true to generate a new client_secret. The old secret will be immediately
    /// invalidated and cannot be recovered. The new secret will be returned in the response.
    /// **Store this securely** - it will not be retrievable after this response.
    #[schema(example = "false")]
    pub regenerate_secret: Option<bool>,

    pub device_code_grant_enabled: Option<bool>,
}

// Database model (used with sqlx::FromRow)
#[derive(Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ClientAppDbModel {
    pub id: Uuid,
    pub realm_id: String,
    pub client_id: String,
    pub name: String,
    pub description: Option<String>,

    // New fields for Client App settings
    pub redirect_uris: Json<Vec<String>>,
    pub enabled: bool,
    pub icon_url: Option<String>,
    pub session_ttl_seconds: i32,
    pub session_renewal_ttl_seconds: Option<i32>,
    pub client_secret: Option<String>,
    pub device_code_grant_enabled: bool,
}

// API response model (used for OpenAPI documentation)
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClientAppItem {
    /// Client application UUID
    ///
    /// Internal unique identifier for the client application.
    #[schema(example = "01234567-89ab-cdef-0123-456789abcdef")]
    pub id: Uuid,

    /// Realm this client belongs to
    ///
    /// The realm ID that this client application is associated with.
    #[schema(example = "my-realm")]
    pub realm_id: String,

    /// OAuth client_id (used in OAuth flows)
    ///
    /// The public identifier used in OAuth authorization flows.
    #[schema(example = "my-web-app")]
    pub client_id: String,

    /// Human-readable application name
    ///
    /// Display name shown to users in authorization screens and admin panels.
    #[schema(example = "My Web Application")]
    pub name: String,

    /// Detailed description
    ///
    /// Description explaining the purpose and functionality of the application.
    #[schema(example = "Internal admin dashboard for organization management")]
    pub description: Option<String>,

    /// Allowed OAuth redirect URIs
    ///
    /// List of valid redirect URIs for OAuth flows. Users will only be redirected
    /// to these URIs after authentication.
    #[schema(example = json!(["https://example.com/callback", "http://localhost:3000/auth/callback"]))]
    pub redirect_uris: Vec<String>,

    /// Whether this client is currently enabled
    ///
    /// When false, the client cannot be used for new OAuth flows.
    #[schema(example = "true")]
    pub enabled: bool,

    /// Application icon URL
    ///
    /// URL to an image file that will be displayed as the app icon.
    #[schema(example = "https://example.com/logo.png")]
    pub icon_url: Option<String>,

    /// Session TTL in seconds
    ///
    /// After this period, the user must re-authenticate.
    #[schema(example = "3600")]
    pub session_ttl_seconds: i32,

    /// Session renewal TTL in seconds
    ///
    /// If set, sessions can be renewed within this window without requiring
    /// full re-authentication.
    #[schema(example = "86400")]
    pub session_renewal_ttl_seconds: Option<i32>,

    /// OAuth client secret (only shown during creation)
    ///
    /// The secret key used for confidential client authentication in OAuth flows.
    /// **Store this securely** - it will not be retrievable after the initial creation response.
    /// This field will be null in update and list responses.
    #[schema(example = "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx")]
    pub client_secret: Option<String>,

    pub device_code_grant_enabled: bool,
}

// Conversion from DB model to API response model
impl From<ClientAppDbModel> for ClientAppItem {
    fn from(db_model: ClientAppDbModel) -> Self {
        Self {
            id: db_model.id,
            realm_id: db_model.realm_id,
            client_id: db_model.client_id,
            name: db_model.name,
            description: db_model.description,
            redirect_uris: db_model.redirect_uris.0,
            enabled: db_model.enabled,
            icon_url: db_model.icon_url,
            session_ttl_seconds: db_model.session_ttl_seconds,
            session_renewal_ttl_seconds: db_model.session_renewal_ttl_seconds,
            client_secret: db_model.client_secret,
            device_code_grant_enabled: db_model.device_code_grant_enabled,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListQuery {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
    // Removed realm_id - it will be extracted from path parameter
}

fn default_page() -> i64 {
    0 // Start from 0 consistently
}

fn default_page_size() -> i64 {
    20
}

/// Pagination metadata for legacy list responses.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaginationMeta {
    #[schema(example = "0")]
    pub page: i64,
    #[schema(example = "20")]
    pub page_size: i64,
    #[schema(example = "42")]
    pub total_count: i64,
    #[schema(example = "3")]
    pub total_pages: i64,
}
