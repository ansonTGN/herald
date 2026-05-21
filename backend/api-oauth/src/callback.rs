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
    let (user_id, jwt_token, client_id) = handle_oauth_callback(
        &state,
        realm_id.clone(),
        provider_type,
        query.code,
        query.state,
    )
    .await?;

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
