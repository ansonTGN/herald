use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, header::SET_COOKIE},
    response::{IntoResponse, Redirect},
};
use axum_valid::Valid;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use herald_api_base::application::http::auth::util::{
    build_set_cookie, extract_ip, normalize_email, rate_limit_hit, verify_turnstile_for_realm,
};
use herald_api_base::application::http::server::api_entities::ApiError;
pub use herald_api_base::application::http::server::api_entities::ErrorResponse;
use herald_api_base::application::http::state::AppState;
use herald_core::domain::audit::{
    ActorType, AuditAction, AuditCategory, AuditEventRepository, AuditResult, AuditTargetType,
    NewAuditEvent,
};
use herald_core::domain::authentication::ports::AuthenticationService;
use herald_core::domain::client::ports::ClientService;
use herald_core::domain::security_constants::{
    DEFAULT_OAUTH_SESSION_TTL_SECONDS, LOGIN_IDENTIFIER_RATE_LIMIT, LOGIN_IP_RATE_LIMIT,
};
use herald_core::domain::user::ports::UserService;
use herald_core::domain::user::value_objects::LoginRequest as DomainLoginRequest;
use herald_core::domain::user_totp::UserTotpRepository;
use herald_core::infrastructure::user_totp::PostgresUserTotpRepository;

#[derive(Serialize, Deserialize, ToSchema, validator::Validate)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequestPayload {
    #[validate(length(min = 1, max = 36))]
    pub client_id: String,
    pub username: Option<String>, // Optional username login
    pub email: Option<String>,    // Optional email login
    #[validate(length(min = 8, max = 36))]
    pub password: String,
    pub turnstile_token: Option<String>, // Optional: required only if Turnstile is enabled for the realm
    // OAuth fields (optional - for third-party app integration)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth_client_id: Option<String>, // OAuth client_id (different from client_id above)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_uri: Option<String>, // OAuth redirect URI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>, // OAuth state token (CSRF protection)
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub message: String,
    pub user_id: Uuid,
    pub realm_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires_totp: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temp_token: Option<String>,
    pub expires_in_seconds: i64,
}

/// Authenticate user with email/username and password
///
/// Supports both email and username login. Returns a session token or requires TOTP verification
/// if two-factor authentication is enabled for the user.
#[utoipa::path(
  post,
  path = "/api/auth/{realmId}/login",
  tag = "auth",
  params(
    ("realmId" = String, Path, description = "Realm ID")
  ),
  request_body = LoginRequestPayload,
  responses(
    (status = 200, description = "Successful login.", body = LoginResponse),
    (status = 400, description = "Bad request", body = ErrorResponse),
    (status = 401, description = "Unauthorized", body = ErrorResponse),
    (status = 403, description = "Forbidden", body = ErrorResponse),
    (status = 429, description = "Too many requests", body = ErrorResponse),
    (status = 500, description = "Internal server error", body = ErrorResponse)
  )
)]
pub async fn login(
    Path(realm_id): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Valid(Json(payload)): Valid<Json<LoginRequestPayload>>,
) -> Result<impl IntoResponse, ApiError> {
    let ip = extract_ip(&headers);
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Custom validation: ensure at least username or email is provided
    if payload.email.is_none() && payload.username.is_none() {
        return Err(ApiError::bad_request(
            "Either username or email must be provided".to_string(),
        ));
    }

    // Normalize if it's an email
    let normalized_email = payload.email.as_ref().map(|e| normalize_email(e));
    let normalized_username = payload.username.clone();

    tracing::debug!(
        realm_id = %realm_id,
        client_id = %payload.client_id,
        email = ?normalized_email,
        username = ?normalized_username,
        ip = %ip,
        "Login attempt"
    );

    // turnstile 校验（根据 realm 配置动态判断）
    verify_turnstile_for_realm(
        &state,
        &realm_id,
        payload.turnstile_token.as_deref(),
        Some(&ip),
    )
    .await?;

    // ip + identifier 限流
    rate_limit_hit(
        &state,
        format!("rl:login:ip:{ip}"),
        LOGIN_IP_RATE_LIMIT.0,
        LOGIN_IP_RATE_LIMIT.1,
    )
    .await?;

    // Rate limit by identifier (email or username)
    if let Some(email) = &normalized_email {
        rate_limit_hit(
            &state,
            format!("rl:login:identifier:{}", email),
            LOGIN_IDENTIFIER_RATE_LIMIT.0,
            LOGIN_IDENTIFIER_RATE_LIMIT.1,
        )
        .await?;
    } else if let Some(username) = &normalized_username {
        rate_limit_hit(
            &state,
            format!("rl:login:identifier:{}", username),
            LOGIN_IDENTIFIER_RATE_LIMIT.0,
            LOGIN_IDENTIFIER_RATE_LIMIT.1,
        )
        .await?;
    }

    // Verify client exists
    let client_app = state
        .service
        .client_service()
        .get_client_app_by_client_id(&realm_id, &payload.client_id)
        .await
        .map_err(|e| {
            tracing::error!(
                realm_id = %realm_id,
                client_id = %payload.client_id,
                error = %e,
                "Client app lookup failed"
            );
            match e {
                herald_core::domain::common::entities::app_errors::CoreError::NotFound => {
                    ApiError::bad_request(format!(
                        "Client app with client_id '{}' not found in realm '{}'",
                        payload.client_id, realm_id
                    ))
                }
                _ => ApiError::internal("Internal server error".to_string()),
            }
        })?;

    // Create login request for service
    let login_request = DomainLoginRequest {
        realm_id: realm_id.clone(),
        email: normalized_email.clone(),
        username: normalized_username.clone(),
        password: payload.password,
    };

    // Call UserService for authentication
    let user = match state.service.user_service().login(login_request).await {
        Ok(u) => u,
        Err(e) => {
            tracing::debug!(
                realm_id = %realm_id,
                email = ?normalized_email,
                username = ?normalized_username,
                error = %e,
                "Login failed"
            );

            // Record login failure audit event
            let actor_id = normalized_email
                .as_deref()
                .or(normalized_username.as_deref())
                .unwrap_or("unknown")
                .to_string();
            if let Err(audit_err) = state
                .audit_event_repository
                .create(NewAuditEvent {
                    realm_id: realm_id.clone(),
                    category: AuditCategory::Auth,
                    action: AuditAction::AuthLoginFailed,
                    actor_id: actor_id.clone(),
                    actor_type: None,
                    actor_name: None,
                    target_type: AuditTargetType::User,
                    target_id: actor_id,
                    target_name: None,
                    result: AuditResult::Failure,
                    details: Some(
                        serde_json::json!({"method": "password", "reason": "invalid_credentials"}),
                    ),
                    ip_address: Some(ip.clone()),
                    user_agent: user_agent.clone(),
                    trace_id: None,
                })
                .await
            {
                tracing::warn!(error = %audit_err, "Failed to record audit event");
            }

            return Err(match e {
                herald_core::domain::common::entities::app_errors::CoreError::NotFound => {
                    ApiError::unauthorized("invalid credentials".to_string())
                }
                herald_core::domain::common::entities::app_errors::CoreError::Unauthorized => {
                    ApiError::unauthorized("invalid credentials".to_string())
                }
                herald_core::domain::common::entities::app_errors::CoreError::Forbidden(_) => {
                    ApiError::forbidden("email not verified or account not active".to_string())
                }
                _ => ApiError::internal("Authentication failed".to_string()),
            });
        }
    };

    tracing::info!(
        user_id = %user.id,
        realm_id = %realm_id,
        client_id = %payload.client_id,
        ip = %ip,
        "Password authentication successful"
    );

    // Check if user has TOTP enabled
    let totp_repo = PostgresUserTotpRepository::new(state.db.clone());
    let totp_config = totp_repo.get_config_by_user_id(user.id).await?;

    // If TOTP is enabled, create temp session and require TOTP verification
    if let Some(config) = totp_config
        && config.enabled
    {
        tracing::debug!(
            user_id = %user.id,
            "User has TOTP enabled, requiring verification"
        );

        // Generate temp token
        let temp_token = format!("totp_login_{}", Uuid::now_v7());
        let temp_key = format!("totp:temp:{}", temp_token);

        let temp_session_data = serde_json::json!({
            "user_id": user.id,
            "realm_id": realm_id,
            "client_id": payload.client_id,
            "client_ip": ip,
        });

        let mut conn = state
            .redis_manager
            .get()
            .await
            .map_err(|_| ApiError::internal("Internal server error".to_string()))?;

        let _: () = conn
            .set_ex(&temp_key, temp_session_data.to_string(), 300) // 5 minutes
            .await
            .map_err(|_| ApiError::internal("Internal server error".to_string()))?;

        // Record login success audit event (user authenticated, TOTP required for second factor)
        if let Err(audit_err) = state
            .audit_event_repository
            .create(NewAuditEvent {
                realm_id: realm_id.clone(),
                category: AuditCategory::Auth,
                action: AuditAction::AuthLogin,
                actor_id: user.id.to_string(),
                actor_type: Some(ActorType::User),
                actor_name: Some(user.email.clone()),
                target_type: AuditTargetType::User,
                target_id: user.id.to_string(),
                target_name: None,
                result: AuditResult::Success,
                details: Some(serde_json::json!({"method": "password", "totp_required": true})),
                ip_address: Some(ip.clone()),
                user_agent: user_agent.clone(),
                trace_id: None,
            })
            .await
        {
            tracing::warn!(error = %audit_err, "Failed to record audit event");
        }

        // Return temp token instead of creating session
        return Ok(Json(LoginResponse {
            message: "ok".to_string(),
            user_id: user.id,
            realm_id: realm_id.clone(),
            requires_totp: Some(true),
            temp_token: Some(temp_token),
            expires_in_seconds: 300,
        })
        .into_response());
    }

    tracing::info!(
        user_id = %user.id,
        realm_id = %realm_id,
        client_id = %payload.client_id,
        ip = %ip,
        "Login successful (no TOTP)"
    );

    // cookie: X-Auth: ${realm-id_uuid_time}, 有效期
    // For OAuth flow: 600 seconds (10 minutes)
    // For normal login: 1800 seconds (30 minutes)
    let is_oauth_flow = payload.oauth_client_id.is_some()
        && payload.redirect_uri.is_some()
        && payload.state.is_some();

    let ttl = if is_oauth_flow {
        DEFAULT_OAUTH_SESSION_TTL_SECONDS
    } else {
        client_app.session_ttl_seconds as u64
    };

    tracing::debug!(
        "Creating session with user_id: {}, realm_id: {}, client_id: {}",
        user.id,
        realm_id,
        payload.client_id
    );

    // Use AuthenticationService to create session with proper client_id
    let auth_service = state.service.authentication_service();
    let (session, identity) = auth_service
        .create_session(user.clone(), payload.client_id.clone(), ip.clone(), ttl)
        .await
        .map_err(|e| match e {
            herald_core::domain::common::entities::app_errors::CoreError::NotFound => {
                tracing::error!("User not found in database after password auth");
                ApiError::internal("Internal server error".to_string())
            }
            _ => ApiError::internal(format!("Failed to create session: {}", e)),
        })?;

    // Use the token from the session created by AuthenticationService
    let token = session.token.clone();

    tracing::debug!("Session created with identity: {}", identity.user_id());

    // Record login success audit event
    if let Err(audit_err) = state
        .audit_event_repository
        .create(NewAuditEvent {
            realm_id: realm_id.clone(),
            category: AuditCategory::Auth,
            action: AuditAction::AuthLogin,
            actor_id: user.id.to_string(),
            actor_type: Some(ActorType::User),
            actor_name: Some(user.email.clone()),
            target_type: AuditTargetType::User,
            target_id: user.id.to_string(),
            target_name: None,
            result: AuditResult::Success,
            details: Some(serde_json::json!({"method": "password"})),
            ip_address: Some(ip.clone()),
            user_agent: user_agent.clone(),
            trace_id: None,
        })
        .await
    {
        tracing::warn!(error = %audit_err, "Failed to record audit event");
    }

    // OAuth flow: redirect with token in URL fragment
    if is_oauth_flow {
        let redirect_uri = payload.redirect_uri.ok_or_else(|| {
            ApiError::bad_request("redirect_uri is required for OAuth flow".to_string())
        })?;
        let state_param = payload
            .state
            .ok_or_else(|| ApiError::bad_request("state is required for OAuth flow".to_string()))?;

        // Build URL fragment with token and state
        // Using fragment (#token=xxx) instead of query parameter (?token=xxx)
        // to prevent token from appearing in server logs
        let fragment = format!("#token={}&state={}", token, state_param);
        let redirect_url = if redirect_uri.contains('#') {
            redirect_uri
        } else {
            format!("{}{}", redirect_uri, fragment)
        };

        tracing::debug!(
            redirect_url = %redirect_url,
            "OAuth login redirect"
        );

        // Return 302 redirect
        let response = (
            [(
                SET_COOKIE,
                build_set_cookie("X-Auth", &token, ttl as i64, state.app_env == "production"),
            )],
            Redirect::temporary(&redirect_url),
        )
            .into_response();
        return Ok(response);
    }

    // Normal login flow
    let ttl = ttl as i64;
    let is_production = state.app_env == "production";
    let set_cookie = build_set_cookie("X-Auth", &token, ttl, is_production);
    let response = (
        [(SET_COOKIE, set_cookie)],
        Json(LoginResponse {
            message: "ok".to_string(),
            user_id: user.id,
            realm_id: realm_id.clone(),
            requires_totp: Some(false),
            temp_token: None,
            expires_in_seconds: ttl,
        }),
    )
        .into_response();
    Ok(response)
}
