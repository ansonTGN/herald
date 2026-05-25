//! Shopify webhook event handlers

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use serde_json::Value;
use tracing::{error, info, warn};

use herald_core::application::{WebhookContext, WebhookProcessResult};

use crate::shopify_webhook_utils::verify_webhook_hmac;
use herald_api_base::application::http::state::AppState;
use herald_core::domain::common::entities::app_errors::CoreError;
use herald_core::domain::points::PointsRepository;
use herald_core::domain::points::subscription_service::CancelMode;
use herald_core::infrastructure::shopify::ShopifyRepository;

#[utoipa::path(
    post,
    path = "/api/third/pay/{realmId}/shopify/webhooks",
    params(
        ("realmId" = String, Path, description = "Realm UUID (tenant identifier)")
    ),
    request_body = Value,
    responses(
        (status = 202, description = "Event accepted for processing"),
        (status = 401, description = "HMAC signature verification failed"),
        (status = 404, description = "Realm or Shopify configuration not found"),
        (status = 400, description = "Invalid request (missing headers or malformed JSON)")
    ),
    tag = "shopify_webhooks",
    summary = "Shopify webhook endpoint",
    description = "Receives and processes webhook events from Shopify subscription billing."
)]
#[allow(clippy::too_many_lines)]
pub async fn shopify_webhook_handler(
    Path(realm_id): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, CoreError> {
    let shopify_topic = headers
        .get("x-shopify-topic")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| CoreError::BadRequest("Missing X-Shopify-Topic header".to_string()))?;

    let event_id = headers
        .get("x-shopify-event-id")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| CoreError::BadRequest("Missing X-Shopify-Event-Id header".to_string()))?;

    let shopify_hmac = headers
        .get("x-shopify-hmac-sha256")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| CoreError::BadRequest("Missing X-Shopify-Hmac-SHA256 header".to_string()))?;

    let shopify_repo = ShopifyRepository::new(state.pool.clone());

    let client_secret = shopify_repo.get_client_secret(&realm_id).await?;

    verify_webhook_hmac(&body, shopify_hmac, &client_secret)?;

    let payload: Value = serde_json::from_slice(&body)
        .map_err(|e| CoreError::BadRequest(format!("Invalid JSON: {}", e)))?;

    let context = WebhookContext {
        realm_id: realm_id.clone(),
        external_event_id: event_id.to_string(),
        payment_provider: "shopify".to_string(),
        event_type: shopify_topic.to_string(),
        payload: payload.clone(),
    };

    let webhook_service = state.webhook_service.clone();
    let handler_state = state.clone();

    let result = webhook_service
        .process_webhook_with_idempotency(context, move |ctx| {
            let state = handler_state.clone();
            let realm_id = ctx.realm_id.clone();
            let event_type = ctx.event_type.clone();
            let payload = ctx.payload.clone();

            async move {
                handle_webhook_event_internal(state, realm_id, &event_type, payload).await
            }
        })
        .await?;

    match result {
        WebhookProcessResult::Processed { event_id } => {
            info!(
                "Webhook event processed successfully: event_id={}, topic={}",
                event_id, shopify_topic
            );
        }
        WebhookProcessResult::Skipped { event_id } => {
            info!(
                "Idempotent webhook event ignored: event_id={}, topic={}",
                event_id, shopify_topic
            );
        }
        WebhookProcessResult::InProgress { event_id } => {
            info!(
                "Webhook event already in progress: event_id={}, topic={}",
                event_id, shopify_topic
            );
        }
    }

    Ok(StatusCode::ACCEPTED)
}

async fn handle_webhook_event_internal(
    state: AppState,
    realm_id: String,
    topic: &str,
    payload: Value,
) -> Result<(), CoreError> {
    match topic {
        "subscription_contracts/create" => {
            handle_subscription_contracts_create(state, realm_id, payload).await
        }
        "subscription_contracts/update" => {
            handle_subscription_contracts_update(state, realm_id, payload).await
        }
        "subscription_billing_attempts/success" => {
            handle_billing_attempt_success(state, realm_id, payload).await
        }
        "subscription_billing_attempts/failure" => {
            handle_billing_attempt_failure(state, realm_id, payload).await
        }
        "refunds/create" => handle_refunds_create(state, realm_id, payload).await,
        "orders/paid" => handle_orders_paid(state, realm_id, payload).await,
        "app/uninstalled" => handle_app_uninstalled(state, realm_id).await,
        _ => {
            warn!("Unsupported webhook topic: {}", topic);
            Ok(())
        }
    }
}

async fn handle_orders_paid(
    state: AppState,
    realm_id: String,
    payload: Value,
) -> Result<(), CoreError> {
    let order_id = payload["admin_graphql_api_id"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| {
            payload["id"]
                .as_i64()
                .filter(|&id| id > 0)
                .map(|id| id.to_string())
        })
        .ok_or_else(|| CoreError::BadRequest("Missing Shopify order id".to_string()))?;

    let shopify_repo = ShopifyRepository::new(state.pool.clone());
    let Some((subscription_id, _customer_id)) =
        shopify_repo.find_binding_by_order_id(&order_id).await?
    else {
        warn!(
            realm_id = %realm_id,
            order_id = %order_id,
            "Shopify orders/paid webhook has no subscription binding"
        );
        return Ok(());
    };

    let subscription_record = shopify_repo
        .find_subscription_with_user(subscription_id)
        .await?
        .ok_or(CoreError::NotFound)?;
    if let (Some(user_id), Some(plan_id)) =
        (subscription_record.user_id, subscription_record.plan_id)
    {
        let period_end = subscription_record
            .current_period_end
            .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::days(30));
        state
            .subscription_service
            .handle_subscription_paid(
                user_id,
                &realm_id,
                plan_id,
                true,
                period_end,
                format!("shopify_order_paid_{}", order_id),
            )
            .await?;
    }

    info!(
        realm_id = %realm_id,
        order_id = %order_id,
        subscription_id = %subscription_id,
        "Processed Shopify orders/paid webhook"
    );

    Ok(())
}

async fn handle_app_uninstalled(state: AppState, realm_id: String) -> Result<(), CoreError> {
    let mut tx = state.pool.begin().await.map_err(|e| {
        CoreError::DatabaseError(format!("Failed to begin Shopify uninstall cleanup: {}", e))
    })?;

    sqlx::query(
        "UPDATE subscription
         SET status = 'canceled', updated_at = NOW()
         WHERE realm_id = $1 AND payment_provider = 'shopify'
           AND status IN ('active', 'trialing', 'past_due', 'scheduled_cancel', 'pending')",
    )
    .bind(&realm_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        CoreError::DatabaseError(format!("Failed to cancel Shopify subscriptions: {}", e))
    })?;

    sqlx::query("DELETE FROM shopify_subscription_binding WHERE realm_id = $1")
        .bind(&realm_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            CoreError::DatabaseError(format!("Failed to delete Shopify bindings: {}", e))
        })?;

    sqlx::query("DELETE FROM shopify_user_binding WHERE realm_id = $1")
        .bind(&realm_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            CoreError::DatabaseError(format!("Failed to delete Shopify user bindings: {}", e))
        })?;

    sqlx::query("DELETE FROM realm_config WHERE realm_id = $1 AND config_type = 'shopify'")
        .bind(&realm_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| CoreError::DatabaseError(format!("Failed to delete Shopify config: {}", e)))?;

    tx.commit().await.map_err(|e| {
        CoreError::DatabaseError(format!("Failed to commit Shopify uninstall cleanup: {}", e))
    })?;

    info!(realm_id = %realm_id, "Processed Shopify app/uninstalled cleanup");

    Ok(())
}

async fn handle_subscription_contracts_create(
    state: AppState,
    realm_id: String,
    payload: Value,
) -> Result<(), CoreError> {
    use herald_core::infrastructure::shopify::models::parse_subscription_contract_payload;

    info!(
        realm_id = %realm_id,
        "Processing subscription_contracts/create webhook"
    );

    let shopify_repo = ShopifyRepository::new(state.pool.clone());

    let contract = parse_subscription_contract_payload(&payload)
        .map_err(|e| CoreError::BadRequest(format!("Invalid Shopify contract payload: {}", e)))?;

    let herald_plan_id = contract.herald_plan_id.ok_or_else(|| {
        CoreError::BadRequest("Missing casPlanId in contract attributes".to_string())
    })?;

    state
        .points_repository
        .find_plan_config(&realm_id, herald_plan_id)
        .await?
        .ok_or_else(|| CoreError::SubscriptionPlanNotFound {
            realm_id: realm_id.clone(),
            plan_id: herald_plan_id.to_string(),
        })?;

    let client_app_id = contract.herald_client_app_id;
    let shop_domain = shopify_repo.get_shop_domain(&realm_id).await?;

    let resolved_user_id = if let Some(herald_user_id) = contract.herald_user_id {
        shopify_repo
            .upsert_user_binding(
                &realm_id,
                &shop_domain,
                &contract.customer_id,
                Some(&contract.customer_id),
                herald_user_id,
            )
            .await?;
        Some(herald_user_id)
    } else if let Some(binding) = shopify_repo
        .find_user_binding_by_customer_id(&realm_id, &shop_domain, &contract.customer_id)
        .await?
    {
        Some(binding.user_id)
    } else {
        None
    };

    let subscription_id = shopify_repo
        .create_subscription(
            &realm_id,
            &contract,
            resolved_user_id,
            herald_plan_id,
            client_app_id,
            "monthly",
        )
        .await?;

    let binding_id = shopify_repo
        .create_binding(subscription_id, &realm_id, &shop_domain, &contract)
        .await?;

    info!(
        realm_id = %realm_id,
        user_id = ?resolved_user_id,
        plan_id = %herald_plan_id,
        subscription_id = %subscription_id,
        binding_id = %binding_id,
        contract_id = %contract.id,
        "Created Shopify subscription binding and subscription record"
    );

    if let Some(user_id) = resolved_user_id {
        state
            .subscription_service
            .handle_subscription_paid(
                user_id,
                &realm_id,
                herald_plan_id,
                false,
                contract.current_period_end,
                format!("shopify_create_{}", contract.id),
            )
            .await
            .map_err(|e| {
                error!(
                    realm_id = %realm_id,
                    user_id = %user_id,
                    error = %e,
                    "Failed to grant initial subscription points"
                );
                e
            })?;

        info!(
            realm_id = %realm_id,
            user_id = %user_id,
            plan_id = %herald_plan_id,
            period_end = %contract.current_period_end,
            "Granted initial subscription points"
        );
    } else {
        info!(
            realm_id = %realm_id,
            subscription_id = %subscription_id,
            contract_id = %contract.id,
            "Shopify subscription created but remains unclaimed"
        );
    }

    Ok(())
}

async fn handle_subscription_contracts_update(
    state: AppState,
    realm_id: String,
    payload: Value,
) -> Result<(), CoreError> {
    use herald_core::infrastructure::shopify::models::parse_subscription_contract_payload;

    info!(
        realm_id = %realm_id,
        "Processing subscription_contracts/update webhook"
    );

    let shopify_repo = ShopifyRepository::new(state.pool.clone());

    let contract = parse_subscription_contract_payload(&payload)
        .map_err(|e| CoreError::BadRequest(format!("Invalid Shopify contract payload: {}", e)))?;

    let herald_plan_id = contract.herald_plan_id.ok_or_else(|| {
        CoreError::BadRequest("Missing casPlanId in contract attributes".to_string())
    })?;

    let existing_binding = shopify_repo
        .find_binding_by_contract_id(&contract.id)
        .await?
        .ok_or_else(|| {
            warn!(
                contract_id = %contract.id,
                "Shopify binding not found for contract update"
            );
            CoreError::NotFound
        })?;

    let (binding_id, subscription_id, current_revision, _customer_id) = existing_binding;
    let new_revision_id = contract
        .contract_revision_id
        .unwrap_or(current_revision + 1);

    if new_revision_id <= current_revision {
        warn!(
            contract_id = %contract.id,
            current_revision = %current_revision,
            new_revision = %new_revision_id,
            "Ignoring out-of-order contract update"
        );
        return Ok(());
    }

    let subscription = shopify_repo
        .find_subscription_by_external_id(&contract.id)
        .await?
        .ok_or_else(|| {
            error!(contract_id = %contract.id, "Subscription not found");
            CoreError::NotFound
        })?;

    let (_sub_id, old_plan_id_option, _sub_realm_id) = subscription;
    let subscription_record = shopify_repo
        .find_subscription_with_user(_sub_id)
        .await?
        .ok_or(CoreError::NotFound)?;
    let mapped_status = map_shopify_status(&contract.status);
    let previous_status = subscription_record.status.clone();

    let old_plan_id = old_plan_id_option.ok_or_else(|| {
        error!(contract_id = %contract.id, "Subscription missing plan_id");
        CoreError::BadRequest("Subscription missing plan_id".to_string())
    })?;

    shopify_repo
        .update_subscription_plan(_sub_id, herald_plan_id, contract.current_period_end)
        .await?;
    shopify_repo
        .update_subscription_status(_sub_id, mapped_status)
        .await?;

    shopify_repo
        .update_binding_revision(binding_id, new_revision_id)
        .await?;

    let old_plan_config = state
        .points_repository
        .find_plan_config(&realm_id, old_plan_id)
        .await?
        .ok_or_else(|| CoreError::SubscriptionPlanNotFound {
            realm_id: realm_id.clone(),
            plan_id: old_plan_id.to_string(),
        })?;

    let new_plan_config = state
        .points_repository
        .find_plan_config(&realm_id, herald_plan_id)
        .await?
        .ok_or_else(|| CoreError::SubscriptionPlanNotFound {
            realm_id: realm_id.clone(),
            plan_id: herald_plan_id.to_string(),
        })?;

    let Some(user_id) = subscription_record.user_id else {
        info!(
            realm_id = %realm_id,
            subscription_id = %subscription_id,
            new_plan_id = %herald_plan_id,
            status = %mapped_status,
            "Updated unclaimed Shopify subscription without user-level side effects"
        );
        return Ok(());
    };

    if is_shopify_cancel_status(mapped_status) {
        let cancel_mode = cancel_mode_for_shopify_status(mapped_status);
        let cancel_period_end =
            matches!(cancel_mode, CancelMode::DefaultCancel).then_some(contract.current_period_end);

        state
            .subscription_service
            .handle_subscription_cancel(user_id, &realm_id, cancel_mode, cancel_period_end)
            .await?;

        info!(
            realm_id = %realm_id,
            user_id = %user_id,
            subscription_id = %subscription_id,
            previous_status = %previous_status,
            new_status = %mapped_status,
            revision_id = %new_revision_id,
            "Processed Shopify subscription cancellation update"
        );
        return Ok(());
    }

    if new_plan_config.points_per_period > old_plan_config.points_per_period {
        state
            .subscription_service
            .handle_subscription_upgrade(
                user_id,
                &realm_id,
                old_plan_id,
                herald_plan_id,
                contract.current_period_end,
            )
            .await?;

        info!(
            realm_id = %realm_id,
            user_id = %user_id,
            old_plan_id = %old_plan_id,
            new_plan_id = %herald_plan_id,
            revision_id = %new_revision_id,
            "Processed subscription upgrade - granted difference points"
        );
    } else if new_plan_config.points_per_period < old_plan_config.points_per_period {
        state
            .subscription_service
            .handle_subscription_downgrade(user_id, &realm_id, old_plan_id, herald_plan_id)
            .await?;

        info!(
            realm_id = %realm_id,
            user_id = %user_id,
            old_plan_id = %old_plan_id,
            new_plan_id = %herald_plan_id,
            revision_id = %new_revision_id,
            "Processed subscription downgrade - no points revoked"
        );
    } else {
        info!(
            realm_id = %realm_id,
            subscription_id = %_sub_id,
            revision_id = %new_revision_id,
            "Plan unchanged - updated metadata only"
        );
    }

    Ok(())
}

async fn handle_billing_attempt_success(
    state: AppState,
    realm_id: String,
    payload: Value,
) -> Result<(), CoreError> {
    use herald_core::infrastructure::shopify::models::parse_billing_attempt_payload;

    info!(
        realm_id = %realm_id,
        "Processing subscription_billing_attempts/success webhook"
    );

    let shopify_repo = ShopifyRepository::new(state.pool.clone());

    let attempt = parse_billing_attempt_payload(&payload).map_err(|e| {
        CoreError::BadRequest(format!("Invalid Shopify billing attempt payload: {}", e))
    })?;

    let binding = shopify_repo
        .find_binding_for_billing(&attempt.subscription_contract_id)
        .await?
        .ok_or_else(|| {
            warn!(
                contract_id = %attempt.subscription_contract_id,
                "Shopify binding not found for contract"
            );
            CoreError::NotFound
        })?;

    let (binding_id, subscription_id, _revision_id, _customer_id) = binding;

    let subscription = shopify_repo
        .find_subscription_by_external_id(&attempt.subscription_contract_id)
        .await?
        .ok_or_else(|| {
            error!(subscription_id = %subscription_id, "Subscription not found");
            CoreError::NotFound
        })?;

    let (_sub_id, plan_id_option, sub_realm_id) = subscription;
    let subscription_record = shopify_repo
        .find_subscription_with_user(subscription_id)
        .await?
        .ok_or(CoreError::NotFound)?;

    let plan_id = plan_id_option.ok_or_else(|| {
        error!(subscription_id = %subscription_id, "Subscription missing plan_id");
        CoreError::BadRequest("Subscription missing plan_id".to_string())
    })?;

    shopify_repo
        .update_binding_billing_attempt(binding_id, &attempt.id, attempt.order_id.as_deref())
        .await?;
    let period_end_from_attempt = attempt.current_period_end;
    if let Some(period_end) = period_end_from_attempt.as_ref() {
        shopify_repo
            .update_subscription_period_end(subscription_id, period_end.to_owned())
            .await?;
    }

    let period_end = period_end_from_attempt
        .or(subscription_record.current_period_end)
        .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::days(30));

    let Some(user_id) = subscription_record.user_id else {
        info!(
            realm_id = %sub_realm_id,
            subscription_id = %subscription_id,
            billing_attempt_id = %attempt.id,
            period_end = %period_end,
            "Billing success received for unclaimed Shopify subscription; grant deferred"
        );
        return Ok(());
    };

    state
        .subscription_service
        .handle_subscription_paid(
            user_id,
            &sub_realm_id,
            plan_id,
            true,
            period_end,
            format!("shopify_renewal_{}", attempt.id),
        )
        .await
        .map_err(|e| {
            error!(
                realm_id = %sub_realm_id,
                user_id = %user_id,
                error = %e,
                "Failed to grant renewal points"
            );
            e
        })?;

    info!(
        realm_id = %sub_realm_id,
        user_id = %user_id,
        subscription_id = %subscription_id,
        plan_id = %plan_id,
        billing_attempt_id = %attempt.id,
        order_id = ?attempt.order_id,
        period_end = %period_end,
        "Processed successful billing attempt - granted renewal points"
    );

    Ok(())
}

async fn handle_billing_attempt_failure(
    state: AppState,
    realm_id: String,
    payload: Value,
) -> Result<(), CoreError> {
    use herald_core::infrastructure::shopify::models::parse_billing_attempt_payload;

    info!(
        realm_id = %realm_id,
        "Processing subscription_billing_attempts/failure webhook"
    );

    let shopify_repo = ShopifyRepository::new(state.pool.clone());

    let attempt = parse_billing_attempt_payload(&payload).map_err(|e| {
        CoreError::BadRequest(format!("Invalid Shopify billing attempt payload: {}", e))
    })?;

    let binding = shopify_repo
        .find_binding_by_contract_id(&attempt.subscription_contract_id)
        .await?
        .ok_or_else(|| {
            warn!(
                contract_id = %attempt.subscription_contract_id,
                "Shopify binding not found for failed billing attempt"
            );
            CoreError::NotFound
        })?;

    let (binding_id, subscription_id, _revision_id, _customer_id) = binding;

    shopify_repo
        .update_subscription_status(subscription_id, "past_due")
        .await?;

    shopify_repo
        .update_binding_billing_attempt_id(binding_id, &attempt.id)
        .await?;

    let error_details = attempt
        .error_code
        .zip(attempt.error_message)
        .map(|(code, msg)| format!("{}: {}", code, msg))
        .unwrap_or_else(|| "Unknown error".to_string());

    warn!(
        realm_id = %realm_id,
        subscription_id = %subscription_id,
        contract_id = %attempt.subscription_contract_id,
        billing_attempt_id = %attempt.id,
        error_details = %error_details,
        "Subscription marked as past_due due to billing failure"
    );

    Ok(())
}

async fn handle_refunds_create(
    state: AppState,
    realm_id: String,
    payload: Value,
) -> Result<(), CoreError> {
    use herald_core::infrastructure::shopify::models::parse_refund_payload;

    info!(
        realm_id = %realm_id,
        "Processing refunds/create webhook"
    );

    let shopify_repo = ShopifyRepository::new(state.pool.clone());

    let refund = parse_refund_payload(&payload)
        .map_err(|e| CoreError::BadRequest(format!("Invalid Shopify refund payload: {}", e)))?;

    let binding = shopify_repo
        .find_binding_by_order_id(&refund.order_id)
        .await?
        .ok_or_else(|| {
            warn!(
                order_id = %refund.order_id,
                "Shopify binding not found for refund"
            );
            CoreError::NotFound
        })?;

    let (subscription_id, _customer_id) = binding;
    let subscription_record = shopify_repo
        .find_subscription_with_user(subscription_id)
        .await?
        .ok_or(CoreError::NotFound)?;
    let sub_realm_id = subscription_record.realm_id.clone();

    let Some(user_id) = subscription_record.user_id else {
        info!(
            realm_id = %sub_realm_id,
            subscription_id = %subscription_id,
            refund_id = %refund.id,
            "Refund received for unclaimed Shopify subscription; no points revoked"
        );
        return Ok(());
    };

    state
        .points_service
        .revoke_subscription_unused(
            &sub_realm_id,
            user_id,
            &format!("shopify_refund_{}", refund.id),
        )
        .await
        .map_err(|e| {
            error!(
                realm_id = %sub_realm_id,
                user_id = %user_id,
                refund_id = %refund.id,
                subscription_id = %subscription_id,
                error = %e,
                "Failed to revoke subscription points on refund"
            );
            e
        })?;

    info!(
        realm_id = %sub_realm_id,
        user_id = %user_id,
        subscription_id = %subscription_id,
        refund_id = %refund.id,
        order_id = %refund.order_id,
        refund_amount = %refund.refund_amount,
        currency = %refund.currency,
        reason = ?refund.reason,
        "Processed refund - revoked unused subscription points"
    );

    Ok(())
}

fn map_shopify_status(status: &str) -> &str {
    match status.to_ascii_lowercase().as_str() {
        "active" => "active",
        "trialing" => "trialing",
        "scheduled_cancel" => "scheduled_cancel",
        "cancelled" | "canceled" => "canceled",
        "expired" => "expired",
        "past_due" => "past_due",
        _ => "active",
    }
}

fn is_shopify_cancel_status(status: &str) -> bool {
    matches!(status, "scheduled_cancel" | "canceled")
}

fn cancel_mode_for_shopify_status(status: &str) -> CancelMode {
    match status {
        "scheduled_cancel" => CancelMode::DefaultCancel,
        _ => CancelMode::ImmediateCancel,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_map_shopify_status() {
        assert_eq!(super::map_shopify_status("ACTIVE"), "active");
        assert_eq!(super::map_shopify_status("cancelled"), "canceled");
    }
}
