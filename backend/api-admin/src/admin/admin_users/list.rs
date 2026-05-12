use crate::admin::admin_users::types::{ErrorResponse, ListUsersQuery, UserResponse};
use axum::{
    Extension,
    extract::{Path, Query, State},
    http::HeaderMap,
};
use herald_api_base::application::http::common::pagination;
use herald_api_base::application::http::server::api_entities::PageResponse;
use herald_api_base::application::http::server::api_entities::{ApiError, ApiResult};
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::authorization::permission_service::PermissionService;
use herald_core::domain::common::policies::ensure_policy;
use herald_core::domain::user::UserService;
use sqlx::Row;

/// List users by realm_id with pagination
///
/// **MIGRATED**: Now uses Extension<Identity> for authentication
/// Realm boundary check is enforced in Service layer
#[utoipa::path(
    get,
    path = "/api/users/{realmId}",
    tag = "users",
    summary = "List users in the realm",
    description = "List users with pagination. Requires `users.view` permission.",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("page" = Option<i32>, Query, description = "Page number (0-based)"),
        ("pageSize" = Option<i32>, Query, description = "Page size"),
        ("email" = Option<String>, Query, description = "Filter users by email (partial match)"),
    ),
    responses(
        (status = 200, description = "List of users", body = PageResponse<UserResponse>),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions (requires users.view) or realm boundary violation", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_users(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Query(query): Query<ListUsersQuery>,
    _headers: HeaderMap,
) -> Result<ApiResult<PageResponse<UserResponse>>, ApiError> {
    use herald_core::domain::common::entities::app_errors::CoreError;

    // Extract identity from request extension (injected by inject_identity middleware)
    let user_id = identity.user_id();
    let identity_realm_id = identity.realm_id();

    // Debug logging to investigate UUID issue
    tracing::debug!("user_id from identity: {}", user_id);
    tracing::debug!("user_id length: {}", user_id.len());
    tracing::debug!(
        "user_id is_uuid: {}",
        uuid::Uuid::parse_str(&user_id).is_ok()
    );

    // MANUAL POLICY CHECK
    // Check specific permission for listing users
    {
        let permission_checker = &state.permission_checker;

        let allowed = permission_checker
            .check_permission(&realm_id, &user_id, "users", "view")
            .await
            .map_err(|e| {
                tracing::error!("Permission check error: {e}");
                ApiError::internal("Internal server error")
            })?;

        tracing::info!(
            realm_id = %realm_id,
            user_id = %user_id,
            resource = "users",
            action = "view",
            allowed,
            "Policy check result"
        );

        ensure_policy(allowed, "Insufficient permissions to list users").map_err(|e| match e {
            CoreError::Forbidden(msg) => {
                tracing::warn!(
                    "Policy check failed: realm_id={}, user_id={}, error={}",
                    realm_id,
                    user_id,
                    msg
                );
                ApiError::forbidden(msg)
            }
            _ => ApiError::internal("Internal server error"),
        })?;
    }

    // STRICT REALM BOUNDARY CHECK
    // Session user can ONLY access their own realm (no exceptions)
    tracing::info!(
        user_realm = %identity_realm_id,
        target_realm = %realm_id,
        "Starting realm boundary check"
    );

    if identity_realm_id != realm_id {
        tracing::warn!(
            user_realm = %identity_realm_id,
            target_realm = %realm_id,
            "Realm boundary violation: user cannot access different realm"
        );
        return Err(ApiError::forbidden(
            "Access denied: cannot list users from a different realm",
        ));
    }

    tracing::debug!(
        user_realm = %identity_realm_id,
        target_realm = %realm_id,
        "Realm access check passed"
    );

    // Normalize pagination parameters
    let norm = pagination::PaginationRequest {
        page: query.page.unwrap_or(0) as i64,
        page_size: query.page_size.unwrap_or(10) as i64,
    }
    .normalize();

    // Call user service directly (Policy and Realm checks already done above)
    let user_service = state.service.user_service();
    let (users, total_count) = user_service
        .list_users(
            identity.clone(),
            realm_id.clone(),
            norm.page as u64,
            norm.page_size as u64,
            query.email,
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to list users: {e}");
            ApiError::internal("Failed to list users")
        })?;

    // Fetch nicknames for all users using LEFT JOIN
    // Following pattern from backend/api/src/application/http/user/profile.rs:54
    let user_ids: Vec<uuid::Uuid> = users.iter().map(|u| u.id).collect();
    let nicknames_query = sqlx::query(
        r#"
        SELECT p.id, p.nickname
        FROM profile p
        WHERE p.id = ANY($1)
        "#,
    )
    .bind(&user_ids)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch user nicknames: {e}");
        ApiError::internal("Failed to fetch user nicknames")
    })?;

    // Build a hashmap of user_id -> nickname
    let nickname_map: std::collections::HashMap<uuid::Uuid, Option<String>> = nicknames_query
        .into_iter()
        .map(|row| (row.get("id"), row.get("nickname")))
        .collect();

    // Map User entities to UserResponse
    let user_responses: Vec<UserResponse> = users
        .into_iter()
        .map(|user| {
            let nickname = nickname_map.get(&user.id).cloned().flatten();
            UserResponse {
                id: user.id,
                realm_id: user.realm_id,
                email: user.email,
                nickname,
                status: user.status as i16,
                created_at: user.created_at.to_rfc3339(),
            }
        })
        .collect();

    Ok(ApiResult::ok(PageResponse {
        items: user_responses,
        page: norm.page,
        page_size: norm.page_size,
        total: total_count,
    }))
}
