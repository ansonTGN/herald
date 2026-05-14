pub mod detail;
pub mod list;
pub mod types;

use crate::application::http::state::AppState;
use axum::Router;

pub fn audit_router() -> Router<AppState> {
    Router::new()
        .route("/", axum::routing::get(list::list_audit_events))
        .route("/{eventId}", axum::routing::get(detail::get_audit_event))
}
