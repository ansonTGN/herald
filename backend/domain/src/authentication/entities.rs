use crate::authentication::identity::{CredentialClass, CredentialScope};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct BrowserAccessTokenData {
    pub realm_id: String,
    pub client_app_id: Uuid,
    pub user_id: String,
    pub family_id: Uuid,
    pub credential_class: CredentialClass,
    pub allowed_scopes: HashSet<CredentialScope>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct BrowserRefreshTokenData {
    pub realm_id: String,
    pub client_app_id: Uuid,
    pub user_id: String,
    pub family_id: Uuid,
    pub successor_digest: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub absolute_expires_at: DateTime<Utc>,
    pub revoked: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BrowserTokenSet {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub refresh_expires_in: u64,
    pub token_type: String,
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum RefreshError {
    #[error("refresh token invalid or expired")]
    Invalid,
    #[error("refresh token reuse detected; family revoked")]
    ReuseDetected,
    #[error("client mismatch")]
    ClientMismatch,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TargetOperation {
    ChangePassword,
    ChangeEmail,
    BindAuthenticator,
    RemoveAuthenticator,
    DeleteAccount,
    ApplyInvoice,
}

impl TargetOperation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ChangePassword => "change_password",
            Self::ChangeEmail => "change_email",
            Self::BindAuthenticator => "bind_authenticator",
            Self::RemoveAuthenticator => "remove_authenticator",
            Self::DeleteAccount => "delete_account",
            Self::ApplyInvoice => "apply_invoice",
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReauthFactor {
    Password,
    Totp,
    Passkey,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "factor", content = "value")]
pub enum ReauthCredential {
    Password(String),
    Totp(String),
    Passkey {
        challenge_token: String,
        assertion: serde_json::Value,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ReauthResult {
    pub realm_id: String,
    pub client_app_id: Uuid,
    pub user_id: String,
    pub target_operation: TargetOperation,
    pub expires_at: DateTime<Utc>,
    pub consumed: bool,
}
