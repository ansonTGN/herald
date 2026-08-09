use axum::extract::FromRequestParts;
use serde::Deserialize;
use tokio::time::{Duration, timeout};

use crate::application::http::server::api_entities::ApiError;
use crate::application::http::state::AppState;
use herald_core::domain::authorization::permission_service::PermissionService;
use herald_core::domain::client::entities::ClientApp;

// Re-export rate limiting functions from the dedicated module
pub use crate::application::http::rate_limit::rate_limit_hit;

/// Extract the request `User-Agent` header as an owned string, if present.
///
/// Handlers that record `user_agent` in audit events or token-family metadata
/// should call this instead of re-inlining the header lookup.
pub fn user_agent_from_headers(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// Extract client IP from request with three-level fallback:
/// 1. X-Real-IP header (set by nginx/traefik)
/// 2. X-Forwarded-For header (first entry)
/// 3. ConnectInfo<SocketAddr> (direct connection)
///
/// Usage in handlers: `client_ip: ClientIp` then `client_ip.0` for the IP string.
#[derive(Debug)]
pub struct ClientIp(pub String);

impl<S: Send + Sync> FromRequestParts<S> for ClientIp {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let headers = &parts.headers;

        // 1. Prefer X-Real-IP header (set by nginx/traefik)
        if let Some(real_ip) = headers
            .get("X-Real-IP")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse::<std::net::IpAddr>().ok())
        {
            return Ok(ClientIp(real_ip.to_string()));
        }

        // 2. Fall back to first entry in X-Forwarded-For
        if let Some(xff) = headers.get("X-Forwarded-For").and_then(|v| v.to_str().ok())
            && let Some(first) = xff.split(',').next()
        {
            let trimmed = first.trim();
            if !trimmed.is_empty() {
                return Ok(ClientIp(trimmed.to_string()));
            }
        }

        // 3. Fall back to direct connection IP (ConnectInfo<SocketAddr>)
        if let Some(connect_info) = parts
            .extensions
            .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        {
            return Ok(ClientIp(connect_info.0.ip().to_string()));
        }

        Ok(ClientIp(String::new()))
    }
}

pub fn normalize_email(email: &str) -> String {
    email.trim().to_ascii_lowercase()
}

pub fn epoch_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(Deserialize)]
struct TurnstileSiteVerifyResp {
    success: bool,
}

/// Verify a Turnstile token against a Client App's Turnstile configuration.
///
/// Turnstile is fully delegated to the Client App (D-PROTECT-01). If the
/// client app has `turnstile_enabled == false` the token is optional and
/// verification is skipped (matching the legacy realm-level "not configured ->
/// skip" semantics). When enabled, a valid token is required.
///
/// This reads only `client_app` fields; it never reads `realm_config`.
pub async fn verify_turnstile_for_client_app(
    state: &AppState,
    client_app: &ClientApp,
    token: Option<&str>,
    ip: &str,
) -> Result<(), ApiError> {
    if !client_app.turnstile_enabled {
        return Ok(());
    }

    let secret = client_app
        .turnstile_secret_key
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| {
            tracing::error!(
                client_id = %client_app.client_id,
                "Turnstile is enabled but secret key is not configured"
            );
            ApiError::internal("Turnstile secret key is not configured")
        })?;

    // Turnstile is enabled, token is required.
    let token = token
        .filter(|t| !t.trim().is_empty())
        .ok_or_else(|| ApiError::bad_request("turnstile token is required"))?;

    // Cloudflare Turnstile 官方测试 secret：开发/测试环境常用，避免测试依赖外网。
    // 只要配置仍是该测试 secret，就直接放行（生产请务必替换为真实 secret）。
    if secret.trim() == "1x0000000000000000000000000000000AA" {
        return Ok(());
    }

    let mut form = vec![("secret", secret), ("response", token)];
    if !ip.trim().is_empty() {
        form.push(("remoteip", ip));
    }

    let resp = state
        .http_client
        .post("https://challenges.cloudflare.com/turnstile/v0/siteverify")
        .form(&form)
        .send()
        .await
        .map_err(|_| ApiError::internal("Internal server error"))?;

    let body: TurnstileSiteVerifyResp = resp
        .json()
        .await
        .map_err(|_| ApiError::internal("Internal server error"))?;

    if !body.success {
        return Err(ApiError::unauthorized("turnstile verification failed"));
    }

    Ok(())
}

/// Check if user registration is enabled for a realm
///
/// Returns true if registration is explicitly enabled, false otherwise.
/// By default, registration is disabled if no config is found.
pub async fn is_registration_enabled(state: &AppState, realm_id: &str) -> Result<bool, ApiError> {
    let row = sqlx::query_as::<_, (String,)>(
        "SELECT config_value FROM realm_config
         WHERE realm_id = $1 AND config_type = 'registration' AND config_key = 'enabled' AND enabled = true",
    )
    .bind(realm_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to query registration config: {e}");
        ApiError::internal("Failed to query registration config")
    })?;

    // Parse the config value as boolean
    let enabled = row
        .and_then(|(value,)| match value.to_lowercase().as_str() {
            "true" | "1" | "yes" => Some(true),
            "false" | "0" | "no" => Some(false),
            _ => None,
        })
        .unwrap_or(false); // Default to disabled if no config found

    Ok(enabled)
}

/// Check if email verification is required for user registration in a realm
///
/// Returns true if email verification is explicitly enabled, false otherwise.
/// By default, email verification is NOT required if no config is found.
pub async fn is_email_verification_required(
    state: &AppState,
    realm_id: &str,
) -> Result<bool, ApiError> {
    let row = sqlx::query_as::<_, (String,)>(
        "SELECT config_value FROM realm_config
         WHERE realm_id = $1
           AND config_type = 'registration'
           AND config_key = 'require_email_verification'
           AND enabled = true",
    )
    .bind(realm_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to query email verification config: {e}");
        ApiError::internal("Failed to query email verification config")
    })?;

    // Parse the config value as boolean
    let required = row
        .and_then(|(value,)| match value.to_lowercase().as_str() {
            "true" | "1" | "yes" => Some(true),
            "false" | "0" | "no" => Some(false),
            _ => None,
        })
        .unwrap_or(false); // Default to NOT required if no config found

    // If email verification is requested, verify email is actually configured
    if required {
        let email_ready = is_email_configured(state, realm_id).await?;
        if !email_ready {
            tracing::warn!(
                realm_id = %realm_id,
                "Email verification requested but email not configured, forcing false"
            );
            return Ok(false);
        }
    }

    Ok(required)
}

/// Check whether self-service realm signup is open on the platform.
///
/// Reads the admin realm's `platform_signup` / `enabled` config row. The
/// public signup entry is fail-closed: a missing or non-true row means the
/// endpoint must refuse to provision, so an unconfigured deployment never
/// accidentally opens self-service to the public internet.
pub async fn is_platform_signup_enabled(state: &AppState) -> Result<bool, ApiError> {
    let row = sqlx::query_as::<_, (String,)>(
        "SELECT config_value FROM realm_config
         WHERE realm_id = $1 AND config_type = 'platform_signup' AND config_key = 'enabled' AND enabled = true",
    )
    .bind(herald_core::domain::realm::ADMIN_REALM_ID)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to query platform signup config: {e}");
        ApiError::internal("Failed to query platform signup config")
    })?;

    let enabled = row
        .and_then(|(value,)| match value.to_lowercase().as_str() {
            "true" | "1" | "yes" => Some(true),
            "false" | "0" | "no" => Some(false),
            _ => None,
        })
        .unwrap_or(false); // fail-closed: missing row => disabled

    Ok(enabled)
}

/// Check if email is configured for a realm
///
/// Returns true if the realm has a complete email configuration, false otherwise.
pub async fn is_email_configured(state: &AppState, realm_id: &str) -> Result<bool, ApiError> {
    let status =
        herald_core::third::email::EmailService::is_email_configured(&state.pool, realm_id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to query email config: {e}");
                ApiError::internal("Failed to query email config")
            })?;

    Ok(status.configured)
}

/// Deserialized shape of the `email_otp` / `settings` config_value JSON
/// (design email-otp-login §5.1). Both fields default to false when absent
/// so a partial or legacy payload degrades to "disabled".
#[derive(serde::Deserialize, Default)]
#[serde(default)]
pub struct EmailOtpSettings {
    pub enabled: bool,
    pub auto_register: bool,
}

/// Load the `email_otp` settings JSON for a realm.
///
/// Reads `realm_config` where `config_type='email_otp'`,
/// `config_key='settings'`, and the row is `enabled=true`. Returns the
/// parsed [`EmailOtpSettings`] (or the disabled default) for the requested
/// realm; never returns an error for a missing or malformed payload — those
/// cases simply yield the all-false default. Callers that need both flags
/// should call this once rather than via the single-flag helpers below, to
/// avoid reading the same row twice.
pub async fn load_email_otp_settings(
    state: &AppState,
    realm_id: &str,
) -> Result<EmailOtpSettings, ApiError> {
    let row = sqlx::query_as::<_, (String,)>(
        "SELECT config_value FROM realm_config
         WHERE realm_id = $1 AND config_type = 'email_otp' AND config_key = 'settings' AND enabled = true",
    )
    .bind(realm_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to query email OTP config: {e}");
        ApiError::internal("Failed to query email OTP config")
    })?;

    let Some((raw,)) = row else {
        return Ok(EmailOtpSettings::default());
    };

    let settings: EmailOtpSettings = serde_json::from_str(&raw).unwrap_or_else(|e| {
        tracing::error!("Failed to parse email OTP settings JSON: {e}; using default");
        EmailOtpSettings::default()
    });

    Ok(settings)
}

/// Check if email OTP login is enabled for a realm (design §5.1).
///
/// Returns true if the `email_otp` / `settings` config row is active and its
/// JSON `enabled` field is `true`. Defaults to false when the config is
/// absent or malformed (opt-in per realm).
pub async fn is_email_otp_enabled(state: &AppState, realm_id: &str) -> Result<bool, ApiError> {
    let settings = load_email_otp_settings(state, realm_id).await?;
    Ok(settings.enabled)
}

/// Check if a user has a specific permission with timeout
///
/// This is a centralized helper for permission checking across the application.
/// It provides consistent timeout handling and error conversion.
///
/// # Arguments
/// * `state` - Application state containing the permission checker
/// * `realm_id` - Realm ID for the permission check
/// * `user_id` - User ID to check permissions for
/// # Arguments
/// * `state` - Application state containing the permission checker
/// * `realm_id` - Realm ID for the permission check
/// * `user_id` - User ID to check permissions for
/// * `resource` - Resource identifier (e.g., "users", "roles")
/// * `action` - Action identifier (e.g., "view", "manage")
///
/// # Returns
/// * `Ok(true)` if permission is granted
/// * `Ok(false)` if permission is denied
/// * `Err(ApiError)` if an error occurs
pub async fn check_permission_with_timeout(
    state: &AppState,
    realm_id: &str,
    user_id: &str,
    resource: &str,
    action: &str,
) -> Result<bool, ApiError> {
    let permission_checker = &state.permission_checker;

    let result = timeout(
        Duration::from_secs(5),
        permission_checker.check_permission(realm_id, user_id, resource, action),
    )
    .await
    .map_err(|e| ApiError::internal(format!("Permission check timeout: {}", e)))?;

    result.map_err(|e| ApiError::internal(format!("Permission check failed: {}", e)))
}

/// Require a specific permission, returning Forbidden error if not granted
///
/// This is a convenience wrapper around `check_permission_with_timeout` that
/// automatically returns a Forbidden error when permission is denied.
///
/// # Arguments
/// * `state` - Application state
/// * `realm_id` - Realm ID for the permission check
/// * `user_id` - User ID to check permissions for
/// * `resource` - Resource identifier
/// * `action` - Action identifier
/// * `permission_name` - Human-readable permission name for error messages
///   (e.g., "permissions.manage")
///
/// # Returns
/// * `Ok(())` if permission is granted
/// * `Err(ApiError::Forbidden)` if permission is denied
/// * `Err(ApiError)` if an error occurs
pub async fn require_permission(
    state: &AppState,
    realm_id: &str,
    user_id: &str,
    resource: &str,
    action: &str,
    permission_name: &str,
) -> Result<(), ApiError> {
    let has_permission =
        check_permission_with_timeout(state, realm_id, user_id, resource, action).await?;

    if !has_permission {
        return Err(ApiError::forbidden(format!(
            "Missing {permission_name} permission"
        )));
    }

    Ok(())
}
