pub mod user_totp;

use crate::application::http::state::AppState;
use axum::{
    Router,
    routing::{delete, get, post},
};

// Re-export for utoipa
pub use user_totp::__path_handle_disable_totp;
pub use user_totp::__path_handle_enable_totp;
pub use user_totp::__path_handle_get_totp_status;
pub use user_totp::__path_handle_regenerate_totp;
pub use user_totp::__path_handle_verify_totp_setup;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/totp", post(user_totp::handle_enable_totp))
        .route("/totp", delete(user_totp::handle_disable_totp))
        .route("/totp/verify", post(user_totp::handle_verify_totp_setup))
        .route("/totp/regenerate", post(user_totp::handle_regenerate_totp))
        .route("/totp/status", get(user_totp::handle_get_totp_status))
}
