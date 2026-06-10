use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};
use utoipa::ToSchema;

use herald_api_base::application::http::server::api_entities::ApiError;
use herald_api_base::application::http::state::AppState;
use herald_core::domain::billing::BillingRepository;
use herald_core::domain::purchase::{CompletePaymentAttemptInput, PaymentCompletionSource};
use herald_core::infrastructure::wechat::{
    WechatOrderStatus, repository::WechatOrderRepository,
    subscription_service::WechatSubscriptionService,
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
    let platform_public_key = config_row.platform_public_key.ok_or_else(|| {
        error!("platform_public_key not found in config");
        ApiError::internal("Configuration error")
    })?;

    verify_signature(&message, signature, &platform_public_key).map_err(|e| {
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
    let existing_event = state
        .billing_repository
        .find_payment_event_by_external_id(&event_id, "wechat")
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

    match (
        webhook_data.event_type.as_str(),
        payment_data.trade_state.as_deref(),
    ) {
        ("TRANSACTION.SUCCESS", Some("SUCCESS") | None) => {
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

            let transaction_id = payment_data
                .transaction_id
                .filter(|s| !s.is_empty())
                .ok_or_else(|| {
                    error!(
                        out_trade_no = %out_trade_no,
                        "Webhook missing transaction_id for successful payment"
                    );
                    ApiError::bad_request("Missing transaction_id")
                })?;
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
                        billing_type_override: None,
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
        ("TRANSACTION.SUCCESS", Some("REFUND")) => {
            let transaction_id = payment_data.transaction_id.as_deref();
            repo.update_order_status(order.id, WechatOrderStatus::Refunded, transaction_id)
                .await
                .map_err(|e| {
                    error!(
                        order_id = %order.id,
                        error = %e,
                        "Failed to mark order as refunded"
                    );
                    ApiError::internal("Failed to update order")
                })?;

            info!(order_id = %order.id, "WeChat order refunded");
        }
        ("TRANSACTION.SUCCESS", Some("CLOSED" | "REVOKED" | "PAYERROR"))
        | ("TRANSACTION.CLOSED", _) => {
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
        ("TRANSACTION.SUCCESS", Some("NOTPAY" | "USERPAYING" | "ACCEPT")) => {
            info!(
                order_id = %order.id,
                trade_state = ?payment_data.trade_state,
                "WeChat order remains pending"
            );
        }
        _ => {
            info!(
                event_type = %webhook_data.event_type,
                trade_state = ?payment_data.trade_state,
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
    pub trade_state: Option<String>,
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
    platform_public_key_pem: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use base64::Engine;
    use rsa::RsaPublicKey;
    use rsa::pkcs1::DecodeRsaPublicKey;
    use rsa::pkcs1v15::Pkcs1v15Sign;
    use rsa::pkcs8::DecodePublicKey;
    use sha2::{Digest, Sha256};

    let decoded_signature = base64::engine::general_purpose::STANDARD.decode(signature)?;
    let public_key = RsaPublicKey::from_public_key_pem(platform_public_key_pem)
        .or_else(|_| RsaPublicKey::from_pkcs1_pem(platform_public_key_pem))?;
    let digest = Sha256::digest(message.as_bytes());

    public_key.verify(Pkcs1v15Sign::new::<Sha256>(), &digest, &decoded_signature)?;

    Ok(())
}

fn decrypt_resource_data(
    ciphertext: &str,
    associated_data: &str,
    nonce: &str,
    api_v3_key: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    use aes_gcm::aead::Payload;
    use aes_gcm::aead::{Aead, KeyInit};
    use base64::Engine;

    let ciphertext_bytes = base64::engine::general_purpose::STANDARD.decode(ciphertext)?;
    let nonce_bytes = nonce.as_bytes();
    if nonce_bytes.len() != 12 {
        return Err(format!(
            "Invalid nonce length: expected 12 bytes, got {}",
            nonce_bytes.len()
        )
        .into());
    }

    let cipher = aes_gcm::Aes256Gcm::new_from_slice(api_v3_key.as_bytes())?;
    let nonce = aes_gcm::Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(
            nonce,
            Payload {
                msg: ciphertext_bytes.as_slice(),
                aad: associated_data.as_bytes(),
            },
        )
        .map_err(|e| format!("Decryption failed: {}", e))?;

    Ok(String::from_utf8(plaintext)?)
}
