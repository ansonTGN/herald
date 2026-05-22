// Principal-based authorization helpers
//
// Reusable authorization checks for management API handlers.
// These helpers resolve Identity -> PrincipalRef and call the RBAC system.

use axum::http::StatusCode;

use herald_api_base::application::http::common::error_codes::ErrorCode;
use herald_api_base::application::http::server::api_entities::api_error::ApiError;
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::authorization::permission_service::PermissionService;

/// Verify that the authenticated identity belongs to the target realm.
///
/// Returns `Ok(())` when the identity's realm matches `realm_id` (or the
/// identity belongs to the `"admin"` realm, which is always allowed).
/// On mismatch, logs a warning and returns a 403 FORBIDDEN error.
///
/// `operation` is a human-readable label (e.g. "user creation") used in
/// the warning log: `"Cross-realm {operation} attempt blocked"`.
pub fn require_realm_membership(
    identity: &Identity,
    realm_id: &str,
    operation: &str,
) -> Result<(), ApiError> {
    let identity_realm = identity.realm_id();
    if identity_realm == "admin" || identity_realm == realm_id {
        Ok(())
    } else {
        tracing::warn!(
            identity_realm = %identity_realm,
            target_realm = %realm_id,
            "Cross-realm {operation} attempt blocked"
        );
        Err(ApiError::with_code(
            StatusCode::FORBIDDEN,
            ErrorCode::CrossRealmAccessForbidden.as_u32(),
            ErrorCode::CrossRealmAccessForbidden.as_str(),
        ))
    }
}

/// Check if the authenticated principal has a specific permission.
///
/// This is the authoritative handler-level authorization check.
/// Returns Ok(()) if allowed, or a 403 FORBIDDEN response if denied.
///
/// Uses `check_principal_permission` which supports all principal types
/// (user, api_key, client) through the unified RBAC system.
pub async fn require_principal_permission(
    state: &AppState,
    identity: &Identity,
    realm_id: &str,
    resource: &str,
    action: &str,
) -> Result<(), ApiError> {
    let principal = identity.principal_ref();
    let allowed = state
        .permission_checker
        .check_principal_permission(
            realm_id,
            principal.principal_type,
            &principal.principal_id,
            resource,
            action,
        )
        .await
        .unwrap_or(false);

    if allowed {
        Ok(())
    } else {
        Err(ApiError::with_code(
            StatusCode::FORBIDDEN,
            ErrorCode::PermissionDenied.as_u32(),
            ErrorCode::PermissionDenied.as_str(),
        ))
    }
}
