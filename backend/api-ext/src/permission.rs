// Permission Check API for Third-Party Integration
//
// Allows third-party apps to check user permissions using API Key authentication.

use axum::{
    Json,
    extract::{Extension, State},
};
use herald_core::domain::authentication::Identity;
use herald_core::domain::authorization::permission_service::PermissionService;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use herald_api_base::application::http::auth::util::load_session;
use herald_api_base::application::http::server::api_entities::{ApiError, ErrorResponse};
use herald_api_base::application::http::state::AppState;

/// Create a denied permission check response
fn denied_response(error: Option<String>) -> Json<PermissionCheckResponse> {
    Json(PermissionCheckResponse {
        allowed: false,
        user_id: None,
        error,
    })
}

/// Permission check request
#[derive(Debug, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct PermissionCheckRequest {
    /// User's session token
    #[serde(alias = "token")]
    pub session_token: String,

    /// Permission rules to check
    pub rules: Vec<PermissionRule>,
}

/// Single permission rule
#[derive(Debug, Deserialize, ToSchema, Clone)]
pub struct PermissionRule {
    /// Resource identifier (e.g., "article", "user")
    pub resource: String,

    /// Action identifier (e.g., "read", "write", "delete")
    pub action: String,
}

/// Permission check response
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PermissionCheckResponse {
    /// Whether all permissions are granted
    pub allowed: bool,

    /// User ID (only present if allowed=true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,

    /// Error message (only present if error occurred)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Check user permissions
///
/// This endpoint allows third-party applications to check if a user
/// has specific permissions using their session token.
///
/// # Authentication
/// Requires valid API Key via X-API-Key header
///
/// # Example
/// ```json
/// {
///   "session_token": "user-session-token",
///   "rules": [
///     {"resource": "article", "action": "read"},
///     {"resource": "article", "action": "write"}
///   ]
/// }
/// ```
#[utoipa::path(
    post,
    path = "/api/ext/permission/check",
    tag = "ext",
    request_body = PermissionCheckRequest,
    responses(
        (status = 200, description = "Permission check completed", body = PermissionCheckResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API Key", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("api_key" = []))
)]
pub async fn check_permission(
    State(state): State<AppState>,
    Extension(_identity): Extension<Identity>,
    Json(req): Json<PermissionCheckRequest>,
) -> Result<Json<PermissionCheckResponse>, ApiError> {
    // Validate request
    if let Err(errors) = req.validate() {
        return Ok(denied_response(Some(format!(
            "Validation error: {}",
            errors
        ))));
    }

    tracing::info!(
        session_token_length = req.session_token.len(),
        rules_count = req.rules.len(),
        "Permission check requested"
    );

    // 1. Validate session token and load session data
    let session_data = match load_session(&state, &req.session_token).await {
        Ok(Some(data)) => data,
        Ok(None) => {
            tracing::warn!("Session not found or expired");
            return Ok(denied_response(Some("session_not_found".to_string())));
        }
        Err(e) => {
            tracing::error!("Failed to load session: {:?}", e);
            return Ok(denied_response(Some("internal_error".to_string())));
        }
    };

    // 2. Parse user_id from session
    let user_id = match Uuid::parse_str(&session_data.user_id) {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("Invalid user_id in session: {}", e);
            return Ok(denied_response(Some("invalid_session".to_string())));
        }
    };

    // 3. Check all permission rules
    let permission_checker = &state.permission_checker;

    let mut all_allowed = true;
    for rule in &req.rules {
        match permission_checker
            .check_permission(
                &session_data.realm_id,
                &user_id.to_string(),
                &rule.resource,
                &rule.action,
            )
            .await
        {
            Ok(allowed) => {
                if !allowed {
                    all_allowed = false;
                    tracing::debug!(
                        realm_id = %session_data.realm_id,
                        user_id = %user_id,
                        resource = %rule.resource,
                        action = %rule.action,
                        "Permission denied"
                    );
                    break; // Early exit on first denial
                }
            }
            Err(e) => {
                tracing::error!("Failed to check permission: {:?}", e);
                return Ok(denied_response(Some("internal_error".to_string())));
            }
        }
    }

    tracing::info!(
        realm_id = %session_data.realm_id,
        user_id = %user_id,
        allowed = all_allowed,
        "Permission check completed"
    );

    Ok(Json(PermissionCheckResponse {
        allowed: all_allowed,
        user_id: all_allowed.then(|| user_id.to_string()),
        error: None,
    }))
}
