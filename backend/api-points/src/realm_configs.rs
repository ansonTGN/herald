use axum::{
    Json,
    extract::{Extension, Path, State},
};
use validator::Validate;

use crate::types::{
    CreateRealmConfigRequest, QuotaWindowInput, RealmDefaultConfigResponse,
    UpdateRealmConfigRequest,
};
use herald_api_base::application::http::common::auth_utils::AdminIdentity;
use herald_api_base::application::http::server::api_entities::{ApiError, ErrorResponse};
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::points::{
    CreateRealmConfigInput, FREE_PERIODIC_QUOTA_WINDOWS_MAX, QuotaWindow, UpdateRealmConfigInput,
    derive_window_key,
};

/// Convert domain `RealmDefaultConfig` to API response. The stored
/// `free_periodic_quota_windows` (domain `Vec<QuotaWindow>`) maps to
/// `Option<Vec<QuotaWindowInput>>`: empty ⟹ `None` (no window grant).
fn realm_config_to_response(
    config: herald_core::domain::points::RealmDefaultConfig,
) -> RealmDefaultConfigResponse {
    let free_periodic_quota_windows = if config.free_periodic_quota_windows.is_empty() {
        None
    } else {
        Some(
            config
                .free_periodic_quota_windows
                .into_iter()
                .map(|w| QuotaWindowInput {
                    // Re-derive the key from the canonical length so the
                    // response always carries the stable identity (the stored
                    // key is the snapshot, but the response is the editable
                    // config view — `QuotaWindowInput` carries no key, so
                    // this round-trip keeps the editor length-driven).
                    window_seconds: w.window_seconds,
                    limit: w.limit,
                })
                .collect(),
        )
    };
    RealmDefaultConfigResponse {
        realm_id: config.realm_id,
        registration_bonus_points: config.registration_bonus_points,
        free_periodic_points_amount: config.free_periodic_points_amount,
        free_periodic_grant_period_type: config.free_periodic_grant_period_type.to_string(),
        free_periodic_validity_days: config.free_periodic_validity_days,
        free_periodic_quota_windows,
        created_at: config.created_at.to_rfc3339(),
        updated_at: config.updated_at.to_rfc3339(),
    }
}

/// Materialize the request-side `QuotaWindowInput` list into the domain
/// `Vec<QuotaWindow>` (deriving the stable `key` per window) with edge
/// validation (design §4.2.2 / §4.4.3):
/// - each window's `validate()` runs (`windowSeconds > 0`, `limit >= 0`);
/// - window count ≤ `FREE_PERIODIC_QUOTA_WINDOWS_MAX` (8).
///
/// `None` ⟹ `None` (create: no window grant; update: leave stored value
/// untouched). `Some([])` ⟹ `Some([])` (update: clear). Invalid input ⟹ 400.
///
/// The domain service re-validates via `validate_free_periodic_quota_windows`,
/// so this is defense-in-depth at the API edge (cheaper, surfaces 400 before
/// the service boundary).
fn materialize_quota_windows(
    windows: Option<Vec<QuotaWindowInput>>,
) -> Result<Option<Vec<QuotaWindow>>, ApiError> {
    let Some(windows) = windows else {
        return Ok(None);
    };
    if windows.len() > FREE_PERIODIC_QUOTA_WINDOWS_MAX {
        return Err(ApiError::bad_request(format!(
            "free_periodic_quota_windows may have at most {} windows, got {}",
            FREE_PERIODIC_QUOTA_WINDOWS_MAX,
            windows.len()
        )));
    }
    let mut out = Vec::with_capacity(windows.len());
    for w in windows {
        w.validate()
            .map_err(|e| ApiError::bad_request(format!("Invalid quota window: {}", e)))?;
        out.push(QuotaWindow {
            window_seconds: w.window_seconds,
            limit: w.limit,
            key: derive_window_key(w.window_seconds),
        });
    }
    Ok(Some(out))
}

/// Get realm default config
#[utoipa::path(
    get,
    path = "/api/points/{realmId}/default-config",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    responses(
        (status = 200, description = "Realm default config retrieved successfully", body = RealmDefaultConfigResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Points"
)]
pub async fn get_realm_default_config(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
) -> Result<Json<RealmDefaultConfigResponse>, ApiError> {
    let admin = AdminIdentity::require(identity, &realm_id, "realm default config")?;
    admin.require_permission(&state, "settings", "view").await?;

    match state
        .realm_config_service
        .get_realm_config(admin.identity().clone(), &realm_id)
        .await
    {
        Ok(config) => Ok(Json(realm_config_to_response(config))),
        Err(e) => Err(ApiError::from(e)),
    }
}

/// Create or initialize realm default config
#[utoipa::path(
    post,
    path = "/api/points/{realmId}/default-config",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    request_body = CreateRealmConfigRequest,
    responses(
        (status = 200, description = "Realm default config created or retrieved successfully", body = RealmDefaultConfigResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Points"
)]
pub async fn create_realm_default_config(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Json(request): Json<CreateRealmConfigRequest>,
) -> Result<Json<RealmDefaultConfigResponse>, ApiError> {
    let admin = AdminIdentity::require(identity, &realm_id, "realm default config")?;
    admin
        .require_permission(&state, "settings", "manage")
        .await?;

    request
        .validate()
        .map_err(|e| ApiError::bad_request(format!("Invalid request: {}", e)))?;

    let input = CreateRealmConfigInput {
        realm_id: realm_id.clone(),
        registration_bonus_points: request.registration_bonus_points,
        free_periodic_points_amount: request.free_periodic_points_amount,
        free_periodic_grant_period_type: request.free_periodic_grant_period_type,
        free_periodic_validity_days: request.free_periodic_validity_days,
        free_periodic_quota_windows: materialize_quota_windows(
            request.free_periodic_quota_windows,
        )?,
    };

    match state
        .realm_config_service
        .create_realm_config(admin.identity().clone(), input)
        .await
    {
        Ok(config) => Ok(Json(realm_config_to_response(config))),
        Err(e) => Err(ApiError::from(e)),
    }
}

/// Update realm default config
#[utoipa::path(
    put,
    path = "/api/points/{realmId}/default-config",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    request_body = UpdateRealmConfigRequest,
    responses(
        (status = 200, description = "Realm default config updated successfully", body = RealmDefaultConfigResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Points"
)]
pub async fn update_realm_default_config(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Json(request): Json<UpdateRealmConfigRequest>,
) -> Result<Json<RealmDefaultConfigResponse>, ApiError> {
    let admin = AdminIdentity::require(identity, &realm_id, "realm default config")?;
    admin
        .require_permission(&state, "settings", "manage")
        .await?;

    request
        .validate()
        .map_err(|e| ApiError::bad_request(format!("Invalid request: {}", e)))?;

    let input = UpdateRealmConfigInput {
        registration_bonus_points: request.registration_bonus_points,
        free_periodic_points_amount: request.free_periodic_points_amount,
        free_periodic_grant_period_type: request.free_periodic_grant_period_type,
        free_periodic_validity_days: request.free_periodic_validity_days,
        free_periodic_quota_windows: materialize_quota_windows(
            request.free_periodic_quota_windows,
        )?,
    };

    match state
        .realm_config_service
        .update_realm_config(admin.identity().clone(), &realm_id, input)
        .await
    {
        Ok(config) => Ok(Json(realm_config_to_response(config))),
        Err(e) => Err(ApiError::from(e)),
    }
}
