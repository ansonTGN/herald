use crate::common::entities::Entity;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
pub struct Session {
    pub token: String,
    pub realm_id: String,
    pub client_id: String,
    pub user_id: String,
    pub expires_at: DateTime<Utc>,
}

impl Entity for Session {
    fn id(&self) -> Uuid {
        Uuid::now_v7() // Placeholder - sessions use string tokens
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.expires_at - chrono::Duration::seconds(1800) // Approximate
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.expires_at
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SessionData {
    pub realm_id: String,
    pub client_id: String,
    pub user_id: String,
    pub client_ip: String,
}

impl Session {
    pub fn new(token: String, data: SessionData, expires_at: DateTime<Utc>) -> Self {
        Self {
            token,
            realm_id: data.realm_id,
            client_id: data.client_id,
            user_id: data.user_id,
            expires_at,
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}
