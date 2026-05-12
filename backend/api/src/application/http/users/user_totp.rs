use axum::extract::{Extension, Json, State};
use axum_valid::Valid;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

pub use crate::application::http::server::api_entities::ErrorResponse;
use crate::application::http::server::api_entities::{ApiError, ApiResult};
use crate::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::user::ports::UserRepository;
use herald_core::domain::user_totp::{
    RealmTotpConfigRepository, UserTotpBackupCode, UserTotpConfig, UserTotpRepository,
    UserTotpService,
};
use herald_core::infrastructure::user::repositories::PostgresUserRepository;
use herald_core::infrastructure::user_totp::{
    PostgresRealmTotpConfigRepository, PostgresUserTotpRepository,
};

// ============================================================================
// Enable TOTP
// ============================================================================

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct EnableTotpRequest {
    pub password: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EnableTotpResponse {
    pub secret: String,
    pub qr_code_url: String,
    pub backup_codes: Vec<String>,
    pub temp_token: String,
}

/// Enable TOTP two-factor authentication for the current user
///
/// Initiates TOTP setup by generating a secret key, QR code URL, and backup codes.
/// The user must verify the TOTP code to complete the setup. Requires password verification.
#[utoipa::path(
    post,
    path = "/api/user/totp",
    tag = "user",
    request_body = EnableTotpRequest,
    responses(
        (status = 200, description = "TOTP setup initiated", body = EnableTotpResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "TOTP not enabled for realm or already enabled", body = ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handle_enable_totp(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Valid(Json(req)): Valid<Json<EnableTotpRequest>>,
) -> Result<ApiResult<EnableTotpResponse>, ApiError> {
    let user_id = Uuid::parse_str(&identity.user_id()).map_err(|e| {
        tracing::error!("Invalid user_id format in identity: {}", e);
        ApiError::internal("Invalid user_id format".to_string())
    })?;

    // 1. Verify current password
    let user_repo = PostgresUserRepository::new(state.db.clone());
    let user = user_repo.get_user_by_id(user_id).await?;

    if user.password_hash.is_none() {
        return Err(ApiError::bad_request(
            "Password authentication not enabled for this account".to_string(),
        ));
    }

    let password_valid = bcrypt::verify(
        &req.password,
        user.password_hash
            .as_ref()
            .ok_or_else(|| ApiError::internal("Password hash not found".to_string()))?,
    )
    .map_err(|_| ApiError::internal("Internal server error".to_string()))?;

    if !password_valid {
        return Err(ApiError::unauthorized("Invalid password".to_string()));
    }

    // 2. Check Realm TOTP configuration
    let realm_repo = PostgresRealmTotpConfigRepository::new(state.db.clone());
    let realm_config = realm_repo.get_realm_totp_config(&user.realm_id).await?;

    let realm_config = realm_config
        .ok_or_else(|| ApiError::bad_request("Realm TOTP is not enabled".to_string()))?;

    if !realm_config.enabled {
        return Err(ApiError::bad_request(
            "Realm TOTP is not enabled".to_string(),
        ));
    }

    // 3. Check if user already has TOTP enabled
    let totp_repo = PostgresUserTotpRepository::new(state.db.clone());
    let existing_config = totp_repo.get_config_by_user_id(user_id).await?;

    // If config exists and is enabled, don't allow restart
    if let Some(ref config) = existing_config
        && config.enabled
    {
        return Err(ApiError::conflict("TOTP already enabled".to_string()));
    }

    // If config exists but is not enabled, delete and recreate
    if existing_config.is_some() {
        totp_repo.delete_config(user_id).await?;
        // Note: delete_config will cascade delete backup codes via ON DELETE CASCADE
    }

    // 4. Generate TOTP secret and backup codes
    let secret = UserTotpService::generate_secret();
    let backup_codes = UserTotpService::generate_backup_codes();

    // 5. Encrypt secret and hash backup codes
    let secret_hash = UserTotpService::encrypt_secret(&secret)?;
    let backup_codes_hashes: Result<Vec<String>, _> = backup_codes
        .iter()
        .map(|code| UserTotpService::hash_backup_code(code))
        .collect();
    let backup_codes_hashes = backup_codes_hashes?;

    // 6. Create TOTP config (disabled until verified)
    // Note: key_version is fixed at 1, reserved for future key rotation
    let totp_config = UserTotpConfig::new(user_id, user.realm_id.clone(), secret_hash, 1);
    let totp_config = totp_repo.create_config(totp_config).await?;

    // 7. Create backup codes
    let backup_code_entities: Vec<UserTotpBackupCode> = backup_codes_hashes
        .iter()
        .map(|hash| UserTotpBackupCode::new(totp_config.id, hash.clone()))
        .collect();
    totp_repo.create_backup_codes(backup_code_entities).await?;

    // 8. Generate temp token for verification step
    let temp_token = format!("totp_setup_{}", Uuid::now_v7());
    let temp_key = format!("totp:setup:temp:{}", temp_token);
    let temp_data = serde_json::json!({
        "user_id": user_id.to_string(),
        "totp_config_id": totp_config.id.to_string(),
    });

    let mut conn = state
        .redis_manager
        .get()
        .await
        .map_err(|_| ApiError::internal("Internal server error".to_string()))?;

    let _: () = conn
        .set_ex(&temp_key, temp_data.to_string(), 300) // 5 minutes
        .await
        .map_err(|e| {
            tracing::error!("Failed to store temp token: {}", e);
            ApiError::internal("Internal server error".to_string())
        })?;

    // 9. Generate QR code URL
    let qr_code_url = UserTotpService::generate_qr_code_url(&secret, &user.email, &user.realm_id);

    Ok(ApiResult::ok(EnableTotpResponse {
        secret,
        qr_code_url,
        backup_codes,
        temp_token,
    }))
}

// ============================================================================
// Verify TOTP (Setup)
// ============================================================================

#[derive(Debug, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct VerifyTotpSetupRequest {
    pub temp_token: String,
    pub code: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VerifyTotpSetupResponse {
    pub message: String,
    pub enabled_at: String,
}

/// Verify TOTP setup and enable two-factor authentication
///
/// Completes the TOTP setup process by verifying the TOTP code generated from the secret.
/// On successful verification, TOTP is enabled for the user account.
#[utoipa::path(
    post,
    path = "/api/user/totp/verify",
    tag = "user",
    request_body = VerifyTotpSetupRequest,
    responses(
        (status = 200, description = "TOTP enabled successfully", body = VerifyTotpSetupResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Invalid or expired temp token or TOTP code", body = ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handle_verify_totp_setup(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Valid(Json(req)): Valid<Json<VerifyTotpSetupRequest>>,
) -> Result<ApiResult<VerifyTotpSetupResponse>, ApiError> {
    let user_id = Uuid::parse_str(&identity.user_id()).map_err(|e| {
        tracing::error!("Invalid user_id format in identity: {}", e);
        ApiError::internal("Invalid user_id format".to_string())
    })?;

    // 1. Retrieve and validate temp token
    let temp_key = format!("totp:setup:temp:{}", req.temp_token);
    let mut conn = state
        .redis_manager
        .get()
        .await
        .map_err(|_| ApiError::internal("Internal server error".to_string()))?;

    let temp_data_json: Option<String> = conn.get(&temp_key).await.map_err(|e| {
        tracing::error!("Failed to get temp token: {}", e);
        ApiError::internal("Failed to get temp token".to_string())
    })?;

    let temp_data_json = temp_data_json.ok_or(ApiError::unauthorized(
        "Invalid or expired temporary token".to_string(),
    ))?;

    let temp_data: serde_json::Value = serde_json::from_str(&temp_data_json).map_err(|e| {
        tracing::error!("Failed to parse temp token JSON: {}", e);
        ApiError::internal("Failed to parse temp token JSON".to_string())
    })?;

    let temp_user_id = temp_data["user_id"]
        .as_str()
        .ok_or(ApiError::internal("Internal server error".to_string()))?;
    let totp_config_id_str = temp_data["totp_config_id"]
        .as_str()
        .ok_or(ApiError::internal("Internal server error".to_string()))?;

    // 2. Verify user_id matches
    if temp_user_id != user_id.to_string() {
        return Err(ApiError::unauthorized("User ID mismatch".to_string()));
    }

    // 3. Get TOTP config
    let totp_config_id = Uuid::parse_str(totp_config_id_str).map_err(|_| {
        tracing::error!("Invalid totp_config_id format: {}", totp_config_id_str);
        ApiError::internal("Invalid totp_config_id format".to_string())
    })?;

    let totp_repo = PostgresUserTotpRepository::new(state.db.clone());
    let mut totp_config = totp_repo.get_config_by_id(totp_config_id).await?;

    // 4. Verify TOTP code
    let secret = UserTotpService::decrypt_secret(&totp_config.secret_hash)?;
    let verified = UserTotpService::verify_totp(&secret, &req.code)?;

    if !verified {
        return Err(ApiError::unauthorized("Invalid TOTP code".to_string()));
    }

    // 5. Enable TOTP config
    totp_config.enable();
    let totp_config = totp_repo.update_config(totp_config).await?;

    // 6. Delete temp token
    let _: () = conn.del(&temp_key).await.map_err(|e| {
        tracing::error!("Failed to delete temp token: {}", e);
        ApiError::internal("Failed to delete temp token".to_string())
    })?;

    Ok(ApiResult::ok(VerifyTotpSetupResponse {
        message: "TOTP enabled successfully".to_string(),
        enabled_at: totp_config
            .verified_at
            .ok_or_else(|| ApiError::internal("Verified at not found".to_string()))?
            .to_rfc3339(),
    }))
}

// ============================================================================
// Disable TOTP
// ============================================================================

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct DisableTotpRequest {
    pub password: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DisableTotpResponse {
    pub message: String,
    pub disabled_at: String,
}

/// Disable TOTP two-factor authentication for the current user
///
/// Disables TOTP and removes the configuration. Requires password verification.
/// Cannot be disabled if the realm has force-enabled TOTP.
#[utoipa::path(
    delete,
    path = "/api/user/totp",
    tag = "user",
    request_body = DisableTotpRequest,
    responses(
        (status = 200, description = "TOTP disabled successfully", body = DisableTotpResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Invalid password", body = ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handle_disable_totp(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Valid(Json(req)): Valid<Json<DisableTotpRequest>>,
) -> Result<ApiResult<DisableTotpResponse>, ApiError> {
    let user_id = Uuid::parse_str(&identity.user_id()).map_err(|e| {
        tracing::error!("Invalid user_id format in identity: {}", e);
        ApiError::internal("Invalid user_id format".to_string())
    })?;

    // 1. Verify current password
    let user_repo = PostgresUserRepository::new(state.db.clone());
    let user = user_repo.get_user_by_id(user_id).await?;

    let password_valid = bcrypt::verify(
        &req.password,
        user.password_hash
            .as_ref()
            .ok_or_else(|| ApiError::internal("Password hash not found".to_string()))?,
    )
    .map_err(|_| ApiError::internal("Internal server error".to_string()))?;

    if !password_valid {
        return Err(ApiError::unauthorized("Invalid password".to_string()));
    }

    // 2. Check Realm TOTP force_enabled setting
    let realm_repo = PostgresRealmTotpConfigRepository::new(state.db.clone());
    let realm_config = realm_repo.get_realm_totp_config(&user.realm_id).await?;

    if let Some(config) = realm_config
        && config.force_enabled
    {
        return Err(ApiError::forbidden("TOTP is enforced by realm".to_string()));
    }

    // 3. Get config and delete it with associated backup codes
    let totp_repo = PostgresUserTotpRepository::new(state.db.clone());
    let config = totp_repo.get_config_by_user_id(user_id).await?;
    if let Some(_config) = config {
        totp_repo.delete_config(user_id).await?;
        // Note: delete_config will cascade delete backup codes via ON DELETE CASCADE
    }

    Ok(ApiResult::ok(DisableTotpResponse {
        message: "TOTP disabled successfully".to_string(),
        disabled_at: chrono::Utc::now().to_rfc3339(),
    }))
}

// ============================================================================
// Regenerate TOTP Secret
// ============================================================================

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct RegenerateTotpRequest {
    pub password: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegenerateTotpResponse {
    pub secret: String,
    pub qr_code_url: String,
    pub backup_codes: Vec<String>,
    pub temp_token: String,
}

/// Regenerate TOTP secret and backup codes
///
/// Generates a new TOTP secret and backup codes, invalidating the old ones.
/// The user must verify the new TOTP code to complete the regeneration. Requires password verification.
#[utoipa::path(
    post,
    path = "/api/user/totp/regenerate",
    tag = "user",
    request_body = RegenerateTotpRequest,
    responses(
        (status = 200, description = "TOTP secret regenerated", body = RegenerateTotpResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Invalid password", body = ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handle_regenerate_totp(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Valid(Json(req)): Valid<Json<RegenerateTotpRequest>>,
) -> Result<ApiResult<RegenerateTotpResponse>, ApiError> {
    let user_id = Uuid::parse_str(&identity.user_id()).map_err(|e| {
        tracing::error!("Invalid user_id format in identity: {}", e);
        ApiError::internal("Invalid user_id format".to_string())
    })?;

    // 1. Verify current password
    let user_repo = PostgresUserRepository::new(state.db.clone());
    let user = user_repo.get_user_by_id(user_id).await?;

    let password_valid = bcrypt::verify(
        &req.password,
        user.password_hash
            .as_ref()
            .ok_or_else(|| ApiError::internal("Password hash not found".to_string()))?,
    )
    .map_err(|_| ApiError::internal("Internal server error".to_string()))?;

    if !password_valid {
        return Err(ApiError::unauthorized("Invalid password".to_string()));
    }

    // 2. Get existing TOTP config
    let totp_repo = PostgresUserTotpRepository::new(state.db.clone());
    let existing_config = totp_repo
        .get_config_by_user_id(user_id)
        .await?
        .ok_or_else(|| ApiError::not_found("TOTP not configured".to_string()))?;

    // 3. Generate new TOTP secret and backup codes
    let secret = UserTotpService::generate_secret();
    let backup_codes = UserTotpService::generate_backup_codes();

    // 4. Encrypt new secret and hash backup codes
    let secret_hash = UserTotpService::encrypt_secret(&secret)?;
    let backup_codes_hashes: Result<Vec<String>, _> = backup_codes
        .iter()
        .map(|code| UserTotpService::hash_backup_code(code))
        .collect();
    let backup_codes_hashes = backup_codes_hashes?;

    // 5. Update config with new secret (disabled until verified)
    let mut updated_config = existing_config.clone();
    updated_config.regenerate_secret(secret_hash);
    let updated_config = totp_repo.update_config(updated_config).await?;

    // 6. Delete old backup codes and create new ones
    totp_repo.delete_backup_codes(updated_config.id).await?;
    let backup_code_entities: Vec<UserTotpBackupCode> = backup_codes_hashes
        .iter()
        .map(|hash| UserTotpBackupCode::new(updated_config.id, hash.clone()))
        .collect();
    totp_repo.create_backup_codes(backup_code_entities).await?;

    // 7. Generate temp token for verification step
    let temp_token = format!("totp_regen_{}", Uuid::now_v7());
    let temp_key = format!("totp:setup:temp:{}", temp_token);
    let temp_data = serde_json::json!({
        "user_id": user_id.to_string(),
        "totp_config_id": updated_config.id.to_string(),
    });

    let mut conn = state
        .redis_manager
        .get()
        .await
        .map_err(|_| ApiError::internal("Internal server error".to_string()))?;

    let _: () = conn
        .set_ex(&temp_key, temp_data.to_string(), 300) // 5 minutes
        .await
        .map_err(|e| {
            tracing::error!("Failed to store temp token: {}", e);
            ApiError::internal("Internal server error".to_string())
        })?;

    // 8. Generate QR code URL
    let qr_code_url = UserTotpService::generate_qr_code_url(&secret, &user.email, &user.realm_id);

    Ok(ApiResult::ok(RegenerateTotpResponse {
        secret,
        qr_code_url,
        backup_codes,
        temp_token,
    }))
}

// ============================================================================
// Get TOTP Status
// ============================================================================

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TotpStatusResponse {
    pub enabled: bool,
    pub enabled_at: Option<String>,
    pub last_verified_at: Option<String>,
    pub backup_codes: BackupCodeStatsResponse,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct BackupCodeStatsResponse {
    pub total: i32,
    pub remaining: i32,
    pub used: i32,
}

/// Get TOTP configuration status for the current user
///
/// Returns the TOTP status including whether it is enabled, when it was enabled,
/// and backup code usage statistics.
#[utoipa::path(
    get,
    path = "/api/user/totp/status",
    tag = "user",
    responses(
        (status = 200, description = "TOTP status retrieved", body = TotpStatusResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handle_get_totp_status(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
) -> Result<ApiResult<TotpStatusResponse>, ApiError> {
    let user_id = Uuid::parse_str(&identity.user_id()).map_err(|e| {
        tracing::error!("Invalid user_id format in identity: {}", e);
        ApiError::internal("Invalid user_id format".to_string())
    })?;

    let totp_repo = PostgresUserTotpRepository::new(state.db.clone());
    let totp_config = totp_repo.get_config_by_user_id(user_id).await?;

    if totp_config.is_none() {
        return Ok(ApiResult::ok(TotpStatusResponse {
            enabled: false,
            enabled_at: None,
            last_verified_at: None,
            backup_codes: BackupCodeStatsResponse {
                total: 0,
                remaining: 0,
                used: 0,
            },
        }));
    }

    let config = totp_config.ok_or_else(|| {
        tracing::error!("totp_config should be Some after is_none() check");
        ApiError::internal("TOTP config not found".to_string())
    })?;
    let backup_stats = totp_repo.get_backup_code_stats(config.id).await?;

    Ok(ApiResult::ok(TotpStatusResponse {
        enabled: config.enabled,
        enabled_at: config.verified_at.map(|dt| dt.to_rfc3339()),
        last_verified_at: config.last_used_at.map(|dt| dt.to_rfc3339()),
        backup_codes: BackupCodeStatsResponse {
            total: backup_stats.total,
            remaining: backup_stats.remaining,
            used: backup_stats.used,
        },
    }))
}
