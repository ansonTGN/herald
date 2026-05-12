#[cfg(test)]
mod tests {
    use validator::Validate;
    #[test]
    fn test_register_request_validation() {
        #[derive(Debug, serde::Deserialize, serde::Serialize, validator::Validate)]
        pub struct TestRegisterRequest {
            #[validate(length(min = 1, max = 36))]
            pub realm_id: String,
            #[validate(email)]
            pub email: String,
            #[validate(length(min = 1, max = 36))]
            pub username: Option<String>,
            #[validate(length(min = 8, max = 36))]
            pub password: String,
            #[validate(length(min = 1))]
            pub turnstile_token: String,
        }

        // 1. 测试有效请求
        let valid_request = TestRegisterRequest {
            realm_id: "test-realm-id".to_string(),
            email: "test@example.com".to_string(),
            username: Some("testuser".to_string()),
            password: "password123".to_string(),
            turnstile_token: "token".to_string(),
        };
        assert!(valid_request.validate().is_ok());

        let invalid_realm = TestRegisterRequest {
            realm_id: "".to_string(),
            email: "test@example.com".to_string(),
            username: Some("testuser".to_string()),
            password: "password123".to_string(),
            turnstile_token: "token".to_string(),
        };
        assert!(invalid_realm.validate().is_err());

        let invalid_email = TestRegisterRequest {
            realm_id: "test-realm-id".to_string(),
            email: "invalid-email".to_string(),
            username: Some("testuser".to_string()),
            password: "password123".to_string(),
            turnstile_token: "token".to_string(),
        };
        assert!(invalid_email.validate().is_err());

        let short_password = TestRegisterRequest {
            realm_id: "test-realm-id".to_string(),
            email: "test@example.com".to_string(),
            username: Some("testuser".to_string()),
            password: "pass".to_string(),
            turnstile_token: "token".to_string(),
        };
        assert!(short_password.validate().is_err());

        let long_password = TestRegisterRequest {
            realm_id: "test-realm-id".to_string(),
            email: "test@example.com".to_string(),
            username: Some("testuser".to_string()),
            password: "01234567890123456789012345678901234567".to_string(),
            turnstile_token: "token".to_string(),
        };
        assert!(long_password.validate().is_err());

        let long_realm_id = TestRegisterRequest {
            realm_id: "01234567890123456789012345678901234567".to_string(),
            email: "test@example.com".to_string(),
            username: Some("testuser".to_string()),
            password: "password123".to_string(),
            turnstile_token: "token".to_string(),
        };
        assert!(long_realm_id.validate().is_err());

        let long_username = TestRegisterRequest {
            realm_id: "test-realm-id".to_string(),
            email: "test@example.com".to_string(),
            username: Some("01234567890123456789012345678901234567".to_string()),
            password: "password123".to_string(),
            turnstile_token: "token".to_string(),
        };
        assert!(long_username.validate().is_err());

        println!("✅ RegisterRequest 验证测试通过！");
    }

    #[test]
    fn test_login_request_validation() {
        #[derive(Debug, serde::Deserialize, serde::Serialize, validator::Validate)]
        pub struct TestLoginRequest {
            #[validate(length(min = 1, max = 36))]
            pub realm_id: String,
            #[validate(length(min = 1, max = 36))]
            pub client_id: String,
            #[validate(email)]
            pub email: String,
            #[validate(length(min = 8, max = 36))]
            pub password: String,
        }

        let valid = TestLoginRequest {
            realm_id: "realm".to_string(),
            client_id: "client".to_string(),
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
        };
        assert!(valid.validate().is_ok());

        let short_pwd = TestLoginRequest {
            realm_id: "realm".to_string(),
            client_id: "client".to_string(),
            email: "test@example.com".to_string(),
            password: "pass".to_string(),
        };
        assert!(short_pwd.validate().is_err());

        let long_pwd = TestLoginRequest {
            realm_id: "realm".to_string(),
            client_id: "client".to_string(),
            email: "test@example.com".to_string(),
            password: "01234567890123456789012345678901234567".to_string(),
        };
        assert!(long_pwd.validate().is_err());

        let invalid_email = TestLoginRequest {
            realm_id: "realm".to_string(),
            client_id: "client".to_string(),
            email: "not-an-email".to_string(),
            password: "password123".to_string(),
        };
        assert!(invalid_email.validate().is_err());

        println!("✅ LoginRequest 验证测试通过！");
    }

    #[test]
    fn test_change_password_request_validation() {
        #[derive(Debug, serde::Deserialize, serde::Serialize, validator::Validate)]
        pub struct TestChangePasswordRequest {
            #[validate(length(min = 1, max = 36))]
            pub old_pass: String,
            #[validate(length(min = 8, max = 36))]
            pub new_pass: String,
        }

        let valid = TestChangePasswordRequest {
            old_pass: "oldpass123".to_string(),
            new_pass: "newpass123".to_string(),
        };
        assert!(valid.validate().is_ok());

        let short = TestChangePasswordRequest {
            old_pass: "oldpass123".to_string(),
            new_pass: "short".to_string(),
        };
        assert!(short.validate().is_err());

        let long = TestChangePasswordRequest {
            old_pass: "oldpass123".to_string(),
            new_pass: "01234567890123456789012345678901234567".to_string(),
        };
        assert!(long.validate().is_err());

        let old_long = TestChangePasswordRequest {
            old_pass: "01234567890123456789012345678901234567".to_string(),
            new_pass: "newpass123".to_string(),
        };
        assert!(old_long.validate().is_err());

        println!("✅ ChangePasswordRequest 验证测试通过！");
    }

    #[test]
    fn test_boundary_values() {
        #[derive(Debug, serde::Deserialize, serde::Serialize, validator::Validate)]
        pub struct TestRequest {
            #[validate(length(min = 8, max = 36))]
            pub password: String,
        }

        let min_valid = TestRequest {
            password: "12345678".to_string(),
        };
        assert!(min_valid.validate().is_ok());

        let min_invalid = TestRequest {
            password: "1234567".to_string(),
        };
        assert!(min_invalid.validate().is_err());

        let max_valid = TestRequest {
            password: "012345678901234567890123456789012345".to_string(),
        };
        assert!(max_valid.validate().is_ok());

        let max_invalid = TestRequest {
            password: "0123456789012345678901234567890123456".to_string(),
        };
        assert!(max_invalid.validate().is_err());

        println!("✅ 边界值测试通过！");
    }
}
