use axum::http::HeaderMap;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use tokio::time::{Duration, timeout};

use crate::application::http::server::api_entities::ApiError;
use crate::application::http::state::AppState;
use herald_core::domain::authorization::permission_service::PermissionService;

// Re-export rate limiting functions from the dedicated module
pub use crate::application::http::rate_limit::rate_limit_hit;

pub fn extract_ip(headers: &HeaderMap) -> String {
    headers
        .get("X-Forwarded-For")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .split(',')
        .next()
        .unwrap_or_default()
        .trim()
        .to_string()
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

pub fn get_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    let cookie = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
    for part in cookie.split(';') {
        let part = part.trim();
        let mut it = part.splitn(2, '=');
        let k = it.next()?.trim();
        let v = it.next()?.trim();
        if k == name {
            return Some(v.to_string());
        }
    }
    None
}

pub fn build_set_cookie(name: &str, value: &str, max_age_secs: i64, is_production: bool) -> String {
    // spec: http-only cookie, 有效期 30分钟
    // 在生产环境（HTTPS）添加 Secure 属性，开发环境（HTTP）不添加
    let secure_flag = if is_production { "; Secure" } else { "" };
    // 添加 Domain=localhost 以支持 Vite 代理场景（开发环境）
    // 在生产环境中，也应该设置正确的 domain
    let domain_flag = if is_production {
        // 生产环境应该从配置中读取 domain，这里先留空，让浏览器使用默认
        ""
    } else {
        // 开发环境：设置 Domain=localhost，支持跨端口访问
        "; Domain=localhost"
    };

    format!(
        "{name}={value}; Path=/; Max-Age={max_age_secs}; HttpOnly; SameSite=Lax{secure_flag}{domain_flag}"
    )
}

pub fn build_clear_cookie(name: &str, is_production: bool) -> String {
    let secure_flag = if is_production { "; Secure" } else { "" };
    let domain_flag = if is_production {
        ""
    } else {
        "; Domain=localhost"
    };

    format!("{name}=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax{secure_flag}{domain_flag}")
}

#[derive(Serialize, Deserialize)]
pub struct SessionData {
    pub realm_id: String,
    pub client_id: String, // External identifier (TEXT, e.g., 'admin-web-console')
    pub user_id: String,
    pub client_ip: String,
    #[serde(default)]
    pub renewal_ttl_seconds: Option<u64>,
}

pub fn renewal_ttl_seconds_from_i32(value: Option<i32>) -> Result<Option<u64>, ApiError> {
    value
        .map(|ttl| {
            u64::try_from(ttl)
                .map_err(|_| ApiError::internal("Session renewal TTL is invalid".to_string()))
        })
        .transpose()
}

pub async fn store_session(
    state: &AppState,
    token: &str,
    data: &SessionData,
    ttl_secs: usize,
) -> Result<(), ApiError> {
    let val = serde_json::to_string(data)
        .map_err(|_| ApiError::internal("Failed to serialize session data"))?;
    let key = format!("sess:{token}");

    // NEW: Use RedisConnectionManager with automatic DB selection
    // - Production: DB 0
    // - Test: DB 1 (automatic isolation, no UUID prefix needed)
    let mut conn = state
        .redis_manager
        .get()
        .await
        .map_err(|_| ApiError::internal("Internal server error"))?;

    let _: () = conn
        .set_ex(key, val, ttl_secs as u64)
        .await
        .map_err(|_| ApiError::internal("Internal server error"))?;

    Ok(())
}

pub async fn load_session(state: &AppState, token: &str) -> Result<Option<SessionData>, ApiError> {
    load_session_with_ip_validation(state, token, None).await
}

pub async fn load_session_with_ttl(
    state: &AppState,
    token: &str,
) -> Result<Option<(SessionData, u64)>, ApiError> {
    let key = format!("sess:{token}");

    let mut conn = state
        .redis_manager
        .get()
        .await
        .map_err(|_| ApiError::internal("Internal server error"))?;

    let (val, ttl): (Option<String>, i64) = redis::pipe()
        .cmd("GET")
        .arg(&key)
        .cmd("TTL")
        .arg(&key)
        .query_async(&mut conn)
        .await
        .map_err(|_| ApiError::internal("Internal server error"))?;

    let Some(val) = val else {
        return Ok(None);
    };

    if ttl == -2 {
        return Ok(None);
    }
    if ttl < 0 {
        tracing::error!(token = %token, ttl, "Session key has invalid TTL");
        return Err(ApiError::internal("Internal server error"));
    }

    let data: SessionData = serde_json::from_str(&val)
        .map_err(|_| ApiError::internal("Failed to deserialize session data"))?;

    Ok(Some((data, ttl as u64)))
}

pub async fn refresh_session_ttl(
    state: &AppState,
    token: &str,
    ttl_secs: u64,
) -> Result<(), ApiError> {
    let key = format!("sess:{token}");
    let ttl_secs = i64::try_from(ttl_secs)
        .map_err(|_| ApiError::internal("Session renewal TTL is invalid".to_string()))?;

    let mut conn = state
        .redis_manager
        .get()
        .await
        .map_err(|_| ApiError::internal("Internal server error"))?;

    let refreshed: bool = conn
        .expire(key, ttl_secs)
        .await
        .map_err(|_| ApiError::internal("Internal server error"))?;

    if !refreshed {
        return Err(ApiError::unauthorized("invalid session"));
    }

    Ok(())
}

/// Load session and optionally validate client IP
///
/// If headers are provided, validates that the client IP matches the session IP
/// to prevent token theft and reuse across different devices.
pub async fn load_session_with_ip_validation(
    state: &AppState,
    token: &str,
    headers: Option<&HeaderMap>,
) -> Result<Option<SessionData>, ApiError> {
    let key = format!("sess:{token}");

    // NEW: Use RedisConnectionManager with automatic DB selection
    let mut conn = state
        .redis_manager
        .get()
        .await
        .map_err(|_| ApiError::internal("Internal server error"))?;

    let val: Option<String> = conn
        .get(key)
        .await
        .map_err(|_| ApiError::internal("Internal server error"))?;

    let Some(val) = val else {
        return Ok(None);
    };
    let data: SessionData = serde_json::from_str(&val)
        .map_err(|_| ApiError::internal("Failed to deserialize session data"))?;

    // Validate client IP if headers are provided
    if let Some(hdrs) = headers {
        let client_ip = extract_ip(hdrs);
        if !client_ip.is_empty() && client_ip != data.client_ip {
            tracing::warn!(
                token = %token,
                session_ip = %data.client_ip,
                client_ip = %client_ip,
                "Session IP mismatch - rejecting access"
            );
            return Ok(None);
        }
    }

    Ok(Some(data))
}

pub async fn delete_session(state: &AppState, token: &str) -> Result<(), ApiError> {
    let key = format!("sess:{token}");

    // NEW: Use RedisConnectionManager with automatic DB selection
    let mut conn = state
        .redis_manager
        .get()
        .await
        .map_err(|_| ApiError::internal("Internal server error"))?;

    let _: () = conn
        .del(key)
        .await
        .map_err(|_| ApiError::internal("Internal server error"))?;

    Ok(())
}

pub async fn require_session(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(String, SessionData), ApiError> {
    let token =
        get_cookie(headers, "X-Auth").ok_or_else(|| ApiError::unauthorized("missing session"))?;
    let sess = load_session(state, &token)
        .await?
        .ok_or_else(|| ApiError::unauthorized("invalid session"))?;
    Ok((token, sess))
}

#[derive(Deserialize)]
struct TurnstileSiteVerifyResp {
    success: bool,
}

/// Verify Turnstile token for a specific realm
///
/// If Turnstile is not configured or disabled for the realm, the token is optional and verification is skipped.
/// If Turnstile is enabled, a valid token is required.
pub async fn verify_turnstile_for_realm(
    state: &AppState,
    realm_id: &str,
    token: Option<&str>,
    ip: Option<&str>,
) -> Result<(), ApiError> {
    let turnstile_config = sqlx::query_as::<_, (String,)>(
        "SELECT config_value FROM realm_config
         WHERE realm_id = $1 AND config_type = 'turnstile' AND config_key = 'site_secret' AND enabled = true",
    )
    .bind(realm_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to query turnstile config: {e}");
        ApiError::internal("Failed to query turnstile config")
    })?;

    let Some((secret,)) = turnstile_config else {
        // Turnstile not configured or disabled for this realm
        // If no token provided, that's fine. If provided, we could still validate but skip for now.
        return Ok(());
    };

    // Turnstile is enabled, token is required
    let token = token
        .and_then(|t| if t.trim().is_empty() { None } else { Some(t) })
        .ok_or_else(|| ApiError::bad_request("turnstile token is required"))?;

    // Cloudflare Turnstile 官方测试 secret：开发/测试环境常用，避免测试依赖外网。
    // 只要配置仍是该测试 secret，就直接放行（生产请务必替换为真实 secret）。
    if secret.trim() == "1x0000000000000000000000000000000AA" {
        return Ok(());
    }

    let mut form = vec![("secret", secret.as_str()), ("response", token)];
    if let Some(ip) = ip
        && !ip.trim().is_empty()
    {
        form.push(("remoteip", ip));
    }

    let resp = reqwest::Client::new()
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
         WHERE realm_id = $1 AND config_type = 'registration' AND config_key = 'allowed' AND enabled = true",
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

    Ok(required)
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

#[cfg(test)]
mod tests {
    use super::SessionData;

    #[test]
    fn session_data_defaults_missing_renewal_ttl_to_none() {
        let data: SessionData = serde_json::from_str(
            r#"{"realm_id":"admin","client_id":"admin-web-console","user_id":"user-1","client_ip":"127.0.0.1"}"#,
        )
        .expect("legacy session without renewal_ttl_seconds should deserialize");

        assert_eq!(data.renewal_ttl_seconds, None);
    }
}
