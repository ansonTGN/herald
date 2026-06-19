//! Payment provider directory handlers.
//!
//! Lists all configured payment providers for a realm (WeChat / Stripe /
//! Creem). These handlers are provider-agnostic; per-provider CRUD lives in
//! the dedicated `<provider>_config_handlers` modules.

use axum::{
    Json,
    extract::{Extension, Path, State},
};

use crate::provider_common_types::{PaymentProviderInfo, PaymentProvidersResponse};
use crate::wechat_config_handlers::get_wechat_config_for_providers;
use herald_api_base::application::http::common::auth_utils::require_authenticated_user_in_realm;
use herald_api_base::application::http::server::api_entities::ApiError;
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::authorization::PermissionService;
use herald_core::domain::realm_config::RealmConfigRepository;

#[utoipa::path(
    get,
    path = "/api/third/pay/{realmId}/providers",
    params(
        ("realmId" = String, Path, description = "Realm UUID")
    ),
    responses(
        (status = 200, description = "Payment providers retrieved successfully.", body = PaymentProvidersResponse),
        (status = 401, description = "Unauthorized - No valid authentication token"),
        (status = 403, description = "Forbidden - User does not have access to this realm"),
        (status = 404, description = "Realm not found")
    ),
    tag = "billing.payment-providers",
    operation_id = "list_payment_providers"
)]
pub async fn list_payment_providers(
    State(state): State<AppState>,
    Path(realm_id): Path<String>,
    Extension(identity): Extension<Identity>,
) -> Result<Json<PaymentProvidersResponse>, ApiError> {
    let _user_id =
        require_authenticated_user_in_realm(&identity, &realm_id, "list payment providers")?;

    let can_manage = if identity.is_user() {
        state
            .permission_checker
            .check_permission(&realm_id, &identity.user_id(), "billing", "manage")
            .await
            .unwrap_or(false)
    } else {
        false
    };

    let mut providers = Vec::new();

    if let Some(wechat_config) = get_wechat_config_for_providers(&state, &realm_id).await?
        && (can_manage || wechat_config.enabled)
    {
        providers.push(wechat_config);
    }

    if let Some(stripe_config) = get_stripe_config_for_providers(&state, &realm_id).await?
        && (can_manage || stripe_config.enabled)
    {
        providers.push(stripe_config);
    }

    if let Some(creem_config) = get_creem_config_for_providers(&state, &realm_id).await?
        && (can_manage || creem_config.enabled)
    {
        providers.push(creem_config);
    }

    Ok(Json(PaymentProvidersResponse { providers }))
}

pub async fn get_stripe_config_for_providers(
    state: &AppState,
    realm_id: &str,
) -> Result<Option<PaymentProviderInfo>, ApiError> {
    let configs = state
        .realm_config_repository
        .get_by_type(realm_id.to_string(), "stripe".to_string())
        .await
        .map_err(|e| {
            tracing::error!("Failed to load Stripe configuration: {}", e);
            ApiError::internal(format!("Database error: {}", e))
        })?;

    if configs.is_empty() {
        return Ok(None);
    }

    let mut last_updated: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut enabled = true;
    let mut has_config = false;

    for rc in &configs {
        last_updated =
            Some(last_updated.map_or(rc.updated_at, |current| current.max(rc.updated_at)));
        enabled &= rc.enabled;

        if rc.config_key == "publishable_key" {
            has_config = true;
        }
    }

    if !has_config {
        return Ok(None);
    }

    Ok(Some(PaymentProviderInfo {
        platform: "stripe".to_string(),
        shop_domain: None,
        api_version: None,
        webhook_endpoint: Some("Stripe webhooks configured".to_string()),
        last_updated: last_updated.map(|dt| dt.to_rfc3339()),
        enabled,
    }))
}

pub async fn get_creem_config_for_providers(
    state: &AppState,
    realm_id: &str,
) -> Result<Option<PaymentProviderInfo>, ApiError> {
    let configs = state
        .realm_config_repository
        .get_by_type(realm_id.to_string(), "creem".to_string())
        .await
        .map_err(|e| {
            tracing::error!("Failed to load Creem configuration: {}", e);
            ApiError::internal(format!("Database error: {}", e))
        })?;

    if configs.is_empty() {
        return Ok(None);
    }

    let mut last_updated: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut enabled = true;
    let mut has_config = false;

    for rc in &configs {
        last_updated =
            Some(last_updated.map_or(rc.updated_at, |current| current.max(rc.updated_at)));
        enabled &= rc.enabled;

        if rc.config_key == "api_key" {
            has_config = true;
        }
    }

    if !has_config {
        return Ok(None);
    }

    Ok(Some(PaymentProviderInfo {
        platform: "creem".to_string(),
        shop_domain: None,
        api_version: None,
        webhook_endpoint: Some("Creem webhooks configured".to_string()),
        last_updated: last_updated.map(|dt| dt.to_rfc3339()),
        enabled,
    }))
}
