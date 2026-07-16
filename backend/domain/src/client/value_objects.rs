use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Validate)]
pub struct CreateClientAppRequest {
    #[validate(length(min = 1))]
    pub realm_id: String,
    #[validate(length(min = 3, max = 36))]
    pub client_id: String,
    #[validate(length(min = 1))]
    pub name: String,
    pub description: Option<String>,

    // New fields for Client App settings
    // redirect_uris is optional during creation (can be added later)
    pub redirect_uris: Option<Vec<String>>,
    pub allowed_origins: Option<Vec<String>>,
    pub email_verify_return_url: Option<String>,
    pub password_reset_return_url: Option<String>,
    pub browser_refresh_absolute_ttl_seconds: Option<i32>,
    pub enabled: Option<bool>,
    pub icon_url: Option<String>,
    pub device_code_grant_enabled: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Validate)]
pub struct UpdateClientAppRequest {
    #[validate(length(min = 1))]
    pub name: Option<String>,
    pub description: Option<String>,

    // New fields for Client App settings
    pub redirect_uris: Option<Vec<String>>,
    pub allowed_origins: Option<Vec<String>>,
    pub email_verify_return_url: Option<String>,
    pub password_reset_return_url: Option<String>,
    pub browser_refresh_absolute_ttl_seconds: Option<i32>,
    pub enabled: Option<bool>,
    pub icon_url: Option<String>,
    pub device_code_grant_enabled: Option<bool>,
    pub regenerate_secret: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ClientAppResponse {
    pub id: String, // UUID 字符串
    pub realm_id: String,
    pub client_id: String,
    pub name: String,
    pub description: Option<String>,

    // New fields for Client App settings
    pub redirect_uris: Vec<String>,
    pub allowed_origins: Vec<String>,
    pub email_verify_return_url: Option<String>,
    pub password_reset_return_url: Option<String>,
    pub browser_refresh_absolute_ttl_seconds: i32,
    pub is_first_party: bool,
    pub enabled: bool,
    pub icon_url: Option<String>,
    pub client_secret: Option<String>,
    pub device_code_grant_enabled: bool,

    pub created_at: String,
    pub updated_at: String,
}

impl From<crate::client::entities::ClientApp> for ClientAppResponse {
    fn from(app: crate::client::entities::ClientApp) -> Self {
        Self {
            id: app.id.to_string(),
            realm_id: app.realm_id,
            client_id: app.client_id,
            name: app.name,
            description: app.description,
            redirect_uris: app.redirect_uris,
            allowed_origins: app.allowed_origins,
            email_verify_return_url: app.email_verify_return_url,
            password_reset_return_url: app.password_reset_return_url,
            browser_refresh_absolute_ttl_seconds: app.browser_refresh_absolute_ttl_seconds,
            is_first_party: app.is_first_party,
            enabled: app.enabled,
            icon_url: app.icon_url,
            client_secret: app.client_secret,
            device_code_grant_enabled: app.device_code_grant_enabled,
            created_at: app.created_at.to_rfc3339(),
            updated_at: app.updated_at.to_rfc3339(),
        }
    }
}
