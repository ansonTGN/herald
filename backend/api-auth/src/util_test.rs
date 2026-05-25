#[cfg(test)]
mod tests {
    use herald_api_base::application::http::auth::util::{build_clear_cookie, build_set_cookie};

    #[test]
    fn test_build_set_cookie_development() {
        let cookie = build_set_cookie("X-Auth", "test_token_value", 1800, false);

        // Development 环境不应该有 Secure 标志
        assert!(
            !cookie.contains("Secure"),
            "Development cookie should not have Secure flag"
        );
        assert!(cookie.contains("X-Auth=test_token_value"));
        assert!(cookie.contains("Max-Age=1800"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Lax"));
        assert!(cookie.contains("Path=/"));
        assert!(
            !cookie.contains("Domain="),
            "Development cookie should be host-only so 127.0.0.1 and localhost both work"
        );
    }

    #[test]
    fn test_build_set_cookie_production() {
        let cookie = build_set_cookie("X-Auth", "test_token_value", 1800, true);

        // Production 环境应该有 Secure 标志
        assert!(
            cookie.contains("Secure"),
            "Production cookie should have Secure flag"
        );
        assert!(cookie.contains("X-Auth=test_token_value"));
        assert!(cookie.contains("Max-Age=1800"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Lax"));
        assert!(cookie.contains("Path=/"));
    }

    #[test]
    fn test_build_clear_cookie_development() {
        let cookie = build_clear_cookie("X-Auth", false);

        // Development 环境不应该有 Secure 标志
        assert!(
            !cookie.contains("Secure"),
            "Development clear cookie should not have Secure flag"
        );
        assert!(cookie.contains("X-Auth="));
        assert!(cookie.contains("Max-Age=0"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Lax"));
        assert!(cookie.contains("Path=/"));
        assert!(
            !cookie.contains("Domain="),
            "Development clear cookie should be host-only"
        );
    }

    #[test]
    fn test_build_clear_cookie_production() {
        let cookie = build_clear_cookie("X-Auth", true);

        // Production 环境应该有 Secure 标志
        assert!(
            cookie.contains("Secure"),
            "Production clear cookie should have Secure flag"
        );
        assert!(cookie.contains("X-Auth="));
        assert!(cookie.contains("Max-Age=0"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Lax"));
        assert!(cookie.contains("Path=/"));
    }
}
