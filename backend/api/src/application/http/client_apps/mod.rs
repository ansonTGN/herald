pub mod create;
pub mod delete;
pub mod get;
pub mod list;
pub mod types;
pub mod update;

use crate::application::http::state::AppState;
use axum::Router;

// Re-export for utoipa
pub use create::__path_create_client_app;
pub use delete::__path_delete_client_app;
pub use get::__path_get_client_app;
pub use list::__path_list_client_apps;
pub use update::__path_update_client_app;
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            axum::routing::get(list::list_client_apps).post(create::create_client_app),
        )
        .route(
            "/{clientAppId}",
            axum::routing::get(get::get_client_app)
                .put(update::update_client_app)
                .delete(delete::delete_client_app),
        )
}
