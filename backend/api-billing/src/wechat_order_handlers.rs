use axum::Json;
use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use chrono::Utc;
use herald_core::domain::authentication::Identity;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::shopify_config_types::GenericErrorResponse;
use crate::wechat_config_types::{
    WechatOrderCreateRequest, WechatOrderCreateResponse, WechatOrderStatusResponse,
};
use herald_api_base::application::http::common::auth_utils::require_authenticated_user_in_realm;
use herald_api_base::application::http::server::api_entities::ApiError;
use herald_api_base::application::http::state::AppState;
use herald_core::domain::billing::BillingRepository;
use herald_core::infrastructure::wechat::models::{CreateOrderParams, WechatPaymentOrder};
use herald_core::infrastructure::wechat::repository::WechatOrderRepository;

#[utoipa::path(
    post,
    path = "/api/third/pay/{realmId}/wechat/create-order",
    params(
        ("realmId" = String, Path, description = "Realm UUID")
    ),
    request_body = WechatOrderCreateRequest,
    responses(
        (status = 201, description = "Order created successfully", body = WechatOrderCreateResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Plan or WeChat configuration not found")
    ),
    tag = "billing.wechat-orders",
    operation_id = "create_wechat_order"
)]
pub async fn create_wechat_order(
    State(state): State<AppState>,
    Path(realm_id): Path<String>,
    Extension(identity): Extension<Identity>,
    Json(request): Json<WechatOrderCreateRequest>,
) -> Result<(StatusCode, Json<WechatOrderCreateResponse>), ApiError> {
    let user_id = require_authenticated_user_in_realm(&identity, &realm_id, "WeChat orders")?;

    let plan_id = request
        .plan_id
        .parse::<Uuid>()
        .map_err(|_| ApiError::bad_request("Invalid plan ID format"))?;

    let client_app_id = request
        .client_app_id
        .as_deref()
        .map(|s| s.parse::<Uuid>())
        .transpose()
        .map_err(|_| ApiError::bad_request("Invalid client app ID format"))?;

    let repo = WechatOrderRepository::new(state.pool.clone());
    let config_row = repo
        .get_wechat_config(&realm_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to load WeChat config: {}", e)))?
        .ok_or_else(|| ApiError::not_found("WeChat configuration not found for this realm"))?;

    let app_id = config_row
        .app_id
        .ok_or_else(|| ApiError::internal("WeChat config missing app_id"))?;
    let mch_id = config_row
        .mch_id
        .ok_or_else(|| ApiError::internal("WeChat config missing mch_id"))?;
    let private_key = config_row
        .private_key
        .ok_or_else(|| ApiError::internal("WeChat config missing private_key"))?;
    let serial_no = config_row
        .serial_no
        .ok_or_else(|| ApiError::internal("WeChat config missing serial_no"))?;
    let v3_key = config_row
        .v3_key
        .ok_or_else(|| ApiError::internal("WeChat config missing v3_key"))?;
    let notify_url = config_row
        .notify_url
        .ok_or_else(|| ApiError::internal("WeChat config missing notify_url"))?;

    let mock_base_url = config_row.mock_base_url;

    let client = herald_core::infrastructure::wechat::client::WechatPayClient::new_async(
        app_id,
        mch_id,
        private_key,
        serial_no,
        v3_key,
        notify_url.clone(),
        mock_base_url,
    )
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create WeChat Pay client: {}", e)))?;

    let plan = state
        .billing_repository
        .find_public_plan_by_id(&realm_id, plan_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to find plan: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Plan not found"))?;

    let amount = plan.price;

    let description = format!("Herald Plan: {}", plan.name);

    let create_params = CreateOrderParams {
        realm_id: realm_id.clone(),
        user_id,
        plan_id,
        client_app_id,
        amount,
        currency: "CNY".to_string(),
        description: description.clone(),
        notify_url: notify_url.clone(),
    };

    let result = tokio::task::spawn_blocking(move || client.create_native_order(&create_params))
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to spawn blocking task for WeChat native order");
            ApiError::internal(format!("WeChat Pay order creation task failed: {}", e))
        })?
        .map_err(|e| {
            error!(error = %e, "Failed to create WeChat native order");
            ApiError::internal(format!("WeChat Pay order creation failed: {}", e))
        })?;

    let order = WechatPaymentOrder {
        id: result.order_id,
        realm_id: realm_id.clone(),
        user_id,
        plan_id,
        client_app_id,
        out_trade_no: result.out_trade_no.clone(),
        transaction_id: None,
        amount,
        currency: "CNY".to_string(),
        code_url: result.code_url.clone(),
        status: herald_core::infrastructure::wechat::models::WechatOrderStatus::Pending,
        description: Some(description),
        paid_at: None,
        closed_at: None,
        expires_at: result.expires_at,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    repo.create_order(&order).await.map_err(|e| {
        error!(error = %e, "Failed to save WeChat order to database");
        ApiError::internal(format!("Failed to save order: {}", e))
    })?;

    info!(
        realm_id = %realm_id,
        order_id = %result.order_id,
        out_trade_no = %result.out_trade_no,
        "WeChat payment order created"
    );

    Ok((
        StatusCode::CREATED,
        Json(WechatOrderCreateResponse {
            order_id: result.order_id.to_string(),
            out_trade_no: result.out_trade_no,
            code_url: result.code_url,
            expires_at: result.expires_at.to_rfc3339(),
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/api/third/pay/{realmId}/wechat/order-status/{orderId}",
    params(
        ("realmId" = String, Path, description = "Realm UUID"),
        ("orderId" = String, Path, description = "Order UUID")
    ),
    responses(
        (status = 200, description = "Order status retrieved", body = WechatOrderStatusResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Order not found")
    ),
    tag = "billing.wechat-orders",
    operation_id = "get_wechat_order_status"
)]
pub async fn get_wechat_order_status(
    State(state): State<AppState>,
    Path((realm_id, order_id)): Path<(String, String)>,
    Extension(identity): Extension<Identity>,
) -> Result<Json<WechatOrderStatusResponse>, ApiError> {
    let identity_user_id =
        require_authenticated_user_in_realm(&identity, &realm_id, "WeChat orders")?;

    let order_uuid = order_id
        .parse::<Uuid>()
        .map_err(|_| ApiError::bad_request("Invalid order ID format"))?;

    let repo = WechatOrderRepository::new(state.pool.clone());
    let order = repo
        .find_order_by_id(order_uuid)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Order not found"))?;

    if order.realm_id != realm_id {
        return Err(ApiError::not_found("Order not found"));
    }

    if order.user_id != identity_user_id {
        return Err(ApiError::forbidden("Access denied"));
    }

    Ok(Json(WechatOrderStatusResponse {
        order_id: order.id.to_string(),
        status: order.status.to_string(),
        trade_state: None,
    }))
}

#[utoipa::path(
    post,
    path = "/api/third/pay/{realmId}/wechat/close-order/{orderId}",
    params(
        ("realmId" = String, Path, description = "Realm UUID"),
        ("orderId" = String, Path, description = "Order UUID")
    ),
    responses(
        (status = 200, description = "Order closed successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Order not found"),
        (status = 409, description = "Order cannot be closed", body = GenericErrorResponse)
    ),
    tag = "billing.wechat-orders",
    operation_id = "close_wechat_order"
)]
pub async fn close_wechat_order(
    State(state): State<AppState>,
    Path((realm_id, order_id)): Path<(String, String)>,
    Extension(identity): Extension<Identity>,
) -> Result<StatusCode, ApiError> {
    let identity_user_id =
        require_authenticated_user_in_realm(&identity, &realm_id, "WeChat orders")?;

    let order_uuid = order_id
        .parse::<Uuid>()
        .map_err(|_| ApiError::bad_request("Invalid order ID format"))?;

    let repo = WechatOrderRepository::new(state.pool.clone());
    let order = repo
        .find_order_by_id(order_uuid)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Order not found"))?;

    if order.realm_id != realm_id {
        return Err(ApiError::not_found("Order not found"));
    }

    if order.user_id != identity_user_id {
        return Err(ApiError::forbidden("Access denied"));
    }

    if order.status != herald_core::infrastructure::wechat::models::WechatOrderStatus::Pending {
        return Err(ApiError::conflict_json(GenericErrorResponse {
            error: "order_not_closable".to_string(),
            message: format!("Order with status '{}' cannot be closed", order.status),
        }));
    }

    let config_row = repo
        .get_wechat_config(&realm_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to load WeChat config: {}", e)))?
        .ok_or_else(|| ApiError::internal("WeChat configuration not found"))?;

    let app_id = config_row
        .app_id
        .ok_or_else(|| ApiError::internal("Missing app_id"))?;
    let mch_id = config_row
        .mch_id
        .ok_or_else(|| ApiError::internal("Missing mch_id"))?;
    let private_key = config_row
        .private_key
        .ok_or_else(|| ApiError::internal("Missing private_key"))?;
    let serial_no = config_row
        .serial_no
        .ok_or_else(|| ApiError::internal("Missing serial_no"))?;
    let v3_key = config_row
        .v3_key
        .ok_or_else(|| ApiError::internal("Missing v3_key"))?;
    let notify_url = config_row
        .notify_url
        .ok_or_else(|| ApiError::internal("Missing notify_url"))?;

    let mock_base_url = config_row.mock_base_url;

    let client = herald_core::infrastructure::wechat::client::WechatPayClient::new_async(
        app_id,
        mch_id,
        private_key,
        serial_no,
        v3_key,
        notify_url,
        mock_base_url,
    )
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create WeChat Pay client: {}", e)))?;

    client.close_order(&order.out_trade_no).await.map_err(|e| {
        warn!(order_id = %order_uuid, error = %e, "Failed to close order on WeChat side");
        ApiError::internal(format!("WeChat close order failed: {}", e))
    })?;

    repo.mark_order_closed(order_uuid).await.map_err(|e| {
        error!(order_id = %order_uuid, error = %e, "Failed to mark order as closed");
        ApiError::internal(format!("Database error: {}", e))
    })?;

    info!(
        realm_id = %realm_id,
        order_id = %order_uuid,
        "WeChat payment order closed"
    );

    Ok(StatusCode::NO_CONTENT)
}
