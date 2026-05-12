pub mod helpers;
pub mod scenarios;
pub mod schema_test_context;
pub mod shared;

// =============================================================================
// 测试辅助函数
// =============================================================================

use axum::body::to_bytes;

/// 从 HTTP 响应中解析 JSON
pub async fn response_json<T>(response: axum::response::Response) -> T
where
    T: serde::de::DeserializeOwned,
{
    let status = response.status();
    eprintln!("Response Status: {}", status);
    let (_parts, body) = response.into_parts();
    let body_bytes = to_bytes(body, usize::MAX).await.unwrap();
    let body_str = std::str::from_utf8(&body_bytes).unwrap();
    // 打印响应内容以便调试
    eprintln!("Response JSON: {}", body_str);
    serde_json::from_slice(&body_bytes).unwrap()
}

/// 从 Set-Cookie 头中提取指定的 token
pub fn extract_set_cookie_token(set_cookie: &str, name: &str) -> Option<String> {
    let prefix = format!("{name}=");
    let rest = set_cookie.strip_prefix(&prefix)?;
    Some(rest.split(';').next()?.to_string())
}
