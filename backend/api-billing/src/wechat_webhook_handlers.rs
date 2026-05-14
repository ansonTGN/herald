use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};
use utoipa::ToSchema;

use herald_api_base::application::http::server::api_entities::ApiError;
use herald_api_base::application::http::state::AppState;
use herald_core::domain::purchase::{CompletePaymentAttemptInput, PaymentCompletionSource};
use herald_core::infrastructure::wechat::{
    repository::WechatOrderRepository, subscription_service::WechatSubscriptionService,
};

#[utoipa::path(
    post,
    path = "/api/third/pay/{realmId}/wechat/webhooks",
    params(
        ("realmId" = String, Path, description = "Realm UUID")
    ),
    request_body = WechatWebhookPayload,
    responses(
        (status = 200, description = "Webhook processed successfully"),
        (status = 401, description = "Signature verification failed"),
        (status = 404, description = "WeChat configuration not found")
    ),
    tag = "billing.wechat-webhooks",
    operation_id = "wechat_webhook_handler"
)]
pub async fn wechat_webhook_handler(
    State(state): State<AppState>,
    Path(realm_id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, ApiError> {
    let repo = WechatOrderRepository::new(state.pool.clone());
    let config_row = repo
        .get_wechat_config(&realm_id)
        .await
        .map_err(|e| {
            error!(realm_id = %realm_id, error = %e, "Failed to load WeChat config for webhook");
            ApiError::internal("Failed to load configuration")
        })?
        .ok_or_else(|| {
            warn!(realm_id = %realm_id, "WeChat config not found for webhook");
            ApiError::not_found("WeChat configuration not found")
        })?;

    let timestamp = headers
        .get("Wechatpay-Timestamp")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            warn!("Missing Wechatpay-Timestamp header");
            ApiError::unauthorized("Missing signature headers")
        })?;

    let nonce = headers
        .get("Wechatpay-Nonce")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            warn!("Missing Wechatpay-Nonce header");
            ApiError::unauthorized("Missing signature headers")
        })?;

    let signature = headers
        .get("Wechatpay-Signature")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            warn!("Missing Wechatpay-Signature header");
            ApiError::unauthorized("Missing signature headers")
        })?;

    let body_str = String::from_utf8_lossy(&body);
    let message = format!("{}\n{}\n{}", timestamp, nonce, body_str);

    let v3_key = config_row.v3_key.ok_or_else(|| {
        error!("v3_key not found in config");
        ApiError::internal("Configuration error")
    })?;

    verify_signature(&message, signature, &v3_key).map_err(|e| {
        error!(error = %e, "Signature verification failed");
        ApiError::unauthorized("Invalid signature")
    })?;

    let webhook_data: WechatWebhookPayload = serde_json::from_slice(&body).map_err(|e| {
        error!(error = %e, "Failed to parse WeChat webhook JSON");
        ApiError::bad_request("Invalid webhook payload")
    })?;

    info!(
        realm_id = %realm_id,
        event_type = %webhook_data.event_type,
        "Processing WeChat webhook"
    );

    let decrypted_data = decrypt_resource_data(
        &webhook_data.resource.ciphertext.unwrap_or_default(),
        &webhook_data.resource.associated_data.unwrap_or_default(),
        &webhook_data.resource.nonce.unwrap_or_default(),
        &v3_key,
    )
    .map_err(|e| {
        error!(error = %e, "Failed to decrypt webhook resource data");
        ApiError::bad_request("Decryption failed")
    })?;

    let payment_data: WechatPaymentData = serde_json::from_str(&decrypted_data).map_err(|e| {
        error!(error = %e, "Failed to parse decrypted payment data");
        ApiError::bad_request("Invalid payment data")
    })?;

    let out_trade_no = payment_data.out_trade_no.clone().unwrap_or_default();
    if out_trade_no.is_empty() {
        return Err(ApiError::bad_request(
            "Missing out_trade_no in payment data",
        ));
    }

    let event_id = payment_data
        .transaction_id
        .clone()
        .unwrap_or_else(|| out_trade_no.clone());
    let existing_event = sqlx::query(
        "SELECT id FROM payment_event
         WHERE external_event_id = $1 AND payment_provider = 'wechat'",
    )
    .bind(&event_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        error!(error = %e, "Failed to check for existing payment event");
        ApiError::internal("Database error")
    })?;

    if existing_event.is_some() {
        info!(
            event_id = %event_id,
            "Webhook already processed (idempotent)"
        );
        return Ok(StatusCode::OK);
    }

    let order = repo
        .find_order_by_out_trade_no(&out_trade_no)
        .await
        .map_err(|e| {
            error!(
                out_trade_no = %out_trade_no,
                error = %e,
                "Failed to find order by out_trade_no"
            );
            ApiError::internal("Database error")
        })?
        .ok_or_else(|| {
            warn!(
                out_trade_no = %out_trade_no,
                "Order not found for webhook"
            );
            ApiError::not_found("Order not found")
        })?;

    match webhook_data.event_type.as_str() {
        "TRANSACTION.SUCCESS" => {
            let amount = payment_data.amount.as_ref().map(|a| a.total).unwrap_or(0) as i32;
            if amount != order.amount {
                error!(
                    out_trade_no = %out_trade_no,
                    expected = order.amount,
                    actual = amount,
                    "Webhook amount mismatch"
                );
                return Err(ApiError::bad_request("Amount mismatch"));
            }

            let transaction_id = payment_data.transaction_id.unwrap_or_default();
            repo.mark_order_paid(order.id, &transaction_id)
                .await
                .map_err(|e| {
                    error!(
                        order_id = %order.id,
                        error = %e,
                        "Failed to mark order as paid"
                    );
                    ApiError::internal("Failed to update order")
                })?;

            let attempt = state
                .payment_attempt_service
                .get_payment_attempt_by_provider_reference("wechat", &out_trade_no)
                .await
                .map_err(|e| {
                    error!(
                        out_trade_no = %out_trade_no,
                        error = %e,
                        "Failed to load payment attempt by provider reference"
                    );
                    ApiError::internal("Failed to load payment attempt")
                })?;

            if let Some(attempt) = attempt {
                state
                    .purchase_service
                    .complete_succeeded_payment_attempt(CompletePaymentAttemptInput {
                        attempt_id: attempt.id,
                        provider_status: "succeeded".to_string(),
                        provider_transaction_id: transaction_id.clone(),
                        completed_at: chrono::Utc::now(),
                        source: PaymentCompletionSource::ProviderWebhook {
                            provider: "wechat".to_string(),
                        },
                    })
                    .await
                    .map_err(|e| {
                        error!(
                            attempt_id = %attempt.id,
                            error = %e,
                            "Failed to complete WeChat payment attempt"
                        );
                        ApiError::internal("Failed to complete payment attempt")
                    })?;

                info!(
                    attempt_id = %attempt.id,
                    transaction_id = %transaction_id,
                    "WeChat payment fulfilled through payment attempt"
                );
            } else {
                let subscription_service = WechatSubscriptionService::new(state.pool.clone());
                subscription_service
                    .create_subscription_and_grant_points(
                        &realm_id,
                        order.user_id,
                        order.plan_id,
                        &transaction_id,
                        payment_data.amount.as_ref().map(|a| a.total).unwrap_or(0) as i32,
                    )
                    .await
                    .map_err(|e| {
                        error!(
                            order_id = %order.id,
                            error = %e,
                            "Failed to create subscription and grant points"
                        );
                        ApiError::internal("Failed to process payment")
                    })?;

                info!(
                    order_id = %order.id,
                    transaction_id = %transaction_id,
                    "WeChat payment successful and subscription created"
                );
            }
        }
        "TRANSACTION.CLOSED" => {
            repo.mark_order_closed(order.id).await.map_err(|e| {
                error!(
                    order_id = %order.id,
                    error = %e,
                    "Failed to mark order as closed"
                );
                ApiError::internal("Failed to update order")
            })?;

            info!(order_id = %order.id, "WeChat order closed");
        }
        _ => {
            info!(
                event_type = %webhook_data.event_type,
                "Unhandled WeChat webhook event type"
            );
        }
    }

    Ok(StatusCode::OK)
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WechatWebhookPayload {
    pub event_type: String,
    pub resource: WebhookResource,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WebhookResource {
    pub out_trade_no: Option<String>,
    pub transaction_id: Option<String>,
    pub amount: Option<WebhookAmount>,
    pub ciphertext: Option<String>,
    pub nonce: Option<String>,
    pub associated_data: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WebhookAmount {
    pub total: i32,
    pub currency: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WechatPaymentData {
    pub out_trade_no: Option<String>,
    pub transaction_id: Option<String>,
    #[serde(default)]
    pub amount: Option<WechatAmount>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WechatAmount {
    pub total: i64,
}

fn verify_signature(
    message: &str,
    signature: &str,
    v3_key: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use base64::Engine;
    use hmac::Mac;
    use subtle::ConstantTimeEq;

    let decoded_signature = base64::engine::general_purpose::STANDARD.decode(signature)?;

    let mut mac = <hmac::Hmac<sha2::Sha256> as hmac::Mac>::new_from_slice(v3_key.as_bytes())?;
    mac.update(message.as_bytes());
    let expected_signature = mac.finalize().into_bytes();

    if decoded_signature.ct_eq(&expected_signature).into() {
        Ok(())
    } else {
        Err("Signature verification failed".into())
    }
}

fn decrypt_resource_data(
    ciphertext: &str,
    associated_data: &str,
    nonce: &str,
    api_v3_key: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    use aes_gcm::aead::{Aead, KeyInit};
    use base64::Engine;

    let key_bytes = md5::compute(api_v3_key).0;

    let ciphertext_bytes = base64::engine::general_purpose::STANDARD.decode(ciphertext)?;
    let nonce_bytes = base64::engine::general_purpose::STANDARD.decode(nonce)?;

    let cipher = aes_gcm::Aes128Gcm::new_from_slice(&key_bytes)?;
    let nonce_bytes = &nonce_bytes[..nonce_bytes.len().min(12)];
    let nonce = aes_gcm::Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext_bytes.as_slice())
        .map_err(|e| format!("Decryption failed: {}", e))?;

    let ad_len = associated_data.len();
    let decrypted_data = if ad_len > 0 && plaintext.len() > ad_len {
        String::from_utf8(plaintext[ad_len..].to_vec())?
    } else {
        String::from_utf8(plaintext)?
    };

    Ok(decrypted_data)
}
