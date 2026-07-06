pub mod delete_account;
pub mod permissions;
pub mod profile;
pub mod roles;

use crate::application::http::state::AppState;
use axum::{
    Router,
    routing::{get, post},
};

// Re-export for utoipa
pub use delete_account::__path_delete_account;
pub use profile::__path_change_password;
pub use profile::__path_get_profile;
pub use profile::__path_update_profile;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/roles", get(roles::get_user_roles))
        .route("/permissions", get(permissions::get_user_permissions))
        .route(
            "/profile",
            get(profile::get_profile).put(profile::update_profile),
        )
        .route("/change-password", post(profile::change_password))
        // Self-service account deletion (soft-delete) — BE-D07.
        // Mounted on the existing `/api/user` group, which already carries the
        // `inject_identity` layer (server/mod.rs); no extra layer needed.
        .route("/me/delete-account", post(delete_account::delete_account))
}
