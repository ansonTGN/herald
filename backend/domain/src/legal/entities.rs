use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Type of legal agreement. Serialized as snake_case (`terms_of_service`,
/// `privacy_policy`) to align with the DB column values and path params.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgreementType {
    TermsOfService,
    PrivacyPolicy,
}

impl AgreementType {
    /// Stable string identifier persisted in the DB `agreement_type` column and
    /// used in URLs. Keep in sync with the `#[serde(rename_all)]` mapping.
    pub const fn as_str(&self) -> &'static str {
        match self {
            AgreementType::TermsOfService => "terms_of_service",
            AgreementType::PrivacyPolicy => "privacy_policy",
        }
    }
}

impl AsRef<str> for AgreementType {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl TryFrom<&str> for AgreementType {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "terms_of_service" => Ok(AgreementType::TermsOfService),
            "privacy_policy" => Ok(AgreementType::PrivacyPolicy),
            other => Err(format!("unknown agreement type: {other}")),
        }
    }
}

/// Origin of an agreement version. `Default` = platform seed template
/// (`realm_id IS NULL`), `Custom` = per-realm published override.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgreementSource {
    Default,
    Custom,
}

impl AgreementSource {
    pub const fn as_str(&self) -> &'static str {
        match self {
            AgreementSource::Default => "default",
            AgreementSource::Custom => "custom",
        }
    }
}

impl From<&str> for AgreementSource {
    /// `'default'` → `Default`, anything else → `Custom`. Mirrors the
    /// `legal_agreement_version.source` column default (`'custom'`) and the
    /// two seeded platform rows (`source = 'default'`).
    fn from(value: &str) -> Self {
        match value {
            "default" => AgreementSource::Default,
            _ => AgreementSource::Custom,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgreementMode {
    FullText,
    Link,
}

impl AgreementMode {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::FullText => "full_text",
            Self::Link => "link",
        }
    }
}

impl From<&str> for AgreementMode {
    fn from(value: &str) -> Self {
        match value {
            "link" => Self::Link,
            _ => Self::FullText,
        }
    }
}

/// Domain entity for an immutable, append-only legal agreement version.
///
/// Maps 1:1 to `herald_entity::LegalAgreementVersionEntity`. Repository
/// implementations (BE-D03) translate between the DB `content: Json` /
/// `source: &str` columns and `content: serde_json::Value` /
/// `source: AgreementSource` here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct LegalAgreementVersion {
    pub id: Uuid,
    /// `None` = platform default template (scope-wide, `realm_id IS NULL`).
    pub realm_id: Option<String>,
    pub agreement_type: AgreementType,
    pub version_no: i32,
    pub version_label: Option<String>,
    /// locale → body map (e.g. `{ "en": "...", "zh-CN": "..." }`).
    pub content: serde_json::Value,
    pub source: AgreementSource,
    pub mode: AgreementMode,
    pub external_url: Option<String>,
    pub published_at: DateTime<Utc>,
    pub published_by: Option<String>,
}

/// Domain entity for a user's consent to a specific agreement version.
///
/// Maps 1:1 to `herald_entity::UserAgreementConsentEntity`. Unique per
/// `(user_id, agreement_type)` (upsert).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct UserAgreementConsent {
    pub id: Uuid,
    pub user_id: Uuid,
    pub realm_id: String,
    pub agreement_type: AgreementType,
    pub consented_version_id: Uuid,
    pub consented_at: DateTime<Utc>,
}

/// Where a consent record originated. Persisted into audit `details.source`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConsentSource {
    Register,
    Login,
    Reconsent,
    Explicit,
}

impl ConsentSource {
    pub const fn as_str(&self) -> &'static str {
        match self {
            ConsentSource::Register => "register",
            ConsentSource::Login => "login",
            ConsentSource::Reconsent => "reconsent",
            ConsentSource::Explicit => "explicit",
        }
    }
}

impl AsRef<str> for ConsentSource {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// One agreement's reconsent gate verdict, returned by `consent_status`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ConsentStatusItem {
    pub agreement_type: AgreementType,
    pub current_version_id: Uuid,
    pub consented_version_id: Option<Uuid>,
    pub needs_reconsent: bool,
}

/// Public agreement list item (no `content` body). Shared by the public
/// agreements endpoint and the login `consent_required` branch.
///
/// `agreement_type` is a raw `String` here (not the enum) so this DTO doubles
/// as an OpenAPI-facing shape consumed by `herald-api` / `herald-api-auth`
/// without forcing those callers to depend on the enum's serde representation.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LegalAgreementSummary {
    pub agreement_type: String,
    pub version_id: Uuid,
    pub version_no: i32,
    pub effective_at: DateTime<Utc>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub mode: AgreementMode,
    pub external_url: Option<String>,
}

/// Per-realm draft of a custom legal agreement, staged before publish.
///
/// Maps 1:1 to `herald_entity::LegalAgreementDraftEntity`. Distinct from
/// [`LegalAgreementVersion`] (which is append-only and published): a draft is
/// mutable, has no `version_no`, and never affects end-user resolution or the
/// consent gate. Exactly one row exists per `(realm_id, agreement_type)`
/// (enforced by `legal_agreement_draft_realm_type_unique`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct LegalAgreementDraft {
    pub id: Uuid,
    pub realm_id: String,
    pub agreement_type: AgreementType,
    /// locale → body map (e.g. `{ "en": "..." }`), same shape as a published
    /// version's `content`.
    pub content: serde_json::Value,
    pub version_label: Option<String>,
    pub mode: AgreementMode,
    pub external_url: Option<String>,
    pub updated_at: DateTime<Utc>,
    pub updated_by: Option<String>,
}
