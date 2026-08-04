// =============================================================================
// Apple Native Login Test Helpers
// =============================================================================
//
// Test-only infrastructure for the Apple native login scenario tests
// (`apple_native_scenarios.rs`). Provides:
//
// - `mint_test_apple_id_token(...)` — signs an Apple-style identity token
//   (JWT, RS256) with a chosen `kid`, configurable claims (sub / aud / exp /
//   iss / email / email_verified as the string "true"/"false" Apple uses) and
//   an optional override private key.
// - `spawn_apple_wiremock_jwks(...)` — starts a `wiremock` `MockServer` that
//   serves a JWKS document at Apple's conventional path `/auth/keys`, with a
//   configurable HTTP status (default 200; pass 500 to exercise the
//   JWKS-unreachable 503 branch).
// - `full_apple_jwks_url(base)` — base + `/auth/keys`, suitable to pass as
//   the Apple native handler's `jwks_url` override.
//
// The RSA keypair fixtures (`default_keypair` / `wrong_keypair`) and the
// provider-agnostic `spawn_wiremock_jwks` building block live in
// `google_one_tap_helpers` (they are not Google-specific — they just generate
// an RSA-2048 keypair and serve a JWKS document). Apple helpers reuse them so
// there is a single source of truth for the keypair fixture across providers.
//
// JWKS INJECTION (dependency injection via AppState):
// --------------------------------------------------
// `verify_apple_id_token(id_token, client_id, http_client, jwks_url)` accepts
// a `jwks_url` parameter, and the production Apple native handler reads it
// from `state.apple_jwks_url` (wired from the `[apple_oauth]` config section;
// default = the real Apple endpoint). Scenario tests override that single
// field on a private owned `AppState` copy via
// `ctx.create_unified_test_router_with_state(|s| s.apple_jwks_url = ...)`, so
// the handler consults the wiremock JWKS served here instead of the real
// Apple endpoint. No process-wide env var is involved, so the scenarios are
// safe under parallel nextest runs.
//
// =============================================================================

#![allow(dead_code)]

use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde::Serialize;

// Re-export the provider-agnostic RSA keypair fixtures so Apple scenario
// tests import everything from one place. They are generated/stored by the
// Google One Tap helpers (OnceLock-backed, per-process).
pub use crate::tests::helpers::google_one_tap_helpers::{default_keypair, wrong_keypair};

/// Apple JWKS path the wiremock mock is mounted on (same as the real Apple
/// endpoint). `verify_apple_id_token` GETs the *full* JWKS URL it is handed,
/// so a scenario must pass `full_apple_jwks_url(base)` — the bare mock base
/// URL would hit `/` and 404.
pub const APPLE_JWKS_PATH: &str = "/auth/keys";

/// Stable `kid` used for the default Apple test JWKS / identity token pair.
pub fn test_kid() -> &'static str {
    "test-apple-native-kid"
}

/// Build the full JWKS URL (base + [`APPLE_JWKS_PATH`]) from a wiremock
/// `MockServer::uri()` base, suitable to pass as the Apple native handler's
/// `jwks_url` override.
pub fn full_apple_jwks_url(base_uri: &str) -> String {
    format!("{}{}", base_uri.trim_end_matches('/'), APPLE_JWKS_PATH)
}

/// Options for minting a test Apple identity token. Each field maps 1:1 to a
/// scenario knob so negative tests can target a specific rejection reason
/// (signature, audience, expiry, issuer). Fields default to a valid token.
///
/// `Serialize` lets the opts double as the JWT claims payload directly (field
/// names match Apple's claim names); `kid` and `override_private_key_pem` are
/// signing inputs, not claims, so they are skipped on serialization.
#[derive(Serialize)]
pub struct MintAppleIdTokenOpts<'a> {
    /// `sub` — Apple's stable user identifier. Defaults to a random UUID.
    pub sub: String,
    /// `aud` — must equal the realm's Apple `client_id`. Defaults to
    /// `apple-test-client-id`.
    pub aud: String,
    /// `iss` — Apple's issuer. Defaults to `https://appleid.apple.com`.
    pub iss: String,
    /// `exp` (unix seconds). Defaults to "now + 1 hour".
    pub exp: u64,
    /// `iat` (unix seconds). Defaults to "now".
    pub iat: u64,
    /// `email`. `None` omits the claim (Apple omits it after the first
    /// authorization). Defaults to a unique `apple-<uuid>@test.com`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// `email_verified` — Apple serializes this as the string "true"/"false"
    /// (never a bool). Defaults to `Some("true")`. Pass `None` to omit the
    /// claim, or `Some("false")` to exercise the unverified path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_verified: Option<String>,
    /// `kid` to place in the JWT header. Must match the `kid` the JWKS
    /// serves. Defaults to [`test_kid`].
    #[serde(skip)]
    pub kid: &'a str,
    /// Optional override private key (PEM). When `None`, the default test
    /// keypair is used; scenarios that need an unvalidatable signature pass
    /// `Some(wrong_keypair().private_key_pem.clone())`.
    #[serde(skip)]
    pub override_private_key_pem: Option<String>,
}

impl<'a> Default for MintAppleIdTokenOpts<'a> {
    fn default() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            sub: uuid::Uuid::now_v7().to_string(),
            aud: "apple-test-client-id".to_string(),
            iss: "https://appleid.apple.com".to_string(),
            exp: now + 3600,
            iat: now,
            email: Some(format!("apple-{}@test.com", uuid::Uuid::now_v7())),
            email_verified: Some("true".to_string()),
            kid: test_kid(),
            override_private_key_pem: None,
        }
    }
}

/// Mint an Apple-style identity token (JWT, RS256) signed with either the
/// default test keypair or a caller-supplied private key. Returns the compact
/// JWT string the Apple native handler will verify.
pub fn mint_test_apple_id_token(opts: &MintAppleIdTokenOpts<'_>) -> String {
    let pem = opts
        .override_private_key_pem
        .as_deref()
        .unwrap_or(&default_keypair().private_key_pem);
    let encoding_key = EncodingKey::from_rsa_pem(pem.as_bytes())
        .expect("failed to build RSA EncodingKey from test PEM");

    let header = Header {
        typ: Some("JWT".to_string()),
        alg: Algorithm::RS256,
        kid: Some(opts.kid.to_string()),
        ..Default::default()
    };

    encode(&header, opts, &encoding_key).expect("failed to encode test Apple identity token")
}

/// Spawn a `wiremock::MockServer` that serves a JWKS document containing the
/// given `kid` / `n` / `e` at Apple's conventional JWKS path `/auth/keys`.
/// Returns the server's base URL so callers can construct a `jwks_url`
/// argument via [`full_apple_jwks_url`].
///
/// `status` lets a scenario force a non-2xx response (e.g. 500) to exercise
/// the "JWKS unreachable → 503" branch.
pub async fn spawn_apple_wiremock_jwks(
    kid: &str,
    n_b64: &str,
    e_b64: &str,
    status: u16,
) -> wiremock::MockServer {
    use wiremock::{
        Mock, ResponseTemplate,
        matchers::{method, path},
    };

    let server = wiremock::MockServer::start().await;
    let body = serde_json::json!({
        "keys": [
            {
                "kid": kid,
                "kty": "RSA",
                "alg": "RS256",
                "use": "sig",
                "n": n_b64,
                "e": e_b64,
            }
        ]
    });
    let response = if (200..300).contains(&status) {
        ResponseTemplate::new(status).set_body_json(body)
    } else {
        ResponseTemplate::new(status).set_body_string("upstream jwks unavailable")
    };
    Mock::given(method("GET"))
        .and(path(APPLE_JWKS_PATH))
        .respond_with(response)
        .mount(&server)
        .await;
    server
}

/// Convenience: spawn an Apple JWKS that serves the default test keypair's
/// public components under the test `kid`, and return both the server and the
/// matching `kid` so the caller can mint tokens with the same `kid`.
pub async fn spawn_apple_default_jwks() -> (wiremock::MockServer, &'static str) {
    let kp = default_keypair();
    let server = spawn_apple_wiremock_jwks(test_kid(), &kp.n_b64, &kp.e_b64, 200).await;
    (server, test_kid())
}
