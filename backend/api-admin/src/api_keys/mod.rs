pub mod create;
pub mod delete;
pub mod get;
pub mod list;
pub mod roles;
pub mod rotate;
pub mod types;
pub mod update;

use axum::Router;
use herald_api_base::application::http::state::AppState;

// Re-export for utoipa
pub use create::__path_create_api_key;
pub use delete::__path_delete_api_key;
pub use get::__path_get_api_key;
pub use list::__path_list_api_keys;
pub use roles::__path_get_api_key_roles;
pub use roles::__path_update_api_key_roles;
pub use rotate::__path_rotate_api_key;
pub use update::__path_update_api_key;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            axum::routing::get(list::list_api_keys).post(create::create_api_key),
        )
        .route(
            "/{apiKeyId}",
            axum::routing::get(get::get_api_key)
                .put(update::update_api_key)
                .delete(delete::delete_api_key),
        )
        .route(
            "/{apiKeyId}/rotate",
            axum::routing::post(rotate::rotate_api_key),
        )
        .route(
            "/{apiKeyId}/roles",
            axum::routing::get(roles::get_api_key_roles).put(roles::update_api_key_roles),
        )
}
