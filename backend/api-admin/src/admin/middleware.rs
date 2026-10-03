use herald_api_base::application::http::server::api_entities::ApiError;
use herald_core::domain::user::UserStatus;
use uuid::Uuid;

const SENSITIVE_PERMISSIONS: &[&str] = &["realm.manage"];

/// Validates that sensitive permissions can only be created in the admin realm
///
/// # Arguments
/// * `permission_name` - The name of the permission being created
/// * `caller_realm_id` - The realm ID of the caller creating the permission
///
/// # Returns
/// * `Ok(())` if the permission can be created
/// * `Err(ApiError::Forbidden)` if the permission is sensitive and caller is not in admin realm
pub fn validate_sensitive_permission_creation(
    permission_name: &str,
    caller_realm_id: &str,
) -> Result<(), ApiError> {
    if SENSITIVE_PERMISSIONS.contains(&permission_name) && caller_realm_id != "admin" {
        return Err(ApiError::forbidden(format!(
            "Permission '{}' can only be created in admin realm",
            permission_name
        )));
    }
    Ok(())
}

/// Whether a permission or policy name uses a platform-reserved wildcard
/// (`All`, or a `*` in either segment). Wildcards are reserved for the
/// platform: the RBAC matcher is exact-match today so this is inert, but a
/// future wildcard matcher would turn such rows into bypasses. Shared by all
/// policy-creation surfaces (permission definitions, role policies, direct
/// user permissions) so they reject the same names in both segments.
pub fn is_reserved_wildcard(resource: &str, action: &str) -> bool {
    resource == "All" || action == "All" || resource.contains('*') || action.contains('*')
}

/// Deleted(3) terminal-state guard for the admin write paths whose service
/// call cannot see the target's status (users.md §4.2): loads the target's
/// account status scoped to the admin session realm and refuses a Deleted
/// target with 409 — the anonymized tombstone must not gain or lose
/// authorization surface. With `require_present`, an unknown or cross-realm
/// id 404s here (permissions.md §4.2 id-oracle); without it the presence
/// check is left to the path's own service call.
pub async fn ensure_target_writable(
    pool: &sqlx::PgPool,
    realm_id: &str,
    target_user_id: Uuid,
    action: &str,
    require_present: bool,
) -> Result<(), ApiError> {
    let target_status: Option<i16> =
        sqlx::query_scalar("SELECT status FROM account WHERE id = $1 AND realm_id = $2")
            .bind(target_user_id)
            .bind(realm_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| {
                tracing::error!(
                    realm_id = realm_id,
                    target_user_id = %target_user_id,
                    action = action,
                    error = %e,
                    "Failed to load target user status"
                );
                ApiError::internal("Failed to load target user")
            })?;
    if require_present && target_status.is_none() {
        return Err(ApiError::not_found("User not found in this realm"));
    }
    if target_status == Some(UserStatus::Deleted as i16) {
        tracing::warn!(
            realm_id = realm_id,
            target_user_id = %target_user_id,
            action = action,
            "Write rejected: target is Deleted (anonymized terminal state)"
        );
        return Err(ApiError::conflict(
            "User is deleted (anonymized) and cannot be edited",
        ));
    }
    Ok(())
}
