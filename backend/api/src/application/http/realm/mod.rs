// Realm management module
//
// This module handles all realm-related HTTP endpoints including:
// - CRUD operations for realms (create, read, update, list)
// - Realm TOTP configuration management
// - Realm Passkey configuration management
// - Realm white-label configuration management

// Sub-modules
pub mod crud;
pub mod passkey_config;
pub mod totp_config;
pub mod validators;
pub mod white_label_config;

// Re-export commonly used types and handlers for external use
pub use crud::{AdminUserResponse, ListRealmsResponse, RealmResponse};
pub use validators::{
    CreateRealmValidator, ListRealmsPaginatedQuery, ListRealmsQuery, UpdateRealmValidator,
};

// Re-export for utoipa

use crate::application::http::state::AppState;
use axum::Router;

/// Combined realm router that handles both CRUD and TOTP configuration
///
/// This router is meant to be nested under `/api/realms` in server/mod.rs.
///
/// Routes (when nested under /api/realms):
/// - GET /api/realms - List all realms (non-paginated, deprecated)
/// - GET /api/realms/paginated - List realms with pagination
/// - POST /api/realms - Create a new realm
/// - GET /api/realms/{realmId} - Get realm details
/// - PUT /api/realms/{realmId} - Update realm
/// - GET /api/realms/{realmId}/config/totp - Get realm TOTP configuration
/// - PUT /api/realms/{realmId}/config/totp - Update realm TOTP configuration
/// - GET /api/realms/{realmId}/config/passkey - Get realm Passkey configuration
/// - PUT /api/realms/{realmId}/config/passkey - Update realm Passkey configuration
/// - GET /api/realms/{realmId}/config/white-label - Get white-label configuration state
/// - PUT /api/realms/{realmId}/config/white-label/draft - Save white-label draft
/// - DELETE /api/realms/{realmId}/config/white-label/draft - Discard white-label draft
/// - POST /api/realms/{realmId}/config/white-label/publish - Publish white-label settings
/// - POST /api/realms/{realmId}/config/white-label/restore - Restore previous white-label settings
pub fn realm_router() -> Router<AppState> {
    Router::new()
        // CRUD routes
        .route(
            "/",
            axum::routing::get(crud::list_realms).post(crud::create_realm),
        )
        .route(
            "/paginated",
            axum::routing::get(crud::list_realms_paginated),
        )
        .route(
            "/{realmId}",
            axum::routing::get(crud::get_realm).put(crud::update_realm),
        )
        // TOTP configuration routes
        .route(
            "/{realmId}/config/totp",
            axum::routing::put(totp_config::handle_update_realm_totp_config),
        )
        .route(
            "/{realmId}/config/totp",
            axum::routing::get(totp_config::handle_get_realm_totp_config),
        )
        // Passkey configuration routes
        .route(
            "/{realmId}/config/passkey",
            axum::routing::put(passkey_config::handle_update_realm_passkey_config),
        )
        .route(
            "/{realmId}/config/passkey",
            axum::routing::get(passkey_config::handle_get_realm_passkey_config),
        )
        // White-label configuration routes
        .route(
            "/{realmId}/config/white-label",
            axum::routing::get(white_label_config::handle_get_white_label_config),
        )
        .route(
            "/{realmId}/config/white-label/draft",
            axum::routing::put(white_label_config::handle_save_white_label_draft)
                .delete(white_label_config::handle_discard_white_label_draft),
        )
        .route(
            "/{realmId}/config/white-label/publish",
            axum::routing::post(white_label_config::handle_publish_white_label_config),
        )
        .route(
            "/{realmId}/config/white-label/restore",
            axum::routing::post(white_label_config::handle_restore_white_label_config),
        )
}
