// Legal agreements (public view) + self-service consent gate.

use axum::{
    Json,
    extract::{Extension, Path, Query, State},
    http::{HeaderMap, StatusCode, header::USER_AGENT},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use herald_api_base::application::http::auth::util::{ClientIp, require_permission};
use herald_core::domain::audit::ActorType;
use herald_core::domain::authentication::Identity;
use herald_core::domain::legal::ConsentSource;
use herald_core::domain::legal::entities::{
    AgreementSource, AgreementType, ConsentStatusItem, LegalAgreementSummary, LegalAgreementVersion,
};
use herald_core::domain::legal::service::AuditActorMeta;

use crate::application::http::server::api_entities::{ApiError, ErrorResponse};
use crate::application::http::state::AppState;

/// Query params shared by the public agreement endpoints. `locale` selects
/// which body is returned in `LegalAgreementDetail.content`; missing/unknown
/// locale falls back to the agreement's default locale (first map key).
#[derive(Debug, Deserialize)]
pub struct LocaleQuery {
    pub locale: Option<String>,
}

/// Full agreement detail (public GET /agreements/{type}).
///
/// Mirrors [`LegalAgreementSummary`] plus the localized `content` body and the
/// optional `version_label`. Summary fields (`title`/`summary`) are always
/// `None` today — the domain `LegalAgreementVersion` has no such columns —
/// but they remain in the schema so the public list/detail shapes line up
/// (and can be populated later without a breaking change).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LegalAgreementDetail {
    pub agreement_type: String,
    pub version_id: Uuid,
    pub version_no: i32,
    pub effective_at: chrono::DateTime<chrono::Utc>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub content: serde_json::Value,
    pub version_label: Option<String>,
}

/// GET /agreements response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AgreementsResponse {
    pub agreements: Vec<LegalAgreementSummary>,
}

/// GET /consent/status response. Reuses the domain [`ConsentStatusItem`] as
/// the response item so the OpenAPI shape stays in lockstep with the
/// reconsent verdict computed by the service.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ConsentStatusResponse {
    pub items: Vec<ConsentStatusItem>,
}

/// POST /consent request body.
#[derive(Debug, Deserialize, ToSchema)]
pub struct RecordConsentRequest {
    pub agreements: Vec<RecordConsentItem>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RecordConsentItem {
    pub agreement_type: String,
    pub version_id: Uuid,
}

fn to_summary(v: &LegalAgreementVersion) -> LegalAgreementSummary {
    LegalAgreementSummary {
        agreement_type: v.agreement_type.as_str().to_string(),
        version_id: v.id,
        version_no: v.version_no,
        effective_at: v.published_at,
        title: None,
        summary: None,
    }
}

fn to_detail(v: &LegalAgreementVersion, locale: Option<&str>) -> LegalAgreementDetail {
    LegalAgreementDetail {
        agreement_type: v.agreement_type.as_str().to_string(),
        version_id: v.id,
        version_no: v.version_no,
        effective_at: v.published_at,
        title: None,
        summary: None,
        content: pick_locale(&v.content, locale),
        version_label: v.version_label.clone(),
    }
}

/// Select a single locale body out of the locale→body map. When `locale` is
/// `None` or absent from the map, fall back to the map's first key (the
/// publisher's default locale). Non-object content is returned as-is so a
/// stray row never panics the serializer.
fn pick_locale(content: &serde_json::Value, locale: Option<&str>) -> serde_json::Value {
    let Some(map) = content.as_object() else {
        return content.clone();
    };
    if let Some(loc) = locale
        && let Some(body) = map.get(loc)
    {
        return body.clone();
    }
    // Fall back to the first entry (default locale), or the original map if empty.
    map.iter()
        .next()
        .map(|(_, body)| body.clone())
        .unwrap_or_else(|| content.clone())
}

/// List the current effective agreements for a realm (Terms + Privacy).
///
/// Public — no `inject_identity`. Each agreement type resolves to its current
/// effective version (realm custom if present, otherwise platform default);
/// missing types are skipped. Returns 404 when no type resolves, which signals
/// a seed-missing deployment anomaly (the platform default templates should
/// always be present).
#[utoipa::path(
    get,
    path = "/api/legal/{realmId}/agreements",
    tag = "legal",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("locale" = Option<String>, Query, description = "Preferred locale for the returned body (falls back to default locale)")
    ),
    responses(
        (status = 200, description = "Current effective agreement summaries", body = AgreementsResponse),
        (status = 404, description = "No effective agreement deployed for this realm", body = ErrorResponse),
        (status = 500, description = "Server error", body = ErrorResponse)
    )
)]
pub async fn list_agreements(
    State(state): State<AppState>,
    Path(realm_id): Path<String>,
    Query(query): Query<LocaleQuery>,
) -> Result<Json<AgreementsResponse>, ApiError> {
    let _ = query; // summaries don't carry bodies; locale only matters for detail
    let service = &state.legal_service;

    let mut agreements = Vec::new();
    for agreement_type in [AgreementType::TermsOfService, AgreementType::PrivacyPolicy] {
        if let Some(version) = service.current_effective(&realm_id, agreement_type).await? {
            agreements.push(to_summary(&version));
        }
    }

    if agreements.is_empty() {
        return Err(ApiError::not_found(
            "No effective legal agreement deployed for this realm",
        ));
    }

    Ok(Json(AgreementsResponse { agreements }))
}

/// Get the full localized detail of a single agreement type for a realm.
///
/// Public — no `inject_identity`. Unknown `agreementType` → 400; no effective
/// version deployed → 404.
#[utoipa::path(
    get,
    path = "/api/legal/{realmId}/agreements/{agreementType}",
    tag = "legal",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("agreementType" = String, Path, description = "Agreement type: terms_of_service | privacy_policy"),
        ("locale" = Option<String>, Query, description = "Preferred locale (falls back to default locale)")
    ),
    responses(
        (status = 200, description = "Agreement detail with localized body", body = LegalAgreementDetail),
        (status = 400, description = "Unknown agreement type", body = ErrorResponse),
        (status = 404, description = "No effective agreement deployed for this type", body = ErrorResponse),
        (status = 500, description = "Server error", body = ErrorResponse)
    )
)]
pub async fn get_agreement(
    State(state): State<AppState>,
    Path((realm_id, agreement_type)): Path<(String, String)>,
    Query(query): Query<LocaleQuery>,
) -> Result<Json<LegalAgreementDetail>, ApiError> {
    let agreement_type =
        AgreementType::try_from(agreement_type.as_str()).map_err(ApiError::bad_request)?;

    let version = state
        .legal_service
        .current_effective(&realm_id, agreement_type)
        .await?
        .ok_or_else(|| {
            ApiError::not_found("No effective legal agreement deployed for this type")
        })?;

    Ok(Json(to_detail(&version, query.locale.as_deref())))
}

/// Reconsent gate verdict for the calling user across both agreement types.
///
/// Self-service — requires `inject_identity`. Cross-realm access is rejected
/// with 403 (each user may only inspect their own realm's consent state).
#[utoipa::path(
    get,
    path = "/api/legal/{realmId}/consent/status",
    tag = "legal",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    responses(
        (status = 200, description = "Per-agreement reconsent verdict", body = ConsentStatusResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Identity does not belong to this realm", body = ErrorResponse),
        (status = 500, description = "Server error", body = ErrorResponse)
    )
)]
pub async fn get_consent_status(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
) -> Result<Json<ConsentStatusResponse>, ApiError> {
    if !identity.has_access_to_realm(&realm_id) {
        return Err(ApiError::forbidden(
            "Identity does not belong to this realm",
        ));
    }

    let user_id = parse_user_id(identity.user_id())?;
    let items = state
        .legal_service
        .consent_status(user_id, &realm_id)
        .await?;
    Ok(Json(ConsentStatusResponse { items }))
}

/// Record the user's explicit consent to one or more agreement versions.
///
/// Self-service — requires `inject_identity`. Each `version_id` must equal the
/// current effective version for its type (enforced in the service layer,
/// BE-D04); a stale version surfaces as 409 so the client re-reads the
/// effective version. Returns 204 on success. The upsert is idempotent on a
/// repeat of the same version.
#[utoipa::path(
    post,
    path = "/api/legal/{realmId}/consent",
    tag = "legal",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    request_body = RecordConsentRequest,
    responses(
        (status = 204, description = "Consent recorded"),
        (status = 400, description = "Unknown agreement type", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Identity does not belong to this realm", body = ErrorResponse),
        (status = 409, description = "version_id is not the current effective version", body = ErrorResponse),
        (status = 500, description = "Server error", body = ErrorResponse)
    )
)]
pub async fn record_consent(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    ClientIp(ip): ClientIp,
    headers: HeaderMap,
    Json(payload): Json<RecordConsentRequest>,
) -> Result<StatusCode, ApiError> {
    if !identity.has_access_to_realm(&realm_id) {
        return Err(ApiError::forbidden(
            "Identity does not belong to this realm",
        ));
    }

    let user_id = parse_user_id(identity.user_id())?;

    let mut items = Vec::with_capacity(payload.agreements.len());
    for item in payload.agreements {
        let agreement_type =
            AgreementType::try_from(item.agreement_type.as_str()).map_err(ApiError::bad_request)?;
        items.push((agreement_type, item.version_id));
    }

    let user_agent = headers
        .get(USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let actor = AuditActorMeta {
        actor_id: identity.user_id(),
        actor_type: ActorType::User,
        actor_name: None,
        ip_address: Some(ip),
        user_agent,
        trace_id: None,
    };

    state
        .legal_service
        .record_consent(user_id, &realm_id, items, ConsentSource::Explicit, actor)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// `identity.user_id()` is a `String`; the legal service takes a `Uuid`. A
/// malformed id means the identity was constructed without a valid user row —
/// surface as 500 rather than silently failing the consent write.
fn parse_user_id(raw: String) -> Result<Uuid, ApiError> {
    Uuid::parse_str(&raw)
        .map_err(|e| ApiError::internal(format!("identity user_id is not a valid uuid: {e}")))
}

// ----------------------------------------------------------------------------
// Admin DTOs
// ----------------------------------------------------------------------------

/// History entry for the admin view: the version-stable fields without the
/// (potentially large) localized `content` body.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LegalAgreementVersionSummary {
    pub version_id: Uuid,
    pub version_no: i32,
    pub effective_at: chrono::DateTime<chrono::Utc>,
    pub source: AgreementSource,
    pub version_label: Option<String>,
}

/// Admin agreement view. `source` reflects whether a realm has any custom
/// version for this type (Custom) or only the platform default (Default).
/// `current_version` is the resolved effective summary; `history` lists prior
/// versions (custom-first, then the default fallback).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AdminAgreementView {
    pub agreement_type: AgreementType,
    pub source: AgreementSource,
    pub current_version: LegalAgreementSummary,
    pub history: Vec<LegalAgreementVersionSummary>,
}

/// GET admin agreements response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AdminAgreementsResponse {
    pub agreements: Vec<AdminAgreementView>,
}

/// PUT publish-custom request body. `content` is a locale → body map and must
/// contain at least one entry (validated by the service layer).
#[derive(Debug, Deserialize, ToSchema)]
pub struct PublishCustomRequest {
    pub content: serde_json::Value,
    pub version_label: Option<String>,
}

/// PUT publish / DELETE revert response: the newly published version's stable
/// identifiers. For revert this is a brand-new snapshot version (new `version_id`),
/// so clients can assert the id changed to trigger user reconsent.
#[derive(Debug, Serialize, ToSchema)]
pub struct PublishVersionResponse {
    pub version_id: Uuid,
    pub version_no: i32,
    pub effective_at: chrono::DateTime<chrono::Utc>,
}

fn to_version_summary(v: &LegalAgreementVersion) -> LegalAgreementVersionSummary {
    LegalAgreementVersionSummary {
        version_id: v.id,
        version_no: v.version_no,
        effective_at: v.published_at,
        source: v.source.clone(),
        version_label: v.version_label.clone(),
    }
}

/// Build the AuditActorMeta for an admin operation. Admin ops are performed by
/// an authenticated user with `ActorType::Admin` (the realm operator managing
/// legal agreements), mirroring how the consent handler derives its actor.
fn admin_actor(
    identity: &Identity,
    ip: Option<String>,
    user_agent: Option<String>,
) -> AuditActorMeta {
    AuditActorMeta {
        actor_id: identity.user_id(),
        actor_type: ActorType::Admin,
        actor_name: None,
        ip_address: ip,
        user_agent,
        trace_id: None,
    }
}

// ----------------------------------------------------------------------------
// Admin handlers
// ----------------------------------------------------------------------------

/// List both agreement types with their source, current effective version, and
/// version history for the realm.
///
/// Admin — requires `inject_identity` + `settings.view` + `has_access_to_realm`.
/// `source` is derived per-type from `has_custom` (Custom when a realm custom
/// version exists, otherwise Default).
#[utoipa::path(
    get,
    path = "/api/legal/admin/{realmId}/agreements",
    tag = "legal",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    responses(
        (status = 200, description = "Admin agreement views with history", body = AdminAgreementsResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden (missing settings.view or cross-realm)", body = ErrorResponse),
        (status = 404, description = "No effective agreement deployed for this realm", body = ErrorResponse),
        (status = 500, description = "Server error", body = ErrorResponse)
    )
)]
pub async fn admin_list_agreements(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
) -> Result<Json<AdminAgreementsResponse>, ApiError> {
    if !identity.has_access_to_realm(&realm_id) {
        return Err(ApiError::forbidden(
            "Identity does not belong to this realm",
        ));
    }
    require_permission(
        &state,
        &realm_id,
        &identity.user_id(),
        "settings",
        "view",
        "settings.view",
    )
    .await?;

    let service = &state.legal_service;
    let mut agreements = Vec::new();
    for agreement_type in [AgreementType::TermsOfService, AgreementType::PrivacyPolicy] {
        let current = match service
            .current_effective(&realm_id, agreement_type.clone())
            .await?
        {
            Some(v) => v,
            None => continue,
        };
        let has_custom = service
            .has_custom(&realm_id, agreement_type.clone())
            .await?;
        let source = if has_custom {
            AgreementSource::Custom
        } else {
            AgreementSource::Default
        };
        let history = service
            .list_history(&realm_id, agreement_type.clone(), 50)
            .await?;
        agreements.push(AdminAgreementView {
            agreement_type,
            source,
            current_version: to_summary(&current),
            history: history.iter().map(to_version_summary).collect(),
        });
    }

    if agreements.is_empty() {
        return Err(ApiError::not_found(
            "No effective legal agreement deployed for this realm",
        ));
    }

    Ok(Json(AdminAgreementsResponse { agreements }))
}

/// Publish a new per-realm custom agreement version.
///
/// Admin — requires `inject_identity` + `settings.manage` + `has_access_to_realm`.
/// Unknown `agreementType` → 400; non-object/empty `content` → 400 (also
/// enforced in the service). On success the service records an
/// `agreement.published` audit event and returns the new version.
#[utoipa::path(
    put,
    path = "/api/legal/admin/{realmId}/agreements/{agreementType}",
    tag = "legal",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("agreementType" = String, Path, description = "Agreement type: terms_of_service | privacy_policy")
    ),
    request_body = PublishCustomRequest,
    responses(
        (status = 200, description = "Newly published version", body = PublishVersionResponse),
        (status = 400, description = "Unknown agreement type or invalid content", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden (missing settings.manage or cross-realm)", body = ErrorResponse),
        (status = 500, description = "Server error", body = ErrorResponse)
    )
)]
pub async fn admin_publish_custom(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, agreement_type)): Path<(String, String)>,
    ClientIp(ip): ClientIp,
    headers: HeaderMap,
    Json(payload): Json<PublishCustomRequest>,
) -> Result<Json<PublishVersionResponse>, ApiError> {
    if !identity.has_access_to_realm(&realm_id) {
        return Err(ApiError::forbidden(
            "Identity does not belong to this realm",
        ));
    }
    require_permission(
        &state,
        &realm_id,
        &identity.user_id(),
        "settings",
        "manage",
        "settings.manage",
    )
    .await?;

    let agreement_type =
        AgreementType::try_from(agreement_type.as_str()).map_err(ApiError::bad_request)?;

    if !payload.content.is_object()
        || payload
            .content
            .as_object()
            .map(|m| m.is_empty())
            .unwrap_or(true)
    {
        return Err(ApiError::bad_request(
            "content must be a non-empty locale map",
        ));
    }

    let user_agent = headers
        .get(USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let actor = admin_actor(&identity, Some(ip), user_agent);

    let new_version = state
        .legal_service
        .publish_custom(
            &realm_id,
            agreement_type,
            payload.content,
            payload.version_label,
            &identity.user_id(),
            actor,
        )
        .await?;

    Ok(Json(PublishVersionResponse {
        version_id: new_version.id,
        version_no: new_version.version_no,
        effective_at: new_version.published_at,
    }))
}

/// Revert a realm's agreement to the platform default.
///
/// Admin — requires `inject_identity` + `settings.manage` + `has_access_to_realm`.
/// Implemented as **snapshot semantics** in the service: the current default
/// body is copied into a brand-new custom version (new `version_id`, monotonic
/// `version_no`), so existing user consent no longer matches and reconsent is
/// triggered. No prior rows are deleted. The handler is a thin pass-through.
#[utoipa::path(
    delete,
    path = "/api/legal/admin/{realmId}/agreements/{agreementType}/custom",
    tag = "legal",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("agreementType" = String, Path, description = "Agreement type: terms_of_service | privacy_policy")
    ),
    responses(
        (status = 200, description = "Newly snapshotted default version", body = PublishVersionResponse),
        (status = 400, description = "Unknown agreement type", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden (missing settings.manage or cross-realm)", body = ErrorResponse),
        (status = 500, description = "Server error", body = ErrorResponse)
    )
)]
pub async fn admin_revert_to_default(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, agreement_type)): Path<(String, String)>,
    ClientIp(ip): ClientIp,
    headers: HeaderMap,
) -> Result<Json<PublishVersionResponse>, ApiError> {
    if !identity.has_access_to_realm(&realm_id) {
        return Err(ApiError::forbidden(
            "Identity does not belong to this realm",
        ));
    }
    require_permission(
        &state,
        &realm_id,
        &identity.user_id(),
        "settings",
        "manage",
        "settings.manage",
    )
    .await?;

    let agreement_type =
        AgreementType::try_from(agreement_type.as_str()).map_err(ApiError::bad_request)?;

    let user_agent = headers
        .get(USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let actor = admin_actor(&identity, Some(ip), user_agent);

    let new_version = state
        .legal_service
        .revert_to_default(&realm_id, agreement_type, &identity.user_id(), actor)
        .await?;

    Ok(Json(PublishVersionResponse {
        version_id: new_version.id,
        version_no: new_version.version_no,
        effective_at: new_version.published_at,
    }))
}
