//! Pure-Rust ES256 (P-256) WebAuthn authenticator for E2E tests.
//!
//! Replaces `webauthn-authenticator-rs::SoftToken`, which pulled openssl
//! in transitively via `webauthn-rs-core`. This simulator produces real
//! attestation objects and assertion signatures the same way a platform
//! authenticator (Touch ID / Windows Hello) does, so the full server-side
//! verification path in `passkey-auth` is exercised.
//!
//! ES256 is chosen over Ed25519 because it is the dominant passkey
//! algorithm — every platform authenticator implements COSE alg -7
//! (ECDSA with SHA-256 over P-256), so this mirrors production traffic.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64URL;
use ciborium::Value as CborValue;
use p256::ecdsa::signature::RandomizedSigner;
use p256::ecdsa::{Signature, SigningKey};
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};

const FLAG_UP: u8 = 1 << 0; // userPresent
const FLAG_UV: u8 = 1 << 2; // userVerified
const FLAG_AT: u8 = 1 << 6; // attestedCredentialData present

/// Synthetic P-256 authenticator. Holds one credential; the sign counter
/// increments on every assertion. Flags default to UP|UV so ceremonies
/// pass the user-verification check.
pub struct Es256Authenticator {
    signing_key: SigningKey,
    credential_id: Vec<u8>,
    counter: u32,
    flags: u8,
    rp_id: String,
}

impl Es256Authenticator {
    pub fn new() -> Self {
        let signing_key = SigningKey::random(&mut OsRng);
        let mut credential_id = vec![0u8; 32];
        rand::RngCore::fill_bytes(&mut OsRng, &mut credential_id);
        Self {
            signing_key,
            credential_id,
            counter: 0,
            flags: FLAG_UP | FLAG_UV,
            rp_id: String::new(),
        }
    }

    /// COSE_Key for a P-256 public key (EC2, alg -7 / ES256).
    /// Shape: {1: 2 (kty=EC2), 3: -7 (alg=ES256), -1: 1 (crv=P-256),
    ///         -2: <x 32B>, -3: <y 32B>}.
    fn cose_public_key(&self) -> Vec<u8> {
        // Encoded point: 0x04 (uncompressed) || X(32) || Y(32).
        let point = self.signing_key.verifying_key().to_encoded_point(false);
        let bytes = point.as_bytes();
        let x = bytes[1..33].to_vec();
        let y = bytes[33..65].to_vec();
        let map = CborValue::Map(vec![
            (int(1), int(2)),          // kty = EC2
            (int(3), int(-7)),         // alg = ES256
            (int(-1), int(1)),         // crv = P-256
            (int(-2), bytes_value(x)), // x
            (int(-3), bytes_value(y)), // y
        ]);
        encode_cbor(&map)
    }

    fn rp_id_hash(&self) -> [u8; 32] {
        Sha256::digest(self.rp_id.as_bytes()).into()
    }

    /// authenticatorData for registration (AT flag set + attested cred data).
    fn auth_data_register(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.rp_id_hash());
        buf.push(self.flags | FLAG_AT);
        buf.extend_from_slice(&self.counter.to_be_bytes());
        // attestedCredentialData: aaguid (16B zero) + credId len + credId + COSE key.
        buf.extend_from_slice(&[0u8; 16]);
        let cid_len = self.credential_id.len() as u16;
        buf.extend_from_slice(&cid_len.to_be_bytes());
        buf.extend_from_slice(&self.credential_id);
        buf.extend_from_slice(&self.cose_public_key());
        buf
    }

    /// authenticatorData for assertion (no AT flag). Increments the counter.
    fn auth_data_assert(&mut self) -> Vec<u8> {
        self.counter += 1;
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.rp_id_hash());
        buf.push(self.flags);
        buf.extend_from_slice(&self.counter.to_be_bytes());
        buf
    }

    /// attestationObject with fmt=none wrapping the registration authData.
    fn attestation_object(&self) -> Vec<u8> {
        let map = CborValue::Map(vec![
            (text("fmt"), text("none")),
            (text("attStmt"), CborValue::Map(vec![])),
            (text("authData"), bytes_value(self.auth_data_register())),
        ]);
        encode_cbor(&map)
    }

    /// ES256 signature over authData || SHA-256(clientDataJSON), as an
    /// authenticator produces for assertion. WebAuthn ES256 signatures
    /// are DER-encoded ECDSA (ASN.1), matching what every browser emits
    /// and what `passkey-auth` parses with `Signature::from_der`.
    fn sign_assertion(&self, auth_data: &[u8], client_data_json_raw: &[u8]) -> Vec<u8> {
        let cdj_hash = Sha256::digest(client_data_json_raw);
        let mut msg = Vec::with_capacity(auth_data.len() + 32);
        msg.extend_from_slice(auth_data);
        msg.extend_from_slice(&cdj_hash);
        let sig: Signature = self.signing_key.sign_with_rng(&mut OsRng, &msg);
        sig.to_der().to_bytes().to_vec()
    }

    // ---- public ceremony helpers ------------------------------------------

    /// Produce a `RegistrationResponse` the server can verify, given the
    /// challenge JSON (`passkey_auth::RegistrationChallenge` shape) and
    /// the origin the browser would report.
    pub fn register(&mut self, options: &serde_json::Value, origin: &str) -> serde_json::Value {
        self.rp_id = options["rp"]["id"]
            .as_str()
            .unwrap_or("localhost")
            .to_string();
        let challenge_b64 = options["challenge"].as_str().expect("challenge field");
        let (_cdj_raw, cdj_b64) = client_data("webauthn.create", challenge_b64, origin);
        let attestation = B64URL.encode(self.attestation_object());
        serde_json::json!({
            "id": B64URL.encode(&self.credential_id),
            "type": "public-key",
            "transports": ["internal"],
            "attestationObject": attestation,
            "clientDataJSON": cdj_b64,
        })
    }

    /// Produce an `AuthenticationResponse` the server can verify, given
    /// the challenge JSON (`passkey_auth::AuthenticationChallenge` shape)
    /// and the origin the browser would report.
    pub fn authenticate(&mut self, options: &serde_json::Value, origin: &str) -> serde_json::Value {
        if let Some(rp_id) = options["rpId"].as_str() {
            self.rp_id = rp_id.to_string();
        }
        let challenge_b64 = options["challenge"].as_str().expect("challenge field");
        let auth_data = self.auth_data_assert();
        let (cdj_raw, cdj_b64) = client_data("webauthn.get", challenge_b64, origin);
        let sig = self.sign_assertion(&auth_data, &cdj_raw);
        serde_json::json!({
            "id": B64URL.encode(&self.credential_id),
            "type": "public-key",
            "rawId": B64URL.encode(&self.credential_id),
            "authenticatorData": B64URL.encode(&auth_data),
            "clientDataJSON": cdj_b64,
            "signature": B64URL.encode(&sig),
        })
    }
}

impl Default for Es256Authenticator {
    fn default() -> Self {
        Self::new()
    }
}

// ---- CBOR / clientData helpers -------------------------------------------

fn client_data(kind: &str, challenge_b64: &str, origin: &str) -> (Vec<u8>, String) {
    let json = format!(
        r#"{{"type":"{kind}","challenge":"{challenge_b64}","origin":"{origin}","crossOrigin":false}}"#
    );
    let raw = json.into_bytes();
    let enc = B64URL.encode(&raw);
    (raw, enc)
}

fn int(i: i64) -> CborValue {
    CborValue::Integer(i.into())
}
fn text(s: &str) -> CborValue {
    CborValue::Text(s.to_string())
}
fn bytes_value(b: Vec<u8>) -> CborValue {
    CborValue::Bytes(b)
}
fn encode_cbor(value: &CborValue) -> Vec<u8> {
    let mut out = Vec::new();
    ciborium::ser::into_writer(value, &mut out).expect("CBOR encode should succeed");
    out
}
