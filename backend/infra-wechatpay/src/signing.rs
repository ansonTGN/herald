//! WeChat Pay v3 request signing, callback signature verification, and
//! callback / certificate decryption.
//!
//! - Request signing & JSAPI `paySign`: RSA-SHA256 with the **merchant
//!   private key** (self-built here; the historical implementation relied on
//!   the now-forbidden SDK for this — DEC-wechat-support-004).
//! - Callback verification: RSA-SHA256 with the **platform public key** (port
//!   of the historical verified `verify_signature`).
//! - Resource / certificate decryption: AES-256-GCM with the **APIv3 Key**
//!   (port of the historical verified `decrypt_resource_data`).

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use rsa::pkcs1::{DecodeRsaPrivateKey, DecodeRsaPublicKey};
use rsa::pkcs1v15::Pkcs1v15Sign;
use rsa::pkcs8::{DecodePrivateKey, DecodePublicKey};
use rsa::{RsaPrivateKey, RsaPublicKey};
use sha2::{Digest, Sha256};

use crate::error::WechatPayError;

/// Authorization scheme prefix used by WeChat Pay v3.
pub const AUTH_SCHEME: &str = "WECHATPAY2-SHA256-RSA2048";

/// Sign a v3 request and return the full `Authorization` header value.
///
/// Per the WeChat Pay v3 spec, the message to sign is:
/// `{method}\n{url_with_query}\n{timestamp}\n{nonce}\n{body}\n`
/// where `url_with_query` is the path + query (no host), `body` is the JSON
/// body for POST or the empty string for GET, and the signature is RSA-SHA256
/// with the merchant private key, base64-encoded.
#[allow(clippy::too_many_arguments)]
pub fn build_authorization_header(
    private_key: &RsaPrivateKey,
    mch_id: &str,
    serial_no: &str,
    method: &str,
    url_with_query: &str,
    body: &str,
    timestamp: &str,
    nonce: &str,
) -> Result<String, WechatPayError> {
    let signature = sign_message(private_key, method, url_with_query, timestamp, nonce, body)?;
    Ok(format!(
        "{AUTH_SCHEME} mchid=\"{mch_id}\",nonce_str=\"{nonce}\",timestamp=\"{timestamp}\",serial_no=\"{serial_no}\",signature=\"{signature}\""
    ))
}

/// Sign the canonical request message with the merchant private key.
fn sign_message(
    key: &RsaPrivateKey,
    method: &str,
    url_with_query: &str,
    timestamp: &str,
    nonce: &str,
    body: &str,
) -> Result<String, WechatPayError> {
    let message = format!("{method}\n{url_with_query}\n{timestamp}\n{nonce}\n{body}\n");
    let digest = Sha256::digest(message.as_bytes());
    let sig = key.sign(Pkcs1v15Sign::new::<Sha256>(), &digest)?;
    Ok(STANDARD.encode(sig))
}

/// Build the JSAPI `paySign` for the `WeixinJSBridge` invocation params.
///
/// Message: `{appId}\n{timeStamp}\n{nonceStr}\n{package}\n` signed with the
/// merchant private key (RSA-SHA256), base64-encoded.
pub fn sign_jsapi_params(
    key: &RsaPrivateKey,
    app_id: &str,
    time_stamp: &str,
    nonce_str: &str,
    package: &str,
) -> Result<String, WechatPayError> {
    let message = format!("{app_id}\n{time_stamp}\n{nonce_str}\n{package}\n");
    let digest = Sha256::digest(message.as_bytes());
    let sig = key.sign(Pkcs1v15Sign::new::<Sha256>(), &digest)?;
    Ok(STANDARD.encode(sig))
}

/// Verify a callback (or response) signature using the platform public key.
///
/// `message` is `{timestamp}\n{nonce}\n{body}\n` (body is the raw request
/// body). `signature_b64` is the base64-encoded RSA-SHA256 signature from the
/// `Wechatpay-Signature` header.
pub fn verify_callback_signature(
    platform_public_key_pem: &str,
    message: &str,
    signature_b64: &str,
) -> Result<(), WechatPayError> {
    let decoded_signature = STANDARD.decode(signature_b64)?;
    // Accept both SPKI (`from_public_key_pem`) and PKCS#1 (`from_pkcs1_pem`)
    // encodings — WeChat platform certs are X.509 SPKI, but the manual
    // override may be either form.
    let public_key = RsaPublicKey::from_public_key_pem(platform_public_key_pem)
        .or_else(|_| RsaPublicKey::from_pkcs1_pem(platform_public_key_pem))
        .map_err(|_| WechatPayError::ConfigInvalid("invalid platform public key".into()))?;
    let digest = Sha256::digest(message.as_bytes());
    public_key
        .verify(Pkcs1v15Sign::new::<Sha256>(), &digest, &decoded_signature)
        .map_err(|_| WechatPayError::SignatureInvalid)
}

/// Decrypt a callback resource or platform certificate: AES-256-GCM with the
/// APIv3 Key, a 12-byte nonce, and `associated_data` as AAD.
pub fn decrypt_aes_gcm(
    ciphertext_b64: &str,
    associated_data: &str,
    nonce: &str,
    api_v3_key: &str,
) -> Result<String, WechatPayError> {
    use aes_gcm::aead::{Aead, KeyInit, Payload};

    let ciphertext_bytes = STANDARD.decode(ciphertext_b64)?;
    let nonce_bytes = nonce.as_bytes();
    if nonce_bytes.len() != 12 {
        return Err(WechatPayError::InvalidNonceLength(nonce_bytes.len()));
    }

    let cipher = aes_gcm::Aes256Gcm::new_from_slice(api_v3_key.as_bytes())
        .map_err(|_| WechatPayError::DecryptFailed)?;
    let nonce = aes_gcm::Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(
            nonce,
            Payload {
                msg: ciphertext_bytes.as_slice(),
                aad: associated_data.as_bytes(),
            },
        )
        .map_err(|_| WechatPayError::DecryptFailed)?;

    Ok(String::from_utf8(plaintext)?)
}

/// Parse the merchant private key from a PKCS#8 or PKCS#1 PEM. Callers cache
/// the result (`WechatPayClient::signing_key`) instead of re-parsing per
/// signature.
pub fn parse_private_key(private_key_pem: &str) -> Result<RsaPrivateKey, WechatPayError> {
    RsaPrivateKey::from_pkcs8_pem(private_key_pem)
        .or_else(|_| RsaPrivateKey::from_pkcs1_pem(private_key_pem))
        .map_err(|e| WechatPayError::ConfigInvalid(format!("invalid merchant private key: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    // 2048-bit RSA keypair generated for tests only (not used anywhere else).
    const TEST_PRIVATE_KEY_PEM: &str = include_str!("../tests/test_private_key.pem");

    fn public_key_pem() -> String {
        use rsa::pkcs8::EncodePublicKey;
        let key = RsaPrivateKey::from_pkcs8_pem(TEST_PRIVATE_KEY_PEM).unwrap();
        rsa::RsaPublicKey::from(&key)
            .to_public_key_pem(rsa::pkcs8::LineEnding::LF)
            .unwrap()
    }

    fn test_key() -> RsaPrivateKey {
        parse_private_key(TEST_PRIVATE_KEY_PEM).unwrap()
    }

    #[test]
    fn request_signature_round_trips() {
        let pub_pem = public_key_pem();
        let auth = build_authorization_header(
            &test_key(),
            "1900000109",
            "abc_serial",
            "POST",
            "/v3/pay/transactions/native",
            "{\"foo\":1}",
            "1700000000",
            "nonce-xyz",
        )
        .unwrap();
        assert!(auth.starts_with("WECHATPAY2-SHA256-RSA2048 mchid=\"1900000109\""));
        // Extract the base64 signature from the header.
        let sig = auth
            .split("signature=\"")
            .nth(1)
            .and_then(|s| s.strip_suffix('"'))
            .expect("signature field present");
        let message = "POST\n/v3/pay/transactions/native\n1700000000\nnonce-xyz\n{\"foo\":1}\n";
        verify_callback_signature(&pub_pem, message, sig).expect("round-trip verifies");
    }

    #[test]
    fn callback_signature_rejects_tampered_body() {
        let pub_pem = public_key_pem();
        let auth = build_authorization_header(
            &test_key(),
            "m",
            "s",
            "POST",
            "/v3/pay/transactions/native",
            "body-a",
            "ts",
            "nn",
        )
        .unwrap();
        let sig = auth
            .split("signature=\"")
            .nth(1)
            .unwrap()
            .strip_suffix('"')
            .unwrap();
        let good = "POST\n/v3/pay/transactions/native\nts\nnn\nbody-a\n";
        verify_callback_signature(&pub_pem, good, sig).expect("original verifies");
        let tampered = "POST\n/v3/pay/transactions/native\nts\nnn\nbody-b\n";
        let err = verify_callback_signature(&pub_pem, tampered, sig).unwrap_err();
        assert!(matches!(err, WechatPayError::SignatureInvalid));
    }

    #[test]
    fn aes_gcm_decrypt_round_trips() {
        use aes_gcm::aead::{Aead, KeyInit, Payload};
        let key = b"0123456789abcdef0123456789abcdef"; // 32 bytes
        let nonce_bytes = b"123456789012"; // 12 bytes
        let cipher = aes_gcm::Aes256Gcm::new_from_slice(key).unwrap();
        let ct = cipher
            .encrypt(
                aes_gcm::Nonce::from_slice(nonce_bytes),
                Payload {
                    msg: b"{\"out_trade_no\":\"CAS_ab_x\"}",
                    aad: b"transaction",
                },
            )
            .unwrap();
        let ct_b64 = STANDARD.encode(&ct);
        let plain = decrypt_aes_gcm(
            &ct_b64,
            "transaction",
            "123456789012",
            std::str::from_utf8(key).unwrap(),
        )
        .unwrap();
        assert_eq!(plain, "{\"out_trade_no\":\"CAS_ab_x\"}");
    }

    #[test]
    fn aes_gcm_rejects_bad_nonce_length() {
        let err =
            decrypt_aes_gcm("AAAA", "ad", "short", "0123456789abcdef0123456789abcdef").unwrap_err();
        assert!(matches!(err, WechatPayError::InvalidNonceLength(5)));
    }

    #[test]
    fn jsapi_pay_sign_round_trips() {
        let pub_pem = public_key_pem();
        let pay_sign = sign_jsapi_params(
            &test_key(),
            "wx123",
            "1700000000",
            "nonce-str",
            "prepay_id=wx2026abc",
        )
        .unwrap();
        // Verify the JSAPI signing message format.
        let message = "wx123\n1700000000\nnonce-str\nprepay_id=wx2026abc\n";
        verify_callback_signature(&pub_pem, message, &pay_sign).expect("jsapi paySign verifies");
    }
}
