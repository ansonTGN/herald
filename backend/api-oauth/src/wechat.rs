// WeChat OAuth handlers (website application login)

use axum::{
    Json,
    extract::{Path, Query, State},
    response::{IntoResponse, Redirect, Response},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::helper::{generate_oauth_auth_url, handle_oauth_callback};
use herald_api_base::application::http::server::api_entities::{ApiError, ErrorResponse};
use herald_api_base::application::http::state::AppState;
use validator::Validate;

#[derive(Debug, Deserialize, Serialize, ToSchema, Validate)]
pub struct WeChatAuthUrlRequest {
    #[serde(alias = "client_id")]
    pub client_id: Option<String>,
    #[serde(alias = "redirect_uri")]
    pub redirect_uri: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WeChatAuthUrlResponse {
    pub auth_url: String,
    pub state: String,
}

/// Generate WeChat authorization URL (QRconnect)
#[utoipa::path(
    get,
    path = "/api/oauth/{realmId}/wechat/login",
    tag = "oauth",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("redirect_uri" = Option<String>, Query, description = "Redirect URI after successful login")
    ),
    responses(
        (status = 200, description = "Authorization URL generated", body = WeChatAuthUrlResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 404, description = "WeChat provider not configured or not enabled", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn wechat_login(
    Path(realm_id): Path<String>,
    Query(query): Query<WeChatAuthUrlRequest>,
    State(state): State<AppState>,
) -> Result<Json<WeChatAuthUrlResponse>, ApiError> {
    tracing::info!(
        realm_id = %realm_id,
        "WeChat authorization URL requested"
    );

    // Generate OAuth authorization URL and state token using the helper function
    let realm_id_clone = realm_id.clone();
    let (auth_url, state_token) = generate_oauth_auth_url(
        &state,
        realm_id_clone,
        "wechat".to_string(),
        query
            .client_id
            .unwrap_or_else(|| "admin-web-console".to_string()),
        query.redirect_uri,
        None,
    )
    .await?;

    tracing::debug!(
        realm_id = %realm_id,
        "WeChat authorization URL generated successfully"
    );

    // Return JSON response directly (B-class exception: OAuth protocol)
    Ok(Json(WeChatAuthUrlResponse {
        auth_url,
        state: state_token,
    }))
}

/// Handle WeChat OAuth callback
#[utoipa::path(
    get,
    path = "/api/oauth/{realmId}/wechat/callback",
    tag = "oauth",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("code" = String, Query, description = "Authorization code"),
        ("state" = String, Query, description = "OAuth state for CSRF protection")
    ),
    responses(
        (status = 302, description = "Redirect to specified redirect_uri or default"),
        (status = 400, description = "Bad request (invalid state or code)", body = ErrorResponse),
        (status = 401, description = "Unauthorized (OAuth failed)", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn wechat_callback(
    Path(realm_id): Path<String>,
    Query(query): Query<std::collections::HashMap<String, String>>,
    State(state): State<AppState>,
) -> Result<Response, ApiError> {
    let code = query
        .get("code")
        .ok_or_else(|| ApiError::bad_request("Missing code parameter".to_string()))?
        .clone();

    let state_token = query
        .get("state")
        .ok_or_else(|| ApiError::bad_request("Missing state parameter".to_string()))?
        .clone();

    tracing::info!(
        realm_id = %realm_id,
        "WeChat callback received"
    );

    // Handle OAuth callback using the helper function
    let realm_id_clone = realm_id.clone();
    let callback = handle_oauth_callback(
        &state,
        realm_id_clone,
        "wechat".to_string(),
        code,
        state_token,
    )
    .await?;

    if let Some(redirect_uri) = callback.downstream_redirect_uri {
        return Ok(Redirect::temporary(&redirect_uri).into_response());
    }

    let user_id = callback.user_id;
    let jwt_token = callback.jwt_token;

    tracing::info!(
        realm_id = %realm_id,
        user_id = %user_id,
        "WeChat callback processed successfully"
    );

    // Get redirect_uri from query params or use default
    let redirect_uri = query
        .get("redirect_uri")
        .cloned()
        .unwrap_or_else(|| format!("{}/", state.public_base_url));

    // Redirect with JWT token in URL fragment to avoid leaking it via logs/history/referrers.
    let fragment_separator = if redirect_uri.contains('#') { '&' } else { '#' };
    let redirect_url = format!(
        "{}{}token={}",
        redirect_uri,
        fragment_separator,
        urlencoding::encode(&jwt_token)
    );
    Ok(Redirect::temporary(&redirect_url).into_response())
}
