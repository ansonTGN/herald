use axum::{
    Json,
    extract::{Path, State},
};
use axum_valid::Valid;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use herald_api_base::application::http::auth::util::{
    ClientIp, normalize_email, rate_limit_hit, verify_turnstile_for_realm,
};
pub use herald_api_base::application::http::server::api_entities::ErrorResponse;
use herald_api_base::application::http::server::api_entities::{ApiError, ApiResult};
use herald_api_base::application::http::state::AppState;
use herald_core::domain::security_constants::RESET_PASSWORD_CONFIRM_IP_RATE_LIMIT;
use herald_core::domain::user::ports::UserService;
use herald_core::third::email::EmailService;

#[derive(Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct VerifyEmailTriggerRequest {
    #[validate(email)]
    pub email: String,
    pub turnstile_token: Option<String>, // Optional: required only if Turnstile is enabled for realm
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct VerifyEmailTriggerResponse {
    pub message: String,
}

/// Trigger email verification
///
/// Sends a verification email to specified email address with a confirmation link.
/// Used to initiate email verification for pending accounts.
#[utoipa::path(
  post,
  path = "/api/auth/{realmId}/verify_email/trigger",
  tag = "auth",
  operation_id = "verify_email_trigger",
  params(
    ("realmId" = String, Path, description = "Realm ID")
  ),
  request_body = VerifyEmailTriggerRequest,
  responses(
    (status = 200, description = "Email verification triggered.", body = VerifyEmailTriggerResponse),
    (status = 400, description = "Bad request", body = ErrorResponse),
    (status = 401, description = "Unauthorized", body = ErrorResponse),
    (status = 429, description = "Too many requests", body = ErrorResponse),
    (status = 500, description = "Internal server error", body = ErrorResponse)
  )
)]
pub async fn trigger(
    Path(realm_id): Path<String>,
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    Valid(Json(payload)): Valid<Json<VerifyEmailTriggerRequest>>,
) -> Result<ApiResult<VerifyEmailTriggerResponse>, ApiError> {
    let email = normalize_email(&payload.email);

    // turnstile 校验（根据 realm 配置动态判断）
    verify_turnstile_for_realm(
        &state,
        &realm_id,
        payload.turnstile_token.as_deref(),
        Some(&ip),
    )
    .await?;

    // ip + email 限流：每分钟最多 5 次
    rate_limit_hit(&state, format!("rl:verify_email:ip:{ip}"), 5, 60).await?;
    rate_limit_hit(&state, format!("rl:verify_email:email:{email}"), 5, 60).await?;

    // Use UserService to trigger email verification
    let code = state
        .service
        .user_service()
        .verify_email_trigger(&realm_id, &email, "verify_email")
        .await
        .map_err(|e| {
            tracing::error!("Failed to store email verification code: {e}");
            ApiError::internal("Failed to store email verification code".to_string())
        })?;

    let link = format!(
        "{}/api/auth/{}/verify_email/confirm/{}",
        state.public_base_url.trim_end_matches('/'),
        realm_id,
        code
    );
    let html =
        format!("<p>Please click to verify your email:</p><p><a href=\"{link}\">{link}</a></p>");
    EmailService::send_html_email(&state.pool, &realm_id, &email, "Verify your email", &html)
        .await
        .map_err(|e| {
            tracing::error!("Failed to send verification email: {e}");
            ApiError::internal("Failed to send verification email".to_string())
        })?;

    Ok(ApiResult::ok(VerifyEmailTriggerResponse {
        message: "ok".to_string(),
    }))
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct VerifyEmailConfirmResponse {
    pub message: String,
}

/// Confirm email verification with code
///
/// Completes email verification process using verification code sent to user's email.
/// Activates user account upon successful verification.
#[utoipa::path(
  get,
  path = "/api/auth/{realmId}/verify_email/confirm/{emailVerificationCode}",
  tag = "auth",
  operation_id = "verify_email_confirm",
  params(
    ("realmId" = String, Path, description = "Realm ID"),
    ("emailVerificationCode" = String, Path, description = "Email verification code")
  ),
  responses(
    (status = 200, description = "Email verified.", body = VerifyEmailConfirmResponse),
    (status = 400, description = "Bad request", body = ErrorResponse),
    (status = 500, description = "Internal server error", body = ErrorResponse)
  )
)]
pub async fn confirm(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    Path((realm_id, code)): Path<(String, String)>,
) -> Result<ApiResult<VerifyEmailConfirmResponse>, ApiError> {
    rate_limit_hit(
        &state,
        format!("rl:verify_email_confirm:ip:{ip}"),
        RESET_PASSWORD_CONFIRM_IP_RATE_LIMIT.0,
        RESET_PASSWORD_CONFIRM_IP_RATE_LIMIT.1,
    )
    .await?;

    // Use UserService to verify email
    state
        .service
        .user_service()
        .verify_email(&code, &realm_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to verify email: {e}");
            match e {
                herald_core::domain::common::entities::app_errors::CoreError::BadRequest(msg) => {
                    ApiError::bad_request(msg)
                }
                _ => ApiError::internal("Failed to verify email".to_string()),
            }
        })?;

    Ok(ApiResult::ok(VerifyEmailConfirmResponse {
        message: "ok".to_string(),
    }))
}
