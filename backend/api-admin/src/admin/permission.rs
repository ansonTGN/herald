use axum::{
    Extension, Json,
    extract::{Path, State},
    http::HeaderMap,
};
use axum_valid::Valid;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use herald_api_base::application::http::auth::util::load_session_with_ip_validation;
use herald_api_base::application::http::common::auth_utils::AdminIdentity;
use herald_api_base::application::http::server::api_entities::{ApiError, ApiResult};
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::authorization::permission_service::PermissionService;
use herald_core::domain::user::PermissionManagementService;
use validator::Validate;

pub use herald_api_base::application::http::server::api_entities::ErrorResponse;

/// Role assignment data for user_roles table
///
/// NOTE: Both `user_id` and `role` must be UUIDs.
/// - `user_id`: account.id (UUID)
/// - `role`: roles.id (UUID) - must use role ID, not role name
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Role {
    pub user_id: Uuid, // account.id (UUID) - serde validates UUID format automatically
    pub role: Uuid,    // roles.id (UUID) - 必须使用角色 ID，禁止使用角色名称
}
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate, Clone)]
pub struct Police {
    pub id: Uuid, // role_id (UUID)
    #[validate(length(min = 3, max = 32))]
    pub resource: String,
    #[validate(length(min = 3, max = 12))]
    pub action: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
#[serde(tag = "p_type")]
pub enum PermissionData {
    #[serde(rename = "g")]
    RoleWrap(Role),
    #[serde(rename = "p")]
    PoliceWrap(Police),
}
impl Validate for PermissionData {
    fn validate(&self) -> Result<(), ::validator::ValidationErrors> {
        match self {
            PermissionData::RoleWrap(t) => t.validate(),
            PermissionData::PoliceWrap(t) => t.validate(),
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct PermissionCreateRequest {
    #[validate(length(min = 1, max = 36))]
    pub client_id: String,
    pub permission: PermissionData,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct Rule {
    pub resource: String,
    pub action: String,
}

#[derive(Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct PermissionCheckRequest {
    #[validate(length(min = 1))]
    pub token: String,
    #[serde(default)]
    pub rules: Option<Vec<Rule>>,
    #[validate(length(min = 1, max = 36))]
    pub client_id: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PermissionCheckResponse {
    pub allowed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<Uuid>,
}

/// Create a permission or role assignment
///
/// Creates either a policy permission (resource/action) or a role assignment (user/role).
/// Requires appropriate permissions based on the operation type.
#[utoipa::path(
  post,
  path = "/api/permission/{realmId}/permissions",
  tag = "permission",
  operation_id = "create_user_permission",
  params(
    ("realmId" = String, Path, description = "Realm ID")
  ),
  request_body = PermissionCreateRequest,
  responses(
    (status = 201, description = "Permission created"),
    (status = 400, description = "Bad request", body = ErrorResponse),
    (status = 403, description = "Forbidden", body = ErrorResponse),
    (status = 500, description = "Internal server error", body = ErrorResponse)
  )
)]
pub async fn create_permission(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Valid(Json(payload)): Valid<Json<PermissionCreateRequest>>,
) -> Result<ApiResult<()>, ApiError> {
    let admin = AdminIdentity::require(identity, &realm_id, "permissions")?;
    admin
        .require_permission(&state, "policies", "manage")
        .await?;

    // 1. 获取 permission_management_service
    let permission_management_service = &state.permission_management_service;

    // 2. 验证策略合法性（Realm Admin 只能创建自己 realm 的策略）
    super::middleware::validate_policy_for_realm_admin(&payload.permission, &realm_id)?;

    // 3. 提取参数
    let client_id = payload.client_id.to_string();
    let (role_id, user_id, role, resource, action) = match &payload.permission {
        PermissionData::PoliceWrap(Police {
            id,
            resource: res,
            action: act,
        }) => (Some(*id), None, None, Some(res.clone()), Some(act.clone())),
        PermissionData::RoleWrap(Role {
            user_id: uid,
            role: r,
        }) => (None, Some(*uid), Some(*r), None, None),
    };

    // 4. 调用 service 层
    permission_management_service
        .create_permission(
            admin.into_identity(),
            &realm_id,
            &client_id,
            role_id,
            user_id,
            role,
            resource,
            action,
        )
        .await
        .map_err(|e| match e {
            herald_core::domain::user::admin_errors::UserAdminError::PermissionDenied(msg) => {
                ApiError::forbidden(msg)
            }
            herald_core::domain::user::admin_errors::UserAdminError::DatabaseError(msg) => {
                tracing::error!("Failed to create permission: {}", msg);
                ApiError::internal(format!("Database error: {}", msg))
            }
            herald_core::domain::user::admin_errors::UserAdminError::InternalError(msg) => {
                tracing::error!("Failed to create permission: {}", msg);
                ApiError::internal(msg)
            }
            _ => {
                tracing::error!("Unexpected error during permission creation");
                ApiError::internal("Unexpected error")
            }
        })?;

    Ok(ApiResult::created(()))
}

/// 权限列表（按 ClientId 分页）
///
#[utoipa::path(
  get,
  path = "/api/permission/{realmId}/permissions/{clientId}",
  tag = "permission",
  params(
    ("realmId" = String, Path, description = "Realm ID"),
    ("clientId" = String, Path, description = "Client ID"),
  ),
  responses(
    (status = 200, description = "Permission list", body = Vec<PermissionData>),
    (status = 500, description = "Internal server error", body = ErrorResponse)
  )
)]
pub async fn list_permission(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    _headers: HeaderMap,
    Path((realm_id, client_id)): Path<(String, String)>,
) -> Result<ApiResult<Vec<PermissionData>>, ApiError> {
    let admin = AdminIdentity::require(identity, &realm_id, "permissions")?;
    admin.require_permission(&state, "policies", "view").await?;

    // 获取 permission_management_service
    let permission_management_service = &state.permission_management_service;

    // 调用 service 层
    let list_data = permission_management_service
        .list_permissions(&realm_id, &client_id)
        .await
        .map_err(|e| match e {
            herald_core::domain::user::admin_errors::UserAdminError::DatabaseError(msg) => {
                tracing::error!("Failed to list permissions: {}", msg);
                ApiError::internal(format!("Database error: {}", msg))
            }
            herald_core::domain::user::admin_errors::UserAdminError::InternalError(msg) => {
                tracing::error!("Failed to list permissions: {}", msg);
                ApiError::internal(msg)
            }
            _ => {
                tracing::error!("Unexpected error during permission listing");
                ApiError::internal("Unexpected error")
            }
        })?;

    // 转换为 PermissionData 格式
    let mut data = Vec::new();

    // 添加 role_policies
    for (role_id, resource, action) in list_data.role_policies {
        data.push(PermissionData::PoliceWrap(Police {
            id: role_id,
            resource,
            action,
        }));
    }

    // 添加 user_roles
    for (user_id, role_id) in list_data.user_roles {
        data.push(PermissionData::RoleWrap(Role {
            user_id,
            role: role_id,
        }));
    }

    Ok(ApiResult::ok(data))
}

/// 删除权限
///
/// 对应规范：post /api/permission/:id
#[utoipa::path(
  post,
  path = "/api/permission/{realmId}/permissions/delete",
  tag = "permission",
  operation_id = "delete_user_permission",
  params(
    ("realmId" = String, Path, description = "Realm ID")
  ),
  request_body = PermissionCreateRequest,
  responses(
    (status = 204, description = "Permission deleted"),
    (status = 404, description = "Permission not found", body = ErrorResponse),
    (status = 500, description = "Internal server error", body = ErrorResponse)
  )
)]
pub async fn delete_permission(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Valid(Json(payload)): Valid<Json<PermissionCreateRequest>>,
) -> Result<ApiResult<()>, ApiError> {
    let admin = AdminIdentity::require(identity, &realm_id, "permissions")?;
    admin
        .require_permission(&state, "policies", "manage")
        .await?;

    // 获取 permission_management_service
    let permission_management_service = &state.permission_management_service;

    // 提取 realm_id 和参数
    let client_id = payload.client_id.to_string();
    let (role_id, user_id, role, resource, action) = match &payload.permission {
        PermissionData::PoliceWrap(Police {
            id,
            resource: res,
            action: act,
        }) => (Some(*id), None, None, Some(res.clone()), Some(act.clone())),
        PermissionData::RoleWrap(Role {
            user_id: uid,
            role: r,
        }) => (None, Some(*uid), Some(*r), None, None),
    };

    // 调用 service 层
    permission_management_service
        .delete_permission(
            admin.into_identity(),
            &realm_id,
            &client_id,
            role_id,
            user_id,
            role,
            resource,
            action,
        )
        .await
        .map_err(|e| match e {
            herald_core::domain::user::admin_errors::UserAdminError::PermissionDenied(msg) => {
                ApiError::forbidden(msg)
            }
            herald_core::domain::user::admin_errors::UserAdminError::DatabaseError(msg) => {
                tracing::error!("Failed to delete permission: {}", msg);
                ApiError::internal(format!("Database error: {}", msg))
            }
            herald_core::domain::user::admin_errors::UserAdminError::InternalError(msg) => {
                tracing::error!("Failed to delete permission: {}", msg);
                ApiError::internal(msg)
            }
            _ => {
                tracing::error!("Unexpected error during permission deletion");
                ApiError::internal("Unexpected error")
            }
        })?;

    Ok(ApiResult::no_content())
}

/// 权限校验
///
/// 对应规范：POST /api/permission/check
#[utoipa::path(
    post,
    path = "/api/permission/check",
    tag = "permission",
    request_body = PermissionCheckRequest,
    responses(
      (status = 200, description = "Permission check result", body = PermissionCheckResponse),
      (status = 400, description = "Bad request", body = ErrorResponse),
      (status = 500, description = "Internal server error", body = ErrorResponse)
    )
  )]
#[tracing::instrument(
    // Governance: `headers` carry auth; `payload` carries
    // the session `token` (credential) and rules that may reference resources
    // bound to a user. All skipped; only the low-cardinality op type recorded.
    skip(state, headers, payload),
    fields(db.operation = "check_permission")
)]
pub async fn check_permission(
    State(state): State<AppState>,
    headers: HeaderMap,
    Valid(Json(payload)): Valid<Json<PermissionCheckRequest>>,
) -> Result<ApiResult<PermissionCheckResponse>, ApiError> {
    // Load session with IP validation
    // IP validation is now performed at the session level (in load_session_with_ip_validation)
    // rather than IP whitelist, to prevent token theft while allowing any IP to initiate requests
    let sess = load_session_with_ip_validation(&state, &payload.token, Some(&headers)).await?;
    let Some(sess) = sess else {
        return Ok(ApiResult::ok(PermissionCheckResponse {
            allowed: false,
            user_id: None,
        }));
    };

    let rules = match payload.rules {
        Some(rules) if !rules.is_empty() => rules,
        _ => {
            let user_id = Uuid::parse_str(&sess.user_id)
                .map_err(|_| ApiError::internal("Session contains invalid user_id".to_string()))?;
            return Ok(ApiResult::ok(PermissionCheckResponse {
                allowed: true,
                user_id: Some(user_id),
            }));
        }
    };

    let mut allowed = false;

    let permission_checker = &state.permission_checker;

    for rule in rules {
        let auth_res = permission_checker
            .check_permission(&sess.realm_id, &sess.user_id, &rule.resource, &rule.action)
            .await
            .unwrap_or(false);

        if auth_res {
            allowed = true;
            break;
        }
    }

    let user_id = Uuid::parse_str(&sess.user_id)
        .map_err(|_| ApiError::internal("Session contains invalid user_id".to_string()))?;

    Ok(ApiResult::ok(PermissionCheckResponse {
        allowed,
        user_id: Some(user_id),
    }))
}

pub fn router() -> axum::Router<AppState> {
    use axum::routing::{get, post};

    axum::Router::new()
        .route("/", post(create_permission))
        .route("/{client_id}", get(list_permission))
        .route("/delete", post(delete_permission))
}

// Governance tests.
//
// Covers: admin `check_permission` instrument skip correctness.
//
// WHY: `check_permission` reads auth from `headers` and a session `token`
// (credential) from `payload`, plus rules that may reference resources bound to
// a user. If the `#[instrument]` macro ever stops skipping those, the token /
// user-bound data leaks into a span field. Source-scan baseline,
// anchored to `fn check_permission` and its immediately-preceding
// `#[tracing::instrument(...)]`.
#[cfg(test)]
mod instrument_skip_tests {
    const SRC: &str = include_str!("permission.rs");

    fn instrument_body_preceding(fn_name: &str) -> String {
        let needle = format!("fn {fn_name}");
        let fn_pos = SRC
            .find(&needle)
            .unwrap_or_else(|| panic!("fn {fn_name} not found in source"));
        let attr_start = SRC[..fn_pos]
            .rfind("#[tracing::instrument(")
            .unwrap_or_else(|| panic!("no #[tracing::instrument( preceding fn {fn_name}"));
        let body_start = attr_start + "#[tracing::instrument(".len();
        // Find the attribute close: the first line at/after body_start whose
        // trimmed content is exactly `)]`. This handles indented closes (e.g.
        // inside an `impl` block) and ignores inline `))]` sequences such as
        // `#[validate(length(...))]` that appear on struct fields.
        let tail = &SRC[body_start..];
        let mut consumed = 0usize;
        for line in tail.lines() {
            let prev = consumed;
            consumed += line.len() + 1; // +1 for the line separator
            if line.trim() == ")]" {
                return tail[..prev].to_string();
            }
        }
        panic!("unterminated #[tracing::instrument( for fn {fn_name}")
    }

    #[test]
    fn instrument_skip_admin_check_permission_excludes_token_and_payload() {
        let body = instrument_body_preceding("check_permission");
        for required in ["state", "headers", "payload"] {
            assert!(
                body.contains(required),
                "check_permission must skip `{required}`; body was:\n{body}"
            );
        }
        for banned in ["token", "password", "email", "secret", "code"] {
            assert!(
                !body.contains(&format!("{banned} ="))
                    && !body.contains(&format!("fields({banned}")),
                "check_permission span must not record a `{banned}` field; body was:\n{body}"
            );
        }
    }
}
