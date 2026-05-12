//! OAuth authorization endpoint for third-party application integration
//!
//! This is a simplified OAuth flow that:
//! 1. Validates client_id and redirect_uri
//! 2. Stores state token in Redis (CSRF protection)
//! 3. Redirects to login page with OAuth parameters
//! 4. After login, redirects back with token in URL fragment

use axum::{
    extract::{Path, Query, State},
    response::Redirect,
};
use redis::AsyncCommands;
use serde::Deserialize;
use utoipa::ToSchema;

use herald_api_base::application::http::server::api_entities::{ApiError, ErrorResponse};
use herald_api_base::application::http::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizeQueryParams {
    pub client_id: String,
    pub redirect_uri: String,
    pub state: String,
    #[serde(default = "default_response_type")]
    pub response_type: String,
}

fn default_response_type() -> String {
    "token".to_string()
}

/// OAuth authorize endpoint
///
/// Initiates the simplified OAuth flow:
/// 1. Validates client_id exists and is enabled
/// 2. Validates redirect_uri is in whitelist
/// 3. Stores state token in Redis (5 minutes TTL)
/// 4. Redirects to login page with OAuth parameters
///
/// # Arguments
/// * `realm_id` - Realm identifier
/// * `params` - OAuth query parameters (client_id, redirect_uri, state, response_type)
///
/// # Returns
/// * 302 redirect to login page with OAuth parameters
///
/// # Errors
/// * 400 - Invalid parameters (missing client_id, redirect_uri, or state)
/// * 400 - Invalid response_type (must be "token")
/// * 404 - Client not found or disabled
/// * 400 - Redirect URI not in whitelist
#[utoipa::path(
    get,
    path = "/api/oauth/{realmId}/authorize",
    tag = "oauth",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("client_id" = String, Query, description = "OAuth Client ID"),
        ("redirect_uri" = String, Query, description = "Redirect URI (must be in whitelist)"),
        ("state" = String, Query, description = "State token (CSRF protection)"),
        ("response_type" = String, Query, description = "Response type (must be 'token')")
    ),
    responses(
        (status = 302, description = "Redirect to login page"),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 404, description = "Client not found", body = ErrorResponse)
    )
)]
pub async fn oauth_authorize(
    Path(realm_id): Path<String>,
    Query(params): Query<AuthorizeQueryParams>,
    State(state): State<AppState>,
) -> Result<Redirect, ApiError> {
    // Validate response_type (must be "token")
    if params.response_type != "token" {
        return Err(ApiError::bad_request(format!(
            "Invalid response_type '{}'. Only 'token' is supported in simplified OAuth flow.",
            params.response_type
        )));
    }

    // Validate client_id and redirect_uri
    let client_row = sqlx::query_as::<_, (String, String, bool)>(
        "SELECT id::text, redirect_uris, enabled FROM client_app
         WHERE realm_id = $1 AND client_id = $2",
    )
    .bind(&realm_id)
    .bind(&params.client_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!(
            realm_id = %realm_id,
            client_id = %params.client_id,
            error = %e,
            "Database query failed: client_app lookup"
        );
        ApiError::internal("Database query failed".to_string())
    })?;

    let Some((_id, redirect_uris, enabled)) = client_row else {
        tracing::debug!(
            realm_id = %realm_id,
            client_id = %params.client_id,
            "OAuth authorize failed: client app not found"
        );
        return Err(ApiError::not_found(format!(
            "Client app with client_id '{}' not found in realm '{}'",
            params.client_id, realm_id
        )));
    };

    if !enabled {
        tracing::debug!(
            realm_id = %realm_id,
            client_id = %params.client_id,
            "OAuth authorize failed: client app is disabled"
        );
        return Err(ApiError::forbidden("Client app is disabled".to_string()));
    }

    // Validate redirect_uri is in whitelist
    let allowed_uris: Vec<String> = serde_json::from_str(&redirect_uris)
        .map_err(|_| ApiError::internal("Failed to parse redirect URIs".to_string()))?;

    // Enforce HTTPS for all environments (Demo uses HTTPS URIs in tests)
    herald_core::domain::client::validation::validate_redirect_uri(
        &params.redirect_uri,
        false, // is_development: always enforce HTTPS
    )
    .map_err(|e| ApiError::bad_request(format!("Invalid redirect_uri: {}", e)))?;

    let is_whitelisted = allowed_uris.iter().any(|uri| {
        // Exact match or path match
        if params.redirect_uri == *uri {
            return true;
        }
        if params.redirect_uri.starts_with(uri) {
            return true;
        }
        false
    });

    if !is_whitelisted {
        tracing::debug!(
            realm_id = %realm_id,
            client_id = %params.client_id,
            redirect_uri = %params.redirect_uri,
            "OAuth authorize failed: redirect_uri not in whitelist"
        );
        return Err(ApiError::bad_request(format!(
            "Redirect URI '{}' is not in the whitelist for client '{}'",
            params.redirect_uri, params.client_id
        )));
    }

    // Store state token in Redis (5 minutes TTL, CSRF protection)
    let state_key = format!("oauth:state:{}", params.state);
    let state_value = serde_json::json!({
        "client_id": params.client_id,
        "realm_id": realm_id,
        "redirect_uri": params.redirect_uri,
    })
    .to_string();

    let mut conn = state
        .redis_manager
        .get()
        .await
        .map_err(|_| ApiError::internal("Internal server error".to_string()))?;
    let _: () = conn
        .set_ex(state_key, state_value, 300) // 5 minutes
        .await
        .map_err(|_| ApiError::internal("Internal server error".to_string()))?;

    tracing::debug!(
        realm_id = %realm_id,
        client_id = %params.client_id,
        redirect_uri = %params.redirect_uri,
        "OAuth authorize successful: redirecting to login"
    );

    // Redirect to login page with OAuth parameters
    let login_url = format!(
        "/{}/login?client_id={}&redirect_uri={}&state={}",
        urlencoding::encode(&realm_id),
        urlencoding::encode(&params.client_id),
        urlencoding::encode(&params.redirect_uri),
        urlencoding::encode(&params.state)
    );

    Ok(Redirect::temporary(&login_url))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_response_type() {
        assert_eq!(default_response_type(), "token");
    }
}
