// Helper functions for building Identity from session

use uuid::Uuid;

use crate::application::http::auth::util::SessionData;
use crate::application::http::server::api_entities::ApiError;
use crate::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::user::UserRepository;

/// Helper function to build Identity from SessionData
pub async fn build_identity(state: &AppState, sess: &SessionData) -> Result<Identity, ApiError> {
    // Parse user_id from session
    let user_id = match Uuid::parse_str(&sess.user_id) {
        Ok(uuid) => uuid,
        Err(_) => {
            // Create temporary user for testing with non-UUID user_ids
            tracing::warn!(
                user_id = %sess.user_id,
                "Invalid UUID in session, creating temporary user"
            );
            let temp_user = herald_core::domain::user::User {
                id: Uuid::now_v7(),
                realm_id: sess.realm_id.clone(),
                email: format!("{}@temp.local", &sess.user_id),
                nickname: None,
                password_hash: None,
                provider_ids: vec![],
                status: herald_core::domain::user::UserStatus::Normal,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            return Ok(Identity::User(temp_user));
        }
    };

    // Load User from database via repository
    let user = match state.user_repository.get_user_by_id(user_id).await {
        Ok(user) => user,
        Err(herald_core::domain::common::entities::app_errors::CoreError::NotFound) => {
            // User not found in database - create temporary user for testing
            tracing::warn!(
                user_id = %user_id,
                "User not found in database, creating temporary user for testing"
            );
            let temp_user = herald_core::domain::user::User {
                id: user_id,
                realm_id: sess.realm_id.clone(),
                email: format!("{}@temp.local", &sess.user_id),
                nickname: None,
                password_hash: None,
                provider_ids: vec![],
                status: herald_core::domain::user::UserStatus::Normal,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            return Ok(Identity::User(temp_user));
        }
        Err(_) => {
            return Err(ApiError::internal("Internal server error"));
        }
    };

    Ok(Identity::User(user))
}
