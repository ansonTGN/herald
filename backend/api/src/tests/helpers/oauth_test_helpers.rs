// =============================================================================
// OAuth Test Helpers
// =============================================================================
//
// Reusable helper functions for OAuth provider testing to eliminate code duplication
// between google_oauth_scenarios.rs and github_oauth_scenarios.rs
//
// =============================================================================

use crate::tests::schema_test_context::SchemaTestContext;
use axum::{
    body::Body,
    http::{Method, Request},
};
use serde_json::json;
use tower::ServiceExt;

/// OAuth provider configuration data
#[derive(Debug, Clone)]
pub struct OAuthProviderConfig {
    pub provider_type: String,
    pub client_id: String,
    pub client_secret: String,
    pub scopes: Vec<String>,
    pub enabled: bool,
}

impl OAuthProviderConfig {
    pub fn new(
        provider_type: &str,
        client_id: &str,
        client_secret: &str,
        scopes: Vec<String>,
        enabled: bool,
    ) -> Self {
        Self {
            provider_type: provider_type.to_string(),
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            scopes,
            enabled,
        }
    }
}

/// Create OAuth provider configuration
///
/// Returns the token and creates the provider config in the database
pub async fn create_oauth_provider_config(
    ctx: &mut SchemaTestContext,
    config: &OAuthProviderConfig,
    token: &str,
    realm_id: &str,
) -> axum::response::Response {
    let app = ctx.create_unified_test_router();

    let create_payload = json!({
        "providerType": config.provider_type,
        "clientId": config.client_id,
        "clientSecret": config.client_secret,
        "scopes": config.scopes,
        "enabled": config.enabled
    });

    let create_request = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/oauth/{}/configs", realm_id))
        .header("content-type", "application/json")
        .header("cookie", format!("X-Auth={}", token))
        .body(Body::from(create_payload.to_string()))
        .unwrap();

    app.oneshot(create_request).await.unwrap()
}

/// Generate OAuth authorization URL and extract state token
pub async fn generate_auth_url_and_extract_state(
    ctx: &mut SchemaTestContext,
    realm_id: &str,
    provider_type: &str,
) -> (axum::response::Response, String) {
    let app = ctx.create_unified_test_router();

    let auth_request = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/oauth/{}/{}/login", realm_id, provider_type))
        .body(Body::empty())
        .unwrap();

    let auth_response = app.oneshot(auth_request).await.unwrap();

    // Save the original status
    let status = auth_response.status();

    // Extract JSON body to get auth_url and state
    let body = axum::body::to_bytes(auth_response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");

    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Handle error responses (404, 400, etc.) that don't have state
    if !status.is_success() {
        return (
            axum::response::Response::builder()
                .status(status)
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
            String::new(),
        );
    }

    // oauth_login is a B-class interface (OAuth protocol), so response is direct JSON: { state: "...", authUrl: "..." }
    let state_token = json["state"]
        .as_str()
        .expect("Response should contain state")
        .to_string();

    // Recreate response with the original status and body
    let auth_response = axum::response::Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();

    (auth_response, state_token)
}

/// Send OAuth callback with code and state
pub async fn send_oauth_callback(
    ctx: &mut SchemaTestContext,
    realm_id: &str,
    provider_type: &str,
    code: &str,
    state_token: &str,
) -> Result<axum::response::Response, tower::BoxError> {
    let app = ctx.create_unified_test_router();

    let callback_query = format!("code={}&state={}", code, state_token);

    let callback_request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/api/oauth/{}/{}/callback?{}",
            realm_id, provider_type, callback_query
        ))
        .body(Body::empty())
        .unwrap();

    app.oneshot(callback_request).await.map_err(|e| match e {})
}

/// Get OAuth provider configuration by type
pub async fn get_oauth_provider_config(
    ctx: &mut SchemaTestContext,
    realm_id: &str,
    provider_type: &str,
    token: &str,
) -> axum::response::Response {
    let app = ctx.create_unified_test_router();

    let get_request = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/oauth/{}/configs/{}", realm_id, provider_type))
        .header("cookie", format!("X-Auth={}", token))
        .body(Body::empty())
        .unwrap();

    app.oneshot(get_request).await.unwrap()
}

/// List enabled OAuth providers
pub async fn list_enabled_oauth_providers(
    ctx: &mut SchemaTestContext,
    realm_id: &str,
    token: &str,
) -> axum::response::Response {
    let app = ctx.create_unified_test_router();

    let list_request = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/oauth/{}/configs?enabled=true", realm_id))
        .header("cookie", format!("X-Auth={}", token))
        .body(Body::empty())
        .unwrap();

    app.oneshot(list_request).await.unwrap()
}

/// Delete OAuth provider configuration
pub async fn delete_oauth_provider_config(
    ctx: &mut SchemaTestContext,
    realm_id: &str,
    provider_type: &str,
    token: &str,
) -> axum::response::Response {
    let app = ctx.create_unified_test_router();

    let delete_request = Request::builder()
        .method(Method::DELETE)
        .uri(format!("/api/oauth/{}/configs/{}", realm_id, provider_type))
        .header("cookie", format!("X-Auth={}", token))
        .body(Body::empty())
        .unwrap();

    app.oneshot(delete_request).await.unwrap()
}

/// Create an existing user with linked OAuth provider
pub async fn create_existing_user_with_oauth_link(
    ctx: &mut SchemaTestContext,
    realm_id: &str,
    test_email: &str,
    provider_type: &str,
    provider_user_id: &str,
    open_id: &str,
) -> uuid::Uuid {
    let existing_user_uuid = uuid::Uuid::now_v7();

    sqlx::query(
        "INSERT INTO account (id, realm_id, email, password, status) VALUES ($1, $2, $3, $4, 1)",
    )
    .bind(existing_user_uuid)
    .bind(realm_id)
    .bind(test_email)
    .bind("$2a$12$existing_user_password")
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to create existing user");

    let provider_uuid = uuid::Uuid::now_v7();

    sqlx::query(
        "INSERT INTO user_oauth_providers (id, user_id, realm_id, provider_type, provider_user_id, open_id) VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(provider_uuid)
    .bind(existing_user_uuid)
    .bind(realm_id)
    .bind(provider_type)
    .bind(provider_user_id)
    .bind(open_id)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to link OAuth provider");

    existing_user_uuid
}

/// Check if user exists in database
pub async fn user_exists(ctx: &SchemaTestContext, email: &str, realm_id: &str) -> bool {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM account WHERE email = $1 AND realm_id = $2)")
        .bind(email)
        .bind(realm_id)
        .fetch_one(&ctx._app_state.pool)
        .await
        .unwrap()
}

/// Check if OAuth provider is linked to user
pub async fn oauth_provider_linked(
    ctx: &SchemaTestContext,
    realm_id: &str,
    provider_type: &str,
) -> bool {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM user_oauth_providers WHERE realm_id = $1 AND provider_type = $2)"
    )
    .bind(realm_id)
    .bind(provider_type)
    .fetch_one(&ctx._app_state.pool)
    .await
    .unwrap()
}

/// Count users in realm
pub async fn count_users(ctx: &SchemaTestContext, realm_id: &str) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM account WHERE realm_id = $1")
        .bind(realm_id)
        .fetch_one(&ctx._app_state.pool)
        .await
        .unwrap()
}

/// Verify provider config in database
pub async fn verify_provider_config_in_db(
    ctx: &SchemaTestContext,
    realm_id: &str,
    provider_type: &str,
    expected_client_id: &str,
    expected_scopes: &[String],
    expected_enabled: bool,
) {
    let (db_client_id, db_scopes, db_enabled): (String, Vec<String>, bool) = sqlx::query_as(
        "SELECT client_id, scopes, enabled FROM oauth_provider_config WHERE realm_id = $1 AND provider_type = $2"
    )
    .bind(realm_id)
    .bind(provider_type)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to query oauth_providers");

    assert_eq!(db_client_id, expected_client_id);
    assert_eq!(db_scopes, expected_scopes);
    assert_eq!(db_enabled, expected_enabled);
}
