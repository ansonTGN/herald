// OAuth token endpoint for authorization code exchange with PKCE validation
//
// Third-party SPAs exchange an authorization code (obtained via the authorize + login flow)
// for a session token. PKCE ensures the code cannot be intercepted and reused.

use axum::{
    Json,
    extract::{Path, State},
};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use utoipa::ToSchema;
use uuid::Uuid;

use herald_api_base::application::http::auth::util::{SessionData, store_session};
use herald_api_base::application::http::server::api_entities::{ApiError, ErrorResponse};
use herald_api_base::application::http::state::AppState;

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

/// OAuth 2.0 token request (RFC 6749)
///
/// Field names use snake_case per OAuth 2.0 specification rather than the
/// project-wide camelCase convention.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct TokenRequest {
    pub grant_type: String,
    pub code: String,
    pub redirect_uri: String,
    pub client_id: String,
    pub code_verifier: String,
}

/// OAuth 2.0 token response (RFC 6749)
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

// ---------------------------------------------------------------------------
// PKCE verification
// ---------------------------------------------------------------------------

/// Verify PKCE code_verifier against stored code_challenge.
///
/// Computes BASE64URL(SHA256(code_verifier)) and compares to the stored challenge.
fn verify_pkce(code_verifier: &str, code_challenge: &str) -> bool {
    let mut hasher = Sha256::new();
    hasher.update(code_verifier.as_bytes());
    let hash = hasher.finalize();
    let computed = URL_SAFE_NO_PAD.encode(hash);
    computed == code_challenge
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/oauth/{realmId}/token",
    tag = "oauth",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
    ),
    request_body = TokenRequest,
    responses(
        (status = 200, description = "Access token issued", body = TokenResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
    )
)]
pub async fn oauth_token(
    Path(realm_id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<TokenRequest>,
) -> Result<Json<TokenResponse>, ApiError> {
    if req.grant_type != "authorization_code" {
        return Err(ApiError::bad_request(
            "grant_type must be 'authorization_code'",
        ));
    }

    // Atomically get-and-delete authorization code (one-time use)
    let mut conn = state
        .redis_manager
        .get()
        .await
        .map_err(|_| ApiError::internal("Internal server error".to_string()))?;

    let key = format!("oauth:code:{}", req.code);
    let code_json: Option<String> = redis::cmd("GETDEL")
        .arg(&key)
        .query_async(&mut conn)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Redis GETDEL failed for OAuth authorization code");
            ApiError::internal("Internal server error".to_string())
        })?;

    let code_json = code_json.ok_or_else(|| {
        ApiError::bad_request("Invalid or expired authorization code".to_string())
    })?;

    let stored: serde_json::Value = serde_json::from_str(&code_json).map_err(|e| {
        tracing::error!(error = %e, "Failed to parse authorization code data");
        ApiError::internal("Internal server error".to_string())
    })?;

    let stored_client_id = stored["client_id"].as_str().unwrap_or("");
    let stored_redirect_uri = stored["redirect_uri"].as_str().unwrap_or("");
    let stored_realm_id = stored["realm_id"].as_str().unwrap_or("");
    let stored_user_id = stored["user_id"].as_str().unwrap_or("");
    let stored_code_challenge = stored["code_challenge"].as_str().unwrap_or("");

    if stored_client_id != req.client_id {
        return Err(ApiError::bad_request("client_id mismatch".to_string()));
    }

    if stored_redirect_uri != req.redirect_uri {
        return Err(ApiError::bad_request("redirect_uri mismatch".to_string()));
    }

    if stored_realm_id != realm_id {
        return Err(ApiError::bad_request("realm_id mismatch".to_string()));
    }

    if !verify_pkce(&req.code_verifier, stored_code_challenge) {
        return Err(ApiError::bad_request(
            "PKCE verification failed".to_string(),
        ));
    }

    let session_config = load_client_session_config(&state, &realm_id, &req.client_id).await?;

    let session_token = Uuid::now_v7().to_string();
    let session_data = SessionData {
        realm_id: realm_id.clone(),
        client_id: req.client_id,
        user_id: stored_user_id.to_string(),
        client_ip: String::new(), // No client IP available from third-party backend
        renewal_ttl_seconds: session_config.renewal_ttl_seconds,
    };

    store_session(
        &state,
        &session_token,
        &session_data,
        session_config.ttl_seconds,
    )
    .await?;

    let expires_in = session_config.ttl_seconds as i64;

    Ok(Json(TokenResponse {
        access_token: session_token,
        token_type: "Bearer".to_string(),
        expires_in,
    }))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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
        renewal_ttl_seconds: renewal_ttl
            .map(|v| {
                u64::try_from(v)
                    .map_err(|_| ApiError::internal("Session renewal TTL is invalid".to_string()))
            })
            .transpose()?,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_pkce_correct_challenge() {
        // Known test vector: code_verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"
        // SHA256 + BASE64URL(no pad) = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
        assert!(verify_pkce(verifier, challenge));
    }

    #[test]
    fn verify_pkce_wrong_verifier_fails() {
        let challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
        assert!(!verify_pkce("wrong_verifier", challenge));
    }

    #[test]
    fn verify_pkce_empty_inputs() {
        // Empty verifier with empty challenge: SHA256("") base64url'd
        let hash = {
            let mut h = Sha256::new();
            h.update(b"");
            URL_SAFE_NO_PAD.encode(h.finalize())
        };
        assert!(verify_pkce("", &hash));
    }
}
