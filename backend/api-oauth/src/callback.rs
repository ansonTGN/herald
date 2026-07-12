// OAuth callback handler

use axum::{
    Form, Json,
    extract::{Path, Query, State},
    http::{HeaderMap, header},
    response::{IntoResponse, Redirect, Response},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::helper::handle_oauth_callback;
use herald_api_base::application::http::auth::util::{
    ClientIp, SessionData, build_set_cookie, renewal_ttl_seconds_from_i32, store_session,
};
use herald_api_base::application::http::server::api_entities::{ApiError, ErrorResponse};
use herald_api_base::application::http::state::AppState;
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, ToSchema, Validate)]
pub struct OAuthCallbackQuery {
    pub code: String,
    pub state: String,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OAuthCallbackResponse {
    pub message: String,
    pub user_id: String,
    pub access_token: String,
}

/// Handle OAuth callback from provider for a realm
#[utoipa::path(
    get,
    path = "/api/oauth/{realmId}/{provider}/callback",
    tag = "oauth",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("provider" = String, Path, description = "OAuth provider type"),
        ("code" = String, Query, description = "Authorization code from provider"),
        ("state" = String, Query, description = "State token for CSRF protection")
    ),
    responses(
        (status = 200, description = "OAuth login successful", body = OAuthCallbackResponse),
        (status = 302, description = "Redirect to application"),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[tracing::instrument(
    // Governance: query carries provider authorization
    // code + state (CSRF) — both secrets. state holds handles; headers may
    // carry cookies; client_ip is PII; realm_id/provider are low-cardinality
    // but conservatively skipped. Only http.route is recorded.
    skip(state, headers, client_ip, query),
    fields(http.route = "/api/oauth/{realmId}/{provider}/callback")
)]
pub async fn oauth_callback(
    State(state): State<AppState>,
    ClientIp(client_ip): ClientIp,
    headers: HeaderMap,
    Path((realm_id, provider)): Path<(String, String)>,
    Query(query): Query<OAuthCallbackQuery>,
) -> Result<Response, ApiError> {
    oauth_callback_inner(state, headers, client_ip, realm_id, provider, query).await
}

pub async fn oauth_callback_form(
    State(state): State<AppState>,
    ClientIp(client_ip): ClientIp,
    headers: HeaderMap,
    Path((realm_id, provider)): Path<(String, String)>,
    Form(query): Form<OAuthCallbackQuery>,
) -> Result<Response, ApiError> {
    oauth_callback_inner(state, headers, client_ip, realm_id, provider, query).await
}

async fn oauth_callback_inner(
    state: AppState,
    headers: HeaderMap,
    client_ip: String,
    realm_id: String,
    provider: String,
    query: OAuthCallbackQuery,
) -> Result<Response, ApiError> {
    // Validate provider
    let provider_type = provider.to_lowercase();
    if !matches!(
        provider_type.as_str(),
        "google" | "github" | "facebook" | "apple"
    ) {
        return Err(ApiError::bad_request(format!(
            "Unsupported OAuth provider: {}",
            provider
        )));
    }

    // Validate query parameters
    query
        .validate()
        .map_err(|e| ApiError::bad_request(format!("Validation error: {}", e)))?;

    // Handle OAuth callback
    let callback = handle_oauth_callback(
        &state,
        realm_id.clone(),
        provider_type,
        query.code,
        query.state,
    )
    .await?;

    if let Some(redirect_uri) = callback.downstream_redirect_uri {
        return Ok(Redirect::temporary(&redirect_uri).into_response());
    }

    let user_id = callback.user_id;
    let jwt_token = callback.jwt_token;
    let client_id = callback.client_id;

    let session_token = Uuid::now_v7().to_string();
    let session_config = load_client_session_config(&state, &realm_id, &client_id).await?;
    let session_data = SessionData {
        realm_id: realm_id.clone(),
        client_id,
        user_id: user_id.to_string(),
        client_ip,
        renewal_ttl_seconds: session_config.renewal_ttl_seconds,
    };

    store_session(
        &state,
        &session_token,
        &session_data,
        session_config.ttl_seconds,
    )
    .await?;

    let set_cookie = build_set_cookie(
        "X-Auth",
        &session_token,
        session_config.ttl_seconds as i64,
        state.app_env == "production",
    );

    let wants_json = headers
        .get(header::ACCEPT)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|accept| accept.contains("application/json"));

    if !wants_json {
        return Ok((
            [(header::SET_COOKIE, set_cookie)],
            Redirect::temporary(&format!("/{realm_id}")),
        )
            .into_response());
    }

    // Return JSON response (for SPA/mobile clients)
    // Note: This is a B-class exception (OAuth protocol callback), so we return Json<T> directly
    Ok((
        [(header::SET_COOKIE, set_cookie)],
        Json(OAuthCallbackResponse {
            message: "OAuth login successful".to_string(),
            user_id: user_id.to_string(),
            access_token: jwt_token,
        }),
    )
        .into_response())

    // Alternatively, redirect to frontend with token:
    // Ok(Redirect::to(&format!("{}/oauth/callback?token=...", redirect_uri)))
}

struct ClientSessionConfig {
    ttl_seconds: usize,
    renewal_ttl_seconds: Option<u64>,
}

async fn load_client_session_config(
    state: &AppState,
    realm_id: &str,
    client_id: &str,
) -> Result<ClientSessionConfig, ApiError> {
    let config = sqlx::query_as::<_, (i32, Option<i32>)>(
        "SELECT session_ttl_seconds, session_renewal_ttl_seconds FROM client_app WHERE realm_id = $1 AND client_id = $2 AND enabled = true",
    )
    .bind(realm_id)
    .bind(client_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!(
            realm_id = %realm_id,
            client_id = %client_id,
            error = %e,
            "Failed to load OAuth client session TTL"
        );
        ApiError::internal("Failed to load client session configuration".to_string())
    })?;

    let Some((ttl, renewal_ttl)) = config else {
        return Err(ApiError::bad_request(
            "OAuth client app is not enabled".to_string(),
        ));
    };

    Ok(ClientSessionConfig {
        ttl_seconds: usize::try_from(ttl)
            .map_err(|_| ApiError::internal("Client session TTL is invalid".to_string()))?,
        renewal_ttl_seconds: renewal_ttl_seconds_from_i32(renewal_ttl)?,
    })
}

// Governance tests.
//
// Covers: oauth `oauth_callback` (callback.rs), `oauth_token`
// (token.rs), and `handle_oauth_callback` (helper.rs) instrument skip
// correctness.
//
// WHY: the oauth callback/token paths carry the provider authorization `code`,
// the CSRF `state`/`state_token`, PKCE `code_verifier`, and `client_id` — all
// secrets. If the `#[instrument]` macro ever stops skipping those, the secret
// leaks into a span field. Source-scan baseline, anchored per
// function to the immediately-preceding `#[tracing::instrument(...)]`.
#[cfg(test)]
mod instrument_skip_tests {
    const CALLBACK_SRC: &str = include_str!("callback.rs");
    const TOKEN_SRC: &str = include_str!("token.rs");
    const HELPER_SRC: &str = include_str!("helper.rs");

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
    fn instrument_skip_oauth_callback_excludes_code_state() {
        let body = instrument_body_preceding(CALLBACK_SRC, "oauth_callback");
        // `query` carries the provider authorization `code` + CSRF `state`.
        for required in ["query", "headers", "client_ip", "state"] {
            assert!(
                body.contains(required),
                "oauth_callback must skip `{required}`; body was:\n{body}"
            );
        }
        for banned in ["code", "token", "secret", "email", "password"] {
            assert!(
                !body.contains(&format!("{banned} ="))
                    && !body.contains(&format!("fields({banned}")),
                "oauth_callback span must not record a `{banned}` field; body was:\n{body}"
            );
        }
    }

    #[test]
    fn instrument_skip_oauth_token_excludes_code_and_verifier() {
        let body = instrument_body_preceding(TOKEN_SRC, "oauth_token");
        // `req` carries authorization code, PKCE code_verifier, client_id.
        assert!(
            body.contains("req"),
            "oauth_token must skip `req` (carries auth code / code_verifier / client_id); body was:\n{body}"
        );
        for banned in ["code", "token", "verifier", "secret", "client_id"] {
            assert!(
                !body.contains(&format!("{banned} ="))
                    && !body.contains(&format!("fields({banned}")),
                "oauth_token span must not record a `{banned}` field; body was:\n{body}"
            );
        }
    }

    #[test]
    fn instrument_skip_oauth_helper_excludes_code_and_state_token() {
        let body = instrument_body_preceding(HELPER_SRC, "handle_oauth_callback");
        for required in ["code", "state_token", "realm_id", "state"] {
            assert!(
                body.contains(required),
                "handle_oauth_callback must skip `{required}`; body was:\n{body}"
            );
        }
        for banned in ["token", "secret", "email", "password"] {
            assert!(
                !body.contains(&format!("{banned} ="))
                    && !body.contains(&format!("fields({banned}")),
                "handle_oauth_callback span must not record a `{banned}` field; body was:\n{body}"
            );
        }
    }
}
