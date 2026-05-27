// Public configuration endpoint for realm settings

use axum::{
    extract::{Path, State},
    http::HeaderMap,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::application::http::realm_config::public_helper::{parse_bool, query_config_value};
pub use crate::application::http::server::api_entities::ErrorResponse;
use crate::application::http::server::api_entities::{ApiError, ApiResult};
use crate::application::http::state::AppState;
use herald_core::domain::realm::{RealmService, RealmSummary};

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegistrationConfig {
    pub enabled: bool,
    pub require_email_verification: bool,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OAuthProviderInfo {
    pub name: String,
    pub display_name: String,
    pub enabled: bool,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PublicConfigResponse {
    pub realm_name: String,
    pub realm_description: Option<String>,
    pub registration: RegistrationConfig,
    pub oauth_providers: Vec<OAuthProviderInfo>,
}

/// Queries a single boolean config value from the realm_config table.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `realm_id` - The realm ID
/// * `config_key` - The config key to query (e.g., "enabled", "require_email_verification")
/// * `default_value` - Default value if config is not found or cannot be parsed
///
/// # Returns
///
/// The parsed boolean value or the default value.
async fn query_bool_config(
    pool: &sqlx::PgPool,
    realm_id: &str,
    config_key: &str,
    default_value: bool,
) -> Result<bool, ApiError> {
    let value = query_config_value(pool, realm_id, "registration", config_key).await?;

    Ok(value.and_then(|v| parse_bool(&v)).unwrap_or(default_value))
}

/// Get public configuration for a realm
///
/// Returns publicly accessible configuration for the specified realm, including:
/// - Registration settings (whether registration is allowed and if email verification is required)
/// - List of enabled OAuth providers
///
/// This endpoint does not require authentication.
#[utoipa::path(
    get,
    path = "/api/public-config/{realmId}",
    tag = "system",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    responses(
        (status = 200, description = "Public configuration retrieved successfully", body = PublicConfigResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_public_config(
    Path(realm_id): Path<String>,
    State(state): State<AppState>,
    _headers: HeaderMap,
) -> Result<ApiResult<PublicConfigResponse>, ApiError> {
    let registration_enabled = query_bool_config(&state.pool, &realm_id, "enabled", false).await?;
    let require_email_verification =
        query_bool_config(&state.pool, &realm_id, "require_email_verification", false).await?;

    let configs = state
        .service
        .oauth_config_service()
        .list_enabled_providers(&realm_id)
        .await
        .map_err(|e| {
            tracing::error!(realm_id = %realm_id, error = %e, error_details = format!("{:?}", e), "Failed to list oauth providers");
            ApiError::internal(format!("Failed to list oauth providers: {}", e))
        })?;

    let oauth_providers: Vec<OAuthProviderInfo> = configs
        .into_iter()
        .map(|config| OAuthProviderInfo {
            name: config.provider_type.as_str().to_string(),
            display_name: config.provider_type.display_name().to_string(),
            enabled: config.enabled,
        })
        .collect();

    let realm_info = match state
        .service
        .realm_service()
        .get_public_realm_info(realm_id.clone())
        .await
    {
        Ok(info) => info,
        Err(_) => RealmSummary {
            id: realm_id.clone(),
            name: realm_id.clone(),
            description: None,
        },
    };

    Ok(ApiResult::ok(PublicConfigResponse {
        realm_name: realm_info.name,
        realm_description: realm_info.description,
        registration: RegistrationConfig {
            enabled: registration_enabled,
            require_email_verification,
        },
        oauth_providers,
    }))
}
