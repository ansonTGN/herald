use crate::handlers::require_billing_permission;
use crate::provider_common_types::{
    GenericErrorResponse, PaymentProviderInfo, ValidationErrorDetail, ValidationErrorResponse,
    validate_request,
};
use crate::wechat_config_types::{
    WechatConfigQuery, WechatConfigRequest, WechatConfigResponse, WechatConfigUpdateRequest,
};
use herald_api_base::application::http::server::api_entities::ApiError;
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::realm_config::{
    ConfigType, RealmConfigRepository, UpsertRealmConfigRequest,
};

use axum::Json;
use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use sqlx::PgPool;
use tracing::info;

#[utoipa::path(
    post,
    path = "/api/third/pay/{realmId}/providers/wechat",
    params(
        ("realmId" = String, Path, description = "Realm UUID")
    ),
    request_body = WechatConfigRequest,
    responses(
        (status = 201, description = "Configuration created successfully", body = WechatConfigResponse),
        (status = 400, description = "Validation failed", body = ValidationErrorResponse),
        (status = 401, description = "Unauthorized"),
        (status = 409, description = "Configuration already exists", body = GenericErrorResponse)
    ),
    tag = "billing.payment-providers",
    operation_id = "create_wechat_config"
)]
pub async fn create_wechat_config(
    State(state): State<AppState>,
    Path(realm_id): Path<String>,
    Extension(identity): Extension<Identity>,
    Json(request): Json<WechatConfigRequest>,
) -> Result<(StatusCode, Json<WechatConfigResponse>), ApiError> {
    require_billing_permission(&state, &identity, &realm_id, "manage").await?;

    validate_request(&request).map_err(ApiError::bad_request_json)?;

    if !request.app_id.starts_with("wx") {
        return Err(ApiError::bad_request_json(ValidationErrorResponse {
            error: "validation_failed".to_string(),
            details: vec![ValidationErrorDetail {
                field: "appId".to_string(),
                message: "App ID must start with 'wx'".to_string(),
            }],
        }));
    }

    if !request.notify_url.starts_with("https://") {
        return Err(ApiError::bad_request_json(ValidationErrorResponse {
            error: "validation_failed".to_string(),
            details: vec![ValidationErrorDetail {
                field: "notifyUrl".to_string(),
                message: "Notify URL must use HTTPS".to_string(),
            }],
        }));
    }

    if !request.private_key.contains("-----BEGIN")
        || !request.private_key.contains("PRIVATE KEY-----")
    {
        return Err(ApiError::bad_request_json(ValidationErrorResponse {
            error: "validation_failed".to_string(),
            details: vec![ValidationErrorDetail {
                field: "privateKey".to_string(),
                message: "Private key must be in PEM format".to_string(),
            }],
        }));
    }

    if let Ok(Some(_)) = get_wechat_config_internal(&state, &realm_id, false).await {
        return Err(ApiError::conflict_json(GenericErrorResponse {
            error: "configuration_already_exists".to_string(),
            message: "A WeChat configuration already exists for this realm. Please edit the existing configuration.".to_string(),
        }));
    }

    save_wechat_config(&state, &realm_id, &request).await?;

    info!(
        realm_id = %realm_id,
        "WeChat payment provider configuration created"
    );

    let config = get_wechat_config_internal(&state, &realm_id, false)
        .await?
        .ok_or_else(|| ApiError::internal("Failed to retrieve saved configuration"))?;

    Ok((StatusCode::CREATED, Json(config)))
}

#[utoipa::path(
    get,
    path = "/api/third/pay/{realmId}/providers/wechat",
    params(
        ("realmId" = String, Path, description = "Realm UUID"),
        ("reveal_secrets" = Option<bool>, Query, description = "Whether to reveal sensitive information (default: false)")
    ),
    responses(
        (status = 200, description = "Configuration retrieved successfully", body = WechatConfigResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Configuration not found")
    ),
    tag = "billing.payment-providers",
    operation_id = "get_wechat_config"
)]
pub async fn get_wechat_config(
    State(state): State<AppState>,
    Path(realm_id): Path<String>,
    Extension(identity): Extension<Identity>,
    Query(query): Query<WechatConfigQuery>,
) -> Result<Json<WechatConfigResponse>, ApiError> {
    require_billing_permission(&state, &identity, &realm_id, "view").await?;

    let config = get_wechat_config_internal(&state, &realm_id, query.reveal_secrets)
        .await?
        .ok_or_else(|| ApiError::not_found("WeChat configuration not found"))?;

    Ok(Json(config))
}

#[utoipa::path(
    put,
    path = "/api/third/pay/{realmId}/providers/wechat",
    params(
        ("realmId" = String, Path, description = "Realm UUID")
    ),
    request_body = WechatConfigUpdateRequest,
    responses(
        (status = 200, description = "Configuration updated successfully", body = WechatConfigResponse),
        (status = 400, description = "Validation failed", body = ValidationErrorResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Configuration not found")
    ),
    tag = "billing.payment-providers",
    operation_id = "update_wechat_config"
)]
pub async fn update_wechat_config(
    State(state): State<AppState>,
    Path(realm_id): Path<String>,
    Extension(identity): Extension<Identity>,
    Json(request): Json<WechatConfigUpdateRequest>,
) -> Result<(StatusCode, Json<WechatConfigResponse>), ApiError> {
    require_billing_permission(&state, &identity, &realm_id, "manage").await?;

    validate_request(&request).map_err(ApiError::bad_request_json)?;

    if let Some(app_id) = &request.app_id
        && !app_id.starts_with("wx")
    {
        return Err(ApiError::bad_request_json(ValidationErrorResponse {
            error: "validation_failed".to_string(),
            details: vec![ValidationErrorDetail {
                field: "appId".to_string(),
                message: "App ID must start with 'wx'".to_string(),
            }],
        }));
    }

    if let Some(notify_url) = &request.notify_url
        && !notify_url.starts_with("https://")
    {
        return Err(ApiError::bad_request_json(ValidationErrorResponse {
            error: "validation_failed".to_string(),
            details: vec![ValidationErrorDetail {
                field: "notifyUrl".to_string(),
                message: "Notify URL must use HTTPS".to_string(),
            }],
        }));
    }

    if let Some(private_key) = &request.private_key
        && (!private_key.contains("-----BEGIN") || !private_key.contains("PRIVATE KEY-----"))
    {
        return Err(ApiError::bad_request_json(ValidationErrorResponse {
            error: "validation_failed".to_string(),
            details: vec![ValidationErrorDetail {
                field: "privateKey".to_string(),
                message: "Private key must be in PEM format".to_string(),
            }],
        }));
    }

    let _existing = get_wechat_config_internal(&state, &realm_id, false)
        .await?
        .ok_or_else(|| ApiError::not_found("WeChat configuration not found"))?;

    update_wechat_config_internal(&state, &realm_id, &request).await?;

    let config = get_wechat_config_internal(&state, &realm_id, false)
        .await?
        .ok_or_else(|| ApiError::internal("Failed to retrieve updated configuration"))?;

    Ok((StatusCode::OK, Json(config)))
}

#[utoipa::path(
    delete,
    path = "/api/third/pay/{realmId}/providers/wechat",
    params(
        ("realmId" = String, Path, description = "Realm UUID")
    ),
    responses(
        (status = 204, description = "Configuration deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Configuration not found"),
        (status = 409, description = "Active subscriptions exist", body = GenericErrorResponse)
    ),
    tag = "billing.payment-providers",
    operation_id = "delete_wechat_config"
)]
pub async fn delete_wechat_config(
    State(state): State<AppState>,
    Path(realm_id): Path<String>,
    Extension(identity): Extension<Identity>,
) -> Result<StatusCode, ApiError> {
    require_billing_permission(&state, &identity, &realm_id, "manage").await?;

    let _existing = get_wechat_config_internal(&state, &realm_id, false)
        .await?
        .ok_or_else(|| ApiError::not_found("WeChat configuration not found"))?;

    let active_count = count_active_wechat_subscriptions(&state.pool, &realm_id).await?;
    if active_count > 0 {
        return Err(ApiError::conflict_json(GenericErrorResponse {
            error: "active_subscriptions_exist".to_string(),
            message: format!(
                "Cannot delete payment provider with {} active subscription(s)",
                active_count
            ),
        }));
    }

    delete_wechat_config_internal(&state, &realm_id).await?;

    info!(
        realm_id = %realm_id,
        "WeChat payment provider configuration deleted"
    );

    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_wechat_config_for_providers(
    state: &AppState,
    realm_id: &str,
) -> Result<Option<PaymentProviderInfo>, ApiError> {
    let configs = state
        .realm_config_repository
        .get_by_type(realm_id.to_string(), "wechat".to_string())
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if configs.is_empty() {
        return Ok(None);
    }

    let mut last_updated: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut enabled = true;

    for rc in &configs {
        last_updated =
            Some(last_updated.map_or(rc.updated_at, |current| current.max(rc.updated_at)));
        enabled &= rc.enabled;
    }

    Ok(Some(PaymentProviderInfo {
        platform: "wechat".to_string(),
        shop_domain: None,
        api_version: None,
        webhook_endpoint: None,
        last_updated: last_updated.map(|dt| dt.to_rfc3339()),
        enabled,
    }))
}

async fn get_wechat_config_internal(
    state: &AppState,
    realm_id: &str,
    reveal_secrets: bool,
) -> Result<Option<WechatConfigResponse>, ApiError> {
    let configs = state
        .realm_config_repository
        .get_by_type(realm_id.to_string(), "wechat".to_string())
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if configs.is_empty() {
        return Ok(None);
    }

    let mut config = WechatConfigResponse {
        platform: "wechat".to_string(),
        app_id: String::new(),
        mch_id: String::new(),
        serial_no: String::new(),
        v3_key: String::new(),
        private_key: String::new(),
        platform_public_key: String::new(),
        notify_url: String::new(),
        created_at: String::new(),
        updated_at: String::new(),
    };

    let mut created_at: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut updated_at: Option<chrono::DateTime<chrono::Utc>> = None;

    for rc in configs {
        created_at = Some(created_at.map_or(rc.created_at, |current| current.min(rc.created_at)));
        updated_at = Some(updated_at.map_or(rc.updated_at, |current| current.max(rc.updated_at)));

        match rc.config_key.as_str() {
            "app_id" => config.app_id = rc.config_value,
            "mch_id" => config.mch_id = rc.config_value,
            "serial_no" => config.serial_no = rc.config_value,
            "private_key" => {
                if reveal_secrets {
                    config.private_key = rc.config_value;
                } else {
                    config.private_key = mask_secret(&rc.config_value);
                }
            }
            "v3_key" => {
                if reveal_secrets {
                    config.v3_key = rc.config_value;
                } else {
                    config.v3_key = mask_v3_key(&rc.config_value);
                }
            }
            "platform_public_key" => {
                if reveal_secrets {
                    config.platform_public_key = rc.config_value;
                } else {
                    config.platform_public_key = mask_secret(&rc.config_value);
                }
            }
            "notify_url" => config.notify_url = rc.config_value,
            _ => {}
        }
    }

    config.created_at = created_at.unwrap_or_else(chrono::Utc::now).to_rfc3339();
    config.updated_at = updated_at.unwrap_or_else(chrono::Utc::now).to_rfc3339();

    Ok(Some(config))
}

async fn save_wechat_config(
    state: &AppState,
    realm_id: &str,
    request: &WechatConfigRequest,
) -> Result<(), ApiError> {
    let config_items: Vec<(&str, &str, bool)> = vec![
        ("app_id", &request.app_id, false),
        ("mch_id", &request.mch_id, false),
        ("private_key", &request.private_key, true),
        ("serial_no", &request.serial_no, false),
        ("v3_key", &request.v3_key, true),
        ("platform_public_key", &request.platform_public_key, true),
        ("notify_url", &request.notify_url, false),
    ];

    let requests: Vec<UpsertRealmConfigRequest> = config_items
        .into_iter()
        .map(|(key, value, is_secret)| UpsertRealmConfigRequest {
            config_type: ConfigType::Wechat,
            config_key: key.to_string(),
            config_value: value.to_string(),
            is_secret: Some(is_secret),
            enabled: Some(true),
            metadata: None,
        })
        .collect();

    state
        .realm_config_repository
        .batch_upsert(realm_id, requests)
        .await
        .map_err(|e| {
            tracing::error!(realm_id = %realm_id, error = %e, "Failed to save wechat config");
            ApiError::internal(format!("Database error: {}", e))
        })?;

    Ok(())
}

async fn update_wechat_config_internal(
    state: &AppState,
    realm_id: &str,
    request: &WechatConfigUpdateRequest,
) -> Result<(), ApiError> {
    let mut updates: Vec<(&str, String)> = vec![];

    if let Some(app_id) = &request.app_id {
        updates.push(("app_id", app_id.clone()));
    }
    if let Some(mch_id) = &request.mch_id {
        updates.push(("mch_id", mch_id.clone()));
    }
    if let Some(private_key) = &request.private_key {
        updates.push(("private_key", private_key.clone()));
    }
    if let Some(serial_no) = &request.serial_no {
        updates.push(("serial_no", serial_no.clone()));
    }
    if let Some(v3_key) = &request.v3_key {
        updates.push(("v3_key", v3_key.clone()));
    }
    if let Some(platform_public_key) = &request.platform_public_key {
        updates.push(("platform_public_key", platform_public_key.clone()));
    }
    if let Some(notify_url) = &request.notify_url {
        updates.push(("notify_url", notify_url.clone()));
    }

    let requests: Vec<UpsertRealmConfigRequest> = updates
        .into_iter()
        .map(|(key, value)| UpsertRealmConfigRequest {
            config_type: ConfigType::Wechat,
            config_key: key.to_string(),
            config_value: value,
            is_secret: None,
            enabled: None,
            metadata: None,
        })
        .collect();

    if !requests.is_empty() {
        state
            .realm_config_repository
            .batch_upsert(realm_id, requests)
            .await
            .map_err(|e| {
                tracing::error!(realm_id = %realm_id, error = %e, "Failed to update wechat config");
                ApiError::internal(format!("Database error: {}", e))
            })?;
    }

    Ok(())
}

async fn delete_wechat_config_internal(state: &AppState, realm_id: &str) -> Result<(), ApiError> {
    state
        .realm_config_repository
        .delete_by_type(realm_id.to_string(), "wechat".to_string())
        .await
        .map_err(|e| {
            tracing::error!(realm_id = %realm_id, error = %e, "Failed to delete wechat config");
            ApiError::internal(format!("Database error: {}", e))
        })?;

    Ok(())
}

async fn count_active_wechat_subscriptions(db: &PgPool, realm_id: &str) -> Result<i64, ApiError> {
    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM subscription
        WHERE realm_id = $1
          AND payment_provider = 'wechat'
          AND status = 'active'
        "#,
    )
    .bind(realm_id)
    .fetch_one(db)
    .await
    .map_err(|e| {
        tracing::error!(realm_id = %realm_id, error = %e, "Failed to count wechat subscriptions");
        ApiError::internal(format!("Database error: {}", e))
    })?;

    Ok(count)
}

fn mask_v3_key(value: &str) -> String {
    if value.len() < 8 {
        return "****".to_string();
    }
    if value.len() == 8 {
        let prefix = &value[..4];
        return format!("{}***", prefix);
    }
    let prefix = &value[..4];
    format!("{}{}", prefix, "*".repeat(24))
}

fn mask_secret(_value: &str) -> String {
    "*********** (configured)".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_v3_key() {
        assert_eq!(
            mask_v3_key("0123456789abcdef0123456789abcdef"),
            "0123************************"
        );
        assert_eq!(mask_v3_key("short"), "****");
        assert_eq!(mask_v3_key("01234567"), "0123***");
    }

    #[test]
    fn test_mask_secret() {
        assert_eq!(mask_secret("any_value_here"), "*********** (configured)");
        assert_eq!(mask_secret(""), "*********** (configured)");
        assert_eq!(mask_secret("super_secret_key"), "*********** (configured)");
    }
}
