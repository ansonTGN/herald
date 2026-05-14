// Identity injection middleware
// Reconstructs Identity enum from SessionData and injects into Request extensions

use crate::application::http::auth::util::require_session;
use crate::application::http::server::api_entities::ApiError;
use crate::application::http::state::AppState;
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::IntoResponse,
};
use herald_core::domain::authentication::Identity;
use herald_core::domain::user::UserRepository;
use uuid::Uuid;

/// Inject Identity into request extensions from session data
///
/// This middleware:
/// 1. Extracts session from cookie/header
/// 2. Loads User or Client entity from database via service layer
/// 3. Constructs Identity enum
/// 4. Injects into request extensions for downstream handlers
///
/// # Error Handling
///
/// Returns 401 Unauthorized if:
/// - Session token is missing
/// - Session is not found in Redis
/// - User/Client entity is not found in database
pub async fn inject_identity(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<impl IntoResponse, ApiError> {
    let headers = req.headers().clone();

    // Extract session from cookie/header
    let (_token, session_data) = require_session(&state, &headers).await?;

    // Parse user_id from session with validation
    let session_user_id = &session_data.user_id;
    tracing::debug!(
        "Session user_id: {}, length: {}",
        session_user_id,
        session_user_id.len()
    );

    let user_id = Uuid::parse_str(session_user_id)
        .map_err(|_| ApiError::bad_request("Invalid user ID in session"))?;

    tracing::debug!("Parsed user_id from session: {}", user_id);

    // Load User from database via repository
    // Note: We're using the repository directly here to avoid circular dependency
    // on the service layer which requires Identity parameter
    let user = state
        .user_repository
        .get_user_by_id(user_id)
        .await
        .map_err(|e| match e {
            herald_core::domain::common::entities::app_errors::CoreError::NotFound => {
                tracing::error!(
                    session_user_id = %session_user_id,
                    parsed_user_id = %user_id,
                    "User not found in database"
                );
                ApiError::unauthorized("User not found")
            }
            _ => ApiError::internal("Internal server error"),
        })?;

    // Verify the loaded user ID matches the session user ID
    let loaded_user_id = user.id.to_string();
    if loaded_user_id != *session_user_id {
        tracing::error!(
            session_user_id = %session_user_id,
            loaded_user_id = %loaded_user_id,
            "User ID mismatch: session user_id doesn't match loaded user ID"
        );
        return Err(ApiError::internal("Internal server error"));
    }

    let identity = Identity::User(user);

    tracing::debug!(
        realm_id = %identity.realm_id(),
        user_id = %identity.user_id(),
        client_id = %session_data.client_id,
        "Identity injected into request"
    );

    // Insert extensions into the request for next handler to extract
    let mut req = req;
    req.extensions_mut().insert(identity.clone());

    Ok(next.run(req).await)
}
