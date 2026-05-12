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
        // 开发环境应该有 Domain=localhost 以支持代理
        assert!(
            cookie.contains("Domain=localhost"),
            "Development cookie should have Domain=localhost for Vite proxy support"
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
        // 清除 cookie 也需要 Domain 以匹配
        assert!(
            cookie.contains("Domain=localhost"),
            "Development clear cookie should have Domain=localhost"
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

    #[test]
    fn test_build_set_cookie_format() {
        let cookie = build_set_cookie("Test-Cookie", "value123", 3600, false);

        // 检查顺序和格式
        let parts: Vec<&str> = cookie.split("; ").collect();

        // 第一部分应该是 name=value
        assert!(parts[0].starts_with("Test-Cookie=value123"));

        // 检查所有必需的属性存在
        assert!(parts.iter().any(|p| p.contains("Path=/")));
        assert!(parts.iter().any(|p| p.contains("Max-Age=3600")));
        assert!(parts.iter().any(|p| p.contains("HttpOnly")));
        assert!(parts.iter().any(|p| p.contains("SameSite=Lax")));
    }

    #[test]
    fn test_build_set_cookie_with_special_chars() {
        // 测试包含特殊字符的 token 值
        let cookie = build_set_cookie("X-Auth", "realm-123_uuid-456_1234567890", 1800, false);

        assert!(cookie.contains("X-Auth=realm-123_uuid-456_1234567890"));
        assert!(cookie.contains("Max-Age=1800"));
    }

    #[test]
    fn test_build_set_cookie_zero_max_age() {
        let cookie = build_set_cookie("X-Auth", "value", 0, false);

        assert!(cookie.contains("Max-Age=0"));
        assert!(!cookie.contains("Secure"));
    }

    #[test]
    fn test_build_set_cookie_negative_max_age() {
        let cookie = build_set_cookie("X-Auth", "value", -100, false);

        assert!(cookie.contains("Max-Age=-100"));
    }

    #[test]
    fn test_different_cookie_names() {
        let cookie1 = build_set_cookie("Session-ID", "abc123", 1800, true);
        let cookie2 = build_set_cookie("Auth-Token", "xyz789", 3600, true);

        assert!(cookie1.contains("Session-ID=abc123"));
        assert!(cookie2.contains("Auth-Token=xyz789"));
        assert!(cookie1.contains("Secure"));
        assert!(cookie2.contains("Secure"));
    }
}
