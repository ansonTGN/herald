use axum::{Json, extract::State, http::HeaderMap};
use axum_valid::Valid;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::application::http::auth::util::require_session;
pub use crate::application::http::server::api_entities::ErrorResponse;
use crate::application::http::server::api_entities::{ApiError, ApiResult};
use crate::application::http::state::AppState;
use herald_core::domain::user::entities::Profile;
use herald_core::domain::user::ports::UserRepository;
use herald_core::infrastructure::user::repositories::PostgresUserRepository;

// ==================== Request/Response Types ====================

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UserProfile {
    pub id: Uuid,
    pub email: String,
    pub nickname: Option<String>,
    pub status: i16,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct UpdateProfileRequest {
    #[validate(length(max = 50))]
    pub nickname: Option<String>,
}

// ==================== Handler Functions ====================

/// Get current user profile
#[utoipa::path(
    get,
    path = "/api/user/profile",
    tag = "user",
    responses(
        (status = 200, description = "User profile", body = UserProfile),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<ApiResult<UserProfile>, ApiError> {
    let (_token, sess) = require_session(&state, &headers).await?;

    // Use repository to access data
    let user_repo = PostgresUserRepository::new(state.db.clone());

    // Get user
    let user_id = uuid::Uuid::parse_str(&sess.user_id).map_err(|e| {
        tracing::error!("Invalid user_id format: {}", e);
        ApiError::internal("Invalid user_id format")
    })?;

    let user = user_repo.get_user_by_id(user_id).await.map_err(|e| {
        tracing::error!("Failed to get user profile: {}", e);
        ApiError::internal("Failed to get user profile")
    })?;

    // Try to get profile for additional nickname
    let nickname = match user_repo.get_profile(user_id).await {
        Ok(profile) => profile.nickname,
        Err(_) => user.nickname, // Fallback to user.nickname if profile doesn't exist
    };

    Ok(ApiResult::ok(UserProfile {
        id: user.id,
        email: user.email,
        nickname,
        status: user.status.into(),
    }))
}

/// Update current user profile
#[utoipa::path(
    put,
    path = "/api/user/profile",
    tag = "user",
    request_body = UpdateProfileRequest,
    responses(
        (status = 200, description = "Profile updated", body = UserProfile),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn update_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Valid(Json(payload)): Valid<Json<UpdateProfileRequest>>,
) -> Result<ApiResult<UserProfile>, ApiError> {
    let (_token, sess) = require_session(&state, &headers).await?;

    // Use repository to access data
    let user_repo = PostgresUserRepository::new(state.db.clone());

    let user_id = uuid::Uuid::parse_str(&sess.user_id).map_err(|e| {
        tracing::error!("Invalid user_id format: {}", e);
        ApiError::internal("Invalid user_id format")
    })?;

    // Update profile if nickname provided
    if let Some(nickname) = &payload.nickname {
        // Try to update existing profile, or create if it doesn't exist
        if user_repo
            .update_profile(user_id, Some(nickname.clone()))
            .await
            .is_err()
        {
            // Profile doesn't exist, create it
            let now = chrono::Utc::now();
            let profile = Profile {
                id: user_id,
                realm_id: sess.realm_id.clone(),
                nickname: Some(nickname.clone()),
                created_at: now,
                updated_at: now,
            };
            user_repo.create_profile(profile).await.map_err(|e| {
                tracing::error!("Failed to create profile: {}", e);
                ApiError::internal("Failed to create profile")
            })?;
        }
    }

    // Fetch updated user and profile
    let user = user_repo.get_user_by_id(user_id).await.map_err(|e| {
        tracing::error!("Failed to fetch updated profile: {}", e);
        ApiError::internal("Failed to fetch updated profile")
    })?;

    let nickname = match user_repo.get_profile(user_id).await {
        Ok(profile) => profile.nickname,
        Err(_) => user.nickname,
    };

    Ok(ApiResult::ok(UserProfile {
        id: user.id,
        email: user.email,
        nickname,
        status: user.status.into(),
    }))
}

/// Change password for current user
#[utoipa::path(
    post,
    path = "/api/user/change-password",
    tag = "user",
    operation_id = "change_user_password",
    request_body = ChangePasswordRequest,
    responses(
        (status = 200, description = "Password changed successfully", body = serde_json::Value),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn change_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Valid(Json(payload)): Valid<Json<ChangePasswordRequest>>,
) -> Result<ApiResult<serde_json::Value>, ApiError> {
    let (_token, sess) = require_session(&state, &headers).await?;

    // Use repository to access data
    let user_repo = PostgresUserRepository::new(state.db.clone());

    let user_id = uuid::Uuid::parse_str(&sess.user_id).map_err(|e| {
        tracing::error!("Invalid user_id format: {}", e);
        ApiError::internal("Invalid user_id format")
    })?;

    // Fetch current user
    let user = user_repo.get_user_by_id(user_id).await.map_err(|e| {
        tracing::error!("Failed to fetch user: {}", e);
        ApiError::internal("Failed to fetch user")
    })?;

    let password_hash = user
        .password_hash
        .ok_or_else(|| ApiError::bad_request("User has no password set"))?;

    // Verify old password
    let is_valid = bcrypt::verify(&payload.old_pass, &password_hash).map_err(|e| {
        tracing::error!("Failed to verify password: {}", e);
        ApiError::internal("Failed to verify password")
    })?;

    if !is_valid {
        return Err(ApiError::unauthorized("Invalid old password"));
    }

    // Hash new password
    let new_password_hash = bcrypt::hash(&payload.new_pass, bcrypt::DEFAULT_COST).map_err(|e| {
        tracing::error!("Failed to hash password: {}", e);
        ApiError::internal("Failed to hash password")
    })?;

    // Update password using repository
    user_repo
        .change_password(&sess.realm_id, user_id, new_password_hash)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update password: {}", e);
            ApiError::internal("Failed to update password")
        })?;

    Ok(ApiResult::ok(
        serde_json::json!({"message": "Password changed successfully"}),
    ))
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordRequest {
    #[validate(length(min = 1, max = 36))]
    pub old_pass: String,
    #[validate(length(min = 8, max = 36))]
    pub new_pass: String,
}
