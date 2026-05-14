use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, header::SET_COOKIE},
    response::IntoResponse,
};
use axum_valid::Valid;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use herald_api_base::application::http::auth::util::{
    SessionData, build_set_cookie, epoch_seconds, extract_ip, rate_limit_hit, store_session,
};
use herald_api_base::application::http::server::api_entities::ApiError;
pub use herald_api_base::application::http::server::api_entities::ErrorResponse;
use herald_api_base::application::http::state::AppState;
use herald_core::domain::client::ports::ClientService;
use herald_core::domain::security_constants::{
    TOTP_LOCKOUT_SECONDS, TOTP_MAX_FAILURES, TOTP_VERIFY_IP_RATE_LIMIT, TOTP_VERIFY_USER_RATE_LIMIT,
};
use herald_core::domain::user_totp::{
    TotpVerificationResultWithBackup, UserTotpRepository, UserTotpService,
};
use herald_core::infrastructure::user_totp::PostgresUserTotpRepository;

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VerifyTotpRequest {
    pub temp_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_code: Option<String>,
}

impl Validate for VerifyTotpRequest {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        let mut errors = validator::ValidationErrors::new();

        // Validate temp_token
        if self.temp_token.is_empty() {
            errors.add("temp_token", validator::ValidationError::new("required"));
        }

        // Validate that either code or backup_code is provided
        if self.code.is_none() && self.backup_code.is_none() {
            errors.add(
                "",
                validator::ValidationError::new("either_code_or_backup_code_required"),
            );
        }

        // Validate code format if provided
        if let Some(code) = &self.code
            && (code.len() != 6 || !code.chars().all(|c| c.is_ascii_digit()))
        {
            errors.add("code", validator::ValidationError::new("invalid_format"));
        }

        // Validate backup_code format if provided
        if let Some(backup_code) = &self.backup_code
            && (backup_code.len() != 6 || !backup_code.chars().all(|c| c.is_alphanumeric()))
        {
            errors.add(
                "backup_code",
                validator::ValidationError::new("invalid_format"),
            );
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VerifyTotpResponse {
    pub message: String,
    pub user_id: String,
    pub token: String,
    pub expires_in_seconds: i64,
}

/// Temporary session data for TOTP verification
#[derive(Serialize, Deserialize)]
struct TempSessionData {
    user_id: String,
    realm_id: String,
    client_id: String,
    client_ip: String,
}

/// Verify TOTP code for two-factor authentication
///
/// Completes the login process for users with TOTP enabled. Accepts either a TOTP code
/// from an authenticator app or a backup code. Creates a permanent session on success.
#[utoipa::path(
    post,
    path = "/api/auth/{realmId}/login/verify-totp",
    tag = "auth",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    request_body = VerifyTotpRequest,
    responses(
        (status = 200, description = "TOTP verification successful", body = VerifyTotpResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Invalid TOTP code or backup code", body = ErrorResponse),
        (status = 429, description = "Too many attempts", body = ErrorResponse),
    )
)]
pub async fn handle_verify_totp(
    Path(realm_id): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Valid(Json(req)): Valid<Json<VerifyTotpRequest>>,
) -> Result<impl IntoResponse, ApiError> {
    let client_ip = extract_ip(&headers);

    // 1. Validate and retrieve temp session from Redis
    let temp_key = format!("totp:temp:{}", req.temp_token);
    let mut conn = state
        .redis_manager
        .get()
        .await
        .map_err(|_| ApiError::internal("Redis connection error".to_string()))?;

    let temp_session_json: Option<String> = conn.get(&temp_key).await.map_err(|e| {
        tracing::error!("Failed to get temp session from Redis: {}", e);
        ApiError::internal("Redis operation error".to_string())
    })?;

    let temp_session_json = temp_session_json.ok_or(ApiError::unauthorized(
        "Invalid or expired temporary token".to_string(),
    ))?;

    let temp_session: TempSessionData = serde_json::from_str(&temp_session_json).map_err(|e| {
        tracing::error!("Failed to parse temp session JSON: {}", e);
        ApiError::internal("Redis operation error".to_string())
    })?;

    // Look up client app for session TTL
    let client_app = state
        .service
        .client_service()
        .get_client_app_by_client_id(&temp_session.realm_id, &temp_session.client_id)
        .await
        .map_err(|_| ApiError::internal("Client app lookup failed".to_string()))?;
    let session_ttl = client_app.session_ttl_seconds as usize;

    // 2. Check rate limits
    rate_limit_hit(
        &state,
        format!("totp:verify:user:{}", temp_session.user_id),
        TOTP_VERIFY_USER_RATE_LIMIT.0,
        TOTP_VERIFY_USER_RATE_LIMIT.1,
    )
    .await?;
    rate_limit_hit(
        &state,
        format!("totp:verify:ip:{}", client_ip),
        TOTP_VERIFY_IP_RATE_LIMIT.0,
        TOTP_VERIFY_IP_RATE_LIMIT.1,
    )
    .await?;

    // 3. Check failure count (Redis)
    let fail_count_key = format!("totp:fail_count:{}", temp_session.user_id);
    let fail_count: Option<i64> = conn.get(&fail_count_key).await.map_err(|e| {
        tracing::error!("Failed to get fail count from Redis: {}", e);
        ApiError::internal("Redis operation error".to_string())
    })?;

    if let Some(count) = fail_count
        && count >= TOTP_MAX_FAILURES
    {
        return Err(ApiError::too_many_requests(
            "Too many failed attempts. Please try again in 15 minutes.".to_string(),
        ));
    }

    // 4. Get user TOTP configuration
    let user_id = Uuid::parse_str(&temp_session.user_id).map_err(|_| {
        tracing::error!(
            "Invalid user_id format in temp session: {}",
            temp_session.user_id
        );
        ApiError::internal("Redis operation error".to_string())
    })?;

    let totp_repo = PostgresUserTotpRepository::new(state.db.clone());
    let totp_config =
        totp_repo
            .get_config_by_user_id(user_id)
            .await?
            .ok_or(ApiError::unauthorized(
                "TOTP not configured for this user".to_string(),
            ))?;

    if !totp_config.enabled {
        return Err(ApiError::unauthorized("TOTP is not enabled".to_string()));
    }

    // 5. Verify TOTP code or backup code using service layer
    let using_totp_code = req.code.is_some();

    // For backup codes, we need to fetch them from the database
    let backup_codes = if req.backup_code.is_some() {
        totp_repo.get_backup_codes(totp_config.id).await?
    } else {
        Vec::new()
    };

    // Check for replay attacks: track last used code (only for TOTP codes)
    let last_code_data: Option<String> = if req.code.is_some() {
        let last_code_key = format!("totp:last_code:{}", temp_session.user_id);
        conn.get(&last_code_key).await.map_err(|e| {
            tracing::error!("Failed to get last TOTP code from Redis: {}", e);
            ApiError::internal("Redis operation error".to_string())
        })?
    } else {
        None
    };

    // Call service layer for verification
    let verification_result = UserTotpService::verify_totp_or_backup_code(
        &totp_config,
        req.code.clone(),
        req.backup_code.clone(),
        backup_codes,
        last_code_data.as_deref(),
    )?;

    // Handle verification result
    match verification_result {
        TotpVerificationResultWithBackup::Valid => {
            // Store this code as the last used code with current timestamp
            let last_code_key = format!("totp:last_code:{}", temp_session.user_id);
            let code_data = format!("{}:{}", req.code.unwrap(), epoch_seconds());
            let _: () = conn.set(&last_code_key, code_data).await.map_err(|e| {
                tracing::error!("Failed to store last TOTP code in Redis: {}", e);
                ApiError::internal("Redis operation error".to_string())
            })?;
            // Expire the tracking after 2 minutes (4 time steps)
            let _: () = conn.expire(&last_code_key, 120).await.map_err(|e| {
                tracing::error!("Failed to set expiry on last TOTP code: {}", e);
                ApiError::internal("Redis operation error".to_string())
            })?;
        }
        TotpVerificationResultWithBackup::BackupCodeUsed(code_id) => {
            // Mark backup code as used
            totp_repo.mark_backup_code_used(code_id).await?;
        }
        TotpVerificationResultWithBackup::Expired => {
            tracing::debug!(
                user_id = %temp_session.user_id,
                "TOTP code expired (outside valid time window)"
            );
            // Fall through to error handling
        }
        TotpVerificationResultWithBackup::Replay => {
            tracing::warn!(
                user_id = %temp_session.user_id,
                "TOTP code reuse detected (replay attack)"
            );
            // Fall through to error handling
        }
    }

    // Check if verification failed
    if !matches!(
        verification_result,
        TotpVerificationResultWithBackup::Valid
            | TotpVerificationResultWithBackup::BackupCodeUsed(_)
    ) {
        // Record failure
        let _: () = conn.incr(&fail_count_key, 1).await.map_err(|e| {
            tracing::error!("Failed to increment fail count: {}", e);
            ApiError::internal("Redis operation error".to_string())
        })?;
        let _: () = conn
            .expire(&fail_count_key, TOTP_LOCKOUT_SECONDS as i64)
            .await
            .map_err(|e| {
                tracing::error!("Failed to set expiry on fail count: {}", e);
                ApiError::internal("Redis operation error".to_string())
            })?;

        // Delete temp token on failure (security measure)
        let _: () = conn.del(&temp_key).await.map_err(|e| {
            tracing::error!("Failed to delete temp token: {}", e);
            ApiError::internal("Redis operation error".to_string())
        })?;

        // Return specific error message
        let error_message = if using_totp_code {
            "验证码已过期或无效".to_string() // Verification code has expired or is invalid
        } else {
            "Invalid verification code".to_string()
        };

        return Err(ApiError::unauthorized(error_message));
    }

    // 6. Delete temp token and failure count
    let _: () = conn.del(&temp_key).await.map_err(|e| {
        tracing::error!("Failed to delete temp token: {}", e);
        ApiError::internal("Redis operation error".to_string())
    })?;
    let _: () = conn.del(&fail_count_key).await.map_err(|e| {
        tracing::error!("Failed to delete fail count: {}", e);
        ApiError::internal("Redis operation error".to_string())
    })?;

    // 7. Update TOTP last used time
    let mut updated_config = totp_config.clone();
    updated_config.update_last_used();
    totp_repo.update_config(updated_config).await?;

    // 8. Create permanent session
    let token = format!("{}_{}_{}", realm_id, temp_session.user_id, epoch_seconds());

    let session_data = SessionData {
        realm_id: temp_session.realm_id.clone(),
        client_id: temp_session.client_id,
        user_id: temp_session.user_id.clone(),
        client_ip,
    };

    store_session(&state, &token, &session_data, session_ttl).await?;

    // 9. Build response with session cookie
    let cookie_header = build_set_cookie(
        "X-Auth",
        &token,
        session_ttl as i64,
        state.app_env == "production",
    );
    let response = (
        [(SET_COOKIE, cookie_header)],
        Json(VerifyTotpResponse {
            message: "ok".to_string(),
            user_id: temp_session.user_id,
            token,
            expires_in_seconds: session_ttl as i64,
        }),
    )
        .into_response();

    Ok(response)
}
