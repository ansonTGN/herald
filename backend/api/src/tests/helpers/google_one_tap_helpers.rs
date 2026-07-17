// =============================================================================
// Google One Tap Test Helpers
// =============================================================================
//
// Test-only infrastructure for the Google One Tap scenario tests
// (`google_one_tap_scenarios.rs`). Provides:
//
// - A deterministic RSA-2048 keypair fixture (`TestRsaKeypair`) generated once
//   per process via `OnceLock`, plus a helper to mint a fully independent
//   "wrong" keypair for the `rejects_invalid_signature` scenario.
// - `mint_test_google_id_token(...)` — signs a Google-style OIDC ID Token
//   (JWT, RS256) with a chosen `kid`, configurable claims (sub / aud / exp /
//   iss / email_verified as bool or string / email / name / picture) and an
//   optional override private key. The resulting JWT is what the One Tap
//   handler's `verify_google_id_token` will decode.
// - `spawn_wiremock_jwks(kid, n_b64, e_b64, status)` — starts a `wiremock`
//   `MockServer` that serves a JWKS document at `/oauth2/v3/certs`, with a
//   configurable HTTP status (default 200; pass 500 to exercise the
//   JWKS-unreachable 503 branch). Returns the server's base URL.
//
// JWKS INJECTION (dependency injection via AppState):
// --------------------------------------------------
// `verify_google_id_token(id_token, client_id, http_client, jwks_url)` accepts
// a `jwks_url` parameter, and the production One Tap handler reads it from
// `state.google_jwks_url` (wired from the `[google_oauth]` config section;
// default = the real Google endpoint). Scenario tests override that single
// field on a private owned `AppState` copy via
// `ctx.create_unified_test_router_with_state(|s| s.google_jwks_url = ...)`, so
// the handler consults the wiremock JWKS served here instead of the real
// Google endpoint. There is no process-wide env var involved, so the
// scenarios are safe under parallel nextest runs.
//
// =============================================================================

#![allow(dead_code)]

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use rsa::{
    RsaPrivateKey, RsaPublicKey,
    pkcs8::{EncodePrivateKey, LineEnding},
    traits::PublicKeyParts,
};
use serde::Serialize;
use std::sync::OnceLock;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

/// RSA-2048 keypair + its JWK public components (base64url of the modulus `n`
/// and exponent `e`), all derived from one generated private key. Held behind
/// a `OnceLock` so the expensive keygen runs once per test process.
pub struct TestRsaKeypair {
    pub private_key_pem: String,
    pub n_b64: String,
    pub e_b64: String,
}

impl TestRsaKeypair {
    /// Build a `TestRsaKeypair` from a generated 2048-bit RSA private key.
    fn from_generated() -> Self {
        use rand::rngs::OsRng;
        // 2048-bit key, per the item spec. Generation is deterministic enough
        // for a per-process fixture; `OsRng` seeds from the OS.
        let mut rng = OsRng;
        let priv_key =
            RsaPrivateKey::new(&mut rng, 2048).expect("failed to generate 2048-bit RSA key");
        Self::from_private_key(priv_key)
    }

    fn from_private_key(priv_key: RsaPrivateKey) -> Self {
        let pub_key = RsaPublicKey::from(&priv_key);
        let n_b64 = URL_SAFE_NO_PAD.encode(pub_key.n().to_bytes_be());
        let e_b64 = URL_SAFE_NO_PAD.encode(pub_key.e().to_bytes_be());
        let private_key_pem = priv_key
            .to_pkcs8_pem(LineEnding::LF)
            .expect("failed to encode RSA private key as PKCS#8 PEM")
            .to_string();
        Self {
            private_key_pem,
            n_b64,
            e_b64,
        }
    }
}

static DEFAULT_KEYPAIR: OnceLock<TestRsaKeypair> = OnceLock::new();
static WRONG_KEYPAIR: OnceLock<TestRsaKeypair> = OnceLock::new();

/// The default test RSA keypair. All "valid" test ID Tokens are signed with
/// this key, and the matching wiremock JWKS serves its public components
/// under the test `kid`.
pub fn default_keypair() -> &'static TestRsaKeypair {
    DEFAULT_KEYPAIR.get_or_init(TestRsaKeypair::from_generated)
}

/// An independent RSA keypair used to sign tokens whose signature must NOT
/// validate against the default JWKS (`rejects_invalid_signature` scenario).
pub fn wrong_keypair() -> &'static TestRsaKeypair {
    WRONG_KEYPAIR.get_or_init(TestRsaKeypair::from_generated)
}

/// Generic JSON-friendly claims value used for `email_verified`: either a
/// bool or a string. Enum mirrors production `StringOrBool` but lives only in
/// the test helper for ergonomic claim construction.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum EmailVerifiedValue {
    Bool(bool),
    Str(String),
}

/// Options for minting a test Google ID Token. Each field maps 1:1 to a
/// scenario knob so negative tests can target a specific rejection reason
/// (signature, audience, expiry, unverified email, issuer).
///
/// `Serialize` lets the opts double as the JWT claims payload directly (field
/// names match the OIDC claim names); `kid` and `override_private_key_pem` are
/// signing inputs, not claims, so they are skipped on serialization.
#[derive(Serialize)]
pub struct MintIdTokenOpts<'a> {
    /// `sub` — Google stable user ID. Defaults to a random UUID.
    pub sub: String,
    /// `aud` — must equal the realm's Google `client_id`. Defaults to
    /// `OAuthProviderTestConfig::google().client_id`.
    pub aud: String,
    /// `iss` — one of Google's accepted issuers. Defaults to
    /// `https://accounts.google.com`.
    pub iss: String,
    /// `exp` (unix seconds). Defaults to "now + 1 hour".
    pub exp: u64,
    /// `iat` (unix seconds). Defaults to "now".
    pub iat: u64,
    /// `email`. Defaults to a unique `ot-<uuid>@test.com`.
    pub email: String,
    /// `email_verified` representation. Defaults to `Bool(true)`. Scenarios
    /// pass `Bool(false)` or `Str("false")` to exercise the 401 branch.
    pub email_verified: EmailVerifiedValue,
    /// `name`. Optional.
    pub name: Option<String>,
    /// `picture`. Optional.
    pub picture: Option<String>,
    /// `kid` to place in the JWT header. Must match the `kid` the JWKS
    /// serves. Defaults to `TEST_KID`.
    #[serde(skip)]
    pub kid: &'a str,
    /// Optional override private key (PEM). When `None`, the default test
    /// keypair is used; scenarios that need an unvalidatable signature pass
    /// `Some(wrong_keypair().private_key_pem.clone())`.
    #[serde(skip)]
    pub override_private_key_pem: Option<String>,
}

impl<'a> Default for MintIdTokenOpts<'a> {
    fn default() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            sub: uuid::Uuid::now_v7().to_string(),
            aud: "google-test-client-id".to_string(),
            iss: "https://accounts.google.com".to_string(),
            exp: now + 3600,
            iat: now,
            email: format!("ot-{}@test.com", uuid::Uuid::now_v7()),
            email_verified: EmailVerifiedValue::Bool(true),
            name: Some("One Tap Test User".to_string()),
            picture: Some("https://test.invalid/photo.png".to_string()),
            kid: test_kid(),
            override_private_key_pem: None,
        }
    }
}

/// Stable `kid` used for the default test JWKS / ID Token pair.
pub fn test_kid() -> &'static str {
    "test-google-one-tap-kid"
}

/// JWKS path the wiremock mock is mounted on (same as the real Google
/// endpoint). `verify_google_id_token` GETs the *full* JWKS URL it is handed,
/// so a scenario must pass `full_jwks_url(base)` — the bare mock base URL
/// (e.g. `http://127.0.0.1:xxxx`) would hit `/` and 404.
pub const GOOGLE_JWKS_PATH: &str = "/oauth2/v3/certs";

/// Build the full JWKS URL (base + [`GOOGLE_JWKS_PATH`]) from a wiremock
/// `MockServer::uri()` base, suitable to pass as the One Tap handler's
/// `jwks_url` override.
pub fn full_jwks_url(base_uri: &str) -> String {
    format!("{}{}", base_uri.trim_end_matches('/'), GOOGLE_JWKS_PATH)
}

/// Mint a Google-style OIDC ID Token (JWT, RS256) signed with either the
/// default test keypair or a caller-supplied private key. Returns the compact
/// JWT string the One Tap handler will verify.
pub fn mint_test_google_id_token(opts: &MintIdTokenOpts<'_>) -> String {
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

    encode(&header, opts, &encoding_key).expect("failed to encode test Google ID Token")
}

/// Spawn a `wiremock::MockServer` that serves a JWKS document containing the
/// given `kid` / `n` / `e` at the conventional Google JWKS path. Returns the
/// server's base URL so callers can construct a `jwks_url` argument.
///
/// `status` lets a scenario force a non-2xx response (e.g. 500) to exercise
/// the "JWKS unreachable → 503" branch.
pub async fn spawn_wiremock_jwks(kid: &str, n_b64: &str, e_b64: &str, status: u16) -> MockServer {
    let server = MockServer::start().await;
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
        .and(path("/oauth2/v3/certs"))
        .respond_with(response)
        .mount(&server)
        .await;
    server
}

/// Convenience: spawn a JWKS that serves the default test keypair's public
/// components under the test `kid`, and return both the base URL and the
/// matching `kid` so the caller can mint tokens with the same `kid`.
pub async fn spawn_default_jwks() -> (MockServer, &'static str) {
    let kp = default_keypair();
    let server = spawn_wiremock_jwks(test_kid(), &kp.n_b64, &kp.e_b64, 200).await;
    (server, test_kid())
}
