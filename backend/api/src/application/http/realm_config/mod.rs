// Realm configuration CRUD handlers

// Re-export public_helper from api-base
pub use herald_api_base::application::http::common::public_helper;

use axum::{
    Json,
    extract::{Extension, Path, State},
    http::StatusCode,
};
use herald_core::domain::authentication::Identity;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::application::http::server::api_entities::{ApiError, ErrorResponse};
use crate::application::http::state::AppState;
use herald_core::domain::authorization::PermissionService;
use herald_core::domain::realm_config::{
    BatchUpsertRealmConfigRequest, ConfigType, RealmConfig, RealmConfigService,
    UpsertRealmConfigRequest,
};

#[derive(Debug, Deserialize, Serialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpsertRealmConfigValidator {
    pub config_type: String,
    #[validate(length(min = 1))]
    pub config_key: String,
    #[validate(length(min = 1))]
    pub config_value: String,
    pub is_secret: Option<bool>,
    pub enabled: Option<bool>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct BatchUpsertRealmConfigValidator {
    #[validate(length(min = 1))]
    pub configs: Vec<UpsertRealmConfigValidator>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RealmConfigResponse {
    pub id: Uuid,
    pub realm_id: String,
    pub config_type: String,
    pub config_key: String,
    pub config_value: String,
    pub is_secret: bool,
    pub enabled: bool,
    pub metadata: Option<serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
}

// Helper function to convert domain entity to response
fn to_response(config: RealmConfig) -> RealmConfigResponse {
    RealmConfigResponse {
        id: config.id,
        realm_id: config.realm_id,
        config_type: config.config_type.into(),
        config_key: config.config_key,
        config_value: config.config_value,
        is_secret: config.is_secret,
        enabled: config.enabled,
        metadata: config.metadata,
        created_at: config.created_at.to_rfc3339(),
        updated_at: config.updated_at.to_rfc3339(),
    }
}

/// 获取指定 realm 的所有配置
#[utoipa::path(
    get,
    path = "/api/configs/{realmId}",
    tag = "realm_config",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    responses(
        (status = 200, description = "所有配置列表", body = Vec<RealmConfigResponse>),
        (status = 401, description = "未授权", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 500, description = "服务器错误", body = ErrorResponse)
    )
)]
pub async fn list_realm_configs(
    Path(realm_id): Path<String>,
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
) -> Result<Json<Vec<RealmConfigResponse>>, ApiError> {
    let realm_config_service = state.service.realm_config_service();

    // Extract identity from request extension (injected by inject_identity middleware)
    let identity_realm_id = identity.realm_id();
    let current_user_id = identity.user_id();

    tracing::debug!(
        realm_id = %identity_realm_id,
        user_id = %current_user_id,
        "Listing realm configs"
    );

    let configs = realm_config_service
        .get_all_configs(identity, realm_id)
        .await
        .map_err(|e| match e {
            herald_core::domain::common::entities::app_errors::CoreError::Forbidden(msg) => {
                ApiError::forbidden(msg)
            }
            _ => {
                tracing::error!("Failed to list realm configs: {e}");
                ApiError::internal("Failed to list realm configs")
            }
        })?;

    let responses = configs.into_iter().map(to_response).collect();
    Ok(Json(responses))
}

/// 获取指定类型的所有配置
#[utoipa::path(
    get,
    path = "/api/configs/{realmId}/{configType}",
    tag = "realm_config",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("configType" = String, Path, description = "配置类型 (oauth, turnstile, registration 等)")
    ),
    responses(
        (status = 200, description = "指定类型的配置列表", body = Vec<RealmConfigResponse>),
        (status = 401, description = "未授权", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 500, description = "服务器错误", body = ErrorResponse)
    )
)]
pub async fn list_realm_configs_by_type(
    Path((realm_id, config_type)): Path<(String, String)>,
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
) -> Result<Json<Vec<RealmConfigResponse>>, ApiError> {
    let realm_config_service = state.service.realm_config_service();

    // Extract identity from request extension (injected by inject_identity middleware)
    let identity_realm_id = identity.realm_id();
    let current_user_id = identity.user_id();

    tracing::debug!(
        realm_id = %identity_realm_id,
        user_id = %current_user_id,
        "Listing realm configs by type"
    );

    let configs = realm_config_service
        .get_configs_by_type(identity, realm_id, config_type)
        .await
        .map_err(|e| match e {
            herald_core::domain::common::entities::app_errors::CoreError::Forbidden(msg) => {
                ApiError::forbidden(msg)
            }
            _ => {
                tracing::error!("Failed to list realm configs by type: {e}");
                ApiError::internal("Failed to list realm configs by type")
            }
        })?;

    let responses = configs.into_iter().map(to_response).collect();
    Ok(Json(responses))
}

/// 获取单个配置
#[utoipa::path(
    get,
    path = "/api/configs/{realmId}/{configType}/{configKey}",
    tag = "realm_config",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("configType" = String, Path, description = "配置类型"),
        ("configKey" = String, Path, description = "配置键")
    ),
    responses(
        (status = 200, description = "配置详情", body = RealmConfigResponse),
        (status = 401, description = "未授权", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "未找到", body = ErrorResponse),
        (status = 500, description = "服务器错误", body = ErrorResponse)
    )
)]
pub async fn get_realm_config(
    Path((realm_id, config_type, config_key)): Path<(String, String, String)>,
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
) -> Result<Json<RealmConfigResponse>, ApiError> {
    let realm_config_service = state.service.realm_config_service();

    // Extract identity from request extension (injected by inject_identity middleware)
    let identity_realm_id = identity.realm_id();
    let current_user_id = identity.user_id();

    tracing::debug!(
        realm_id = %identity_realm_id,
        user_id = %current_user_id,
        "Getting realm config"
    );

    let config = realm_config_service
        .get_config(identity, realm_id, config_type, config_key)
        .await
        .map_err(|e| match e {
            herald_core::domain::common::entities::app_errors::CoreError::Forbidden(msg) => {
                ApiError::forbidden(msg)
            }
            herald_core::domain::common::entities::app_errors::CoreError::NotFound => {
                ApiError::not_found("Config not found")
            }
            _ => {
                tracing::error!("Failed to get realm config: {e}");
                ApiError::internal("Failed to get realm config")
            }
        })?
        .ok_or_else(|| ApiError::not_found("Config not found"))?;

    Ok(Json(to_response(config)))
}

/// 创建或更新配置（Upsert）
#[utoipa::path(
    put,
    path = "/api/configs/{realmId}",
    tag = "realm_config",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    request_body = UpsertRealmConfigValidator,
    responses(
        (status = 200, description = "配置已创建或更新", body = RealmConfigResponse),
        (status = 400, description = "请求参数错误", body = ErrorResponse),
        (status = 401, description = "未授权", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 500, description = "服务器错误", body = ErrorResponse)
    )
)]
pub async fn upsert_realm_config(
    Path(realm_id): Path<String>,
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Json(payload): Json<UpsertRealmConfigValidator>,
) -> Result<Json<RealmConfigResponse>, ApiError> {
    // 验证请求
    payload
        .validate()
        .map_err(|e| ApiError::bad_request(format!("验证错误: {}", e)))?;

    let realm_config_service = state.service.realm_config_service();

    // Extract identity from request extension (injected by inject_identity middleware)
    let identity_realm_id = identity.realm_id();
    let current_user_id = identity.user_id();

    tracing::debug!(
        realm_id = %identity_realm_id,
        user_id = %current_user_id,
        "Upserting realm config"
    );

    let request = UpsertRealmConfigRequest {
        config_type: ConfigType::from(payload.config_type),
        config_key: payload.config_key,
        config_value: payload.config_value,
        is_secret: payload.is_secret,
        enabled: payload.enabled,
        metadata: payload.metadata,
    };

    let config = realm_config_service
        .upsert_config(identity, realm_id, request)
        .await
        .map_err(|e| match e {
            herald_core::domain::common::entities::app_errors::CoreError::Forbidden(msg) => {
                ApiError::forbidden(msg)
            }
            _ => {
                tracing::error!("Failed to upsert realm config: {e}");
                ApiError::internal("Failed to upsert realm config")
            }
        })?;

    Ok(Json(to_response(config)))
}

/// 批量创建或更新配置
#[utoipa::path(
    post,
    path = "/api/configs/{realmId}/batch",
    tag = "realm_config",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    request_body = BatchUpsertRealmConfigRequest,
    responses(
        (status = 200, description = "批量配置已创建或更新", body = Vec<RealmConfigResponse>),
        (status = 400, description = "请求参数错误", body = ErrorResponse),
        (status = 401, description = "未授权", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 500, description = "服务器错误", body = ErrorResponse)
    )
)]
pub async fn batch_upsert_realm_configs(
    Path(realm_id): Path<String>,
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Json(payload): Json<BatchUpsertRealmConfigValidator>,
) -> Result<Json<Vec<RealmConfigResponse>>, ApiError> {
    tracing::debug!(
        realm_id = %realm_id,
        config_count = payload.configs.len(),
        "Received batch upsert request"
    );

    // 验证请求
    payload
        .validate()
        .map_err(|e| ApiError::bad_request(format!("验证错误: {}", e)))?;

    let realm_config_service = state.service.realm_config_service();

    // Extract identity from request extension (injected by inject_identity middleware)
    let identity_realm_id = identity.realm_id();
    let current_user_id = identity.user_id();

    tracing::debug!(
        realm_id = %identity_realm_id,
        user_id = %current_user_id,
        "Batch upserting realm configs"
    );

    let requests: Vec<UpsertRealmConfigRequest> = payload
        .configs
        .into_iter()
        .map(|r| UpsertRealmConfigRequest {
            config_type: ConfigType::from(r.config_type),
            config_key: r.config_key,
            config_value: r.config_value,
            is_secret: r.is_secret,
            enabled: r.enabled,
            metadata: r.metadata,
        })
        .collect();

    // Log the requests for debugging
    for (index, req) in requests.iter().enumerate() {
        tracing::debug!(
            index = index,
            realm_id = %realm_id,
            config_type = format!("{:?}", req.config_type),
            config_key = %req.config_key,
            enabled = req.enabled,
            is_secret = req.is_secret,
            "Batch upsert request item"
        );
    }

    let configs = realm_config_service
        .batch_upsert_configs(
            identity,
            realm_id.clone(),
            herald_core::domain::realm_config::BatchUpsertRealmConfigRequest { configs: requests },
        )
        .await
        .map_err(|e| match e {
            herald_core::domain::common::entities::app_errors::CoreError::Forbidden(msg) => {
                ApiError::forbidden(msg)
            }
            _ => {
                tracing::error!(realm_id = %realm_id, error = %e, "Failed to batch upsert realm configs");
                ApiError::internal("Failed to batch upsert realm configs")
            }
        })?;

    tracing::debug!(
        realm_id = %realm_id,
        config_count = configs.len(),
        "Batch upsert completed successfully"
    );

    // Invalidate realm cache after config update (fire-and-forget to avoid blocking response)
    let realm_id_clone = realm_id.clone();
    let permission_checker = state.permission_checker.clone();
    tokio::spawn(async move {
        let start = std::time::Instant::now();
        match permission_checker
            .invalidate_realm_cache(&realm_id_clone)
            .await
        {
            Ok(_) => {
                tracing::debug!(
                    realm_id = %realm_id_clone,
                    duration_ms = start.elapsed().as_millis(),
                    "Realm cache invalidated successfully"
                );
            }
            Err(e) => {
                tracing::warn!(
                    realm_id = %realm_id_clone,
                    error = %e,
                    "Failed to invalidate realm cache (non-critical)"
                );
            }
        }
    });

    let responses = configs.into_iter().map(to_response).collect();
    Ok(Json(responses))
}

/// 删除配置
#[utoipa::path(
    delete,
    path = "/api/configs/{realmId}/{configType}/{configKey}",
    tag = "realm_config",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("configType" = String, Path, description = "配置类型"),
        ("configKey" = String, Path, description = "配置键")
    ),
    responses(
        (status = 204, description = "配置已删除"),
        (status = 401, description = "未授权", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "未找到", body = ErrorResponse),
        (status = 500, description = "服务器错误", body = ErrorResponse)
    )
)]
pub async fn delete_realm_config(
    Path((realm_id, config_type, config_key)): Path<(String, String, String)>,
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
) -> Result<StatusCode, ApiError> {
    let realm_config_service = state.service.realm_config_service();

    // Extract identity from request extension (injected by inject_identity middleware)
    let identity_realm_id = identity.realm_id();
    let current_user_id = identity.user_id();

    tracing::debug!(
        realm_id = %identity_realm_id,
        user_id = %current_user_id,
        "Deleting realm config"
    );

    realm_config_service
        .delete_config(identity, realm_id, config_type, config_key)
        .await
        .map_err(|e| match e {
            herald_core::domain::common::entities::app_errors::CoreError::Forbidden(msg) => {
                ApiError::forbidden(msg)
            }
            herald_core::domain::common::entities::app_errors::CoreError::NotFound => {
                ApiError::not_found("Config not found")
            }
            _ => {
                tracing::error!("Failed to delete realm config: {e}");
                ApiError::internal("Failed to delete realm config")
            }
        })?;

    Ok(StatusCode::NO_CONTENT)
}
