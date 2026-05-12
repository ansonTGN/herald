// OAuth login handler

use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::helper::generate_oauth_auth_url;
use herald_api_base::application::http::server::api_entities::{ApiError, ErrorResponse};
use herald_api_base::application::http::state::AppState;

#[derive(Debug, Deserialize, Serialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct OAuthLoginRequest {
    pub redirect_uri: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OAuthLoginResponse {
    pub auth_url: String,
    pub state: String,
}

/// Initiate OAuth login flow for a realm
#[utoipa::path(
    get,
    path = "/api/oauth/{realmId}/{provider}/login",
    tag = "oauth",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("provider" = String, Path, description = "OAuth provider type (google, github, facebook, apple, wechat, wechat_miniprogram)"),
        ("redirect_uri" = Option<String>, Query, description = "Redirect URI after successful login")
    ),
    responses(
        (status = 200, description = "OAuth login initiated", body = OAuthLoginResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 404, description = "OAuth provider not configured for this realm", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn oauth_login(
    State(state): State<AppState>,
    Path((realm_id, provider)): Path<(String, String)>,
    Query(query): Query<OAuthLoginRequest>,
) -> Result<Json<OAuthLoginResponse>, ApiError> {
    // Validate provider
    let provider_type = provider.to_lowercase();
    if !matches!(
        provider_type.as_str(),
        "google" | "github" | "facebook" | "apple" | "wechat" | "wechat_miniprogram"
    ) {
        return Err(ApiError::bad_request(format!(
            "Unsupported OAuth provider: {}",
            provider
        )));
    }

    // Generate OAuth authorization URL and state token
    let (auth_url, state_token) =
        generate_oauth_auth_url(&state, realm_id, provider_type, query.redirect_uri).await?;

    // Return JSON response directly (B-class exception: OAuth protocol)
    Ok(Json(OAuthLoginResponse {
        auth_url,
        state: state_token,
    }))
}
