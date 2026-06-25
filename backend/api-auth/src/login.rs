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

use herald_api_base::application::http::auth::util::{
    ClientIp, build_set_cookie, normalize_email, rate_limit_hit, renewal_ttl_seconds_from_i32,
    verify_turnstile_for_realm,
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
    OAUTH_STATE_TTL_SECONDS,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(required = false)]
    pub redirect_to: Option<String>,
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
#[tracing::instrument(
    // BE-D08 governance (§4.5/§5.4): payload carries password (credential),
    // turnstile_token, oauth state, email/username (PII); realm_id is
    // conservatively skipped. state holds session/redis handles; headers may
    // carry cookies. Only the low-cardinality operation type is recorded.
    skip(state, headers, payload, realm_id, ip),
    fields(db.system = "postgres", db.operation = "login")
)]
pub async fn login(
    Path(realm_id): Path<String>,
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    headers: HeaderMap,
    Valid(Json(payload)): Valid<Json<LoginRequestPayload>>,
) -> Result<impl IntoResponse, ApiError> {
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

        // Build temp session data, preserving OAuth context if present
        let mut temp_session_data = serde_json::json!({
            "user_id": user.id,
            "realm_id": realm_id,
            "client_id": payload.client_id,
            "client_ip": ip,
        });
        if let Some(ref oauth_client_id) = payload.oauth_client_id {
            temp_session_data["oauth_client_id"] = serde_json::json!(oauth_client_id);
        }
        if let Some(ref redirect_uri) = payload.redirect_uri {
            temp_session_data["redirect_uri"] = serde_json::json!(redirect_uri);
        }
        if let Some(ref oauth_state) = payload.state {
            temp_session_data["state"] = serde_json::json!(oauth_state);
        }

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
                target_name: Some(user.email.clone()),
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
            redirect_to: None,
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

    // Determine OAuth flow presence before any session creation
    let is_oauth_flow = payload.oauth_client_id.is_some()
        && payload.redirect_uri.is_some()
        && payload.state.is_some();

    // OAuth flow: validate Redis state, generate authorization code, return JSON redirect
    if is_oauth_flow {
        let oauth_client_id = payload.oauth_client_id.as_deref().ok_or_else(|| {
            ApiError::bad_request("oauth_client_id is required for OAuth flow".to_string())
        })?;
        let redirect_uri = payload.redirect_uri.as_deref().ok_or_else(|| {
            ApiError::bad_request("redirect_uri is required for OAuth flow".to_string())
        })?;
        let state_param = payload
            .state
            .as_deref()
            .ok_or_else(|| ApiError::bad_request("state is required for OAuth flow".to_string()))?;

        // Validate Redis state (atomic GET+DELETE for one-time use)
        let state_key = format!("oauth:state:{}", state_param);
        let mut conn = state
            .redis_manager
            .get()
            .await
            .map_err(|_| ApiError::internal("Internal server error".to_string()))?;

        let state_json: Option<String> = redis::cmd("GETDEL")
            .arg(&state_key)
            .query_async(&mut conn)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Redis GETDEL failed for OAuth state");
                ApiError::internal("Internal server error".to_string())
            })?;

        let state_json = state_json.ok_or_else(|| {
            ApiError::bad_request(
                "OAuth state not found or already used. Please restart the authorization flow."
                    .to_string(),
            )
        })?;

        let state_data: serde_json::Value = serde_json::from_str(&state_json).map_err(|e| {
            tracing::error!(error = %e, "Failed to parse OAuth state JSON");
            ApiError::internal("Internal server error".to_string())
        })?;

        // Validate that Redis state fields match the request payload
        let stored_client_id = state_data["client_id"].as_str().unwrap_or("");
        let stored_realm_id = state_data["realm_id"].as_str().unwrap_or("");
        let stored_redirect_uri = state_data["redirect_uri"].as_str().unwrap_or("");

        if stored_client_id != oauth_client_id {
            return Err(ApiError::bad_request(
                "OAuth state client_id mismatch".to_string(),
            ));
        }
        if stored_realm_id != realm_id {
            return Err(ApiError::bad_request(
                "OAuth state realm_id mismatch".to_string(),
            ));
        }
        if stored_redirect_uri != redirect_uri {
            return Err(ApiError::bad_request(
                "OAuth state redirect_uri mismatch".to_string(),
            ));
        }

        // Generate authorization code
        let auth_code = format!("ac_{}", Uuid::now_v7());
        let code_challenge = state_data["code_challenge"]
            .as_str()
            .unwrap_or("")
            .to_string();

        // Store authorization code in Redis
        let code_key = format!("oauth:code:{}", auth_code);
        let code_value = serde_json::json!({
            "code_challenge": code_challenge,
            "client_id": oauth_client_id,
            "redirect_uri": redirect_uri,
            "user_id": user.id.to_string(),
            "realm_id": realm_id,
        })
        .to_string();

        let _: () = conn
            .set_ex(&code_key, code_value, OAUTH_STATE_TTL_SECONDS)
            .await
            .map_err(|_| ApiError::internal("Internal server error".to_string()))?;

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
                target_name: Some(user.email.clone()),
                result: AuditResult::Success,
                details: Some(serde_json::json!({"method": "password", "oauth": true})),
                ip_address: Some(ip.clone()),
                user_agent: user_agent.clone(),
                trace_id: None,
            })
            .await
        {
            tracing::warn!(error = %audit_err, "Failed to record audit event");
        }

        tracing::debug!(
            auth_code = %auth_code,
            redirect_uri = %redirect_uri,
            "OAuth authorization code generated"
        );

        let redirect_to = format!("{}?code={}&state={}", redirect_uri, auth_code, state_param);

        return Ok(Json(LoginResponse {
            message: "ok".to_string(),
            user_id: user.id,
            realm_id: realm_id.clone(),
            requires_totp: Some(false),
            temp_token: None,
            expires_in_seconds: DEFAULT_OAUTH_SESSION_TTL_SECONDS as i64,
            redirect_to: Some(redirect_to),
        })
        .into_response());
    }

    // Normal login flow: create session
    let ttl = client_app.session_ttl_seconds as u64;
    let renewal_ttl_seconds = renewal_ttl_seconds_from_i32(client_app.session_renewal_ttl_seconds)?;

    tracing::debug!(
        "Creating session with user_id: {}, realm_id: {}, client_id: {}",
        user.id,
        realm_id,
        payload.client_id
    );

    // Use AuthenticationService to create session with proper client_id
    let auth_service = state.service.authentication_service();
    let (session, identity) = auth_service
        .create_session(
            user.clone(),
            payload.client_id.clone(),
            ip.clone(),
            ttl,
            renewal_ttl_seconds,
        )
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
            target_name: Some(user.email.clone()),
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
            redirect_to: None,
        }),
    )
        .into_response();
    Ok(response)
}

// BE-T03 governance tests (design §5.4 / §4.5).
//
// Covers: BE-D08 — `login` (login.rs) and `register` (register.rs) instrument
// skip correctness.
//
// WHY: the login/register handlers take `payload` (carries password,
// turnstile_token, email/username) and `headers` (may carry cookies). If the
// `#[instrument]` macro ever stops skipping those, the credential/PII leaks
// into a span field and is exported off-process. This source-scan is the
// design §6.1 baseline governance assertion — anchored per function to the
// immediately-preceding `#[tracing::instrument(...)]`, so it cannot match a
// `skip(...)` in a comment/string/other function.
#[cfg(test)]
mod instrument_skip_tests {
    const LOGIN_SRC: &str = include_str!("login.rs");
    const REGISTER_SRC: &str = include_str!("register.rs");

    fn instrument_body_preceding(src: &str, fn_name: &str) -> String {
        let needle = format!("fn {fn_name}");
        let fn_pos = src
            .find(&needle)
            .unwrap_or_else(|| panic!("fn {fn_name} not found in source"));
        let attr_start = src[..fn_pos]
            .rfind("#[tracing::instrument(")
            .unwrap_or_else(|| panic!("no #[tracing::instrument( preceding fn {fn_name}"));
        let body_start = attr_start + "#[tracing::instrument(".len();
        // Find the attribute close: the first line at/after body_start whose
        // trimmed content is exactly `)]`. This handles indented closes (e.g.
        // inside an `impl` block) and ignores inline `))]` sequences such as
        // `#[validate(length(...))]` that appear on struct fields.
        let tail = &src[body_start..];
        let mut consumed = 0usize;
        for line in tail.lines() {
            let prev = consumed;
            consumed += line.len() + 1; // +1 for the line separator
            if line.trim() == ")]" {
                return tail[..prev].to_string();
            }
        }
        panic!("unterminated #[tracing::instrument( for fn {fn_name}")
    }

    #[test]
    fn instrument_skip_auth_login_excludes_password_email_token() {
        let body = instrument_body_preceding(LOGIN_SRC, "login");
        for required in ["payload", "headers", "realm_id", "state", "ip"] {
            assert!(
                body.contains(required),
                "login must skip `{required}`; body was:\n{body}"
            );
        }
        for banned in ["password", "email", "token", "code", "secret"] {
            assert!(
                !body.contains(&format!("{banned} ="))
                    && !body.contains(&format!("fields({banned}")),
                "login span must not record a `{banned}` field; body was:\n{body}"
            );
        }
    }

    #[test]
    fn instrument_skip_auth_register_excludes_password_email() {
        let body = instrument_body_preceding(REGISTER_SRC, "register");
        for required in ["payload", "realm_id", "state", "ip"] {
            assert!(
                body.contains(required),
                "register must skip `{required}`; body was:\n{body}"
            );
        }
        for banned in ["password", "email", "token", "code", "secret"] {
            assert!(
                !body.contains(&format!("{banned} ="))
                    && !body.contains(&format!("fields({banned}")),
                "register span must not record a `{banned}` field; body was:\n{body}"
            );
        }
    }
}
