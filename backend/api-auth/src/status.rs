use axum::extract::{Extension, State};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use herald_api_base::application::http::server::api_entities::{ApiError, ApiResult};
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::{
    CredentialClass, CredentialScope, Identity, TokenCredentialContext,
};
use herald_core::domain::authorization::permission_service::PermissionService;

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StatusResponse {
    pub authenticated: bool,
    pub realm_id: Option<String>,
    pub user_id: Option<String>,
    /// Retained in the response shape for clients that display RBAC grants;
    /// browser-token authorization itself is governed by `scopes`.
    pub permissions: Option<Vec<String>>,
    pub client_id: String,
    pub credential_class: CredentialClass,
    pub scopes: Vec<CredentialScope>,
}

#[utoipa::path(
  get,
  path = "/api/auth/status",
  tag = "auth",
  responses((status = 200, body = StatusResponse), (status = 401)),
  security(("bearer_auth" = []))
)]
pub async fn status(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Extension(context): Extension<TokenCredentialContext>,
) -> Result<ApiResult<StatusResponse>, ApiError> {
    let user = identity
        .as_user()
        .ok_or_else(|| ApiError::forbidden("authenticated user token required"))?;
    let mut scopes: Vec<_> = context.allowed_scopes.into_iter().collect();
    scopes.sort_by_key(|scope| format!("{scope:?}"));
    let permissions = state
        .permission_checker
        .get_user_permissions(&user.realm_id, &user.id.to_string())
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                user_id = %user.id,
                realm_id = %user.realm_id,
                "Failed to fetch user permissions for status"
            );
            ApiError::internal("Failed to fetch permissions")
        })?;
    Ok(ApiResult::ok(StatusResponse {
        authenticated: true,
        realm_id: Some(user.realm_id.clone()),
        user_id: Some(user.id.to_string()),
        permissions: Some(permissions),
        client_id: context.client_id,
        credential_class: context.credential_class,
        scopes,
    }))
}
