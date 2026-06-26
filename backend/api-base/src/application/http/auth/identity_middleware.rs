// Identity injection middleware
// Reconstructs Identity enum from SessionData and injects into Request extensions

use crate::application::http::auth::util::{
    build_set_cookie, get_cookie, load_session_with_ttl, refresh_session_ttl,
};
use crate::application::http::server::api_entities::ApiError;
use crate::application::http::state::AppState;
use axum::{
    extract::{Request, State},
    http::{HeaderValue, header::SET_COOKIE},
    middleware::Next,
    response::{IntoResponse, Response},
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
#[tracing::instrument(
    // Governance: `req` headers carry the X-Auth session
    // token (cookie) — a credential, MUST be skipped.
    skip(state, req, next),
    fields(http.route = "inject_identity")
)]
pub async fn inject_identity(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<impl IntoResponse, ApiError> {
    let headers = req.headers().clone();

    // Extract session from cookie/header
    let token =
        get_cookie(&headers, "X-Auth").ok_or_else(|| ApiError::unauthorized("missing session"))?;
    let (session_data, current_ttl) = load_session_with_ttl(&state, &token)
        .await?
        .ok_or_else(|| ApiError::unauthorized("invalid session"))?;

    let renewal_ttl = session_data.renewal_ttl_seconds;
    let should_renew = renewal_ttl.is_some_and(|rt| current_ttl <= (rt / 2).max(1));

    if should_renew {
        refresh_session_ttl(
            &state,
            &token,
            renewal_ttl.expect("checked by should_renew"),
        )
        .await?;
    }

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

    let mut response: Response = next.run(req).await;
    if should_renew {
        let rt = renewal_ttl.expect("checked by should_renew");
        let max_age = i64::try_from(rt)
            .map_err(|_| ApiError::internal("Session renewal TTL is invalid".to_string()))?;
        let set_cookie = build_set_cookie("X-Auth", &token, max_age, state.app_env == "production");
        let value = HeaderValue::from_str(&set_cookie)
            .map_err(|_| ApiError::internal("Internal server error"))?;
        response.headers_mut().append(SET_COOKIE, value);
    }

    Ok(response)
}

// Governance tests.
//
// Covers: `inject_identity` instrument skip correctness.
//
// WHY: `inject_identity` reads the X-Auth session cookie/token from request
// headers — a credential. If the `#[instrument]` macro ever stops skipping
// `req`, the raw token leaks into a span field and is exported off-process.
// `#[instrument]` generates span fields at compile time from the macro
// attribute; the reliable way to assert the skip is to inspect the attribute
// text anchored to THIS function (a runtime test-subscriber would require a
// live Request/cookie and only observes the fields the macro actually emitted
// — same property, more brittle setup). This source-scan is the
// baseline governance assertion; it fails loud the moment a refactor drops
// `req` from `skip(...)` or adds a sensitive-named field. The lookup is
// anchored to `fn inject_identity` and takes only the immediately-preceding
// `#[tracing::instrument(...)]` block, so it cannot be tripped by an unrelated
// `skip(...)` in a string literal or a different function.
#[cfg(test)]
mod instrument_skip_tests {
    const SRC: &str = include_str!("identity_middleware.rs");

    /// Extract the inner body of the `#[tracing::instrument(...)]` attribute
    /// that immediately precedes `fn <name>`. Anchored to the target function
    /// so it only ever inspects that function's own attribute — never a global
    /// or cross-function match.
    fn instrument_body_preceding(fn_name: &str) -> String {
        let needle = format!("fn {fn_name}");
        let fn_pos = SRC
            .find(&needle)
            .unwrap_or_else(|| panic!("fn {fn_name} not found in source"));
        let attr_start = SRC[..fn_pos]
            .rfind("#[tracing::instrument(")
            .unwrap_or_else(|| panic!("no #[tracing::instrument( preceding fn {fn_name}"));
        // The attribute body starts after the marker and runs until the
        // matching closing "))". We find the first "))" at/beyond attr_start —
        // the attribute body itself contains no nested "))", so this is the
        // attribute's own terminator.
        let body_start = attr_start + "#[tracing::instrument(".len();
        // Find the attribute close: the first line at/after body_start whose
        // trimmed content is exactly `)]`. This handles indented closes (e.g.
        // inside an `impl` block) and ignores inline `))]` sequences such as
        // `#[validate(length(...))]` that appear on struct fields.
        let tail = &SRC[body_start..];
        let mut consumed = 0usize;
        for line in tail.lines() {
            let prev = consumed;
            consumed += line.len() + 1; // +1 for the line separator
            if line.trim() == ")]" {
                return tail[..prev].to_string();
            }
        }
        panic!("unterminated #[tracing::instrument( for fn {fn_name}")
    }

    #[test]
    fn instrument_skip_inject_identity_excludes_request_credential() {
        let body = instrument_body_preceding("inject_identity");
        assert!(
            body.contains("skip(") || body.contains("skip_all"),
            "inject_identity must declare a skip(..)/skip_all; attribute body was:\n{body}"
        );
        // `skip_all` would also satisfy the requirement, but the production
        // code uses an explicit list — assert the exact sensitive-bearing
        // params are present so a future change to `skip(self, next)` (dropping
        // `req`) fails loud.
        assert!(
            body.contains("req"),
            "inject_identity must skip `req` (carries X-Auth session token); body was:\n{body}"
        );
        assert!(
            body.contains("state"),
            "inject_identity must skip `state`; body was:\n{body}"
        );
    }

    #[test]
    fn instrument_skip_inject_identity_records_no_sensitive_field() {
        let body = instrument_body_preceding("inject_identity");
        for banned in ["token", "password", "email", "code", "secret", "auth"] {
            assert!(
                !body.contains(&format!("{banned} ="))
                    && !body.contains(&format!("fields({banned}")),
                "inject_identity span must not record a `{banned}` field; body was:\n{body}"
            );
        }
    }
}
