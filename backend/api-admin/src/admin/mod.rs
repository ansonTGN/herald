use axum::Router;
use axum::middleware::from_fn_with_state;
use herald_api_base::application::http::auth::identity_middleware::{
    inject_token_identity, require_first_party_token,
};
use herald_api_base::application::http::state::AppState;

pub mod admin_users;
pub mod middleware;
pub mod permission;
pub mod permission_definitions;

// Re-export for utoipa

/// Admin router with permission middleware applied
/// This is the secure version that should be used in production
/// Routes are mounted at /api/roles/{realmId}/...
///
/// **Architecture Note**: Permission checks are performed in Service layer (HTTP handlers)
/// NOT in HTTP middleware, following six-sided architecture principles.
pub fn admin_router_with_middleware(state: AppState) -> Router<AppState> {
    use crate::role_definitions;

    Router::new()
        // 🔴 P0: RBAC 元数据管理 - 仅主管理员可访问
        // Permission checks are done in Service layer or HTTP handlers
        .nest(
            "/{realmId}/define",
            role_definitions::role_defs_router()
                .layer(axum::middleware::from_fn(require_first_party_token))
                .layer(from_fn_with_state(state.clone(), inject_token_identity)),
        )
        // 🟡 P1: 用户管理 - 主管理员 + 次管理员可访问
        // Permission checks are done in HTTP handlers using enforce()
        .nest(
            "/{realmId}/users",
            admin_users::router()
                .layer(axum::middleware::from_fn(require_first_party_token))
                .layer(from_fn_with_state(state.clone(), inject_token_identity)),
        )
}
