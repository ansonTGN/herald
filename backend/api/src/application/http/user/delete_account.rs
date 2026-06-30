// Account self-deletion (soft-delete) handler — BE-D07.
//
// POST /api/user/me/delete-account, mounted on the existing `/api/user` route
// group (already covered by `inject_identity` in server/mod.rs). The body
// carries a single `password` field for second-factor confirmation.
//
// Error mapping (see design §4.2.2):
//   - 401 password wrong / missing (CoreError::Unauthorized)
//   - 409 account already deleted (CoreError::Conflict)
//   - 500 anything else
// The `From<CoreError> for ApiError` impl already covers this mapping, so the
// handler is a thin pass-through over the service.

use axum::{
    Json,
    extract::{Extension, State},
    http::StatusCode,
};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::application::http::server::api_entities::{ApiError, ErrorResponse};
use crate::application::http::state::AppState;
use herald_core::domain::authentication::Identity;

/// POST /api/user/me/delete-account request body.
#[derive(Debug, Deserialize, ToSchema)]
pub struct DeleteAccountRequest {
    /// Current password — second-factor confirmation for this irreversible
    /// operation. bcrypt-verified against the account's stored hash.
    pub password: String,
}

/// Self-service account deletion (soft-delete).
///
/// Requires an authenticated session (`inject_identity`). Anonymizes PII,
/// marks the account `Deleted`, clears TOTP, cancels in-effect subscriptions,
/// revokes all sessions (incl. the caller's), and records a `user.delete`
/// audit event. One-time purchases are NOT refunded. Returns 204 on success —
/// the caller's token is invalidated by the session wipe.
#[utoipa::path(
    post,
    path = "/api/user/me/delete-account",
    tag = "user",
    request_body = DeleteAccountRequest,
    responses(
        (status = 204, description = "Account deleted (soft-delete); caller session revoked"),
        (status = 401, description = "Wrong or missing password", body = ErrorResponse),
        (status = 409, description = "Account is already deleted", body = ErrorResponse),
        (status = 500, description = "Server error", body = ErrorResponse)
    )
)]
pub async fn delete_account(
    State(app_state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Json(req): Json<DeleteAccountRequest>,
) -> Result<StatusCode, ApiError> {
    app_state
        .self_delete_service
        .self_delete_account(&identity, &req.password)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
