use axum::{
    Json,
    extract::State,
    http::{HeaderMap, header::SET_COOKIE},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use herald_api_base::application::http::auth::util::{
    ClientIp, build_clear_cookie, delete_session, get_cookie, load_session,
};
use herald_api_base::application::http::server::api_entities::ApiError;
pub use herald_api_base::application::http::server::api_entities::ErrorResponse;
use herald_api_base::application::http::state::AppState;
use herald_core::domain::audit::{
    AuditAction, AuditCategory, AuditEventRepository, AuditResult, AuditTargetType, NewAuditEvent,
};

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LogoutResponse {
    pub message: String,
}

/// Logout user and invalidate session
///
/// Clears the session cookie and invalidates the user's session token.
#[utoipa::path(
  get,
  path = "/api/auth/logout",
  tag = "auth",
  responses(
    (status = 200, description = "Logged out.", body = LogoutResponse),
    (status = 500, description = "Internal server error", body = ErrorResponse)
  )
)]
pub async fn logout(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let token = get_cookie(&headers, "X-Auth");
    let session = match token.as_deref() {
        Some(token) => load_session(&state, token).await?,
        None => None,
    };

    if let Some(ref token_val) = token {
        // best effort
        let _ = delete_session(&state, token_val).await;
    }

    // Record logout audit event
    let actor_id = token.unwrap_or_else(|| "anonymous".to_string());
    if let Err(audit_err) = state
        .audit_event_repository
        .create(NewAuditEvent {
            realm_id: session
                .as_ref()
                .map(|session| session.realm_id.clone())
                .unwrap_or_else(|| "unknown".to_string()),
            category: AuditCategory::Auth,
            action: AuditAction::AuthLogout,
            actor_id: actor_id.clone(),
            actor_type: None,
            actor_name: None,
            target_type: AuditTargetType::User,
            target_id: actor_id,
            target_name: None,
            result: AuditResult::Success,
            details: None,
            ip_address: Some(ip),
            user_agent,
            trace_id: None,
        })
        .await
    {
        tracing::warn!(error = %audit_err, "Failed to record audit event");
    }

    let is_production = state.app_env == "production";
    let clear_cookie = build_clear_cookie("X-Auth", is_production);
    Ok((
        [(SET_COOKIE, clear_cookie)],
        Json(LogoutResponse {
            message: "ok".to_string(),
        }),
    ))
}
