// Flexible Authentication Middleware for Points API
//
// This middleware supports both session-based authentication (cookies)
// and API key authentication (X-API-Key header) for points endpoints.
//
// Priority:
// 1. First tries API key authentication (X-API-Key header)
// 2. Falls back to session-based authentication (X-Auth cookie)
// 3. Returns 401 if neither authentication method succeeds

use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use herald_api_base::application::http::common::api_key_utils::{
    API_KEY_CACHE_TTL_SECONDS, ApiKeyValidationStatus, cached_to_entity, check_cached_key_status,
    check_entity_status,
};
use herald_api_base::application::http::common::error_codes::ErrorCode;
use herald_api_base::application::http::server::api_entities::ApiError;
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::client_api_keys::services::ClientApiKeyService;
use herald_core::domain::user::UserRepository;
use herald_core::infrastructure::client_api_keys::cache::ApiKeyCacheValue;
use tracing::{debug, error, info, warn};

// Note: API key validation functions (is_cached_key_valid, is_entity_valid, cached_to_entity)
// are now imported from common::api_key_utils to eliminate duplication

/// Try API key authentication
#[tracing::instrument(
    // Governance: `headers` carry the raw X-API-Key credential.
    skip(state, headers)
)]
async fn try_api_key_auth(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<Option<Identity>, ApiError> {
    // Extract X-API-Key header
    let api_key_header = match headers.get("X-API-Key").and_then(|v| v.to_str().ok()) {
        Some(key) => key,
        None => return Ok(None), // No API key, try session auth
    };

    let api_key = api_key_header.trim();
    if api_key.is_empty() {
        return Ok(None); // Empty API key, try session auth
    }

    debug!("API Key authentication attempt (length: {})", api_key.len());

    // Check Redis cache using plaintext API key as cache key
    let cached: Option<ApiKeyCacheValue> = match state.api_key_cache.get(api_key).await {
        Ok(Some(cached)) => {
            debug!(enabled = cached.enabled, "Cache hit for API key");
            Some(cached)
        }
        Ok(None) => {
            debug!("Cache miss for API key");
            None
        }
        Err(e) => {
            error!("Redis cache error: {}", e);
            return Err(ApiError::with_code(
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorCode::InternalError.as_u32(),
                "Redis cache error",
            ));
        }
    };

    if let Some(cached) = cached {
        debug!("Cache hit for API key, validating...");

        // Check if cached key is valid
        let status = check_cached_key_status(&cached);
        if status != ApiKeyValidationStatus::Valid {
            warn!("Cached API key is invalid: {:?}", status);
            return Err(ApiError::with_code(
                StatusCode::UNAUTHORIZED,
                status.to_error_code().as_u32(),
                status.to_error_code().to_string(),
            ));
        }

        // Convert cached value to domain entity and inject identity
        let api_key_entity = match cached_to_entity(cached) {
            Ok(entity) => entity,
            Err(e) => {
                error!("Failed to convert cached value: {}", e);
                return Err(ApiError::with_code(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ErrorCode::InternalError.as_u32(),
                    "Failed to convert cached value",
                ));
            }
        };

        let api_key_id = api_key_entity.id.clone();
        let realm_id = api_key_entity.realm_id.clone();

        debug!("API key authenticated via cache (realm: {})", realm_id);

        // Update usage stats asynchronously (even on cache hit)
        let repo = state.api_key_repo.clone();
        tokio::spawn(async move {
            if let Err(e) = repo.update_usage_stats(&api_key_id, Utc::now()).await {
                error!("Failed to update API key usage stats: {}", e);
            }
        });

        return Ok(Some(Identity::ThirdParty(api_key_entity)));
    }

    debug!("Cache miss, computing hash and querying database");

    // Compute SHA-256 hash of the API key for O(1) lookup
    let api_key_hash = ClientApiKeyService::hash_api_key(api_key);

    // Query API key by hash (O(1) lookup)
    let api_key_record = match state.api_key_repo.find_by_hash(&api_key_hash).await {
        Ok(Some(record)) => record,
        Ok(None) => {
            warn!("No valid API key found for hash: {}", api_key_hash);
            return Err(ApiError::with_code(
                StatusCode::UNAUTHORIZED,
                ErrorCode::InvalidApiKey.as_u32(),
                ErrorCode::InvalidApiKey.to_string(),
            ));
        }
        Err(e) => {
            error!("Database error finding API key: {}", e);
            return Err(ApiError::with_code(
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorCode::InternalError.as_u32(),
                "Database error finding API key",
            ));
        }
    };

    // Validate enabled and expiration using shared utility
    let status = check_entity_status(&api_key_record);
    if status != ApiKeyValidationStatus::Valid {
        warn!(
            "API key is invalid (disabled or expired): {}, status: {:?}",
            api_key_record.id, status
        );
        return Err(ApiError::with_code(
            StatusCode::UNAUTHORIZED,
            status.to_error_code().as_u32(),
            status.to_error_code().to_string(),
        ));
    }

    // Update usage stats asynchronously (don't block request)
    let api_key_id = api_key_record.id.clone();
    let repo = state.api_key_repo.clone();
    tokio::spawn(async move {
        if let Err(e) = repo.update_usage_stats(&api_key_id, Utc::now()).await {
            error!("Failed to update API key usage stats: {}", e);
        }
    });

    // Write to cache for next request (use plaintext API key as cache key)
    let cache_value = (&api_key_record).into();
    if let Err(e) = state
        .api_key_cache
        .set(api_key, &cache_value, API_KEY_CACHE_TTL_SECONDS)
        .await
    {
        warn!("Failed to cache API key: {}", e);
    }

    let realm_id = api_key_record.realm_id.clone();
    info!("API key authenticated via database (realm: {})", realm_id);

    Ok(Some(Identity::ThirdParty(api_key_record)))
}

/// Flexible authentication middleware that supports both API key and session auth
///
/// Authentication flow:
/// 1. Check for X-API-Key header → authenticate as ThirdParty
/// 2. If no API key, check for X-Auth cookie → authenticate as User
/// 3. Return 401 if neither authentication method succeeds
///
/// This allows:
/// - Third-party clients to use API keys (consume points, webhooks)
/// - Regular users to view their balance via session cookies
/// - Admin users to manage accounts via session cookies
#[tracing::instrument(
    // Governance: req headers carry X-API-Key / X-Auth
    // (token/session) — both credentials MUST be skipped.
    skip(state, req, next),
    fields(http.route = "points_flexible_auth")
)]
pub async fn flexible_auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Response {
    // Try API key authentication first
    let api_key_auth_result = try_api_key_auth(&state, req.headers()).await;

    match api_key_auth_result {
        Ok(Some(identity)) => {
            // API key authentication succeeded
            req.extensions_mut().insert(identity);
            return next.run(req).await;
        }
        Ok(None) => {
            // No API key provided, try session authentication
        }
        Err(e) => {
            // API key authentication failed with an error
            debug!("API key auth failed: {}", e);
            // Fall through to session auth
        }
    }

    // Try session-based authentication
    match herald_api_base::application::http::auth::util::require_session(&state, req.headers())
        .await
    {
        Ok((_token, session_data)) => {
            // Parse user_id from session with validation
            let session_user_id = &session_data.user_id;
            debug!(
                "Session user_id: {}, length: {}",
                session_user_id,
                session_user_id.len()
            );

            let user_id = match uuid::Uuid::parse_str(session_user_id) {
                Ok(id) => id,
                Err(_) => {
                    return ApiError::with_code(
                        StatusCode::UNAUTHORIZED,
                        ErrorCode::Unauthorized.as_u32(),
                        "Invalid user ID in session",
                    )
                    .into_response();
                }
            };

            debug!("Parsed user_id from session: {}", user_id);

            // Load User from database via repository
            let user = match state.user_repository.get_user_by_id(user_id).await {
                Ok(user) => user,
                Err(e) => match e {
                    herald_core::domain::common::entities::app_errors::CoreError::NotFound => {
                        error!(
                            session_user_id = %session_user_id,
                            parsed_user_id = %user_id,
                            "User not found in database"
                        );
                        return ApiError::with_code(
                            StatusCode::UNAUTHORIZED,
                            ErrorCode::Unauthorized.as_u32(),
                            "User not found",
                        )
                        .into_response();
                    }
                    _ => {
                        return ApiError::with_code(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            ErrorCode::InternalError.as_u32(),
                            "Internal server error",
                        )
                        .into_response();
                    }
                },
            };

            // Verify the loaded user ID matches the session user ID
            let loaded_user_id = user.id.to_string();
            if loaded_user_id != *session_user_id {
                error!(
                    session_user_id = %session_user_id,
                    loaded_user_id = %loaded_user_id,
                    "User ID mismatch: session user_id doesn't match loaded user ID"
                );
                return ApiError::with_code(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ErrorCode::InternalError.as_u32(),
                    "Internal server error",
                )
                .into_response();
            }

            let identity = Identity::User(user);

            debug!(
                realm_id = %identity.realm_id(),
                user_id = %identity.user_id(),
                client_id = %session_data.client_id,
                "Identity injected into request"
            );

            // Insert extensions into the request for next handler to extract
            req.extensions_mut().insert(identity);

            next.run(req).await
        }
        Err(_) => {
            // Both authentication methods failed
            ApiError::with_code(
                StatusCode::UNAUTHORIZED,
                ErrorCode::Unauthorized.as_u32(),
                "Authentication required (API key or session cookie)",
            )
            .into_response()
        }
    }
}

// Governance tests.
//
// Covers: points `flexible_auth_middleware` + `try_api_key_auth`
// (auth_middleware.rs), `grant_points` (grant.rs), `list_transactions`
// (transactions.rs) instrument skip correctness.
//
// WHY: the auth middleware reads X-API-Key / X-Auth (token/session) from
// request headers — credentials. The grant/transactions handlers carry
// `identity` (user_id/realm_id) and the request body/query (target user_id).
// If the `#[instrument]` macro ever stops skipping those, the credential/PII
// leaks into a span field. Source-scan baseline, anchored per
// function to the immediately-preceding `#[tracing::instrument(...)]`.
#[cfg(test)]
mod instrument_skip_tests {
    const AUTH_SRC: &str = include_str!("auth_middleware.rs");
    const GRANT_SRC: &str = include_str!("grant.rs");
    const TX_SRC: &str = include_str!("transactions.rs");

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
    fn instrument_skip_points_flexible_auth_excludes_api_key_and_token() {
        let body = instrument_body_preceding(AUTH_SRC, "flexible_auth_middleware");
        // `req` headers carry X-API-Key / X-Auth (token/session).
        for required in ["req", "state", "next"] {
            assert!(
                body.contains(required),
                "flexible_auth_middleware must skip `{required}`; body was:\n{body}"
            );
        }
        for banned in ["token", "api_key", "apikey", "secret", "password", "auth"] {
            assert!(
                !body.contains(&format!("{banned} ="))
                    && !body.contains(&format!("fields({banned}")),
                "flexible_auth_middleware span must not record a `{banned}` field; body was:\n{body}"
            );
        }
    }

    #[test]
    fn instrument_skip_points_try_api_key_auth_excludes_headers() {
        let body = instrument_body_preceding(AUTH_SRC, "try_api_key_auth");
        // `headers` carry the raw X-API-Key credential.
        assert!(
            body.contains("headers"),
            "try_api_key_auth must skip `headers` (raw X-API-Key); body was:\n{body}"
        );
        for banned in ["token", "api_key", "apikey", "secret"] {
            assert!(
                !body.contains(&format!("{banned} ="))
                    && !body.contains(&format!("fields({banned}")),
                "try_api_key_auth span must not record a `{banned}` field; body was:\n{body}"
            );
        }
    }

    #[test]
    fn instrument_skip_points_grant_excludes_identity_and_body() {
        let body = instrument_body_preceding(GRANT_SRC, "grant_points");
        // Uses `skip_all` — assert that's still the case (covers identity,
        // realm_id, request body which carries target user_id).
        assert!(
            body.contains("skip_all"),
            "grant_points must use skip_all (identity carries user_id/realm_id; body carries target user_id); body was:\n{body}"
        );
        for banned in ["token", "password", "email", "secret", "user_id"] {
            assert!(
                !body.contains(&format!("{banned} ="))
                    && !body.contains(&format!("fields({banned}")),
                "grant_points span must not record a `{banned}` field; body was:\n{body}"
            );
        }
    }

    #[test]
    fn instrument_skip_points_list_transactions_excludes_identity_and_filters() {
        let body = instrument_body_preceding(TX_SRC, "list_transactions");
        assert!(
            body.contains("skip_all"),
            "list_transactions must use skip_all (identity + query filters carry user_id/bucket_id); body was:\n{body}"
        );
        for banned in ["token", "password", "email", "secret", "user_id"] {
            assert!(
                !body.contains(&format!("{banned} ="))
                    && !body.contains(&format!("fields({banned}")),
                "list_transactions span must not record a `{banned}` field; body was:\n{body}"
            );
        }
    }
}
