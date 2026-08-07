// =============================================================================
// OAuth 测试辅助函数
// =============================================================================
//
// 提供OAuth集成测试的辅助函数，支持Beeceptor Mock Server测试。
//
// ## 主要功能
//
// - OAuth提供者配置管理
// - 模拟OAuth授权请求
// - 用户创建与验证
// - Mock授权码提取
//
// ## 支持的OAuth提供者
//
// - Google OAuth
// - GitHub OAuth
// - WeChat OAuth
// - WeChat Mini Program
//
// ## 测试架构
//
// - 使用Beeceptor Mock Server模拟OAuth提供者
// - 独立Schema隔离测试数据
// - Redis状态管理
// - Mock URLs 从 `mock_oauth_urls` 模块导入
//
// ## 参考
// - OAuth Mock规范: `.ai/future/third.md`
// - OAuth实现: `api/src/application/http/oauth/`
// - Mock URL 常量: `mock_oauth_urls`

use crate::schema_test_context::SchemaTestContext as TestContext;
use serde_json::json;
use uuid::Uuid;

/// ============================================================================
/// OAuth 提供者配置管理
/// ============================================================================
///
/// 创建测试用的OAuth提供者配置
///
/// 在测试数据库中启用指定类型的OAuth提供者。
///
/// # Arguments
/// * `ctx` - 测试上下文
/// * `provider_type` - 提供者类型 (google, github, wechat, wechat_miniprogram)
/// * `client_id` - OAuth客户端ID
/// * `client_secret` - OAuth客户端密钥
/// * `scopes` - OAuth权限范围列表
///
/// # Returns
/// 提供者配置的UUID
///
/// # Example
/// ```no_run
/// # async fn test(ctx: &mut TestContext) {
/// let provider_id = create_mock_oauth_provider_config(
///     ctx,
///     "google",
///     "test-client-id",
///     "test-client-secret",
///     vec!["openid".to_string(), "email".to_string()]
/// ).await;
/// # }
/// ```
pub async fn create_mock_oauth_provider_config(
    ctx: &TestContext,
    provider_type: &str,
    client_id: &str,
    client_secret: &str,
    scopes: Vec<String>,
) -> String {
    let provider_uuid = Uuid::now_v7();

    // 验证provider_type是否有效
    let valid_types = [
        "google",
        "github",
        "facebook",
        "apple",
        "wechat",
        "wechat_miniprogram",
    ];
    if !valid_types.contains(&provider_type) {
        tracing::error!(
            provider_type = %provider_type,
            valid_types = ?valid_types,
            "Invalid provider type"
        );
        panic!(
            "Invalid provider type: {}. Must be one of: {:?}",
            provider_type, valid_types
        );
    }

    sqlx::query(
        r#"
        INSERT INTO oauth_providers (id, realm_id, provider_type, client_id, client_secret, scopes, enabled)
        VALUES ($1, $2, $3, $4, $5, $6, true)
        "#
    )
    .bind(provider_uuid)
    .bind(&ctx._realm_id)
    .bind(provider_type)
    .bind(client_id)
    .bind(client_secret)
    .bind(scopes)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create OAuth provider config");

    tracing::debug!(
        provider_id = %provider_uuid,
        provider_type = %provider_type,
        realm_id = %ctx._realm_id,
        "Created OAuth provider configuration"
    );

    provider_uuid.to_string()
}

/// 创建Google OAuth提供者配置（预设配置）
///
/// 使用预设的测试配置创建Google OAuth提供者。
///
/// # Arguments
/// * `ctx` - 测试上下文
///
/// # Returns
/// 提供者配置的UUID
pub async fn create_google_oauth_provider_config(ctx: &TestContext) -> String {
    create_mock_oauth_provider_config(
        ctx,
        "google",
        "google-test-client-id",
        "google-test-client-secret",
        vec![
            "openid".to_string(),
            "email".to_string(),
            "profile".to_string(),
        ],
    )
    .await
}

/// 创建GitHub OAuth提供者配置（预设配置）
///
/// 使用预设的测试配置创建GitHub OAuth提供者。
///
/// # Arguments
/// * `ctx` - 测试上下文
///
/// # Returns
/// 提供者配置的UUID
pub async fn create_github_oauth_provider_config(ctx: &TestContext) -> String {
    create_mock_oauth_provider_config(
        ctx,
        "github",
        "github-test-client-id",
        "github-test-client-secret",
        vec!["user:email".to_string(), "read:user".to_string()],
    )
    .await
}

/// 创建WeChat OAuth提供者配置（预设配置）
///
/// 使用预设的测试配置创建WeChat OAuth提供者。
///
/// # Arguments
/// * `ctx` - 测试上下文
///
/// # Returns
/// 提供者配置的UUID
pub async fn create_wechat_oauth_provider_config(ctx: &TestContext) -> String {
    create_mock_oauth_provider_config(
        ctx,
        "wechat",
        "wechat-test-app-id",
        "wechat-test-app-secret",
        vec!["snsapi_userinfo".to_string()],
    )
    .await
}

/// 创建WeChat Mini Program提供者配置（预设配置）
///
/// 使用预设的测试配置创建WeChat Mini Program提供者。
///
/// # Arguments
/// * `ctx` - 测试上下文
///
/// # Returns
/// 提供者配置的UUID
pub async fn create_wechat_miniprogram_provider_config(ctx: &TestContext) -> String {
    create_mock_oauth_provider_config(
        ctx,
        "wechat_miniprogram",
        "wx-test-app-id",
        "wx-test-app-secret",
        vec![],
    )
    .await
}

/// ============================================================================
/// OAuth 授权请求生成
/// ============================================================================
///
/// 生成OAuth授权URL请求体
///
/// 构造标准OAuth授权请求，适用于测试验证。
///
/// # Arguments
/// * `ctx` - 测试上下文
/// * `provider_type` - 提供者类型
/// * `redirect_uri` - 重定向URI（可选，默认使用测试服务器回调）
///
/// # Returns
/// 请求URL和JSON体
pub fn generate_oauth_auth_url_request(
    ctx: &TestContext,
    provider_type: &str,
    redirect_uri: Option<String>,
) -> (String, serde_json::Value) {
    let url = format!("/api/{}/oauth/{}", ctx._realm_id, provider_type);

    let body = if let Some(uri) = redirect_uri {
        json!({
            "redirect_uri": uri
        })
    } else {
        json!({})
    };

    (url, body)
}

/// ============================================================================
/// OAuth 用户验证
/// ============================================================================
///
/// 验证OAuth用户已创建
///
/// 检查数据库中是否存在指定邮箱的用户。
///
/// # Arguments
/// * `ctx` - 测试上下文
/// * `email` - 用户邮箱
///
/// # Returns
/// 如果用户存在则返回用户ID，否则返回None
pub async fn verify_oauth_user_created(ctx: &TestContext, email: &str) -> Option<String> {
    let user_id: Option<String> =
        sqlx::query_scalar("SELECT id::text FROM account WHERE email = $1 AND realm_id = $2")
            .bind(email)
            .bind(&ctx._realm_id)
            .fetch_optional(&ctx.app_state.pool)
            .await
            .unwrap();

    if let Some(id) = &user_id {
        tracing::debug!(
            user_id = %id,
            email = %email,
            "Verified OAuth user exists"
        );
    } else {
        tracing::debug!(
            email = %email,
            "OAuth user not found"
        );
    }

    user_id
}

/// 验证OAuth用户信息
///
/// 检查用户的详细信息（状态、创建时间等）。
///
/// # Arguments
/// * `ctx` - 测试上下文
/// * `user_id` - 用户ID
///
/// # Returns
/// 用户信息元组 (email, status, created_at)
pub async fn verify_oauth_user_details(
    ctx: &TestContext,
    user_id: &str,
) -> (String, i32, chrono::DateTime<chrono::Utc>) {
    let row = sqlx::query_as::<_, (String, i32, chrono::DateTime<chrono::Utc>)>(
        "SELECT email, status, created_at FROM account WHERE id = $1 AND realm_id = $2",
    )
    .bind(user_id)
    .bind(&ctx._realm_id)
    .fetch_one(&ctx.app_state.pool)
    .await
    .expect("User not found");

    tracing::debug!(
        user_id = %user_id,
        email = %row.0,
        status = %row.1,
        "Verified OAuth user details"
    );

    row
}

/// ============================================================================
/// Mock 授权码提取
/// ============================================================================
///
/// 从Beeceptor响应中提取授权码
///
/// 解析Mock服务器的响应，提取授权码用于OAuth回调测试。
///
/// # Arguments
/// * `response_body` - 响应体（HTML或JSON）
///
/// # Returns
/// 提取的授权码
///
/// # Note
/// Beeceptor Mock Server的响应格式：
/// - HTML格式: `<a href="/callback?code=AUTH_CODE&state=STATE">...</a>`
/// - JSON格式: `{"code": "AUTH_CODE", "state": "STATE"}`
pub fn extract_mock_auth_code(response_body: &str) -> Option<String> {
    // 尝试从HTML中提取
    if response_body.contains("<a href")
        && let Some(start) = response_body.find("code=")
    {
        let code_part = &response_body[start + 5..];
        if let Some(end) = code_part.find('&') {
            return Some(code_part[..end].to_string());
        } else if let Some(end) = code_part.find('"') {
            return Some(code_part[..end].to_string());
        }
    }

    // 尝试从JSON中提取
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(response_body)
        && let Some(code) = json.get("code").and_then(|c| c.as_str())
    {
        return Some(code.to_string());
    }

    None
}

/// 从Beeceptor响应中提取state参数
///
/// 解析Mock服务器的响应，提取state参数用于CSRF验证。
///
/// # Arguments
/// * `response_body` - 响应体（HTML或JSON）
///
/// # Returns
/// 提取的state参数
pub fn extract_mock_state(response_body: &str) -> Option<String> {
    // 尝试从HTML中提取
    if response_body.contains("<a href")
        && let Some(start) = response_body.find("state=")
    {
        let state_part = &response_body[start + 6..];
        if let Some(end) = state_part.find('"') {
            return Some(state_part[..end].to_string());
        }
    }

    // 尝试从JSON中提取
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(response_body)
        && let Some(state) = json.get("state").and_then(|s| s.as_str())
    {
        return Some(state.to_string());
    }

    None
}

/// ============================================================================
/// Mock 用户信息生成
/// ============================================================================
///
/// 生成Google OAuth Mock用户信息
///
/// 创建模拟的Google OAuth用户信息，用于测试验证。
///
/// # Arguments
/// * `email` - 用户邮箱
/// * `name` - 用户姓名（可选）
///
/// # Returns
/// Google用户信息JSON
pub fn generate_mock_google_user_info(email: &str, name: Option<&str>) -> serde_json::Value {
    let google_id = uuid::Uuid::now_v7().to_string();

    json!({
        "id": google_id,
        "email": email,
        "verified_email": true,
        "picture": format!("https://example.com/avatar/{}", google_id),
        "name": name.unwrap_or("Test User")
    })
}

/// 生成GitHub OAuth Mock用户信息
///
/// 创建模拟的GitHub OAuth用户信息，用于测试验证。
///
/// # Arguments
/// * `email` - 用户邮箱
/// * `login` - GitHub用户名（可选）
///
/// # Returns
/// GitHub用户信息JSON
pub fn generate_mock_github_user_info(email: &str, login: Option<&str>) -> serde_json::Value {
    let github_id = uuid::Uuid::now_v7().to_string();

    json!({
        "id": github_id.parse::<i64>().unwrap_or(123456),
        "login": login.unwrap_or("testuser"),
        "email": email,
        "avatar_url": format!("https://example.com/avatar/{}", github_id),
        "name": "Test User"
    })
}

/// ============================================================================
/// 测试清理函数
/// ============================================================================
///
/// 清理测试用的OAuth提供者配置
///
/// 删除测试创建的OAuth提供者配置，避免干扰其他测试。
///
/// # Arguments
/// * `ctx` - 测试上下文
/// * `provider_id` - 提供者配置ID
pub async fn cleanup_oauth_provider_config(ctx: &TestContext, provider_id: &str) {
    let result = sqlx::query("DELETE FROM oauth_providers WHERE id = $1 AND realm_id = $2")
        .bind(provider_id)
        .bind(&ctx._realm_id)
        .execute(&ctx.app_state.pool)
        .await;

    match result {
        Ok(rows_affected) => {
            if rows_affected.rows_affected() > 0 {
                tracing::debug!(
                    provider_id = %provider_id,
                    "Cleaned up OAuth provider config"
                );
            }
        }
        Err(e) => {
            tracing::warn!(
                provider_id = %provider_id,
                error = %e,
                "Failed to clean up OAuth provider config"
            );
        }
    }
}
