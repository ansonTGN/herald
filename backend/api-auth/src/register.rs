use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use axum_valid::Valid;
use herald_api_base::application::http::auth::util::{
    extract_ip, is_email_verification_required, is_registration_enabled, normalize_email,
    rate_limit_hit, verify_turnstile_for_realm,
};
pub use herald_api_base::application::http::server::api_entities::ErrorResponse;
use herald_api_base::application::http::server::api_entities::{ApiError, ApiResult};
use herald_api_base::application::http::state::AppState;
use herald_core::domain::security_constants::{REGISTER_EMAIL_RATE_LIMIT, REGISTER_IP_RATE_LIMIT};
use herald_core::domain::user::ports::UserService;
use herald_core::domain::user::value_objects::RegisterRequest as DomainRegisterRequest;
use herald_core::third::email::EmailService;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 1, max = 36))]
    pub username: Option<String>, // Optional username
    #[validate(length(min = 8, max = 36))]
    pub password: String,
    pub turnstile_token: Option<String>, // Optional: required only if Turnstile is enabled for realm
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegisterResponse {
    pub message: String,
    pub verification_required: bool,
}

/// Register new user account
///
/// Creates a new user account with email and optional username. If email verification is required
/// for realm, account will be created in a pending state and a verification email will be sent.
#[utoipa::path(
  post,
  path = "/api/auth/{realmId}/register",
  tag = "auth",
  params(
    ("realmId" = String, Path, description = "Realm ID")
  ),
  request_body = RegisterRequest,
  responses(
    (status = 200, description = "Successful registration.", body = RegisterResponse),
    (status = 400, description = "Bad request", body = ErrorResponse),
    (status = 409, description = "Email already registered", body = ErrorResponse),
    (status = 429, description = "Too many requests", body = ErrorResponse),
    (status = 500, description = "Internal server error", body = ErrorResponse)
  )
)]
pub async fn register(
    Path(realm_id): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Valid(Json(payload)): Valid<Json<RegisterRequest>>,
) -> Result<ApiResult<RegisterResponse>, ApiError> {
    let email = normalize_email(&payload.email);

    tracing::info!(
        realm_id = %realm_id,
        email = %email,
        "Registration attempt"
    );

    let registration_enabled = is_registration_enabled(&state, &realm_id).await?;
    if !registration_enabled {
        tracing::debug!(
            realm_id = %realm_id,
            email = %email,
            "Registration failed: registration disabled for realm"
        );
        return Err(ApiError::bad_request(
            "Registration is not allowed for this realm".to_string(),
        ));
    }

    let ip = extract_ip(&headers);
    // turnstile 校验（根据 realm 配置动态判断）
    verify_turnstile_for_realm(
        &state,
        &realm_id,
        payload.turnstile_token.as_deref(),
        Some(&ip),
    )
    .await?;

    // ip + email 限流
    rate_limit_hit(
        &state,
        format!("rl:register:ip:{ip}"),
        REGISTER_IP_RATE_LIMIT.0,
        REGISTER_IP_RATE_LIMIT.1,
    )
    .await?;
    rate_limit_hit(
        &state,
        format!("rl:register:email:{email}"),
        REGISTER_EMAIL_RATE_LIMIT.0,
        REGISTER_EMAIL_RATE_LIMIT.1,
    )
    .await?;

    // Check if email verification is required
    let verification_required = is_email_verification_required(&state, &realm_id).await?;

    // Use UserService to register the user
    let user = state
        .service
        .user_service()
        .register(DomainRegisterRequest {
            realm_id: realm_id.clone(),
            email: email.clone(),
            password: payload.password,
        })
        .await
        .map_err(|e| {
            tracing::error!(
                realm_id = %realm_id,
                email = %email,
                error = %e,
                "Registration failed"
            );
            match e {
                herald_core::domain::common::entities::app_errors::CoreError::Conflict(msg) => {
                    ApiError::conflict(msg)
                }
                _ => ApiError::internal("Failed to create user".to_string()),
            }
        })?;

    // Default repository behavior creates users in pending status.
    // If this realm does not require email verification, activate immediately.
    if !verification_required {
        state
            .service
            .user_service()
            .activate_user(user.id)
            .await
            .map_err(|e| {
                tracing::error!(
                    user_id = %user.id,
                    error = %e,
                    "Failed to activate user after registration"
                );
                ApiError::internal("Failed to activate user".to_string())
            })?;

        if let Err(e) = state
            .registration_service
            .handle_user_registration(user.id, &realm_id)
            .await
        {
            tracing::error!(
                realm_id = %realm_id,
                user_id = %user.id,
                error = %e,
                "Failed to grant registration points, but user was created successfully"
            );
            // Don't fail registration if points grant fails
        }
    }

    // Create and send email verification code ONLY if required
    if verification_required {
        let code = state
            .service
            .user_service()
            .verify_email_trigger(&realm_id, &email, "register")
            .await
            .map_err(|e| {
                tracing::error!("Failed to create email verification code: {}", e);
                ApiError::internal("Failed to store email verification code".to_string())
            })?;

        // Send email (best effort only when configured)
        let link = format!(
            "{}/api/{}/auth/verify_email/confirm/{}",
            state.public_base_url.trim_end_matches('/'),
            realm_id,
            code
        );
        let html =
            format!("<p>Please click to verify email:</p><p><a href=\"{link}\">{link}</a></p>");
        EmailService::send_html_email(&state.pool, &realm_id, &email, "Verify your email", &html)
            .await
            .map_err(|e| {
                tracing::error!("Failed to send verification email: {e}");
                ApiError::internal("Failed to send verification email".to_string())
            })?;
    }

    tracing::info!(
        realm_id = %realm_id,
        email = %email,
        verification_required = %verification_required,
        "Registration successful"
    );

    let response = RegisterResponse {
        message: if verification_required {
            "Registration successful. Please check your email to verify your account.".to_string()
        } else {
            "Registration successful.".to_string()
        },
        verification_required,
    };

    Ok(ApiResult::ok(response))
}
