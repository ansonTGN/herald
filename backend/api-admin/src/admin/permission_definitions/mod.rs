use axum::{
    Router,
    routing::{get, post},
};
use herald_api_base::application::http::state::AppState;

mod create;
mod delete;
mod get;
mod list;
pub mod types;
mod update;

#[cfg(test)]
mod tests;

pub use create::*;
pub use delete::*;
pub use get::*;
pub use list::*;
pub use update::*;

// Re-export for utoipa
pub use create::__path_create_permission as __path_create_permission_definition;
pub use delete::__path_delete_permission as __path_delete_permission_definition;
pub use get::__path_get_permission as __path_get_permission_definition;
pub use list::__path_list_permissions as __path_list_permission_definitions;
pub use update::__path_update_permission as __path_update_permission_definition;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_permission).get(list_permissions))
        .route(
            "/{permissionDefinitionId}",
            get(get_permission)
                .put(update_permission)
                .delete(delete_permission),
        )
}
