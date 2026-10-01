// OAuth entities

use crate::common::CoreError;
use crate::common::entities::Entity;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// OAuth provider configuration entity (stored per realm)
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
pub struct OAuthProviderConfig {
    pub id: Uuid,
    pub realm_id: String,
    pub provider_type: ProviderType,
    pub client_id: String,
    pub client_secret: String,
    pub scopes: Vec<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Entity for OAuthProviderConfig {
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

impl OAuthProviderConfig {
    pub fn new(config: CreateOAuthProviderConfigRequest) -> Result<Self, CoreError> {
        // LDAP identity links reuse the `provider` table (type='ldap') but are
        // never created through the OAuth provider configuration surface;
        // refusing here keeps the admin API from minting an OAuth config row
        // that would collide with directory-login links.
        if config.provider_type == ProviderType::Ldap {
            return Err(CoreError::BadRequest(
                "LDAP is not an OAuth provider".to_string(),
            ));
        }

        let now = Utc::now();
        let scopes = config
            .scopes
            .unwrap_or_else(|| default_scopes(&config.provider_type));

        // Validate scopes
        validate_scopes(&config.provider_type, &scopes)?;

        Ok(Self {
            id: crate::common::entities::generate_uuid_v7(),
            realm_id: config.realm_id,
            provider_type: config.provider_type,
            client_id: config.client_id,
            client_secret: config.client_secret,
            scopes,
            enabled: config.enabled.unwrap_or(true),
            created_at: now,
            updated_at: now,
        })
    }
}

fn default_scopes(provider_type: &ProviderType) -> Vec<String> {
    match provider_type {
        ProviderType::Google => vec![
            "openid".to_string(),
            "email".to_string(),
            "profile".to_string(),
        ],
        ProviderType::GitHub => vec!["user:email".to_string()],
        ProviderType::Facebook => vec!["email".to_string()],
        ProviderType::Apple => vec!["name".to_string(), "email".to_string()],
        ProviderType::Discord => vec!["identify".to_string(), "email".to_string()],
        ProviderType::WeChat => vec!["snsapi_login".to_string()],
        ProviderType::WeChatMiniProgram => vec![],
        // LDAP links carry no OAuth scope concept; the variant exists so the
        // `provider` table round-trips type='ldap' rows.
        ProviderType::Ldap => vec![],
    }
}

// Public so repositories can re-validate on update paths (PUT must uphold
// the same provider-scope contract as create).
pub fn validate_scopes(provider_type: &ProviderType, scopes: &[String]) -> Result<(), CoreError> {
    match provider_type {
        // WeChat website only accepts snsapi_login
        ProviderType::WeChat => {
            for scope in scopes {
                if scope != "snsapi_login" {
                    return Err(CoreError::BadRequest(format!(
                        "Invalid scope for WeChat: {}. Only 'snsapi_login' is allowed.",
                        scope
                    )));
                }
            }
        }
        // WeChat Mini Program doesn't use scopes
        ProviderType::WeChatMiniProgram if !scopes.is_empty() => {
            return Err(CoreError::BadRequest(
                "WeChat Mini Program does not support scopes.".to_string(),
            ));
        }
        ProviderType::WeChatMiniProgram => {}
        // Unreachable via OAuthProviderConfig::new, which rejects Ldap;
        // listed so the match stays exhaustive as the enum grows.
        ProviderType::Ldap => {}
        // Other providers accept any scope for now
        // Could add more strict validation in the future
        _ => {}
    }
    Ok(())
}

/// OAuth provider entity (user's linked OAuth account)
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
pub struct OAuthProvider {
    pub id: Uuid,
    pub realm_id: String,
    pub provider_type: ProviderType,
    pub open_id: String,
    pub union_id: Option<String>,
    pub email: Option<String>,
    pub user_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Entity for OAuthProvider {
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

/// OAuth provider type
///
/// The `Ldap` variant is NOT an OAuth provider: it exists so the `provider`
/// identity-link table can round-trip `type='ldap'` rows created by LDAP
/// login (`PostgresOAuthRepository` parses the type column through this
/// enum). `OAuthProviderConfig::new` rejects it, keeping the OAuth
/// configuration surface from producing ldap rows.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderType {
    Google,
    // The serde spelling must stay identical to FromStr/as_str (one contract
    // shared with the provider-type DB columns and the API's String DTOs);
    // container-level snake_case would split these camel-cased variants
    // ("GitHub" -> "git_hub"), so each carries an explicit rename.
    #[serde(rename = "github")]
    GitHub,
    Facebook,
    Apple,
    Discord,
    #[serde(rename = "wechat")]
    WeChat,
    #[serde(rename = "wechat_miniprogram")]
    WeChatMiniProgram,
    Ldap,
}

impl FromStr for ProviderType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "google" => Ok(ProviderType::Google),
            "github" => Ok(ProviderType::GitHub),
            "facebook" => Ok(ProviderType::Facebook),
            "apple" => Ok(ProviderType::Apple),
            "discord" => Ok(ProviderType::Discord),
            "wechat" => Ok(ProviderType::WeChat),
            "wechat_miniprogram" => Ok(ProviderType::WeChatMiniProgram),
            "ldap" => Ok(ProviderType::Ldap),
            _ => Err(format!("Unknown provider type: {}", s)),
        }
    }
}

impl ProviderType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderType::Google => "google",
            ProviderType::GitHub => "github",
            ProviderType::Facebook => "facebook",
            ProviderType::Apple => "apple",
            ProviderType::Discord => "discord",
            ProviderType::WeChat => "wechat",
            ProviderType::WeChatMiniProgram => "wechat_miniprogram",
            ProviderType::Ldap => "ldap",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ProviderType::Google => "Google",
            ProviderType::GitHub => "GitHub",
            ProviderType::Facebook => "Facebook",
            ProviderType::Apple => "Apple",
            ProviderType::Discord => "Discord",
            ProviderType::WeChat => "WeChat",
            ProviderType::WeChatMiniProgram => "WeChat Mini Program",
            ProviderType::Ldap => "LDAP",
        }
    }
}

impl OAuthProvider {
    pub fn new(config: CreateOAuthProviderConfig) -> Self {
        let now = Utc::now();
        Self {
            id: crate::common::entities::generate_uuid_v7(),
            realm_id: config.realm_id,
            provider_type: config.provider_type,
            open_id: config.open_id,
            union_id: config.union_id,
            email: config.email,
            user_id: config.user_id,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreateOAuthProviderConfig {
    pub realm_id: String,
    pub provider_type: ProviderType,
    pub open_id: String,
    pub union_id: Option<String>,
    pub email: Option<String>,
    pub user_id: Option<Uuid>,
}

/// Request to create/update OAuth provider configuration
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Validate)]
pub struct CreateOAuthProviderConfigRequest {
    pub realm_id: String,
    pub provider_type: ProviderType,
    #[validate(length(min = 1))]
    pub client_id: String,
    #[validate(length(min = 1))]
    pub client_secret: String,
    pub scopes: Option<Vec<String>>,
    pub enabled: Option<bool>,
}

/// Request to update OAuth provider configuration
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Validate)]
pub struct UpdateOAuthProviderConfigRequest {
    #[validate(length(min = 1))]
    pub client_id: Option<String>,
    #[validate(length(min = 1))]
    pub client_secret: Option<String>,
    pub scopes: Option<Vec<String>>,
    pub enabled: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // The enum's string forms are one contract shared across the
    // provider.type / oauth_provider_config.provider_type columns, the
    // admin API's String DTOs and the frontend PROVIDER_TYPES constants.
    // Any surface that serializes the enum directly must emit exactly the
    // FromStr spellings — a divergence (the "git_hub" class of bug) would
    // make serde-produced JSON unparseable by every other surface, so every
    // variant is pinned here in both directions.
    #[test]
    fn provider_type_serde_matches_from_str_and_as_str() {
        for spelling in [
            "google",
            "github",
            "facebook",
            "apple",
            "discord",
            "wechat",
            "wechat_miniprogram",
            "ldap",
        ] {
            let parsed: ProviderType = spelling
                .parse()
                .unwrap_or_else(|e| panic!("FromStr must accept {spelling}: {e}"));
            assert_eq!(parsed.as_str(), spelling);

            let json = serde_json::to_string(&parsed).unwrap();
            assert_eq!(json, format!("\"{spelling}\""));
            let round_tripped: ProviderType = serde_json::from_str(&json)
                .unwrap_or_else(|e| panic!("serde must accept {json}: {e}"));
            assert_eq!(round_tripped, parsed);
        }
    }
}
