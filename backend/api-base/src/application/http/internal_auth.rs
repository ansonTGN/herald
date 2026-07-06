//! Internal API key middleware.
//!
//! Guards demo/test-only "internal" HTTP endpoints that bypass normal user
//! authentication. Access is gated solely by a shared secret (`X-Internal-API-Key`
//! header) read from the `INTERNAL_API_KEY` environment variable. When that env
//! var is unset or empty, every request is rejected (401), so in a production
//! build without the env var the routes are effectively inert while still
//! compiled in — matching the behavior of the existing internal fulfill route.

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Constant-time string comparison.
///
/// Compares two ASCII strings byte-by-byte without short-circuiting, so timing
/// does not leak the position of the first mismatched byte. Returns early (non-
/// constant time) only when the lengths differ, which does not reveal secret
/// material. Mirrors `herald_infra_shopify::constant_time_compare` to avoid
/// pulling that crate into `api-base`.
fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (byte_a, byte_b) in a.bytes().zip(b.bytes()) {
        result |= byte_a ^ byte_b;
    }

    result == 0
}

/// Middleware that validates the `X-Internal-API-Key` header against the
/// `INTERNAL_API_KEY` environment variable.
///
/// Rejects with 401 UNAUTHORIZED when the header is missing, the env var is
/// unset/empty, or the values differ. On success it forwards to the next layer
/// unchanged (no identity is injected — callers must not rely on one).
pub async fn internal_api_key_middleware(req: Request, next: Next) -> Response {
    let provided_key = req
        .headers()
        .get("X-Internal-API-Key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim);

    let expected_key = std::env::var("INTERNAL_API_KEY")
        .ok()
        .filter(|key| !key.trim().is_empty());

    match (provided_key, expected_key) {
        (Some(provided), Some(expected)) if constant_time_compare(provided, &expected) => {
            next.run(req).await
        }
        _ => StatusCode::UNAUTHORIZED.into_response(),
    }
}
