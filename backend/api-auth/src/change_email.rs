use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use axum_valid::Valid;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use herald_api_base::application::http::auth::util::{
    ClientIp, epoch_seconds, normalize_email, rate_limit_hit, require_session,
};
pub use herald_api_base::application::http::server::api_entities::ErrorResponse;
use herald_api_base::application::http::server::api_entities::{ApiError, ApiResult};
use herald_api_base::application::http::state::AppState;
use herald_core::third::email::EmailService;

#[derive(Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ChangeEmailRequest {
    #[validate(email)]
    pub new_email: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ChangeEmailResponse {
    pub message: String,
}

/// Request to change the email address for the authenticated user
///
/// Initiates an email change process by sending a verification code to the new email address.
/// The user must click the confirmation link in the email to complete the change.
#[utoipa::path(
  post,
  path = "/api/auth/{realmId}/change_email/request",
  tag = "auth",
  operation_id = "change_email_request",
  params(
    ("realmId" = String, Path, description = "Realm ID")
  ),
  request_body = ChangeEmailRequest,
  responses(
    (status = 200, description = "Change email request accepted.", body = ChangeEmailResponse),
    (status = 400, description = "Bad request", body = ErrorResponse),
    (status = 401, description = "Unauthorized", body = ErrorResponse),
    (status = 409, description = "Email already in use", body = ErrorResponse),
    (status = 429, description = "Too many requests", body = ErrorResponse),
    (status = 500, description = "Internal server error", body = ErrorResponse)
  )
)]
pub async fn request(
    Path(_realm_id): Path<String>,
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    headers: HeaderMap,
    Valid(Json(payload)): Valid<Json<ChangeEmailRequest>>,
) -> Result<ApiResult<ChangeEmailResponse>, ApiError> {
    let (_token, sess) = require_session(&state, &headers).await?;
    let new_email = normalize_email(&payload.new_email);

    // Apply rate limiting: 1 request per minute per IP and email
    rate_limit_hit(&state, format!("rl:change_email:req:ip:{ip}"), 1, 120).await?;
    rate_limit_hit(
        &state,
        format!("rl:change_email:req:email:{new_email}"),
        1,
        120,
    )
    .await?;

    // Generate verification code and create email change request
    let code =
        request_email_change_internal(&state, &sess.realm_id, &sess.user_id, &new_email).await?;

    // Send confirmation email
    send_confirmation_email(&state, &sess.realm_id, &new_email, &code).await?;

    Ok(ApiResult::ok(ChangeEmailResponse {
        message: "ok".to_string(),
    }))
}

/// Internal function to create email change request
/// This contains the business logic for generating and storing verification codes
async fn request_email_change_internal(
    state: &AppState,
    realm_id: &str,
    user_id: &str,
    new_email: &str,
) -> Result<String, ApiError> {
    let code = format!(
        "{}_{}_{}_{}",
        realm_id,
        user_id,
        uuid::Uuid::now_v7(),
        epoch_seconds()
    );

    // Store verification code in database
    sqlx::query(
        "INSERT INTO email_verification_code (email, type, verification_code) VALUES ($1, $2, $3)",
    )
    .bind(new_email)
    .bind("change_email")
    .bind(&code)
    .execute(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create email verification code: {}", e);
        ApiError::internal("Failed to create verification code")
    })?;

    Ok(code)
}

/// Internal function to send confirmation email
async fn send_confirmation_email(
    state: &AppState,
    realm_id: &str,
    new_email: &str,
    code: &str,
) -> Result<(), ApiError> {
    let link = format!(
        "{}/api/auth/{}/change_email/confirm/{}",
        state.public_base_url.trim_end_matches('/'),
        realm_id,
        code
    );
    let html =
        format!("<p>Please click to confirm change email:</p><p><a href=\"{link}\">{link}</a></p>");
    EmailService::send_html_email(
        &state.pool,
        realm_id,
        new_email,
        "Confirm your new email",
        &html,
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to send confirmation email: {}", e);
        ApiError::internal("Failed to send confirmation email")
    })?;
    Ok(())
}

/// Confirm email change with verification code
///
/// Completes the email change process using the verification code sent to the new email address.
#[utoipa::path(
  get,
  path = "/api/auth/{realmId}/change_email/confirm/{changeCode}",
  tag = "auth",
  operation_id = "change_email_confirm",
  params(
    ("realmId" = String, Path, description = "Realm ID"),
    ("changeCode" = String, Path, description = "Change email confirmation code")
  ),
  responses(
    (status = 200, description = "Email changed.", body = ChangeEmailResponse),
    (status = 400, description = "Bad request", body = ErrorResponse),
    (status = 401, description = "Unauthorized", body = ErrorResponse),
    (status = 403, description = "Forbidden", body = ErrorResponse),
    (status = 409, description = "Email already in use", body = ErrorResponse),
    (status = 500, description = "Internal server error", body = ErrorResponse)
  )
)]
pub async fn confirm(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((realm_id, code)): Path<(String, String)>,
) -> Result<ApiResult<ChangeEmailResponse>, ApiError> {
    let (_token, sess) = require_session(&state, &headers).await?;

    if sess.realm_id != realm_id {
        return Err(ApiError::forbidden(
            "cannot confirm email change for a different realm",
        ));
    }

    // Use the current authenticated session as the source of truth so a stolen
    // confirmation code cannot be replayed across users or sessions.
    confirm_email_change_internal(&state, &sess.realm_id, &sess.user_id, &code).await?;

    Ok(ApiResult::ok(ChangeEmailResponse {
        message: "ok".to_string(),
    }))
}

/// Internal function to handle email change confirmation with transaction management
/// This function contains the business logic that should ideally be in a service layer
async fn confirm_email_change_internal(
    state: &AppState,
    current_realm_id: &str,
    current_user_id: &str,
    code: &str,
) -> Result<(), ApiError> {
    // Parse verification code format: realmId_userId_uuid_ts
    // Use rsplitn to split from the right so realm_id (which may contain underscores) is preserved
    let mut parts = code.rsplitn(3, '_');
    let _ts = parts
        .next()
        .ok_or_else(|| ApiError::bad_request("invalid change code".to_string()))?;
    let _uuid = parts
        .next()
        .ok_or_else(|| ApiError::bad_request("invalid change code".to_string()))?;
    let remainder = parts
        .next()
        .ok_or_else(|| ApiError::bad_request("invalid change code".to_string()))?;

    // remainder is "realmId_userId" — split from the right once more to get user_id cleanly
    let mut rp = remainder.rsplitn(2, '_');
    let user_id_str = rp
        .next()
        .ok_or_else(|| ApiError::bad_request("invalid change code".to_string()))?;
    let realm_id = rp.next().unwrap_or("");

    // Parse user_id as UUID for database query
    let user_id = uuid::Uuid::parse_str(user_id_str)
        .map_err(|_| ApiError::bad_request("invalid user_id in change code".to_string()))?;

    if realm_id != current_realm_id || user_id_str != current_user_id {
        return Err(ApiError::forbidden(
            "cannot confirm email change for a different user",
        ));
    }

    // Begin transaction for atomic code lookup + email update
    let mut tx = state.pool.begin().await.map_err(|e| {
        tracing::error!("Failed to begin transaction: {}", e);
        ApiError::internal("Failed to begin transaction")
    })?;

    // Retrieve the new email from verification code (inside tx for atomicity)
    let new_email: Option<String> = sqlx::query_scalar(
        "SELECT email FROM email_verification_code WHERE verification_code = $1 AND type = 'change_email' ORDER BY id DESC LIMIT 1 FOR UPDATE",
    )
    .bind(code)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("Failed to retrieve email verification code: {}", e);
        ApiError::internal("Failed to retrieve verification code")
    })?;

    let new_email =
        new_email.ok_or_else(|| ApiError::bad_request("change code not found".to_string()))?;

    // Update user email
    let update_result = sqlx::query(
        "UPDATE account SET email = $1, updated_at = NOW() WHERE realm_id = $2 AND id = $3",
    )
    .bind(&new_email)
    .bind(current_realm_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await;

    match update_result {
        Ok(_) => {
            // Delete the verification code after successful update
            sqlx::query("DELETE FROM email_verification_code WHERE verification_code = $1 AND type = 'change_email'")
                .bind(code)
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to delete verification code: {}", e);
                    ApiError::internal("Failed to delete verification code")
                })?;

            // Commit transaction
            tx.commit().await.map_err(|e| {
                tracing::error!("Failed to commit transaction: {}", e);
                ApiError::internal("Failed to commit transaction")
            })?;
            Ok(())
        }
        Err(e) => {
            // Handle unique constraint violation (email already exists)
            if let sqlx::Error::Database(db_err) = &e
                && db_err.code().as_deref() == Some("23505")
            {
                return Err(ApiError::conflict("email already in use".to_string()));
            }
            tracing::error!("Failed to change email: {}", e);
            Err(ApiError::internal("Failed to change email".to_string()))
        }
    }
}
