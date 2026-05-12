// OAuth callback handler

use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::helper::handle_oauth_callback;
use herald_api_base::application::http::server::api_entities::{ApiError, ErrorResponse};
use herald_api_base::application::http::state::AppState;

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
    Path((realm_id, provider)): Path<(String, String)>,
    Query(query): Query<OAuthCallbackQuery>,
) -> Result<Json<OAuthCallbackResponse>, ApiError> {
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
    let (user_id, jwt_token) =
        handle_oauth_callback(&state, realm_id, provider_type, query.code, query.state).await?;

    // Return JSON response (for SPA/mobile clients)
    // Note: This is a B-class exception (OAuth protocol callback), so we return Json<T> directly
    Ok(Json(OAuthCallbackResponse {
        message: "OAuth login successful".to_string(),
        user_id: user_id.to_string(),
        access_token: jwt_token,
    }))

    // Alternatively, redirect to frontend with token:
    // Ok(Redirect::to(&format!("{}/oauth/callback?token=...", redirect_uri)))
}
