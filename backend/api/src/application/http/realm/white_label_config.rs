// Realm white-label configuration handlers, DTOs, and helpers

use axum::{
    Json,
    extract::{Extension, Path, State},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::application::http::server::api_entities::ApiError;
use crate::application::http::state::AppState;
use herald_api_base::application::http::common::auth_utils::AdminIdentity;
use herald_core::domain::authentication::Identity;
use herald_core::domain::common::entities::app_errors::CoreError;
use herald_core::domain::realm_config::{
    BatchUpsertRealmConfigRequest, ConfigType, RealmConfig, RealmConfigService,
    UpsertRealmConfigRequest,
};

pub use crate::application::http::server::api_entities::ErrorResponse;

const SETTINGS_KEY: &str = "settings";
const DRAFT_KEY: &str = "draft";
const PREVIOUS_SETTINGS_KEY: &str = "previous_settings";

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WhiteLabelBackground {
    pub r#type: WhiteLabelBackgroundType,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum WhiteLabelBackgroundType {
    Image,
    Gradient,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WhiteLabelConfig {
    pub logo_url: Option<String>,
    pub accent_color: Option<String>,
    pub background: Option<WhiteLabelBackground>,
    pub footer_text: Option<String>,
    pub login_title: Option<String>,
    pub login_subtitle: Option<String>,
    pub register_title: Option<String>,
    pub register_subtitle: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWhiteLabelConfigRequest {
    pub logo_url: Option<String>,
    pub accent_color: Option<String>,
    pub background: Option<WhiteLabelBackground>,
    pub footer_text: Option<String>,
    pub login_title: Option<String>,
    pub login_subtitle: Option<String>,
    pub register_title: Option<String>,
    pub register_subtitle: Option<String>,
}

impl UpdateWhiteLabelConfigRequest {
    pub fn normalize(self) -> Result<WhiteLabelConfig, ApiError> {
        let logo_url = normalize_optional_string(self.logo_url);
        if let Some(url) = logo_url.as_deref() {
            validate_http_url(url, "logoUrl")?;
        }

        Ok(WhiteLabelConfig {
            logo_url,
            accent_color: normalize_optional_string(self.accent_color),
            background: normalize_background(self.background)?,
            footer_text: normalize_optional_string(self.footer_text),
            login_title: normalize_optional_string(self.login_title),
            login_subtitle: normalize_optional_string(self.login_subtitle),
            register_title: normalize_optional_string(self.register_title),
            register_subtitle: normalize_optional_string(self.register_subtitle),
        })
    }
}

impl WhiteLabelConfig {
    pub fn to_storage_json(&self) -> Result<String, ApiError> {
        serde_json::to_string(self).map_err(|e| {
            tracing::error!("Failed to serialize white-label config: {}", e);
            ApiError::internal("Failed to serialize white-label config")
        })
    }
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WhiteLabelConfigStateResponse {
    pub published: WhiteLabelConfig,
    pub draft: Option<WhiteLabelConfig>,
    pub has_previous: bool,
    pub published_updated_at: Option<String>,
    pub draft_updated_at: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SaveWhiteLabelDraftResponse {
    pub message: String,
    pub draft: WhiteLabelConfig,
    pub draft_updated_at: String,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WhiteLabelLifecycleResponse {
    pub message: String,
    pub published: WhiteLabelConfig,
    pub draft: Option<WhiteLabelConfig>,
    pub has_previous: bool,
    pub published_updated_at: Option<String>,
    pub draft_updated_at: Option<String>,
}

/// Get white-label management state.
#[utoipa::path(
    get,
    path = "/api/realms/{realmId}/config/white-label",
    tag = "realms",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    responses(
        (status = 200, description = "White-label configuration state", body = WhiteLabelConfigStateResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Realm not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handle_get_white_label_config(
    State(state): State<AppState>,
    Path(realm_id): Path<String>,
    Extension(identity): Extension<Identity>,
) -> Result<Json<WhiteLabelConfigStateResponse>, ApiError> {
    let admin = AdminIdentity::require(identity, &realm_id, "realm white-label configuration")?;
    admin.require_permission(&state, "settings", "view").await?;

    Ok(Json(
        load_state(&state, admin.identity().clone(), realm_id).await?,
    ))
}

/// Save white-label draft without publishing.
#[utoipa::path(
    put,
    path = "/api/realms/{realmId}/config/white-label/draft",
    tag = "realms",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    request_body = UpdateWhiteLabelConfigRequest,
    responses(
        (status = 200, description = "White-label draft saved", body = SaveWhiteLabelDraftResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Realm not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handle_save_white_label_draft(
    State(state): State<AppState>,
    Path(realm_id): Path<String>,
    Extension(identity): Extension<Identity>,
    Json(req): Json<UpdateWhiteLabelConfigRequest>,
) -> Result<Json<SaveWhiteLabelDraftResponse>, ApiError> {
    let admin = AdminIdentity::require(identity, &realm_id, "realm white-label configuration")?;
    admin
        .require_permission(&state, "settings", "manage")
        .await?;

    let draft = req.normalize()?;
    let request = build_white_label_upsert_request(DRAFT_KEY, &draft)?;
    let config = state
        .service
        .realm_config_service()
        .upsert_config(admin.identity().clone(), realm_id, request)
        .await
        .map_err(map_realm_config_error)?;

    Ok(Json(SaveWhiteLabelDraftResponse {
        message: "White-label draft saved".to_string(),
        draft,
        draft_updated_at: config.updated_at.to_rfc3339(),
    }))
}

/// Discard white-label draft.
#[utoipa::path(
    delete,
    path = "/api/realms/{realmId}/config/white-label/draft",
    tag = "realms",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    responses(
        (status = 200, description = "White-label draft discarded", body = WhiteLabelLifecycleResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Realm not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handle_discard_white_label_draft(
    State(state): State<AppState>,
    Path(realm_id): Path<String>,
    Extension(identity): Extension<Identity>,
) -> Result<Json<WhiteLabelLifecycleResponse>, ApiError> {
    let admin = AdminIdentity::require(identity, &realm_id, "realm white-label configuration")?;
    admin
        .require_permission(&state, "settings", "manage")
        .await?;

    delete_white_label_config(
        &state,
        admin.identity().clone(),
        realm_id.clone(),
        DRAFT_KEY,
        true,
    )
    .await?;
    let state_response = load_state(&state, admin.identity().clone(), realm_id).await?;

    Ok(Json(WhiteLabelLifecycleResponse {
        message: "White-label draft discarded".to_string(),
        published: state_response.published,
        draft: None,
        has_previous: state_response.has_previous,
        published_updated_at: state_response.published_updated_at,
        draft_updated_at: None,
    }))
}

/// Publish request body or the existing white-label draft.
#[utoipa::path(
    post,
    path = "/api/realms/{realmId}/config/white-label/publish",
    tag = "realms",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    request_body = Option<UpdateWhiteLabelConfigRequest>,
    responses(
        (status = 200, description = "White-label configuration published", body = WhiteLabelLifecycleResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Realm not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handle_publish_white_label_config(
    State(state): State<AppState>,
    Path(realm_id): Path<String>,
    Extension(identity): Extension<Identity>,
    payload: Option<Json<UpdateWhiteLabelConfigRequest>>,
) -> Result<Json<WhiteLabelLifecycleResponse>, ApiError> {
    let admin = AdminIdentity::require(identity, &realm_id, "realm white-label configuration")?;
    admin
        .require_permission(&state, "settings", "manage")
        .await?;

    let identity = admin.identity().clone();
    let published_input = if let Some(Json(req)) = payload {
        req.normalize()?
    } else {
        load_config(&state, identity.clone(), realm_id.clone(), DRAFT_KEY)
            .await?
            .ok_or_else(|| ApiError::bad_request("No white-label draft exists to publish"))?
            .config
    };

    let current_published = load_config(&state, identity.clone(), realm_id.clone(), SETTINGS_KEY)
        .await?
        .map(|entry| entry.config)
        .unwrap_or_default();

    // Atomically write both settings and previous_settings: if either write fails,
    // neither is committed, so previous_settings never points at a stale snapshot
    // and a failed publish leaves published branding untouched. Draft deletion is
    // best-effort afterwards — a leftover draft does not corrupt published state.
    let batch = BatchUpsertRealmConfigRequest {
        configs: vec![
            build_white_label_upsert_request(PREVIOUS_SETTINGS_KEY, &current_published)?,
            build_white_label_upsert_request(SETTINGS_KEY, &published_input)?,
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
    delete_white_label_config(&state, identity, realm_id, DRAFT_KEY, true).await?;

    Ok(Json(WhiteLabelLifecycleResponse {
        message: "White-label configuration published".to_string(),
        published: published_input,
        draft: None,
        has_previous: true,
        published_updated_at: Some(published.updated_at.to_rfc3339()),
        draft_updated_at: None,
    }))
}

/// Restore previous white-label settings.
#[utoipa::path(
    post,
    path = "/api/realms/{realmId}/config/white-label/restore",
    tag = "realms",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    responses(
        (status = 200, description = "Previous white-label configuration restored", body = WhiteLabelLifecycleResponse),
        (status = 400, description = "No previous white-label settings", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Realm not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handle_restore_white_label_config(
    State(state): State<AppState>,
    Path(realm_id): Path<String>,
    Extension(identity): Extension<Identity>,
) -> Result<Json<WhiteLabelLifecycleResponse>, ApiError> {
    let admin = AdminIdentity::require(identity, &realm_id, "realm white-label configuration")?;
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
    .ok_or_else(|| ApiError::bad_request("No previous white-label settings to restore"))?
    .config;
    let current_published = load_config(&state, identity.clone(), realm_id.clone(), SETTINGS_KEY)
        .await?
        .map(|entry| entry.config)
        .unwrap_or_default();

    // Atomically swap settings and previous_settings: if either write fails,
    // neither is committed, so a failed restore cannot destroy the rollback snapshot.
    let batch = BatchUpsertRealmConfigRequest {
        configs: vec![
            build_white_label_upsert_request(PREVIOUS_SETTINGS_KEY, &current_published)?,
            build_white_label_upsert_request(SETTINGS_KEY, &previous)?,
        ],
    };
    let mut committed = state
        .service
        .realm_config_service()
        .batch_upsert_configs(identity, realm_id, batch)
        .await
        .map_err(map_realm_config_error)?;
    let restored = committed
        .pop()
        .expect("batch returns entries in input order; settings is the last entry");

    Ok(Json(WhiteLabelLifecycleResponse {
        message: "Previous white-label configuration restored".to_string(),
        published: previous,
        draft: None,
        has_previous: true,
        published_updated_at: Some(restored.updated_at.to_rfc3339()),
        draft_updated_at: None,
    }))
}

struct LoadedWhiteLabelConfig {
    config: WhiteLabelConfig,
    updated_at: String,
}

async fn load_state(
    state: &AppState,
    identity: Identity,
    realm_id: String,
) -> Result<WhiteLabelConfigStateResponse, ApiError> {
    let published = load_config(state, identity.clone(), realm_id.clone(), SETTINGS_KEY).await?;
    let draft = load_config(state, identity.clone(), realm_id.clone(), DRAFT_KEY).await?;
    let has_previous = load_config(state, identity, realm_id, PREVIOUS_SETTINGS_KEY)
        .await?
        .is_some();

    Ok(WhiteLabelConfigStateResponse {
        published: published
            .as_ref()
            .map(|entry| entry.config.clone())
            .unwrap_or_default(),
        draft: draft.as_ref().map(|entry| entry.config.clone()),
        has_previous,
        published_updated_at: published.map(|entry| entry.updated_at),
        draft_updated_at: draft.map(|entry| entry.updated_at),
    })
}

async fn load_config(
    state: &AppState,
    identity: Identity,
    realm_id: String,
    config_key: &str,
) -> Result<Option<LoadedWhiteLabelConfig>, ApiError> {
    let entry = state
        .service
        .realm_config_service()
        .get_config(
            identity,
            realm_id.clone(),
            ConfigType::WhiteLabel.as_ref().to_string(),
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
) -> LoadedWhiteLabelConfig {
    let config =
        serde_json::from_str::<WhiteLabelConfig>(&entry.config_value).unwrap_or_else(|e| {
            tracing::error!(
                realm_id = %realm_id,
                config_type = %ConfigType::WhiteLabel.as_ref(),
                config_key = %config_key,
                error = %e,
                "Failed to parse white-label config JSON"
            );
            WhiteLabelConfig::default()
        });

    LoadedWhiteLabelConfig {
        config,
        updated_at: entry.updated_at.to_rfc3339(),
    }
}

fn build_white_label_upsert_request(
    config_key: &str,
    config: &WhiteLabelConfig,
) -> Result<UpsertRealmConfigRequest, ApiError> {
    Ok(UpsertRealmConfigRequest {
        config_type: ConfigType::WhiteLabel,
        config_key: config_key.to_string(),
        config_value: config.to_storage_json()?,
        is_secret: Some(false),
        enabled: Some(true),
        metadata: None,
    })
}

async fn delete_white_label_config(
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
            ConfigType::WhiteLabel.as_ref().to_string(),
            config_key.to_string(),
        )
        .await;

    match result {
        Ok(()) => Ok(()),
        Err(CoreError::NotFound) if ignore_not_found => Ok(()),
        Err(e) => Err(map_realm_config_error(e)),
    }
}

fn map_realm_config_error(error: CoreError) -> ApiError {
    match error {
        CoreError::Forbidden(msg) => ApiError::forbidden(msg),
        CoreError::NotFound => ApiError::not_found("Realm not found"),
        CoreError::BadRequest(msg) => ApiError::bad_request(msg),
        _ => {
            tracing::error!("White-label realm config operation failed: {}", error);
            ApiError::internal("Internal server error")
        }
    }
}

fn normalize_background(
    background: Option<WhiteLabelBackground>,
) -> Result<Option<WhiteLabelBackground>, ApiError> {
    let Some(background) = background else {
        return Ok(None);
    };

    let value = background.value.trim().to_string();
    if value.is_empty() {
        return Ok(None);
    }

    match background.r#type {
        WhiteLabelBackgroundType::Image => validate_http_url(&value, "background.value")?,
        WhiteLabelBackgroundType::Gradient => validate_gradient(&value)?,
    }

    Ok(Some(WhiteLabelBackground {
        r#type: background.r#type,
        value,
    }))
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn validate_http_url(value: &str, field_name: &str) -> Result<(), ApiError> {
    if value.starts_with("http://") || value.starts_with("https://") {
        return Ok(());
    }

    Err(ApiError::bad_request(format!(
        "{field_name} must be an http:// or https:// URL"
    )))
}

fn validate_gradient(value: &str) -> Result<(), ApiError> {
    if value.starts_with("linear-gradient(") || value.starts_with("radial-gradient(") {
        return Ok(());
    }

    Err(ApiError::bad_request(
        "background.value must start with linear-gradient( or radial-gradient(",
    ))
}
