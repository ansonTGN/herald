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
    extract_ip, normalize_email, rate_limit_hit, verify_turnstile_for_realm,
};
pub use herald_api_base::application::http::server::api_entities::ErrorResponse;
use herald_api_base::application::http::server::api_entities::{ApiError, ApiResult};
use herald_api_base::application::http::state::AppState;
use herald_core::domain::user::ports::UserService;

#[derive(Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ResetPasswordRequestRequest {
    #[validate(email)]
    pub email: String,
    pub turnstile_token: Option<String>, // Optional: required only if Turnstile is enabled for realm
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ResetPasswordRequestResponse {
    pub message: String,
}

/// Request password reset
///
/// Initiates a password reset process by sending a verification code to user's email address.
/// Always returns success to prevent email enumeration.
#[utoipa::path(
  post,
  path = "/api/auth/{realmId}/reset_password/request",
  tag = "auth",
  operation_id = "reset_password_request",
  params(
    ("realmId" = String, Path, description = "Realm ID")
  ),
  request_body = ResetPasswordRequestRequest,
  responses(
    (status = 200, description = "Request accepted (always ok).", body = ResetPasswordRequestResponse),
    (status = 400, description = "Bad request", body = ErrorResponse),
    (status = 429, description = "Too many requests", body = ErrorResponse),
    (status = 500, description = "Internal server error", body = ErrorResponse)
  )
)]
pub async fn request(
    Path(realm_id): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Valid(Json(payload)): Valid<Json<ResetPasswordRequestRequest>>,
) -> Result<ApiResult<ResetPasswordRequestResponse>, ApiError> {
    let ip = extract_ip(&headers);
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
    rate_limit_hit(&state, format!("rl:reset_password:req:ip:{ip}"), 5, 60).await?;
    rate_limit_hit(
        &state,
        format!("rl:reset_password:req:email:{email}"),
        5,
        60,
    )
    .await?;

    // Use UserService to request password reset
    let code = state
        .service
        .user_service()
        .reset_password_request(&realm_id, &email, "reset_password")
        .await
        .map_err(|e| {
            tracing::error!("Failed to create reset password code: {}", e);
            ApiError::internal("Failed to store reset password code".to_string())
        })?;

    // Send email
    if let Some(resend) = &state.resend {
        let link = format!(
            "{}/api/{}/auth/reset_password/confirm/{}",
            state.public_base_url.trim_end_matches('/'),
            realm_id,
            code
        );
        let html =
            format!("<p>please click to reset passowrd：</p><p><a href=\"{link}\">{link}</a></p>");
        // best effort：不把邮件失败暴露给调用方（仍返回 ok）
        let _ = resend.send_html(&email, "Reset your password", &html).await;
    }

    Ok(ApiResult::ok(ResetPasswordRequestResponse {
        message: "ok".to_string(),
    }))
}

#[derive(Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ResetPasswordConfirmRequest {
    #[validate(length(min = 8, max = 36))]
    pub new_pass: String,
    pub turnstile_token: Option<String>, // Optional: required only if Turnstile is enabled for realm
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ResetPasswordConfirmResponse {
    pub message: String,
}

/// Confirm password reset with verification code
///
/// Completes password reset process using verification code sent to user's email.
#[utoipa::path(
  post,
  path = "/api/auth/{realmId}/reset_password/confirm/{resetCode}",
  tag = "auth",
  operation_id = "reset_password_confirm",
  params(
    ("realmId" = String, Path, description = "Realm ID"),
    ("resetCode" = String, Path, description = "Reset password code")
  ),
  request_body = ResetPasswordConfirmRequest,
  responses(
    (status = 200, description = "Password reset successful.", body = ResetPasswordConfirmResponse),
    (status = 400, description = "Bad request", body = ErrorResponse),
    (status = 429, description = "Too many requests", body = ErrorResponse),
    (status = 500, description = "Internal server error", body = ErrorResponse)
  )
)]
pub async fn confirm(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((realm_id, code)): Path<(String, String)>,
    Valid(Json(payload)): Valid<Json<ResetPasswordConfirmRequest>>,
) -> Result<ApiResult<ResetPasswordConfirmResponse>, ApiError> {
    let ip = extract_ip(&headers);

    // turnstile 校验（根据 realm 配置动态判断）
    verify_turnstile_for_realm(
        &state,
        &realm_id,
        payload.turnstile_token.as_deref(),
        Some(&ip),
    )
    .await?;

    rate_limit_hit(&state, format!("rl:reset_password:confirm:ip:{ip}"), 5, 60).await?;

    // Use UserService to confirm password reset
    state
        .service
        .user_service()
        .reset_password_confirm(&code, payload.new_pass, &realm_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to confirm password reset: {}", e);
            match e {
                herald_core::domain::common::entities::app_errors::CoreError::BadRequest(msg) => {
                    ApiError::bad_request(msg)
                }
                _ => ApiError::internal("Failed to reset password".to_string()),
            }
        })?;

    Ok(ApiResult::ok(ResetPasswordConfirmResponse {
        message: "ok".to_string(),
    }))
}
