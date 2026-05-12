pub mod create;
pub mod delete;
pub mod get;
pub mod list;
pub mod permissions;
pub mod types;

pub use create::create_role;
pub use delete::delete_role;
pub use get::get_role;
pub use list::list_roles;
pub mod update;
pub use permissions::{
    assign_permission_to_role, get_role_permissions, remove_permission_from_role,
};
pub use update::update_role;

// Re-export for utoipa
pub use create::__path_create_role;
pub use delete::__path_delete_role;
pub use get::__path_get_role;
pub use list::__path_list_roles;
pub use permissions::__path_assign_permission_to_role;
pub use permissions::__path_get_role_permissions;
pub use permissions::__path_remove_permission_from_role;
pub use update::__path_update_role;

use axum::routing::{delete, get, post};

// Create individual router handlers for nested routes
pub fn role_defs_router() -> axum::Router<herald_api_base::application::http::state::AppState> {
    axum::Router::new()
        .route("/", post(create_role).get(list_roles))
        .route(
            "/{roleId}",
            get(get_role).put(update_role).delete(delete_role),
        )
        .route(
            "/{roleId}/permissions",
            post(assign_permission_to_role).get(get_role_permissions),
        )
        .route(
            "/{roleId}/permissions/{permissionId}",
            delete(remove_permission_from_role),
        )
}
