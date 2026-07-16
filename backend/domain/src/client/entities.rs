use crate::common::entities::Entity;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
pub struct ClientApp {
    pub id: Uuid,
    pub realm_id: String,
    pub client_id: String,
    pub name: String,
    pub description: Option<String>,

    // New fields for Client App settings
    #[schema(example = json!(["https://example.com/callback"]))]
    pub redirect_uris: Vec<String>,
    pub allowed_origins: Vec<String>,
    pub email_verify_return_url: Option<String>,
    pub password_reset_return_url: Option<String>,
    pub browser_refresh_absolute_ttl_seconds: i32,
    pub is_first_party: bool,
    #[schema(example = true)]
    pub enabled: bool,
    pub icon_url: Option<String>,
    pub client_secret: Option<String>,
    pub device_code_grant_enabled: bool,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Entity for ClientApp {
    fn id(&self) -> Uuid {
        self.id
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

// 辅助方法
impl ClientApp {
    pub fn id_as_string(&self) -> String {
        self.id.to_string()
    }
}

#[derive(Debug, Clone)]
pub struct CreateClientAppConfig {
    pub realm_id: String,
    pub client_id: String,
    pub name: String,
    pub description: Option<String>,

    // New fields for Client App settings
    // redirect_uris is optional during creation (can be added later)
    pub redirect_uris: Option<Vec<String>>,
    pub enabled: Option<bool>,
    pub icon_url: Option<String>,
    pub device_code_grant_enabled: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct UpdateClientAppConfig {
    pub name: Option<String>,
    pub description: Option<String>,

    // New fields for Client App settings
    pub redirect_uris: Option<Vec<String>>,
    pub enabled: Option<bool>,
    pub icon_url: Option<String>,
    pub device_code_grant_enabled: Option<bool>,
    pub regenerate_secret: bool,
}
