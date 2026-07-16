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

/// Read a browser access token from a successful JSON response while preserving
/// the response for the caller's existing status/body assertions.
pub async fn extract_bearer_token(
    response: axum::response::Response,
) -> (axum::response::Response, Option<String>) {
    let (parts, body) = response.into_parts();
    let bytes = to_bytes(body, usize::MAX).await.unwrap();
    let token = serde_json::from_slice::<serde_json::Value>(&bytes)
        .ok()
        .and_then(|value| {
            value
                .get("accessToken")
                .or_else(|| value.get("access_token"))
                .and_then(|token| token.as_str())
                .map(str::to_owned)
        });
    (
        axum::response::Response::from_parts(parts, axum::body::Body::from(bytes)),
        token,
    )
}
