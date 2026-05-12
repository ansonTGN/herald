use herald_core::domain::common::entities::app_errors::CoreError;
use hmac::{Hmac, Mac};
use sha2::Sha256;

/// Verify Creem webhook signature
///
/// # Arguments
///
/// * `payload` - Raw request body bytes
/// * `signature_header` - Value of `creem-signature` header (hex-encoded HMAC digest)
/// * `webhook_secret` - Webhook secret from Creem dashboard
///
/// # Returns
///
/// * `Ok(())` if signature is valid
/// * `Err(CoreError)` if signature doesn't match or signature format is invalid
///
/// # Security
///
/// - Uses HMAC-SHA256 for signature verification
/// - Uses constant-time comparison to prevent timing attacks
pub fn verify_webhook_signature(
    payload: &[u8],
    signature_header: &str,
    webhook_secret: &str,
) -> Result<(), CoreError> {
    // Compute HMAC-SHA256 on the raw request body.
    let mut mac = Hmac::<Sha256>::new_from_slice(webhook_secret.as_bytes()).map_err(|_| {
        tracing::error!("Invalid webhook secret key");
        CoreError::InvalidWebhookSecret
    })?;

    mac.update(payload);

    let received_signature_bytes =
        hex::decode(signature_header).map_err(|_| CoreError::InvalidWebhookSignature)?;

    mac.verify_slice(&received_signature_bytes)
        .map_err(|_| CoreError::InvalidWebhookSignature)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_webhook_signature_valid() {
        let secret = "whsec_test_secret";
        let payload = br#"{"id":"evt_123","type":"subscription.paid"}"#;

        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(payload);
        let signature = hex::encode(mac.finalize().into_bytes());

        let result = verify_webhook_signature(payload, &signature, secret);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_webhook_signature_invalid() {
        let secret = "whsec_test_secret";
        let payload = br#"{"id":"evt_123","type":"subscription.paid"}"#;
        let result = verify_webhook_signature(payload, "invalid_signature", secret);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_webhook_signature_malformed() {
        let secret = "whsec_test_secret";
        let payload = br#"{"id":"evt_123","type":"subscription.paid"}"#;
        let signature_header = "t=123,v1=abc";

        let result = verify_webhook_signature(payload, signature_header, secret);
        assert!(result.is_err());
    }
}
