// Realm custom-domain configuration handlers, DTOs, and helpers.
//
// Mirrors the white-label configuration lifecycle (draft/publish/restore,
// design §4.2.2) using `ConfigType::CustomDomain` and the same three
// `config_key` slots (`settings` / `draft` / `previous_settings`). The
// publish/restore handlers additionally write/rollback the
// `custom_domain_mapping` host→realm table (design §4.2.2 publish/restore,
// BE-D02 repo) so that request-time host resolution and Caddy On-Demand TLS
// authorization reflect the published hostname.

use axum::{
    Json,
    extract::{Extension, Path, Query, State},
    http::HeaderMap,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::application::http::server::api_entities::ApiError;
use crate::application::http::state::AppState;
use herald_api_base::application::http::common::auth_utils::AdminIdentity;
use herald_api_base::application::http::common::public_helper::normalize_custom_domain_host;
use herald_core::domain::authentication::Identity;
use herald_core::domain::common::entities::app_errors::CoreError;
use herald_core::domain::custom_domain::CustomDomainMappingRepository;
use herald_core::domain::realm_config::{
    BatchUpsertRealmConfigRequest, ConfigType, CustomDomainConfig, CustomDomainStatus, RealmConfig,
    RealmConfigService, UpsertRealmConfigRequest, normalize_and_validate_hostname,
};

pub use crate::application::http::server::api_entities::ErrorResponse;

const SETTINGS_KEY: &str = "settings";
const DRAFT_KEY: &str = "draft";
const PREVIOUS_SETTINGS_KEY: &str = "previous_settings";

// ---------------------------------------------------------------------------
// Response / request DTOs
// ---------------------------------------------------------------------------

/// Custom-domain management state shown on the realm admin config page.
#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CustomDomainConfigStateResponse {
    /// Currently published configuration (effective for host→realm resolution).
    /// `hostname = null` means the realm has no custom domain published.
    pub published: CustomDomainConfig,
    /// Unpublished draft, if any.
    pub draft: Option<CustomDomainConfig>,
    /// Whether a `previous_settings` snapshot exists (one-step restore available).
    pub has_previous: bool,
    /// Herald-owned hostname tenants must CNAME their custom login domain to
    /// (global config, e.g. `custom.herald.com`).
    pub cname_target: String,
    /// Live CNAME/TLS status of the published hostname. `null` when no
    /// hostname is published or no mapping row exists yet.
    pub status: Option<CustomDomainStatus>,
}

/// Request body for saving a custom-domain draft.
#[derive(Debug, Clone, Default, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCustomDomainConfigRequest {
    /// Precise custom login hostname (e.g. `login.acme.com`). `null`/empty
    /// clears the draft hostname.
    pub hostname: Option<String>,
}

/// Response shape returned by publish / restore lifecycle operations.
#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CustomDomainLifecycleResponse {
    pub message: String,
    /// Whether a `previous_settings` snapshot now exists.
    pub has_previous: bool,
    /// Live CNAME/TLS status of the published hostname after the operation.
    pub status: Option<CustomDomainStatus>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// Get custom-domain management state.
#[utoipa::path(
    get,
    path = "/api/realms/{realmId}/config/custom-domain",
    tag = "realms",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    responses(
        (status = 200, description = "Custom-domain configuration state", body = CustomDomainConfigStateResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Realm not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handle_get_custom_domain_config(
    State(state): State<AppState>,
    Path(realm_id): Path<String>,
    Extension(identity): Extension<Identity>,
) -> Result<Json<CustomDomainConfigStateResponse>, ApiError> {
    let admin = AdminIdentity::require(identity, &realm_id, "realm custom-domain configuration")?;
    admin.require_permission(&state, "settings", "view").await?;

    Ok(Json(
        load_state(&state, admin.identity().clone(), realm_id).await?,
    ))
}

/// Save custom-domain draft without publishing.
#[utoipa::path(
    put,
    path = "/api/realms/{realmId}/config/custom-domain/draft",
    tag = "realms",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    request_body = UpdateCustomDomainConfigRequest,
    responses(
        (status = 200, description = "Custom-domain draft saved"),
        (status = 400, description = "Invalid custom domain", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 409, description = "Custom domain already in use", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handle_save_custom_domain_draft(
    State(state): State<AppState>,
    Path(realm_id): Path<String>,
    Extension(identity): Extension<Identity>,
    Json(req): Json<UpdateCustomDomainConfigRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let admin = AdminIdentity::require(identity, &realm_id, "realm custom-domain configuration")?;
    admin
        .require_permission(&state, "settings", "manage")
        .await?;

    let identity = admin.identity().clone();

    // Empty / whitespace hostname clears the draft. We still persist an empty
    // config row so the lifecycle reads consistently, mirroring white-label's
    // "draft present even when fields are blank" shape.
    let normalized_hostname = match req.hostname.as_deref() {
        None => None,
        Some(raw) if raw.trim().is_empty() => None,
        Some(raw) => {
            let hostname = normalize_and_validate_hostname(raw)
                .map_err(|_| ApiError::bad_request("Invalid custom domain"))?;
            Some(hostname)
        }
    };

    // Global uniqueness: a hostname draft occupies the name across all realms
    // so two realms can't draft-collide then fail at publish. We check both
    // other realms' realm_config custom_domain rows (settings + draft) and the
    // published mapping table. The current realm's own rows are excluded.
    if let Some(ref hostname) = normalized_hostname {
        assert_hostname_globally_unique(&state, &realm_id, hostname).await?;
    }

    let draft = CustomDomainConfig {
        hostname: normalized_hostname,
    };
    let request = build_custom_domain_upsert_request(DRAFT_KEY, &draft)?;
    state
        .service
        .realm_config_service()
        .upsert_config(identity, realm_id, request)
        .await
        .map_err(map_realm_config_error)?;

    Ok(Json(serde_json::json!({
        "message": "Custom-domain draft saved",
        "draft": draft,
    })))
}

/// Discard custom-domain draft.
#[utoipa::path(
    delete,
    path = "/api/realms/{realmId}/config/custom-domain/draft",
    tag = "realms",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    responses(
        (status = 200, description = "Custom-domain draft discarded"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Realm not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handle_discard_custom_domain_draft(
    State(state): State<AppState>,
    Path(realm_id): Path<String>,
    Extension(identity): Extension<Identity>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let admin = AdminIdentity::require(identity, &realm_id, "realm custom-domain configuration")?;
    admin
        .require_permission(&state, "settings", "manage")
        .await?;

    delete_custom_domain_config(
        &state,
        admin.identity().clone(),
        realm_id.clone(),
        DRAFT_KEY,
        true,
    )
    .await?;

    Ok(Json(serde_json::json!({
        "message": "Custom-domain draft discarded",
    })))
}

/// Publish the custom-domain draft (writes the host→realm mapping).
#[utoipa::path(
    post,
    path = "/api/realms/{realmId}/config/custom-domain/publish",
    tag = "realms",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    responses(
        (status = 200, description = "Custom-domain configuration published", body = CustomDomainLifecycleResponse),
        (status = 400, description = "No draft to publish / invalid", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 409, description = "Custom domain already in use", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handle_publish_custom_domain_config(
    State(state): State<AppState>,
    Path(realm_id): Path<String>,
    Extension(identity): Extension<Identity>,
) -> Result<Json<CustomDomainLifecycleResponse>, ApiError> {
    let admin = AdminIdentity::require(identity, &realm_id, "realm custom-domain configuration")?;
    admin
        .require_permission(&state, "settings", "manage")
        .await?;

    let identity = admin.identity().clone();

    let draft = load_config(&state, identity.clone(), realm_id.clone(), DRAFT_KEY)
        .await?
        .ok_or_else(|| ApiError::bad_request("No custom-domain draft exists to publish"))?
        .config;

    // Publishing requires a concrete hostname — an empty draft cannot become
    // the published mapping (there is nothing to CNAME/resolve).
    let new_hostname = draft.hostname.clone().ok_or_else(|| {
        ApiError::bad_request("Cannot publish a custom-domain draft without a hostname")
    })?;

    // Re-validate global uniqueness at publish time. The draft was checked when
    // saved, but another realm may have published the same name since; and the
    // mapping table is the authoritative request-time source, so we confirm
    // against it before committing config + mapping.
    assert_hostname_globally_unique(&state, &realm_id, &new_hostname).await?;

    let current_published = load_config(&state, identity.clone(), realm_id.clone(), SETTINGS_KEY)
        .await?
        .map(|entry| entry.config)
        .unwrap_or_default();

    // Write the host→realm mapping BEFORE committing the config rows. The
    // mapping op is atomic (own conflict guard), so a failure here leaves only
    // read-only state touched and the draft intact — the admin can retry
    // publish. Committing config first (the old order) and *then* writing the
    // mapping left the realm with `settings` already pointing at the new
    // hostname but no enabled mapping row, and the draft deleted — not
    // retryable. `upsert_for_realm` deletes the realm's prior enabled hostname
    // (if different), resets CNAME/TLS status to pending unless the hostname is
    // unchanged (idempotent), and surfaces a hostname owned by another realm as
    // Conflict.
    let mapping = state
        .custom_domain_mapping_repo
        .upsert_for_realm(&realm_id, &new_hostname)
        .await
        .map_err(map_mapping_error)?;

    // Atomically write both settings and previous_settings: if either write
    // fails, neither is committed, so previous_settings never points at a stale
    // snapshot and a failed publish leaves published branding untouched.
    //
    // Residual non-atomicity (not shared-transaction without a repo refactor):
    // if this config batch fails after the mapping committed, the mapping
    // reflects the new hostname while `settings` still shows the old value.
    // That is benign — host resolution keys on the mapping (the request-time
    // source of truth) and stays correct, and the admin can re-run publish
    // (mapping upsert is idempotent, the draft is preserved). Draft deletion is
    // best-effort afterwards (mirrors white-label).
    let batch = BatchUpsertRealmConfigRequest {
        configs: vec![
            build_custom_domain_upsert_request(PREVIOUS_SETTINGS_KEY, &current_published)?,
            build_custom_domain_upsert_request(SETTINGS_KEY, &draft)?,
        ],
    };
    let mut committed = state
        .service
        .realm_config_service()
        .batch_upsert_configs(identity.clone(), realm_id.clone(), batch)
        .await
        .map_err(map_realm_config_error)?;
    let published = committed
        .pop()
        .expect("batch returns entries in input order; settings is the last entry");
    debug_assert!(
        committed.len() == 1
            && committed[0].config_key == PREVIOUS_SETTINGS_KEY
            && published.config_key == SETTINGS_KEY,
        "batch_upsert_configs must preserve input order"
    );
    delete_custom_domain_config(&state, identity.clone(), realm_id.clone(), DRAFT_KEY, true)
        .await?;

    // The mapping was just written; derive status from it instead of re-querying.
    let status = Some(CustomDomainStatus {
        cname_verified: mapping.cname_verified,
        tls_ready: mapping.tls_ready,
        checked_at: mapping.status_checked_at,
    });

    Ok(Json(CustomDomainLifecycleResponse {
        message: "Custom-domain configuration published".to_string(),
        has_previous: true,
        status,
    }))
}

/// Restore previous custom-domain settings (rolls back the host→realm mapping).
#[utoipa::path(
    post,
    path = "/api/realms/{realmId}/config/custom-domain/restore",
    tag = "realms",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    responses(
        (status = 200, description = "Previous custom-domain configuration restored", body = CustomDomainLifecycleResponse),
        (status = 400, description = "No previous custom-domain settings to restore", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 409, description = "Custom domain already in use", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handle_restore_custom_domain_config(
    State(state): State<AppState>,
    Path(realm_id): Path<String>,
    Extension(identity): Extension<Identity>,
) -> Result<Json<CustomDomainLifecycleResponse>, ApiError> {
    let admin = AdminIdentity::require(identity, &realm_id, "realm custom-domain configuration")?;
    admin
        .require_permission(&state, "settings", "manage")
        .await?;

    let identity = admin.identity().clone();

    let previous = load_config(
        &state,
        identity.clone(),
        realm_id.clone(),
        PREVIOUS_SETTINGS_KEY,
    )
    .await?
    .ok_or_else(|| ApiError::bad_request("No previous custom-domain settings to restore"))?
    .config;

    let current_published = load_config(&state, identity.clone(), realm_id.clone(), SETTINGS_KEY)
        .await?
        .map(|entry| entry.config)
        .unwrap_or_default();

    // Perform the mapping rollback BEFORE committing the config swap. The
    // mapping op is atomic; a failure here touches only the mapping table and
    // leaves `settings`/`previous_settings` unchanged (the restore is
    // retryable). Committing the config swap first and then failing the mapping
    // left `settings` pointing at the restored hostname while the old mapping
    // was still live — request-time resolution would serve the wrong hostname.
    //
    // Residual non-atomicity (not shared-transaction without a repo refactor):
    // if the config swap fails after the mapping committed, the mapping already
    // reflects the restored hostname while `settings` is stale. That is benign —
    // host resolution keys on the mapping and stays correct, and the admin can
    // re-run restore (idempotent). Two cases:
    //  (1) restored previous config HAS a hostname → re-publish the restored
    //      hostname (upsert_for_realm deletes the realm's superseded current
    //      hostname row and re-enables the restored one).
    //  (2) restored previous config has NO hostname (restore to "no custom
    //      domain") → delete all mapping rows for this realm.
    let status = match previous.hostname.as_deref() {
        Some(restored_hostname) => {
            let mapping = state
                .custom_domain_mapping_repo
                .upsert_for_realm(&realm_id, restored_hostname)
                .await
                .map_err(map_mapping_error)?;
            Some(CustomDomainStatus {
                cname_verified: mapping.cname_verified,
                tls_ready: mapping.tls_ready,
                checked_at: mapping.status_checked_at,
            })
        }
        None => {
            // Restore target is "no custom domain": remove every mapping row
            // for this realm (both the superseded and any prior hostname).
            state
                .custom_domain_mapping_repo
                .delete_by_realm_or_hostname(Some(realm_id.clone()), None)
                .await
                .map_err(map_mapping_error)?;
            None
        }
    };

    // Now swap the config rows to match the rolled-back mapping. Atomic batch:
    // if either write fails neither is committed (previous_settings never
    // points at a stale snapshot).
    let batch = BatchUpsertRealmConfigRequest {
        configs: vec![
            build_custom_domain_upsert_request(PREVIOUS_SETTINGS_KEY, &current_published)?,
            build_custom_domain_upsert_request(SETTINGS_KEY, &previous)?,
        ],
    };
    state
        .service
        .realm_config_service()
        .batch_upsert_configs(identity, realm_id.clone(), batch)
        .await
        .map_err(map_realm_config_error)?;

    Ok(Json(CustomDomainLifecycleResponse {
        message: "Previous custom-domain configuration restored".to_string(),
        has_previous: true,
        status,
    }))
}

// ---------------------------------------------------------------------------
// Internal endpoint: Caddy On-Demand TLS ask authorization
//
// Unauthenticated top-level route registered in `server/mod.rs` (NOT under
// `/api/realms`, so no `inject_identity`). It shares the
// `custom_domain_mapping` table with the management handlers with a minimal
// response shape (design §4.2.2):
//   - ask → `{"authorized": true}` only (NO realm info — certificate-abuse
//           gate; leaking realmId would let an attacker map a host to a realm
//           without owning it).
// It filters on the unified effectiveness predicate `enabled = true`
// (design §5.1「生效判定」); `cname_verified`/`tls_ready` are display-only.
//
// (The public host→realmId resolve endpoint was removed: realm routing now
// always relies on the {realmId} path segment.)
// ---------------------------------------------------------------------------

/// Query parameters for the internal custom-domain ask endpoint.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CustomDomainHostQuery {
    /// The hostname to look up (e.g. `login.acme.com`). Compared as-is against
    /// the normalized, lowercased `custom_domain_mapping.hostname` column.
    pub host: String,
}

/// Response body for the Caddy On-Demand TLS ask authorization gate.
///
/// Deliberately contains ONLY the `authorized` boolean — no realm id or any
/// other realm information (design §4.2.2 ask, certificate-abuse gate).
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CustomDomainAuthorizeResponse {
    pub authorized: bool,
}

/// Caddy On-Demand TLS ask authorization endpoint.
///
/// Returns `200 {"authorized": true}` when `host` matches a published+enabled
/// `custom_domain_mapping` row (design §4.2.2 ask / §5.1 effectiveness rule);
/// `404` on a miss (Caddy declines TLS issuance); `401` when the
/// `X-Herald-Ask-Key` header is missing or mismatches the configured shared
/// secret. Never exposes realm information — this is a certificate-abuse gate.
#[utoipa::path(
    get,
    path = "/api/internal/custom-domain/authorize",
    tag = "realms",
    params(
        ("host" = String, Query, description = "Hostname to authorize for TLS issuance")
    ),
    responses(
        (status = 200, description = "Host is authorized for TLS issuance", body = CustomDomainAuthorizeResponse),
        (status = 401, description = "Missing or mismatched X-Herald-Ask-Key", body = ErrorResponse),
        (status = 404, description = "Host is not a published custom domain; Caddy declines issuance", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
)]
pub async fn handle_custom_domain_authorize(
    State(state): State<AppState>,
    Query(query): Query<CustomDomainHostQuery>,
    headers: HeaderMap,
) -> Result<Json<CustomDomainAuthorizeResponse>, ApiError> {
    // Shared-key gate. `ask_key` is validated non-empty at server startup
    // (build_app_state_with_migrations, design §4.2.2), so an empty configured
    // key cannot reach here in production. Constant-time comparison is not
    // required for a high-entropy shared secret checked once per TLS ask
    // (low-frequency, non-user-controlled timing), but we avoid short-circuit
    // on length by comparing the trimmed header to the configured value.
    let provided = headers
        .get("x-herald-ask-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if provided.is_empty() || provided != state.custom_domain_ask_key {
        return Err(ApiError::unauthorized("Invalid ask key"));
    }

    // Effectiveness predicate: `enabled = true` only (design §5.1). The repo
    // filters this; `cname_verified`/`tls_ready` are display-only and play no
    // role in authorization (otherwise ask ↔ TLS issuance would be circular).
    // Normalize the read path symmetrically with the write path (publish stores
    // a lowercase, trailing-dot-stripped hostname). Caddy's `host`/SNI may
    // arrive with differing case or a trailing dot; without normalizing here,
    // a legitimately published domain would miss and return 404, declining TLS
    // issuance. Full validation (`normalize_and_validate_hostname`) is avoided
    // on this hot path — a syntactically invalid host simply won't match a row.
    let host = normalize_custom_domain_host(&query.host)
        .ok_or_else(|| ApiError::not_found("Custom domain not found"))?;
    let mapping = state
        .custom_domain_mapping_repo
        .find_by_hostname(&host)
        .await
        .map_err(map_mapping_error)?;

    match mapping {
        // Hit → authorize. NEVER include realm_id or any realm info in the
        // body (design §4.2.2 ask / certificate-abuse gate).
        Some(_) => Ok(Json(CustomDomainAuthorizeResponse { authorized: true })),
        // Miss → 404 so Caddy declines issuance for unregistered hosts.
        None => Err(ApiError::not_found("Custom domain not found")),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

struct LoadedCustomDomainConfig {
    config: CustomDomainConfig,
    #[allow(dead_code)]
    updated_at: String,
}

async fn load_state(
    state: &AppState,
    identity: Identity,
    realm_id: String,
) -> Result<CustomDomainConfigStateResponse, ApiError> {
    let published = load_config(state, identity.clone(), realm_id.clone(), SETTINGS_KEY).await?;
    let draft = load_config(state, identity.clone(), realm_id.clone(), DRAFT_KEY).await?;
    let has_previous = load_config(state, identity, realm_id.clone(), PREVIOUS_SETTINGS_KEY)
        .await?
        .is_some();

    let published_config = published
        .as_ref()
        .map(|entry| entry.config.clone())
        .unwrap_or_default();

    // The published hostname drives the mapping lookup for live status. A
    // missing/empty hostname means no mapping row, hence null status.
    let status = match published_config.hostname.as_deref() {
        Some(hostname) => load_status(state, hostname).await?,
        None => None,
    };

    Ok(CustomDomainConfigStateResponse {
        published: published_config,
        draft: draft.as_ref().map(|entry| entry.config.clone()),
        has_previous,
        cname_target: state.custom_domain_cname_target.clone(),
        status,
    })
}

/// Load the live CNAME/TLS status for a published hostname from the mapping
/// table. Returns `None` when no enabled row exists.
async fn load_status(
    state: &AppState,
    hostname: &str,
) -> Result<Option<CustomDomainStatus>, ApiError> {
    let row = state
        .custom_domain_mapping_repo
        .find_by_hostname(hostname)
        .await
        .map_err(map_mapping_error)?;
    Ok(row.map(|mapping| CustomDomainStatus {
        cname_verified: mapping.cname_verified,
        tls_ready: mapping.tls_ready,
        checked_at: mapping.status_checked_at,
    }))
}

async fn load_config(
    state: &AppState,
    identity: Identity,
    realm_id: String,
    config_key: &str,
) -> Result<Option<LoadedCustomDomainConfig>, ApiError> {
    let entry = state
        .service
        .realm_config_service()
        .get_config(
            identity,
            realm_id.clone(),
            ConfigType::CustomDomain.as_ref().to_string(),
            config_key.to_string(),
        )
        .await
        .map_err(map_realm_config_error)?;

    Ok(entry.map(|entry| parse_config_entry(&realm_id, config_key, entry)))
}

fn parse_config_entry(
    realm_id: &str,
    config_key: &str,
    entry: RealmConfig,
) -> LoadedCustomDomainConfig {
    let config =
        serde_json::from_str::<CustomDomainConfig>(&entry.config_value).unwrap_or_else(|e| {
            tracing::error!(
                realm_id = %realm_id,
                config_type = %ConfigType::CustomDomain.as_ref(),
                config_key = %config_key,
                error = %e,
                "Failed to parse custom-domain config JSON"
            );
            CustomDomainConfig::default()
        });

    LoadedCustomDomainConfig {
        config,
        updated_at: entry.updated_at.to_rfc3339(),
    }
}

fn build_custom_domain_upsert_request(
    config_key: &str,
    config: &CustomDomainConfig,
) -> Result<UpsertRealmConfigRequest, ApiError> {
    let config_value = serde_json::to_string(config).map_err(|e| {
        tracing::error!("Failed to serialize custom-domain config: {}", e);
        ApiError::internal("Failed to serialize custom-domain config")
    })?;

    Ok(UpsertRealmConfigRequest {
        config_type: ConfigType::CustomDomain,
        config_key: config_key.to_string(),
        config_value,
        is_secret: Some(false),
        enabled: Some(true),
        metadata: None,
    })
}

async fn delete_custom_domain_config(
    state: &AppState,
    identity: Identity,
    realm_id: String,
    config_key: &str,
    ignore_not_found: bool,
) -> Result<(), ApiError> {
    let result = state
        .service
        .realm_config_service()
        .delete_config(
            identity,
            realm_id,
            ConfigType::CustomDomain.as_ref().to_string(),
            config_key.to_string(),
        )
        .await;

    match result {
        Ok(()) => Ok(()),
        Err(CoreError::NotFound) if ignore_not_found => Ok(()),
        Err(e) => Err(map_realm_config_error(e)),
    }
}

/// Assert a hostname is not claimed by another realm.
///
/// Checks two sources (design §4.2.2 409):
/// 1. The `custom_domain_mapping` table (published hostnames) — via the repo
///    port, filtered to enabled rows.
/// 2. Other realms' `realm_config` `custom_domain` rows for the `settings`
///    (published) and `draft` keys — a direct SQL query against `state.pool`
///    because the `RealmConfigRepository` port is per-realm and cannot express
///    a cross-realm scan. The current realm's own rows are excluded so saving
///    a realm's own draft/publish is not a self-conflict.
///
/// Returns `ApiError::conflict("Custom domain already in use")` on conflict.
async fn assert_hostname_globally_unique(
    state: &AppState,
    realm_id: &str,
    hostname: &str,
) -> Result<(), ApiError> {
    // 1) Published mapping table.
    if let Some(mapping) = state
        .custom_domain_mapping_repo
        .find_by_hostname(hostname)
        .await
        .map_err(map_mapping_error)?
        && mapping.realm_id != realm_id
    {
        return Err(ApiError::conflict("Custom domain already in use"));
    }

    // 2) Other realms' realm_config custom_domain settings/draft rows.
    //
    // We look for any row of config_type='custom_domain' whose config_value
    // JSON contains the exact normalized hostname, on a *different* realm.
    // Matching on the serialized `{"hostname":"<value>"}` JSON substring is
    // safe here because the hostname was normalized (lowercase, no quotes /
    // escapes possible) before this call, so it cannot break out of the JSON
    // string token.
    let pattern = format!("\"hostname\":\"{hostname}\"");
    let conflict: Option<(String,)> = sqlx::query_as(
        "SELECT realm_id FROM realm_config
         WHERE config_type = 'custom_domain'
           AND config_key IN ('settings', 'draft')
           AND realm_id <> $1
           AND config_value LIKE $2
         LIMIT 1",
    )
    .bind(realm_id)
    .bind(format!("%{pattern}%"))
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!(
            realm_id = %realm_id,
            hostname = %hostname,
            "Failed to check custom-domain uniqueness: {e}"
        );
        ApiError::internal("Failed to check custom-domain uniqueness")
    })?;

    if conflict.is_some() {
        return Err(ApiError::conflict("Custom domain already in use"));
    }

    Ok(())
}

fn map_realm_config_error(error: CoreError) -> ApiError {
    match error {
        CoreError::Forbidden(msg) => ApiError::forbidden(msg),
        CoreError::NotFound => ApiError::not_found("Realm not found"),
        CoreError::BadRequest(msg) => ApiError::bad_request(msg),
        CoreError::Conflict(msg) => ApiError::conflict(msg),
        _ => {
            tracing::error!("Custom-domain realm config operation failed: {}", error);
            ApiError::internal("Internal server error")
        }
    }
}

fn map_mapping_error(error: CoreError) -> ApiError {
    match error {
        CoreError::Conflict(msg) => ApiError::conflict(msg),
        CoreError::NotFound => ApiError::not_found("Custom-domain mapping not found"),
        _ => {
            tracing::error!("Custom-domain mapping operation failed: {}", error);
            ApiError::internal("Internal server error")
        }
    }
}
