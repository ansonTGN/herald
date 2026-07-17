use axum::{
    Json,
    extract::{Extension, State},
    http::HeaderMap,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use herald_api_base::application::http::auth::util::{ClientIp, user_agent_from_headers};
use herald_api_base::application::http::server::api_entities::ApiError;
use herald_api_base::application::http::state::AppState;
use herald_core::domain::audit::{
    ActorType, AuditAction, AuditCategory, AuditEventRepository, AuditResult, AuditTargetType,
    NewAuditEvent,
};
use herald_core::domain::authentication::{BrowserTokenService, Identity, TokenCredentialContext};
use herald_core::infrastructure::authentication::RedisBrowserTokenService;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LogoutResponse {
    pub message: String,
}

#[utoipa::path(
  post,
  path = "/api/auth/logout",
  tag = "auth",
  responses((status = 200, body = LogoutResponse), (status = 401)),
  security(("bearer_auth" = []))
)]
pub async fn logout(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Extension(context): Extension<TokenCredentialContext>,
    ClientIp(ip): ClientIp,
    headers: HeaderMap,
) -> Result<Json<LogoutResponse>, ApiError> {
    RedisBrowserTokenService::new(state.redis_manager.clone())
        .revoke_family(context.family_id)
        .await?;

    let user_agent = user_agent_from_headers(&headers);
    let user = identity
        .as_user()
        .ok_or_else(|| ApiError::forbidden("authenticated user token required"))?;
    if let Err(error) = state
        .audit_event_repository
        .create(NewAuditEvent {
            realm_id: user.realm_id.clone(),
            category: AuditCategory::Auth,
            action: AuditAction::AuthLogout,
            actor_id: user.id.to_string(),
            actor_type: Some(ActorType::User),
            actor_name: Some(user.email.clone()),
            target_type: AuditTargetType::User,
            target_id: user.id.to_string(),
            target_name: Some(user.email.clone()),
            result: AuditResult::Success,
            details: None,
            ip_address: Some(ip),
            user_agent,
            trace_id: None,
        })
        .await
    {
        tracing::warn!(%error, "Failed to record logout audit event");
    }

    Ok(Json(LogoutResponse {
        message: "ok".into(),
    }))
}
