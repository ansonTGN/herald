/// Unit tests: Shopify webhook utility functions
#[cfg(test)]
mod tests {
    use crate::shopify_webhook_utils::*;
    use base64::{Engine, engine::general_purpose::STANDARD as BASE64_STANDARD};
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    #[test]
    fn test_unit_valid_hmac_verification() {
        let client_secret = "test_client_secret";
        let body = b"test_body";

        let mut mac = HmacSha256::new_from_slice(client_secret.as_bytes()).unwrap();
        mac.update(body);
        let calculated_hmac = mac.finalize().into_bytes();
        let valid_hmac = BASE64_STANDARD.encode(calculated_hmac);

        assert!(verify_webhook_hmac(body, &valid_hmac, client_secret).is_ok());
    }

    #[test]
    fn test_unit_invalid_hmac_rejected() {
        let client_secret = "test_client_secret";
        let body = b"test_body";
        let invalid_hmac = "invalid_hmac_signature";

        assert!(matches!(
            verify_webhook_hmac(body, invalid_hmac, client_secret),
            Err(herald_core::domain::common::entities::app_errors::CoreError::Unauthorized)
        ));
    }

    #[test]
    fn test_unit_tampered_body_rejected() {
        let client_secret = "test_client_secret";
        let original_body = b"original_body";

        let mut mac = HmacSha256::new_from_slice(client_secret.as_bytes()).unwrap();
        mac.update(original_body);
        let calculated_hmac = mac.finalize().into_bytes();
        let valid_hmac = BASE64_STANDARD.encode(calculated_hmac);

        let tampered_body = b"tampered_body";

        assert!(matches!(
            verify_webhook_hmac(tampered_body, &valid_hmac, client_secret),
            Err(herald_core::domain::common::entities::app_errors::CoreError::Unauthorized)
        ));
    }

    #[test]
    fn test_unit_empty_body_hmac() {
        let client_secret = "test_client_secret";
        let body = b"";

        let mut mac = HmacSha256::new_from_slice(client_secret.as_bytes()).unwrap();
        mac.update(body);
        let calculated_hmac = mac.finalize().into_bytes();
        let valid_hmac = BASE64_STANDARD.encode(calculated_hmac);

        assert!(verify_webhook_hmac(body, &valid_hmac, client_secret).is_ok());
    }

    #[test]
    fn test_unit_special_characters_hmac() {
        let client_secret = "test_client_secret";
        let body = b"{\"data\": \"test!@#$%^&*()_+{}|:<>?~`-=[]\\\\;',./\"}";

        let mut mac = HmacSha256::new_from_slice(client_secret.as_bytes()).unwrap();
        mac.update(body);
        let calculated_hmac = mac.finalize().into_bytes();
        let valid_hmac = BASE64_STANDARD.encode(calculated_hmac);

        assert!(verify_webhook_hmac(body, &valid_hmac, client_secret).is_ok());
    }

    #[test]
    fn test_unit_constant_time_compare() {
        let client_secret = "test_client_secret";
        let body = b"test_body";
        let different_hmac = "different_hmac_signature";

        assert!(verify_webhook_hmac(body, different_hmac, client_secret).is_err());

        let mut mac = HmacSha256::new_from_slice(client_secret.as_bytes()).unwrap();
        mac.update(body);
        let calculated_hmac = mac.finalize().into_bytes();
        let exact_hmac = BASE64_STANDARD.encode(calculated_hmac);

        assert!(verify_webhook_hmac(body, &exact_hmac, client_secret).is_ok());
    }

    #[test]
    fn test_unit_different_length_hmac_rejected() {
        let client_secret = "test_client_secret";
        let body = b"test_body";
        let short_hmac = "short";

        assert!(matches!(
            verify_webhook_hmac(body, short_hmac, client_secret),
            Err(herald_core::domain::common::entities::app_errors::CoreError::Unauthorized)
        ));
    }

    #[test]
    fn test_unit_base64_encoded_hmac() {
        let client_secret = "test_client_secret";
        let body = b"test_body_for_base64";

        let mut mac = HmacSha256::new_from_slice(client_secret.as_bytes()).unwrap();
        mac.update(body);
        let hmac_bytes = mac.finalize().into_bytes();
        let base64_hmac = BASE64_STANDARD.encode(hmac_bytes);

        assert!(verify_webhook_hmac(body, &base64_hmac, client_secret).is_ok());

        let raw_hmac = format!("{:x}", hmac_bytes);
        assert!(verify_webhook_hmac(body, &raw_hmac, client_secret).is_err());
    }

    #[test]
    fn test_unit_unicode_hmac() {
        let client_secret = "test_client_secret";
        let body = "Hello World!".as_bytes();

        let mut mac = HmacSha256::new_from_slice(client_secret.as_bytes()).unwrap();
        mac.update(body);
        let calculated_hmac = mac.finalize().into_bytes();
        let valid_hmac = BASE64_STANDARD.encode(calculated_hmac);

        assert!(verify_webhook_hmac(body, &valid_hmac, client_secret).is_ok());
    }

    #[test]
    fn test_unit_long_body_hmac() {
        let client_secret = "test_client_secret";
        let long_body = "a".repeat(10000);
        let body = long_body.as_bytes();

        let mut mac = HmacSha256::new_from_slice(client_secret.as_bytes()).unwrap();
        mac.update(body);
        let calculated_hmac = mac.finalize().into_bytes();
        let valid_hmac = BASE64_STANDARD.encode(calculated_hmac);

        assert!(verify_webhook_hmac(body, &valid_hmac, client_secret).is_ok());
    }

    #[test]
    fn test_unit_hmac_key_case_sensitive() {
        let body = b"test_body";

        let lowercase_secret = "secret";
        let mut mac1 = HmacSha256::new_from_slice(lowercase_secret.as_bytes()).unwrap();
        mac1.update(body);
        let hmac1 = BASE64_STANDARD.encode(mac1.finalize().into_bytes());

        let uppercase_secret = "SECRET";
        let mut mac2 = HmacSha256::new_from_slice(uppercase_secret.as_bytes()).unwrap();
        mac2.update(body);
        let hmac2 = BASE64_STANDARD.encode(mac2.finalize().into_bytes());

        assert_ne!(hmac1, hmac2);

        assert!(verify_webhook_hmac(body, &hmac1, lowercase_secret).is_ok());
        assert!(verify_webhook_hmac(body, &hmac1, uppercase_secret).is_err());
    }

    #[test]
    fn test_unit_hmac_uniqueness() {
        let client_secret = "test_client_secret";

        let body1 = b"body1";
        let mut mac1 = HmacSha256::new_from_slice(client_secret.as_bytes()).unwrap();
        mac1.update(body1);
        let hmac1 = BASE64_STANDARD.encode(mac1.finalize().into_bytes());

        let body2 = b"body2";
        let mut mac2 = HmacSha256::new_from_slice(client_secret.as_bytes()).unwrap();
        mac2.update(body2);
        let hmac2 = BASE64_STANDARD.encode(mac2.finalize().into_bytes());

        assert_ne!(hmac1, hmac2);

        assert!(verify_webhook_hmac(body1, &hmac1, client_secret).is_ok());
        assert!(verify_webhook_hmac(body2, &hmac2, client_secret).is_ok());

        assert!(verify_webhook_hmac(body1, &hmac2, client_secret).is_err());
        assert!(verify_webhook_hmac(body2, &hmac1, client_secret).is_err());
    }
}
