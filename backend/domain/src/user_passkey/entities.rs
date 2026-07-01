use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct UserPasskeyCredential {
    pub id: Uuid,
    pub user_id: Uuid,
    pub realm_id: String,
    pub credential_id: Vec<u8>,
    pub credential_public_key: Vec<u8>,
    pub counter: u64,
    pub transports: Vec<String>,
    pub aaguid: Option<Uuid>,
    pub backup_eligible: bool,
    pub backup_state: bool,
    pub user_verified: bool,
    pub nickname: Option<String>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl UserPasskeyCredential {
    pub fn to_view(&self) -> PasskeyCredentialView {
        PasskeyCredentialView {
            id: self.id,
            nickname: self.nickname.clone(),
            created_at: self.created_at,
            last_used_at: self.last_used_at,
            backup_eligible: self.backup_eligible,
            backup_state: self.backup_state,
            transports: self.transports.clone(),
            aaguid: self.aaguid,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct PasskeyCredentialView {
    pub id: Uuid,
    pub nickname: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub backup_eligible: bool,
    pub backup_state: bool,
    pub transports: Vec<String>,
    pub aaguid: Option<Uuid>,
}
