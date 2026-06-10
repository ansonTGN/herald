// Webhook Event Handlers for Stripe Billing Events
//
// Handles subscription lifecycle events (checkout.session.completed, customer.subscription.*)
// and payment events (charge.refunded). All handlers follow the Creem webhook pattern.

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::time::Duration;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::webhook_common::{
    create_placeholder_transaction, metadata_value, parse_attempt_id, parse_event_id,
    parse_optional_uuid_field, parse_uuid_field,
};
use crate::webhook_subscription_helpers::{
    SyncSubscriptionInput, resolve_entitlement_key, save_subscription_history_in_txn,
    sync_subscription_in_txn,
};
use herald_api_base::application::http::state::AppState;
use herald_core::domain::billing::invoice_service::map_stripe_invoice_status;
use herald_core::domain::billing::{
    ACTOR_WEBHOOK, BillingRepository, BillingType, ExternalInvoiceData, HistoryEventType,
    InvoiceProvider, InvoiceRepository, PaymentEvent, Subscription, SubscriptionHistoryService,
    SubscriptionStatus, detect_change_type,
};
use herald_core::domain::common::entities::app_errors::CoreError;
use herald_core::domain::payment_attempt::PaymentAttemptStatus;
use herald_core::domain::points::IdempotencyResult;
use herald_core::domain::points::entities::{PointsTransaction, RevocationType, TransactionType};
use herald_core::domain::points::ports::PointsRepository;
use herald_core::domain::points::subscription_service::CancelMode;
use herald_core::domain::purchase::metadata_keys;
use herald_core::domain::purchase::{CompletePaymentAttemptInput, PaymentCompletionSource};

struct StripeCheckoutCompletedPayload {
    event_id: String,
    user_id: Uuid,
    client_app_id: Uuid,
    entitlement_key: String,
    is_trial: bool,
    stripe_subscription_id: Option<String>,
    stripe_product_id: String,
}

struct StripeSubscriptionCreatedPayload {
    event_id: String,
    stripe_subscription_id: String,
    user_id: Uuid,
    entitlement_key: String,
    client_app_id: Option<Uuid>,
    external_product_id: String,
    cancel_at_period_end: bool,
    current_period_start: Option<DateTime<Utc>>,
    current_period_end: DateTime<Utc>,
    status: SubscriptionStatus,
}

struct StripeSubscriptionUpdatedPayload {
    event_id: String,
    stripe_subscription_id: String,
    user_id: Uuid,
    previous_entitlement_key: String,
    current_entitlement_key: String,
    external_product_id: String,
    cancel_at_period_end: bool,
    current_period_start: Option<DateTime<Utc>>,
    current_period_end: DateTime<Utc>,
    status: SubscriptionStatus,
}

struct StripeSubscriptionDeletedPayload {
    event_id: String,
    stripe_subscription_id: String,
    user_id: Uuid,
    entitlement_key: Option<String>,
    cancel_at_period_end: bool,
    current_period_start: Option<DateTime<Utc>>,
    current_period_end: DateTime<Utc>,
}

struct StripeChargeRefundedPayload {
    event_id: String,
    charge_id: String,
    amount: i64,
    amount_refunded: i64,
    user_id: Uuid,
    subscription_id: Option<Uuid>,
    refund_type: String,
}

struct StripeInvoicePaidPayload {
    event_id: String,
    stripe_subscription_id: String,
    user_id: Uuid,
    entitlement_key: String,
    current_period_start: Option<DateTime<Utc>>,
    current_period_end: DateTime<Utc>,
}

struct StripePaymentIntentSucceededPayload {
    attempt_id: Uuid,
    payment_intent_id: String,
    completed_at: DateTime<Utc>,
}

struct StripePaymentFailedPayload {
    attempt_id: Uuid,
    provider_reference: String,
    provider_status: String,
    completed_at: DateTime<Utc>,
}

fn parse_stripe_datetime(value: &Value, field_name: &str) -> Result<DateTime<Utc>, CoreError> {
    if let Some(timestamp) = value.as_i64() {
        return DateTime::<Utc>::from_timestamp(timestamp, 0).ok_or_else(|| {
            CoreError::BadRequest(format!("Invalid unix timestamp for {}", field_name))
        });
    }

    if let Some(timestamp) = value.as_u64() {
        return DateTime::<Utc>::from_timestamp(timestamp as i64, 0).ok_or_else(|| {
            CoreError::BadRequest(format!("Invalid unix timestamp for {}", field_name))
        });
    }

    if let Some(value) = value.as_str() {
        return DateTime::parse_from_rfc3339(value)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|_| {
                CoreError::BadRequest(format!("Invalid RFC3339 timestamp for {}", field_name))
            });
    }

    Err(CoreError::BadRequest(format!(
        "Missing or invalid {}",
        field_name
    )))
}

fn parse_optional_stripe_datetime(value: &Value) -> Result<Option<DateTime<Utc>>, CoreError> {
    if value.is_null() {
        return Ok(None);
    }

    parse_stripe_datetime(value, "timestamp").map(Some)
}

fn parse_stripe_subscription_status(
    status: Option<&str>,
    cancel_at_period_end: bool,
) -> Result<SubscriptionStatus, CoreError> {
    let parsed = match status.unwrap_or("active") {
        "active" => SubscriptionStatus::Active,
        "trialing" => SubscriptionStatus::Trialing,
        "canceled" => SubscriptionStatus::Canceled,
        "past_due" | "unpaid" => SubscriptionStatus::PastDue,
        "paused" => SubscriptionStatus::Paused,
        "incomplete" | "incomplete_expired" => SubscriptionStatus::Incomplete,
        other => {
            return Err(CoreError::BadRequest(format!(
                "Unsupported Stripe subscription status: {}",
                other
            )));
        }
    };

    if cancel_at_period_end && parsed == SubscriptionStatus::Active {
        Ok(SubscriptionStatus::ScheduledCancel)
    } else {
        Ok(parsed)
    }
}

/// Extract herald_entitlement_key from Stripe metadata, falling back to local mapping.
async fn resolve_stripe_entitlement_key(
    app_state: &AppState,
    realm_id: &str,
    metadata: &Value,
    external_product_id: &str,
) -> Result<String, CoreError> {
    let metadata_key = metadata["herald_entitlement_key"]
        .as_str()
        .or_else(|| metadata["entitlementKey"].as_str());
    resolve_entitlement_key(
        app_state,
        realm_id,
        "stripe",
        external_product_id,
        metadata_key,
    )
    .await
}

fn parse_checkout_completed_payload(
    event: &Value,
) -> Result<StripeCheckoutCompletedPayload, CoreError> {
    let metadata = &event["data"]["object"]["metadata"];
    let client_app_id = parse_uuid_field(
        metadata_value(metadata, "herald_client_app_id", "clientAppId"),
        "clientAppId",
    )?;
    let user_id = parse_uuid_field(
        metadata_value(metadata, "herald_user_id", "userId"),
        "userId",
    )?;

    // Resolve entitlement_key from metadata
    let entitlement_key = metadata["herald_entitlement_key"]
        .as_str()
        .or_else(|| metadata["entitlementKey"].as_str())
        .unwrap_or("")
        .to_string();

    Ok(StripeCheckoutCompletedPayload {
        event_id: parse_event_id(event)?,
        user_id,
        client_app_id,
        entitlement_key,
        is_trial: metadata["herald_trial_days"]
            .as_u64()
            .or_else(|| metadata["trialDays"].as_u64())
            .is_some_and(|days| days > 0),
        stripe_subscription_id: event["data"]["object"]["subscription"]
            .as_str()
            .map(str::to_string),
        stripe_product_id: event["data"]["object"]["display_items"]
            .as_array()
            .and_then(|items| items.first())
            .and_then(|item| item["price"]["product"].as_str())
            .map(str::to_string)
            .unwrap_or_default(),
    })
}

fn parse_subscription_created_payload(
    event: &Value,
) -> Result<StripeSubscriptionCreatedPayload, CoreError> {
    let metadata = &event["data"]["object"]["metadata"];
    let stripe_subscription_id = event["data"]["object"]["id"]
        .as_str()
        .ok_or_else(|| CoreError::BadRequest("Missing subscription id".to_string()))?
        .to_string();
    let user_id = parse_uuid_field(
        metadata_value(metadata, "herald_user_id", "userId"),
        "userId",
    )?;
    let cancel_at_period_end = event["data"]["object"]["cancel_at_period_end"]
        .as_bool()
        .unwrap_or(false);
    let status = parse_stripe_subscription_status(
        event["data"]["object"]["status"].as_str(),
        cancel_at_period_end,
    )?;

    let external_product_id = event["data"]["object"]["items"]["data"][0]["price"]["product"]
        .as_str()
        .map(str::to_string)
        .unwrap_or_default();

    // Resolve entitlement_key from metadata
    let entitlement_key = metadata["herald_entitlement_key"]
        .as_str()
        .or_else(|| metadata["entitlementKey"].as_str())
        .unwrap_or("")
        .to_string();

    Ok(StripeSubscriptionCreatedPayload {
        event_id: parse_event_id(event)?,
        stripe_subscription_id,
        user_id,
        entitlement_key,
        client_app_id: parse_optional_uuid_field(metadata_value(
            metadata,
            "herald_client_app_id",
            "clientAppId",
        )),
        external_product_id,
        cancel_at_period_end,
        current_period_start: parse_optional_stripe_datetime(
            &event["data"]["object"]["current_period_start"],
        )?,
        current_period_end: parse_stripe_datetime(
            &event["data"]["object"]["current_period_end"],
            "current_period_end",
        )?,
        status,
    })
}

fn parse_subscription_updated_payload(
    event: &Value,
) -> Result<StripeSubscriptionUpdatedPayload, CoreError> {
    let metadata = &event["data"]["object"]["metadata"];
    let previous_entitlement_key = event["data"]["previous_attributes"]["items"]["data"][0]["price"]["metadata"]["herald_entitlement_key"]
        .as_str()
        .or_else(|| event["data"]["previous_attributes"]["items"]["data"][0]["price"]["metadata"]["entitlementKey"].as_str())
        .unwrap_or("")
        .to_string();
    let current_entitlement_key = event["data"]["object"]["items"]["data"][0]["price"]["metadata"]
        ["herald_entitlement_key"]
        .as_str()
        .or_else(|| {
            event["data"]["object"]["items"]["data"][0]["price"]["metadata"]["entitlementKey"]
                .as_str()
        })
        .unwrap_or("")
        .to_string();
    let cancel_at_period_end = event["data"]["object"]["cancel_at_period_end"]
        .as_bool()
        .unwrap_or(false);
    let status = parse_stripe_subscription_status(
        event["data"]["object"]["status"].as_str(),
        cancel_at_period_end,
    )?;

    Ok(StripeSubscriptionUpdatedPayload {
        event_id: parse_event_id(event)?,
        stripe_subscription_id: event["data"]["object"]["id"]
            .as_str()
            .ok_or_else(|| CoreError::BadRequest("Missing subscription id".to_string()))?
            .to_string(),
        user_id: parse_uuid_field(
            metadata_value(metadata, "herald_user_id", "userId"),
            "userId",
        )?,
        previous_entitlement_key,
        current_entitlement_key,
        external_product_id: event["data"]["object"]["items"]["data"][0]["price"]["product"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_default(),
        cancel_at_period_end,
        current_period_start: parse_optional_stripe_datetime(
            &event["data"]["object"]["current_period_start"],
        )?,
        current_period_end: parse_stripe_datetime(
            &event["data"]["object"]["current_period_end"],
            "current_period_end",
        )?,
        status,
    })
}

fn parse_subscription_deleted_payload(
    event: &Value,
) -> Result<StripeSubscriptionDeletedPayload, CoreError> {
    let metadata = &event["data"]["object"]["metadata"];
    Ok(StripeSubscriptionDeletedPayload {
        event_id: parse_event_id(event)?,
        stripe_subscription_id: event["data"]["object"]["id"]
            .as_str()
            .ok_or_else(|| CoreError::BadRequest("Missing subscription id".to_string()))?
            .to_string(),
        user_id: parse_uuid_field(
            metadata_value(metadata, "herald_user_id", "userId"),
            "userId",
        )?,
        entitlement_key: metadata["herald_entitlement_key"]
            .as_str()
            .or_else(|| metadata["entitlementKey"].as_str())
            .map(str::to_string),
        cancel_at_period_end: event["data"]["object"]["cancel_at_period_end"]
            .as_bool()
            .unwrap_or(false),
        current_period_start: parse_optional_stripe_datetime(
            &event["data"]["object"]["current_period_start"],
        )?,
        current_period_end: parse_stripe_datetime(
            &event["data"]["object"]["current_period_end"],
            "current_period_end",
        )?,
    })
}

fn parse_charge_refunded_payload(event: &Value) -> Result<StripeChargeRefundedPayload, CoreError> {
    Ok(StripeChargeRefundedPayload {
        event_id: parse_event_id(event)?,
        charge_id: event["data"]["object"]["id"]
            .as_str()
            .ok_or_else(|| CoreError::BadRequest("Missing charge id".to_string()))?
            .to_string(),
        amount: event["data"]["object"]["amount"]
            .as_i64()
            .ok_or_else(|| CoreError::BadRequest("Missing or invalid amount".to_string()))?,
        amount_refunded: event["data"]["object"]["amount_refunded"]
            .as_i64()
            .ok_or_else(|| {
                CoreError::BadRequest("Missing or invalid amount_refunded".to_string())
            })?,
        user_id: parse_uuid_field(
            metadata_value(
                &event["data"]["object"]["metadata"],
                "herald_user_id",
                "userId",
            ),
            "userId",
        )?,
        subscription_id: parse_optional_uuid_field(metadata_value(
            &event["data"]["object"]["metadata"],
            "herald_subscription_id",
            "subscriptionId",
        )),
        refund_type: event["data"]["object"]["metadata"]["refundType"]
            .as_str()
            .unwrap_or("subscription")
            .to_string(),
    })
}

fn parse_invoice_paid_payload(event: &Value) -> Result<StripeInvoicePaidPayload, CoreError> {
    let metadata = &event["data"]["object"]["metadata"];
    let entitlement_key = metadata["herald_entitlement_key"]
        .as_str()
        .or_else(|| metadata["entitlementKey"].as_str())
        .unwrap_or("")
        .to_string();

    Ok(StripeInvoicePaidPayload {
        event_id: parse_event_id(event)?,
        stripe_subscription_id: event["data"]["object"]["subscription"]
            .as_str()
            .ok_or_else(|| CoreError::BadRequest("Missing subscription id".to_string()))?
            .to_string(),
        user_id: parse_uuid_field(
            metadata_value(metadata, "herald_user_id", "userId"),
            "userId",
        )?,
        entitlement_key,
        current_period_start: parse_optional_stripe_datetime(
            &event["data"]["object"]["current_period_start"],
        )?,
        current_period_end: parse_stripe_datetime(
            &event["data"]["object"]["current_period_end"],
            "current_period_end",
        )?,
    })
}

fn parse_payment_intent_succeeded_payload(
    event: &Value,
) -> Result<StripePaymentIntentSucceededPayload, CoreError> {
    let object = &event["data"]["object"];
    let attempt_id =
        parse_attempt_id(&object["metadata"][metadata_keys::ATTEMPT_ID]).ok_or_else(|| {
            CoreError::BadRequest("Missing attemptId in payment intent metadata".to_string())
        })?;

    Ok(StripePaymentIntentSucceededPayload {
        attempt_id,
        payment_intent_id: object["id"]
            .as_str()
            .ok_or_else(|| CoreError::BadRequest("Missing payment intent id".to_string()))?
            .to_string(),
        completed_at: parse_optional_stripe_datetime(&object["created"])?.unwrap_or_else(Utc::now),
    })
}

fn parse_payment_failed_payload(
    event: &Value,
) -> Result<Option<StripePaymentFailedPayload>, CoreError> {
    let object = &event["data"]["object"];
    let Some(attempt_id) = parse_attempt_id(&object["metadata"][metadata_keys::ATTEMPT_ID]) else {
        return Ok(None);
    };

    Ok(Some(StripePaymentFailedPayload {
        attempt_id,
        provider_reference: object["id"]
            .as_str()
            .ok_or_else(|| CoreError::BadRequest("Missing failed payment object id".to_string()))?
            .to_string(),
        provider_status: object["status"].as_str().unwrap_or("failed").to_string(),
        completed_at: parse_optional_stripe_datetime(&object["created"])?.unwrap_or_else(Utc::now),
    }))
}

async fn fulfill_payment_attempt(
    app_state: &AppState,
    attempt_id: Uuid,
    provider_status: &str,
    provider_transaction_id: String,
    completed_at: DateTime<Utc>,
    billing_type_override: Option<BillingType>,
) -> Result<(), CoreError> {
    app_state
        .purchase_service
        .complete_succeeded_payment_attempt(CompletePaymentAttemptInput {
            attempt_id,
            provider_status: provider_status.to_string(),
            provider_transaction_id,
            completed_at,
            source: PaymentCompletionSource::ProviderWebhook {
                provider: "stripe".to_string(),
            },
            billing_type_override,
        })
        .await?;

    Ok(())
}

async fn fail_payment_attempt(
    app_state: &AppState,
    realm_id: &str,
    payload: StripePaymentFailedPayload,
) -> Result<(), CoreError> {
    app_state
        .payment_attempt_service
        .mark_payment_failed(
            realm_id,
            payload.attempt_id,
            payload.provider_status,
            payload.completed_at,
        )
        .await?;

    info!(
        realm_id = %realm_id,
        attempt_id = %payload.attempt_id,
        provider_reference = %payload.provider_reference,
        "Stripe payment attempt marked failed"
    );

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn sync_stripe_subscription_with_history_in_txn(
    app_state: &AppState,
    realm_id: &str,
    user_id: Uuid,
    external_subscription_id: &str,
    client_app_id: Option<Uuid>,
    entitlement_key: String,
    external_product_id: String,
    external_price_id: Option<String>,
    status: SubscriptionStatus,
    current_period_start: Option<DateTime<Utc>>,
    current_period_end: Option<DateTime<Utc>>,
    cancel_at_period_end: bool,
    cancel_at: Option<DateTime<Utc>>,
    existing_subscription: Option<Subscription>,
    history_event_type: HistoryEventType,
) -> Result<(Subscription, Option<Subscription>), CoreError> {
    let txn = app_state.billing_repository.begin_transaction().await?;

    let (subscription, previous_subscription) = sync_subscription_in_txn(
        &txn,
        SyncSubscriptionInput {
            provider: "stripe",
            realm_id: realm_id.to_string(),
            user_id: Some(user_id),
            external_subscription_id: external_subscription_id.to_string(),
            external_product_id,
            client_app_id,
            entitlement_key,
            external_price_id,
            provider_metadata: None,
            status: status.clone(),
            current_period_start,
            current_period_end,
            cancel_at_period_end,
            cancel_at,
            existing_subscription,
        },
    )
    .await?
    .ok_or_else(|| {
        CoreError::InternalServerError(
            "stripe subscription sync failed to create or update subscription".to_string(),
        )
    })?;

    save_subscription_history_in_txn(
        &app_state.billing_repository,
        &txn,
        previous_subscription.as_ref(),
        &subscription,
        history_event_type,
    )
    .await?;

    txn.commit()
        .await
        .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

    Ok((subscription, previous_subscription))
}

#[allow(clippy::too_many_arguments)]
async fn sync_stripe_subscription_with_detected_history_in_txn(
    app_state: &AppState,
    realm_id: &str,
    user_id: Uuid,
    external_subscription_id: &str,
    client_app_id: Option<Uuid>,
    entitlement_key: String,
    external_product_id: String,
    external_price_id: Option<String>,
    status: SubscriptionStatus,
    current_period_start: Option<DateTime<Utc>>,
    current_period_end: Option<DateTime<Utc>>,
    cancel_at_period_end: bool,
    cancel_at: Option<DateTime<Utc>>,
    existing_subscription: Option<Subscription>,
) -> Result<(Subscription, Option<Subscription>), CoreError> {
    let txn = app_state.billing_repository.begin_transaction().await?;

    let (subscription, previous_subscription) = sync_subscription_in_txn(
        &txn,
        SyncSubscriptionInput {
            provider: "stripe",
            realm_id: realm_id.to_string(),
            user_id: Some(user_id),
            external_subscription_id: external_subscription_id.to_string(),
            external_product_id,
            client_app_id,
            entitlement_key,
            external_price_id,
            provider_metadata: None,
            status,
            current_period_start,
            current_period_end,
            cancel_at_period_end,
            cancel_at,
            existing_subscription,
        },
    )
    .await?
    .ok_or_else(|| {
        CoreError::InternalServerError(
            "stripe subscription sync failed to create or update subscription".to_string(),
        )
    })?;

    let history_event_type = match previous_subscription {
        Some(ref previous) => detect_change_type(previous, &subscription),
        None => HistoryEventType::Created,
    };

    save_subscription_history_in_txn(
        &app_state.billing_repository,
        &txn,
        previous_subscription.as_ref(),
        &subscription,
        history_event_type,
    )
    .await?;

    txn.commit()
        .await
        .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

    Ok((subscription, previous_subscription))
}

async fn sync_subscription_input_with_history_in_txn(
    app_state: &AppState,
    input: SyncSubscriptionInput,
    history_event_type: HistoryEventType,
) -> Result<Option<(Subscription, Option<Subscription>)>, CoreError> {
    let txn = app_state.billing_repository.begin_transaction().await?;
    let synced = sync_subscription_in_txn(&txn, input).await?;

    if let Some((subscription, previous_subscription)) = synced.as_ref() {
        save_subscription_history_in_txn(
            &app_state.billing_repository,
            &txn,
            previous_subscription.as_ref(),
            subscription,
            history_event_type,
        )
        .await?;
    }

    txn.commit()
        .await
        .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

    Ok(synced)
}

async fn sync_subscription_input_with_detected_history_in_txn(
    app_state: &AppState,
    input: SyncSubscriptionInput,
) -> Result<Option<(Subscription, Option<Subscription>)>, CoreError> {
    let txn = app_state.billing_repository.begin_transaction().await?;
    let synced = sync_subscription_in_txn(&txn, input).await?;

    if let Some((subscription, previous_subscription)) = synced.as_ref() {
        let history_event_type = match previous_subscription {
            Some(previous) => detect_change_type(previous, subscription),
            None => HistoryEventType::Created,
        };
        save_subscription_history_in_txn(
            &app_state.billing_repository,
            &txn,
            previous_subscription.as_ref(),
            subscription,
            history_event_type,
        )
        .await?;
    }

    txn.commit()
        .await
        .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

    Ok(synced)
}

// ============================================================================
// Webhook Event Handlers
// ============================================================================

/// Handle checkout.session.completed events
///
/// Dispatches based on checkout mode:
/// - With attemptId in metadata: completes the payment attempt and fulfills (both one-time and recurring)
/// - Without attemptId + mode=payment: logs warning, no fulfillment (orphan one-time event)
/// - Without attemptId + mode=subscription (or absent): creates subscription (legacy/legacy webhook)
async fn handle_checkout_session_completed(
    app_state: AppState,
    event: Value,
    realm_id: &str,
    _idempotency_key: &str,
) -> Result<PointsTransaction, CoreError> {
    // Extract mode from the checkout session for dispatch
    let mode = event["data"]["object"]["mode"].as_str().map(str::to_string);

    // If attemptId is present in metadata, fulfill via payment attempt flow.
    // This covers both one-time (mode=payment) and recurring (mode=subscription) purchases
    // initiated through PurchaseService.
    if let Some(attempt_id) =
        parse_attempt_id(&event["data"]["object"]["metadata"][metadata_keys::ATTEMPT_ID])
    {
        let payment_status = event["data"]["object"]["payment_status"]
            .as_str()
            .unwrap_or("unpaid");
        if payment_status != "paid" && payment_status != "no_payment_required" {
            let strategy = read_async_points_strategy(&app_state.pool, realm_id).await;
            if strategy != AsyncPointsStrategy::Eager {
                info!(
                    realm_id = %realm_id,
                    attempt_id = %attempt_id,
                    payment_status = %payment_status,
                    mode = ?mode,
                    "Checkout session completed before payment settled - waiting for async result"
                );

                return Ok(create_placeholder_transaction(
                    attempt_id,
                    realm_id,
                    TransactionType::Recharge,
                ));
            }
            // Eager strategy: fall through to fulfillment despite unpaid status
            info!(
                realm_id = %realm_id,
                attempt_id = %attempt_id,
                payment_status = %payment_status,
                "Eager strategy: fulfilling despite unpaid async payment"
            );
        }

        let provider_transaction_id = event["data"]["object"]["subscription"]
            .as_str()
            .or_else(|| event["data"]["object"]["payment_intent"].as_str())
            .or_else(|| event["data"]["object"]["id"].as_str())
            .ok_or_else(|| CoreError::BadRequest("Missing provider transaction id".to_string()))?
            .to_string();
        let completed_at = parse_optional_stripe_datetime(&event["data"]["object"]["created"])?
            .unwrap_or_else(Utc::now);

        info!(
            realm_id = %realm_id,
            attempt_id = %attempt_id,
            mode = ?mode,
            "Processing checkout.session.completed with attemptId - fulfilling payment attempt"
        );

        let billing_type_override = mode.as_deref().and_then(|m| match m {
            "payment" => Some(BillingType::OneTime),
            _ => None,
        });

        fulfill_payment_attempt(
            &app_state,
            attempt_id,
            "succeeded",
            provider_transaction_id,
            completed_at,
            billing_type_override,
        )
        .await?;

        return Ok(create_placeholder_transaction(
            attempt_id,
            realm_id,
            TransactionType::Recharge,
        ));
    }

    // No attemptId — dispatch based on mode
    let payload = parse_checkout_completed_payload(&event)?;
    let event_id = payload.event_id.as_str();

    if mode.as_deref() == Some("payment") {
        // mode=payment without attemptId: one-time checkout we cannot fulfill
        warn!(
            realm_id = %realm_id,
            event_id = %event_id,
            user_id = %payload.user_id,
            "checkout.session.completed with mode=payment but no attemptId - cannot fulfill, recording audit event"
        );

        return Ok(create_placeholder_transaction(
            payload.user_id,
            realm_id,
            TransactionType::Recharge,
        ));
    }

    // Subscription flow requires a subscription id
    let stripe_subscription_id = payload.stripe_subscription_id.as_deref().ok_or_else(|| {
        CoreError::BadRequest("Missing subscription id for subscription checkout".to_string())
    })?;

    info!(
        realm_id = %realm_id,
        event_id = %event_id,
        "Processing checkout.session.completed event"
    );

    // Resolve entitlement_key via fallback chain
    let entitlement_key = if payload.entitlement_key.is_empty() {
        resolve_stripe_entitlement_key(
            &app_state,
            realm_id,
            &event["data"]["object"]["metadata"],
            &payload.stripe_product_id,
        )
        .await?
    } else {
        payload.entitlement_key.clone()
    };

    // Determine status
    let status = if payload.is_trial {
        SubscriptionStatus::Trialing
    } else {
        SubscriptionStatus::Active
    };

    // Check if subscription already exists (customer.subscription.created webhook
    // may have arrived first and already created it via sync_subscription).
    let existing_subscription = app_state
        .billing_repository
        .find_by_external_subscription_id(stripe_subscription_id, "stripe")
        .await?;

    let now = chrono::Utc::now();
    let (_created_subscription, _) = if let Some(existing) = existing_subscription {
        info!(
            realm_id = %realm_id,
            subscription_id = %existing.id,
            stripe_subscription_id = %stripe_subscription_id,
            event_id = %event_id,
            "Checkout completed - subscription already exists from subscription.created webhook, updating"
        );

        sync_stripe_subscription_with_history_in_txn(
            &app_state,
            realm_id,
            payload.user_id,
            stripe_subscription_id,
            Some(payload.client_app_id),
            entitlement_key.clone(),
            payload.stripe_product_id.clone(),
            existing.external_price_id.clone(),
            status,
            existing.current_period_start.or(Some(now)),
            existing.current_period_end,
            existing.cancel_at_period_end,
            existing.cancel_at,
            Some(existing),
            HistoryEventType::Created,
        )
        .await?
    } else {
        let (created, previous) = sync_stripe_subscription_with_history_in_txn(
            &app_state,
            realm_id,
            payload.user_id,
            stripe_subscription_id,
            Some(payload.client_app_id),
            entitlement_key.clone(),
            payload.stripe_product_id.clone(),
            None,
            status,
            Some(now),
            None,
            false,
            None,
            None,
            HistoryEventType::Created,
        )
        .await?;

        info!(
            realm_id = %realm_id,
            subscription_id = %created.id,
            client_app_id = %payload.client_app_id,
            entitlement_key = %entitlement_key,
            stripe_subscription_id = %stripe_subscription_id,
            event_id = %event_id,
            "Checkout completed - subscription created"
        );

        (created, previous)
    };

    // Return a placeholder transaction for compatibility
    // Actual subscription points will be granted by customer.subscription.created event
    Ok(create_placeholder_transaction(
        payload.client_app_id,
        realm_id,
        TransactionType::SubscriptionGrant,
    ))
}

async fn handle_payment_intent_succeeded(
    app_state: AppState,
    event: Value,
    realm_id: &str,
    _idempotency_key: &str,
) -> Result<PointsTransaction, CoreError> {
    let payload = parse_payment_intent_succeeded_payload(&event)?;

    fulfill_payment_attempt(
        &app_state,
        payload.attempt_id,
        "succeeded",
        payload.payment_intent_id,
        payload.completed_at,
        None,
    )
    .await?;

    Ok(create_placeholder_transaction(
        payload.attempt_id,
        realm_id,
        TransactionType::Recharge,
    ))
}

async fn handle_checkout_session_async_succeeded(
    app_state: AppState,
    event: Value,
    realm_id: &str,
    _idempotency_key: &str,
) -> Result<PointsTransaction, CoreError> {
    let object = &event["data"]["object"];
    let Some(attempt_id) = parse_attempt_id(&object["metadata"][metadata_keys::ATTEMPT_ID]) else {
        warn!(
            realm_id = %realm_id,
            event_id = %parse_event_id(&event)?,
            "Stripe async checkout success has no attemptId metadata - ignoring"
        );
        return Ok(create_placeholder_transaction(
            Uuid::now_v7(),
            realm_id,
            TransactionType::Recharge,
        ));
    };
    let provider_transaction_id = object["subscription"]
        .as_str()
        .or_else(|| object["payment_intent"].as_str())
        .or_else(|| object["id"].as_str())
        .ok_or_else(|| CoreError::BadRequest("Missing provider transaction id".to_string()))?
        .to_string();
    let completed_at = parse_optional_stripe_datetime(&object["created"])?.unwrap_or_else(Utc::now);
    let billing_type_override = object["mode"].as_str().and_then(|mode| match mode {
        "payment" => Some(BillingType::OneTime),
        _ => None,
    });

    // Idempotency check: if eager strategy already fulfilled during checkout.session.completed,
    // the attempt is already Succeeded — skip fulfillment.
    // Also guards against the race where async_payment_failed arrives first (status = Failed/Cancelled).
    let existing_attempt = match app_state
        .payment_attempt_service
        .find_payment_attempt(realm_id, attempt_id)
        .await
    {
        Ok(Some(attempt)) => Some(attempt),
        Ok(None) => None, // attempt not found in this realm — proceed with fulfillment
        Err(e) => return Err(e), // DB error — propagate
    };
    if let Some(attempt) = existing_attempt {
        if attempt.status == PaymentAttemptStatus::Succeeded {
            info!(
                realm_id = %realm_id,
                attempt_id = %attempt_id,
                "Async payment succeeded but attempt already fulfilled (eager strategy) - skipping"
            );
            return Ok(create_placeholder_transaction(
                attempt_id,
                realm_id,
                TransactionType::Recharge,
            ));
        }
        if attempt.status.is_terminal() {
            warn!(
                realm_id = %realm_id,
                attempt_id = %attempt_id,
                status = %attempt.status,
                "Async payment succeeded but attempt already in terminal state — success event lost (race with failure)"
            );
            return Ok(create_placeholder_transaction(
                attempt_id,
                realm_id,
                TransactionType::Recharge,
            ));
        }
        // Pending/RequiresAction: proceed with normal fulfillment
    }

    fulfill_payment_attempt(
        &app_state,
        attempt_id,
        "succeeded",
        provider_transaction_id,
        completed_at,
        billing_type_override,
    )
    .await?;

    Ok(create_placeholder_transaction(
        attempt_id,
        realm_id,
        TransactionType::Recharge,
    ))
}

async fn handle_checkout_session_async_failed(
    app_state: AppState,
    event: Value,
    realm_id: &str,
    _idempotency_key: &str,
) -> Result<PointsTransaction, CoreError> {
    let object = &event["data"]["object"];
    let Some(attempt_id) = parse_attempt_id(&object["metadata"][metadata_keys::ATTEMPT_ID]) else {
        warn!(
            realm_id = %realm_id,
            event_id = %parse_event_id(&event)?,
            "Stripe async checkout failure has no attemptId metadata - ignoring"
        );
        return Ok(create_placeholder_transaction(
            Uuid::now_v7(),
            realm_id,
            TransactionType::Recharge,
        ));
    };

    // Fetch payment attempt to determine strategy path
    // Distinguish "not found" from DB errors: NotFound is safe (conservative path),
    // but DB errors must propagate so the webhook retry loop can recover.
    let attempt = match app_state
        .payment_attempt_service
        .find_payment_attempt(realm_id, attempt_id)
        .await
    {
        Ok(Some(attempt)) => Some(attempt),
        Ok(None) => None, // attempt not found in this realm — conservative path
        Err(e) => return Err(e), // DB error — propagate
    };

    let needs_revocation = attempt
        .as_ref()
        .is_some_and(|a| a.status == PaymentAttemptStatus::Succeeded);

    if !needs_revocation {
        // Conservative strategy path: attempt is Pending/Failed/not-found
        let payload = StripePaymentFailedPayload {
            attempt_id,
            provider_reference: object["id"].as_str().unwrap_or("").to_string(),
            provider_status: object["payment_status"]
                .as_str()
                .unwrap_or("failed")
                .to_string(),
            completed_at: parse_optional_stripe_datetime(&object["created"])?
                .unwrap_or_else(Utc::now),
        };
        fail_payment_attempt(&app_state, realm_id, payload).await?;
        return Ok(create_placeholder_transaction(
            attempt_id,
            realm_id,
            TransactionType::Recharge,
        ));
    }

    // Eager strategy path: attempt was already Succeeded — must revoke points
    let attempt = attempt.unwrap();
    let mode_str = object["mode"].as_str();
    let billing_type_override = mode_str.and_then(|mode| match mode {
        "payment" => Some(BillingType::OneTime),
        _ => None,
    });

    if mode_str.is_none() {
        warn!(
            realm_id = %realm_id,
            attempt_id = %attempt_id,
            "mode field missing in async_payment_failed event — defaulting to subscription revocation path"
        );
    }

    info!(
        realm_id = %realm_id,
        attempt_id = %attempt_id,
        user_id = %attempt.user_id,
        billing_type = ?billing_type_override,
        "Async payment failed after eager fulfillment — revoking points"
    );

    let revocation_result = if billing_type_override == Some(BillingType::OneTime) {
        // One-time purchase: revoke only the TopupCredit ledger from this specific attempt
        // (source_id = attempt_id), avoiding over-broad revocation of unrelated topup credits.
        app_state
            .points_service
            .revoke_points_by_source_id(
                realm_id,
                attempt.user_id,
                &attempt_id.to_string(),
                RevocationType::RefundRevoke,
                format!("Async payment failed revocation for attempt {}", attempt_id),
            )
            .await?
    } else {
        // Subscription: cancel subscription + revoke SubscriptionCredit (done internally by handle_subscription_cancel)
        let result = app_state
            .subscription_service
            .handle_subscription_cancel(
                attempt.user_id,
                realm_id,
                CancelMode::ImmediateCancel,
                None,
            )
            .await?;

        // Update subscription record status to "canceled" — scope to the specific subscription
        // to avoid canceling unrelated subscriptions for the same user.
        // Try external_subscription_id from checkout session first, then fall back to
        // the most recent subscription for this entitlement.
        let stripe_subscription_id = object["subscription"].as_str();
        let rows_updated = if let Some(ext_sub_id) = stripe_subscription_id {
            sqlx::query(
                "UPDATE subscription
                 SET status = 'canceled', cancel_at = NOW(), updated_at = NOW()
                 WHERE realm_id = $1 AND user_id = $2 AND external_subscription_id = $3
                   AND status IN ('active', 'trialing', 'past_due', 'scheduled_cancel', 'pending')",
            )
            .bind(realm_id)
            .bind(attempt.user_id)
            .bind(ext_sub_id)
            .execute(&app_state.pool)
            .await?
            .rows_affected()
        } else {
            warn!(
                realm_id = %realm_id,
                attempt_id = %attempt_id,
                "No subscription field in async_payment_failed event — querying entitlement_key for scoped cancel"
            );
            let entitlement_key: Option<String> = sqlx::query_scalar(
                "SELECT entitlement_key FROM entitlement_mapping WHERE id = $1 AND realm_id = $2",
            )
            .bind(attempt.target_id)
            .bind(realm_id)
            .fetch_optional(&app_state.pool)
            .await?
            .flatten();
            if let Some(ekey) = entitlement_key {
                sqlx::query(
                    "UPDATE subscription
                     SET status = 'canceled', cancel_at = NOW(), updated_at = NOW()
                     WHERE realm_id = $1 AND user_id = $2 AND entitlement_key = $3
                       AND status IN ('active', 'trialing', 'past_due', 'scheduled_cancel', 'pending')",
                )
                .bind(realm_id)
                .bind(attempt.user_id)
                .bind(&ekey)
                .execute(&app_state.pool)
                .await?
                .rows_affected()
            } else {
                warn!(
                    realm_id = %realm_id,
                    attempt_id = %attempt_id,
                    target_id = %attempt.target_id,
                    "Cannot resolve entitlement_key for scoped subscription cancel — skipping raw SQL update"
                );
                0
            }
        };

        info!(
            realm_id = %realm_id,
            user_id = %attempt.user_id,
            rows_updated,
            "Subscription status set to canceled after async payment failure"
        );

        result
    };

    // Debt recording: check if total_revoked < original granted points
    // One-time purchases use source_id = attempt_id, subscriptions use source_id = entitlement_key.
    // Combined query retrieves both id and granted_amount in a single trip (Finding 7).
    let source_id_for_ledger = if billing_type_override == Some(BillingType::OneTime) {
        attempt_id.to_string()
    } else {
        // Subscription credits are keyed by entitlement_key in grant_points_atomic
        let entitlement_key: Option<String> = sqlx::query_scalar(
            "SELECT entitlement_key FROM entitlement_mapping WHERE id = $1 AND realm_id = $2",
        )
        .bind(attempt.target_id)
        .bind(realm_id)
        .fetch_optional(&app_state.pool)
        .await?
        .flatten();
        entitlement_key.unwrap_or_else(|| attempt_id.to_string())
    };
    let ledger_row: Option<(Uuid, i64)> = sqlx::query_as(
        "SELECT id, granted_amount FROM points_credit_ledger WHERE realm_id = $1 AND user_id = $2 AND source_id = $3 LIMIT 1"
    )
        .bind(realm_id)
        .bind(attempt.user_id)
        .bind(&source_id_for_ledger)
        .fetch_optional(&app_state.pool)
        .await?;
    let original_points = ledger_row.as_ref().map(|r| r.1).unwrap_or(0);

    if revocation_result.total_revoked < original_points {
        let debt_amount = original_points - revocation_result.total_revoked;
        // Resolve ledger_id: prefer the revocation result, fall back to the combined query above.
        // This handles the case where all credits were already consumed (no ledger entries to revoke).
        let ledger_id = revocation_result
            .ledger_ids
            .first()
            .copied()
            .or_else(|| ledger_row.as_ref().map(|r| r.0))
            .unwrap_or_else(Uuid::nil);
        let reason = format!(
            "debt:original={},recovered={},shortfall={},reason=async_payment_failed_insufficient_balance",
            original_points, revocation_result.total_revoked, debt_amount
        );
        sqlx::query(
            "INSERT INTO points_revocation_records (id, ledger_id, user_id, realm_id, revocation_type, revoked_amount, reason, reference_id, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())"
        )
            .bind(Uuid::now_v7())
            .bind(ledger_id)
            .bind(attempt.user_id)
            .bind(realm_id)
            .bind(RevocationType::RefundRevoke.as_str())
            .bind(debt_amount)
            .bind(&reason)
            .bind(attempt_id.to_string())
            .execute(&app_state.pool)
            .await?;

        info!(
            realm_id = %realm_id,
            attempt_id = %attempt_id,
            user_id = %attempt.user_id,
            original_points = original_points,
            total_revoked = revocation_result.total_revoked,
            debt_amount = debt_amount,
            "Recorded debt for insufficient balance during async failure revocation"
        );
    }

    // Mark payment attempt as Failed via dedicated async recovery method
    app_state
        .payment_attempt_service
        .mark_failed_for_async_recovery(
            realm_id,
            attempt_id,
            object["payment_status"]
                .as_str()
                .unwrap_or("failed")
                .to_string(),
            parse_optional_stripe_datetime(&object["created"])?.unwrap_or_else(Utc::now),
        )
        .await?;

    Ok(create_placeholder_transaction(
        attempt_id,
        realm_id,
        TransactionType::Recharge,
    ))
}

async fn handle_checkout_session_expired(
    app_state: AppState,
    event: Value,
    realm_id: &str,
    _idempotency_key: &str,
) -> Result<PointsTransaction, CoreError> {
    let object = &event["data"]["object"];
    let Some(attempt_id) = parse_attempt_id(&object["metadata"][metadata_keys::ATTEMPT_ID]) else {
        warn!(
            realm_id = %realm_id,
            event_id = %parse_event_id(&event)?,
            "Stripe checkout expired has no attemptId metadata - ignoring"
        );
        return Ok(create_placeholder_transaction(
            Uuid::now_v7(),
            realm_id,
            TransactionType::Recharge,
        ));
    };
    let payload = StripePaymentFailedPayload {
        attempt_id,
        provider_reference: object["id"].as_str().unwrap_or("").to_string(),
        provider_status: "expired".to_string(),
        completed_at: parse_optional_stripe_datetime(&object["expires_at"])?
            .unwrap_or_else(Utc::now),
    };

    fail_payment_attempt(&app_state, realm_id, payload).await?;

    Ok(create_placeholder_transaction(
        attempt_id,
        realm_id,
        TransactionType::Recharge,
    ))
}

async fn handle_payment_failed(
    app_state: AppState,
    event: Value,
    realm_id: &str,
    _idempotency_key: &str,
) -> Result<PointsTransaction, CoreError> {
    let event_id = parse_event_id(&event)?;
    let Some(payload) = parse_payment_failed_payload(&event)? else {
        warn!(
            realm_id = %realm_id,
            event_id = %event_id,
            "Stripe payment_failed event has no attemptId metadata - ignoring"
        );
        return Ok(create_placeholder_transaction(
            Uuid::now_v7(),
            realm_id,
            TransactionType::Recharge,
        ));
    };

    let attempt_id = payload.attempt_id;
    fail_payment_attempt(&app_state, realm_id, payload).await?;

    Ok(create_placeholder_transaction(
        attempt_id,
        realm_id,
        TransactionType::Recharge,
    ))
}

/// Handle customer.subscription.created events
///
/// Grants subscription points when a new subscription is created.
async fn handle_subscription_created(
    app_state: AppState,
    event: Value,
    realm_id: &str,
    _idempotency_key: &str,
) -> Result<PointsTransaction, CoreError> {
    let payload = parse_subscription_created_payload(&event)?;
    let event_id = payload.event_id.as_str();

    info!(
        realm_id = %realm_id,
        event_id = %event_id,
        "Processing customer.subscription.created event"
    );

    // Resolve entitlement_key via fallback chain
    let entitlement_key = if payload.entitlement_key.is_empty() {
        resolve_stripe_entitlement_key(
            &app_state,
            realm_id,
            &event["data"]["object"]["metadata"],
            &payload.external_product_id,
        )
        .await?
    } else {
        payload.entitlement_key.clone()
    };

    let external_price_id = event["data"]["object"]["items"]["data"][0]["price"]["id"]
        .as_str()
        .map(str::to_string);

    let (_subscription, _previous_subscription) = sync_stripe_subscription_with_history_in_txn(
        &app_state,
        realm_id,
        payload.user_id,
        &payload.stripe_subscription_id,
        payload.client_app_id,
        entitlement_key.clone(),
        payload.external_product_id.clone(),
        external_price_id,
        payload.status.clone(),
        payload.current_period_start,
        Some(payload.current_period_end),
        payload.cancel_at_period_end,
        None,
        None,
        HistoryEventType::Created,
    )
    .await?;

    app_state
        .subscription_service
        .handle_subscription_paid(
            payload.user_id,
            realm_id,
            &entitlement_key,
            false,
            payload.current_period_end,
            payload.event_id.clone(),
        )
        .await?;

    info!(
        realm_id = %realm_id,
        user_id = %payload.user_id,
        entitlement_key = %entitlement_key,
        stripe_subscription_id = %payload.stripe_subscription_id,
        event_id = %event_id,
        current_period_end = %payload.current_period_end,
        "Subscription created - ledger and aggregate synced"
    );

    Ok(create_placeholder_transaction(
        payload.user_id,
        realm_id,
        TransactionType::SubscriptionGrant,
    ))
}

/// Handle customer.subscription.updated events
///
/// Handles subscription upgrades and downgrades.
async fn handle_subscription_updated(
    app_state: AppState,
    event: Value,
    realm_id: &str,
    _idempotency_key: &str,
) -> Result<PointsTransaction, CoreError> {
    let payload = parse_subscription_updated_payload(&event)?;
    let event_id = payload.event_id.as_str();

    info!(
        realm_id = %realm_id,
        event_id = %event_id,
        "Processing customer.subscription.updated event"
    );

    // Resolve entitlement_keys via fallback chain
    let current_entitlement_key = if payload.current_entitlement_key.is_empty() {
        resolve_stripe_entitlement_key(
            &app_state,
            realm_id,
            &event["data"]["object"]["items"]["data"][0]["price"]["metadata"],
            &payload.external_product_id,
        )
        .await?
    } else {
        payload.current_entitlement_key.clone()
    };

    // Fetch existing subscription once — reuse for both entitlement resolution and sync
    let existing_subscription_for_update = if payload.previous_entitlement_key.is_empty() {
        app_state
            .billing_repository
            .find_by_external_subscription_id(&payload.stripe_subscription_id, "stripe")
            .await?
    } else {
        None
    };

    let previous_entitlement_key = if payload.previous_entitlement_key.is_empty() {
        // Try to get from existing subscription
        let from_db = existing_subscription_for_update
            .as_ref()
            .map(|s| s.entitlement_key.clone())
            .unwrap_or_default();

        if from_db.is_empty() {
            // Pre-migration subscription with no entitlement_key — resolve via mapping
            resolve_stripe_entitlement_key(
                &app_state,
                realm_id,
                &event["data"]["previous_attributes"]["items"]["data"][0]["price"]["metadata"],
                &payload.external_product_id,
            )
            .await?
        } else {
            from_db
        }
    } else {
        payload.previous_entitlement_key.clone()
    };

    if previous_entitlement_key == current_entitlement_key {
        let external_price_id = event["data"]["object"]["items"]["data"][0]["price"]["id"]
            .as_str()
            .map(str::to_string);
        let (_subscription, _previous_subscription) =
            sync_stripe_subscription_with_detected_history_in_txn(
                &app_state,
                realm_id,
                payload.user_id,
                &payload.stripe_subscription_id,
                None,
                current_entitlement_key.clone(),
                payload.external_product_id.clone(),
                external_price_id,
                payload.status.clone(),
                payload.current_period_start,
                Some(payload.current_period_end),
                payload.cancel_at_period_end,
                if payload.cancel_at_period_end {
                    Some(payload.current_period_end)
                } else {
                    None
                },
                existing_subscription_for_update.clone(),
            )
            .await?;

        return Ok(create_placeholder_transaction(
            payload.user_id,
            realm_id,
            TransactionType::SubscriptionGrant,
        ));
    }

    // Get plan configs to determine if upgrade or downgrade
    let old_mapping = app_state
        .points_repository
        .find_points_policy_by_entitlement_key(realm_id, &previous_entitlement_key)
        .await?
        .ok_or_else(|| {
            CoreError::InternalServerError(format!(
                "Entitlement mapping not found for previous key '{}' during subscription update",
                previous_entitlement_key
            ))
        })?;

    let new_mapping = app_state
        .points_repository
        .find_points_policy_by_entitlement_key(realm_id, &current_entitlement_key)
        .await?
        .ok_or(CoreError::EntitlementMappingNotFound)?;

    let is_upgrade = {
        let old_points = old_mapping.points_per_period.unwrap_or(0);
        let new_points = new_mapping.points_per_period.unwrap_or(0);
        if old_points == 0 && new_points == 0 {
            let external_price_id = event["data"]["object"]["items"]["data"][0]["price"]["id"]
                .as_str()
                .map(str::to_string);
            let (_subscription, _previous_subscription) =
                sync_stripe_subscription_with_detected_history_in_txn(
                    &app_state,
                    realm_id,
                    payload.user_id,
                    &payload.stripe_subscription_id,
                    None,
                    current_entitlement_key.clone(),
                    payload.external_product_id.clone(),
                    external_price_id,
                    payload.status.clone(),
                    payload.current_period_start,
                    Some(payload.current_period_end),
                    payload.cancel_at_period_end,
                    if payload.cancel_at_period_end {
                        Some(payload.current_period_end)
                    } else {
                        None
                    },
                    existing_subscription_for_update.clone(),
                )
                .await?;
            tracing::info!(
                realm_id = %realm_id,
                "Both mappings have no points configured; skipping upgrade/downgrade classification"
            );
            return Ok(create_placeholder_transaction(
                payload.user_id,
                realm_id,
                TransactionType::SubscriptionUpgrade,
            ));
        }
        new_points > old_points
    };

    if is_upgrade {
        let external_price_id = event["data"]["object"]["items"]["data"][0]["price"]["id"]
            .as_str()
            .map(str::to_string);
        let (_subscription, _previous_subscription) = sync_stripe_subscription_with_history_in_txn(
            &app_state,
            realm_id,
            payload.user_id,
            &payload.stripe_subscription_id,
            None,
            current_entitlement_key.clone(),
            payload.external_product_id.clone(),
            external_price_id,
            payload.status.clone(),
            payload.current_period_start,
            Some(payload.current_period_end),
            payload.cancel_at_period_end,
            if payload.cancel_at_period_end {
                Some(payload.current_period_end)
            } else {
                None
            },
            existing_subscription_for_update.clone(),
            HistoryEventType::Upgraded,
        )
        .await?;

        app_state
            .subscription_service
            .handle_subscription_upgrade(
                payload.user_id,
                realm_id,
                &previous_entitlement_key,
                &current_entitlement_key,
                payload.current_period_end,
            )
            .await?;

        info!(
            realm_id = %realm_id,
            user_id = %payload.user_id,
            stripe_subscription_id = %payload.stripe_subscription_id,
            old_entitlement_key = %previous_entitlement_key,
            new_entitlement_key = %current_entitlement_key,
            event_id = %event_id,
            "Processed subscription upgrade"
        );

        Ok(create_placeholder_transaction(
            payload.user_id,
            realm_id,
            TransactionType::SubscriptionUpgrade,
        ))
    } else {
        let external_price_id = event["data"]["object"]["items"]["data"][0]["price"]["id"]
            .as_str()
            .map(str::to_string);
        let (_subscription, _previous_subscription) = sync_stripe_subscription_with_history_in_txn(
            &app_state,
            realm_id,
            payload.user_id,
            &payload.stripe_subscription_id,
            None,
            current_entitlement_key.clone(),
            payload.external_product_id.clone(),
            external_price_id,
            payload.status.clone(),
            payload.current_period_start,
            Some(payload.current_period_end),
            payload.cancel_at_period_end,
            if payload.cancel_at_period_end {
                Some(payload.current_period_end)
            } else {
                None
            },
            existing_subscription_for_update.clone(),
            HistoryEventType::Downgraded,
        )
        .await?;

        app_state
            .subscription_service
            .handle_subscription_downgrade(
                payload.user_id,
                realm_id,
                &previous_entitlement_key,
                &current_entitlement_key,
            )
            .await?;

        info!(
            realm_id = %realm_id,
            user_id = %payload.user_id,
            stripe_subscription_id = %payload.stripe_subscription_id,
            old_entitlement_key = %previous_entitlement_key,
            new_entitlement_key = %current_entitlement_key,
            event_id = %event_id,
            "Processed subscription downgrade"
        );

        // Placeholder for idempotency; actual downgrade classification is in
        // subscription_history via HistoryEventType::Downgraded above.
        Ok(create_placeholder_transaction(
            payload.user_id,
            realm_id,
            TransactionType::SubscriptionDowngrade,
        ))
    }
}

/// Handle customer.subscription.paused / customer.subscription.resumed events
///
/// Lightweight handler that syncs status without upgrade/downgrade logic.
/// Paused/resumed events typically lack `previous_attributes.items`, so using
/// `handle_subscription_updated` would produce an empty `previous_entitlement_key`
/// and trigger incorrect upgrade/downgrade logic.
async fn handle_subscription_status_change(
    app_state: AppState,
    event: Value,
    realm_id: &str,
    _idempotency_key: &str,
) -> Result<PointsTransaction, CoreError> {
    let event_id = parse_event_id(&event)?;
    let object = &event["data"]["object"];
    let stripe_subscription_id = object["id"]
        .as_str()
        .ok_or_else(|| CoreError::BadRequest("Missing subscription id".to_string()))?
        .to_string();
    let metadata = &object["metadata"];
    let user_id = parse_uuid_field(
        metadata_value(metadata, "herald_user_id", "userId"),
        "userId",
    )?;
    let cancel_at_period_end = object["cancel_at_period_end"].as_bool().unwrap_or(false);
    let status = parse_stripe_subscription_status(object["status"].as_str(), cancel_at_period_end)?;
    let external_product_id = object["items"]["data"][0]["price"]["product"]
        .as_str()
        .map(str::to_string)
        .unwrap_or_default();
    let entitlement_key = metadata["herald_entitlement_key"]
        .as_str()
        .or_else(|| metadata["entitlementKey"].as_str())
        .unwrap_or("")
        .to_string();
    let external_price_id = object["items"]["data"][0]["price"]["id"]
        .as_str()
        .map(str::to_string);
    let current_period_start = parse_optional_stripe_datetime(&object["current_period_start"])?;
    let current_period_end =
        parse_stripe_datetime(&object["current_period_end"], "current_period_end")?;

    info!(
        realm_id = %realm_id,
        event_id = %event_id,
        stripe_subscription_id = %stripe_subscription_id,
        status = ?status,
        "Processing customer.subscription paused/resumed event"
    );

    // Resolve entitlement_key via fallback chain
    let entitlement_key = if entitlement_key.is_empty() {
        resolve_stripe_entitlement_key(&app_state, realm_id, metadata, &external_product_id).await?
    } else {
        entitlement_key
    };

    let existing_sub = app_state
        .billing_repository
        .find_by_external_subscription_id(&stripe_subscription_id, "stripe")
        .await?;

    let _synced = sync_stripe_subscription_with_detected_history_in_txn(
        &app_state,
        realm_id,
        user_id,
        &stripe_subscription_id,
        None,
        entitlement_key.clone(),
        external_product_id,
        external_price_id,
        status.clone(),
        current_period_start,
        Some(current_period_end),
        cancel_at_period_end,
        if cancel_at_period_end {
            Some(current_period_end)
        } else {
            None
        },
        existing_sub,
    )
    .await?;

    Ok(create_placeholder_transaction(
        user_id,
        realm_id,
        TransactionType::SubscriptionGrant,
    ))
}

/// Handle customer.subscription.deleted events
///
/// Handles subscription cancellation (immediate or end-of-period).
async fn handle_subscription_deleted(
    app_state: AppState,
    event: Value,
    realm_id: &str,
    _idempotency_key: &str,
) -> Result<PointsTransaction, CoreError> {
    let payload = parse_subscription_deleted_payload(&event)?;
    let event_id = payload.event_id.as_str();

    info!(
        realm_id = %realm_id,
        event_id = %event_id,
        "Processing customer.subscription.deleted event"
    );

    let existing_subscription = app_state
        .billing_repository
        .find_by_external_subscription_id(&payload.stripe_subscription_id, "stripe")
        .await?;
    let entitlement_key = existing_subscription
        .as_ref()
        .map(|s| s.entitlement_key.clone())
        .or(payload.entitlement_key)
        .ok_or_else(|| CoreError::BadRequest("Missing or invalid entitlement_key".to_string()))?;
    let external_product_id = existing_subscription
        .as_ref()
        .map(|subscription| subscription.external_product_id.clone())
        .unwrap_or_default();
    let external_price_id = existing_subscription
        .as_ref()
        .and_then(|s| s.external_price_id.clone());
    let cancel_mode = if payload.cancel_at_period_end {
        CancelMode::DefaultCancel
    } else {
        CancelMode::ImmediateCancel
    };

    let status = if payload.cancel_at_period_end {
        SubscriptionStatus::ScheduledCancel
    } else {
        SubscriptionStatus::Canceled
    };

    let (_subscription, _previous_subscription) = sync_stripe_subscription_with_history_in_txn(
        &app_state,
        realm_id,
        payload.user_id,
        &payload.stripe_subscription_id,
        existing_subscription
            .as_ref()
            .and_then(|subscription| subscription.client_app_id),
        entitlement_key,
        external_product_id,
        external_price_id,
        status,
        payload.current_period_start,
        Some(payload.current_period_end),
        payload.cancel_at_period_end,
        Some(if payload.cancel_at_period_end {
            payload.current_period_end
        } else {
            Utc::now()
        }),
        existing_subscription.clone(),
        HistoryEventType::Canceled,
    )
    .await?;

    app_state
        .subscription_service
        .handle_subscription_cancel(
            payload.user_id,
            realm_id,
            cancel_mode,
            if payload.cancel_at_period_end {
                Some(payload.current_period_end)
            } else {
                None
            },
        )
        .await?;

    info!(
        realm_id = %realm_id,
        user_id = %payload.user_id,
        stripe_subscription_id = %payload.stripe_subscription_id,
        event_id = %event_id,
        cancel_at_period_end = payload.cancel_at_period_end,
        "Subscription deleted - cancel flow completed"
    );

    Ok(create_placeholder_transaction(
        payload.user_id,
        realm_id,
        TransactionType::CancelRevoke,
    ))
}

/// Handle charge.refunded events
///
/// Revokes unused points based on refund type.
async fn handle_charge_refunded(
    app_state: AppState,
    event: Value,
    realm_id: &str,
    _idempotency_key: &str,
) -> Result<PointsTransaction, CoreError> {
    let payload = parse_charge_refunded_payload(&event)?;
    let event_id = payload.event_id.as_str();

    info!(
        realm_id = %realm_id,
        event_id = %event_id,
        "Processing charge.refunded event"
    );

    info!(
        realm_id = %realm_id,
        charge_id = %payload.charge_id,
        amount = payload.amount_refunded,
        refund_type = %payload.refund_type,
        user_id = %payload.user_id,
        event_id = %event_id,
        "Processing refund - revoking points"
    );

    // Revoke points based on refund type
    match payload.refund_type.as_str() {
        "topup" => {
            // Proportionally revoke topup credits based on refund ratio
            let _output = app_state
                .points_service
                .revoke_topup_proportional(
                    realm_id,
                    payload.user_id,
                    payload.amount_refunded,
                    payload.amount,
                    &payload.charge_id,
                )
                .await?;

            info!(
                realm_id = %realm_id,
                user_id = %payload.user_id,
                charge_id = %payload.charge_id,
                amount = payload.amount_refunded,
                "Topup refund - proportionally revoked topup credits"
            );
        }
        _ => {
            // Revoke all unused subscription credits (default)
            let _output = app_state
                .points_service
                .revoke_subscription_unused(realm_id, payload.user_id, &payload.charge_id)
                .await?;

            info!(
                realm_id = %realm_id,
                user_id = %payload.user_id,
                charge_id = %payload.charge_id,
                "Subscription refund - revoked unused subscription credits"
            );
        }
    }

    if let Some(subscription_id) = payload.subscription_id
        && let Some(subscription) = app_state
            .billing_repository
            .find_subscription_by_id(subscription_id)
            .await?
    {
        let history_event = SubscriptionHistoryService::create_subscription_refunded_event(
            &subscription,
            serde_json::json!({
                "provider": "stripe",
                "chargeId": payload.charge_id,
                "amountRefunded": payload.amount_refunded,
                "refundType": payload.refund_type,
            }),
            Some(ACTOR_WEBHOOK.to_string()),
        );
        app_state
            .billing_repository
            .save_history_event(history_event)
            .await?;
    }

    Ok(create_placeholder_transaction(
        payload.user_id,
        realm_id,
        TransactionType::RefundRevoke,
    ))
}

/// Handle invoice.payment_succeeded events
///
/// Grants points on subscription renewal.
async fn handle_invoice_payment_succeeded(
    app_state: AppState,
    event: Value,
    realm_id: &str,
    _idempotency_key: &str,
) -> Result<PointsTransaction, CoreError> {
    let payload = parse_invoice_paid_payload(&event)?;
    let event_id = payload.event_id.as_str();

    info!(
        realm_id = %realm_id,
        event_id = %event_id,
        "Processing invoice.payment_succeeded event"
    );

    let existing_subscription = app_state
        .billing_repository
        .find_by_external_subscription_id(&payload.stripe_subscription_id, "stripe")
        .await?;

    // Resolve entitlement_key via fallback chain
    let entitlement_key = if payload.entitlement_key.is_empty() {
        let external_product_id = existing_subscription
            .as_ref()
            .map(|s| s.external_product_id.clone())
            .unwrap_or_default();
        resolve_stripe_entitlement_key(
            &app_state,
            realm_id,
            &event["data"]["object"]["metadata"],
            &external_product_id,
        )
        .await?
    } else {
        payload.entitlement_key.clone()
    };

    let external_product_id = existing_subscription
        .as_ref()
        .map(|subscription| subscription.external_product_id.clone())
        .unwrap_or_default();
    let external_price_id = existing_subscription
        .as_ref()
        .and_then(|s| s.external_price_id.clone());

    let (_subscription, _previous_subscription) = sync_stripe_subscription_with_history_in_txn(
        &app_state,
        realm_id,
        payload.user_id,
        &payload.stripe_subscription_id,
        existing_subscription
            .as_ref()
            .and_then(|subscription| subscription.client_app_id),
        entitlement_key.clone(),
        external_product_id,
        external_price_id,
        SubscriptionStatus::Active,
        payload.current_period_start,
        Some(payload.current_period_end),
        false,
        None,
        existing_subscription.clone(),
        HistoryEventType::Renewed,
    )
    .await?;

    app_state
        .subscription_service
        .handle_subscription_paid(
            payload.user_id,
            realm_id,
            &entitlement_key,
            true,
            payload.current_period_end,
            payload.event_id.clone(),
        )
        .await?;

    info!(
        realm_id = %realm_id,
        user_id = %payload.user_id,
        entitlement_key = %entitlement_key,
        stripe_subscription_id = %payload.stripe_subscription_id,
        event_id = %event_id,
        current_period_end = %payload.current_period_end,
        "Invoice payment succeeded - renewal ledger granted"
    );

    Ok(create_placeholder_transaction(
        payload.user_id,
        realm_id,
        TransactionType::SubscriptionRenewal,
    ))
}

/// Handle invoice.created / invoice.finalized / invoice.paid / invoice.voided events
///
/// Syncs Stripe invoice state to Herald external invoice via upsert.
/// This is distinct from `invoice.payment_succeeded` which handles credit granting.
async fn handle_stripe_invoice_event(
    app_state: AppState,
    event: Value,
    realm_id: &str,
    _idempotency_key: &str,
) -> Result<PointsTransaction, CoreError> {
    let event_id = parse_event_id(&event)?;
    let object = &event["data"]["object"];

    let stripe_invoice_id = object["id"]
        .as_str()
        .ok_or_else(|| CoreError::BadRequest("Missing Stripe invoice id".to_string()))?
        .to_string();

    let stripe_status = object["status"].as_str().unwrap_or("draft");

    let status = map_stripe_invoice_status(stripe_status)?;

    let total = object["total"].as_i64().unwrap_or(0);

    let currency = object["currency"].as_str().unwrap_or("usd").to_string();

    let hosted_invoice_url = object["hosted_invoice_url"].as_str().map(str::to_string);
    let invoice_pdf = object["invoice_pdf"].as_str().map(str::to_string);

    // Resolve account_id: try subscription lookup first, then metadata userId
    let stripe_subscription_id = object["subscription"].as_str();
    let mut account_id: Option<Uuid> = None;

    if let Some(stripe_sub_id) = stripe_subscription_id
        && let Ok(Some(subscription)) = app_state
            .billing_repository
            .find_by_external_subscription_id(stripe_sub_id, "stripe")
            .await
        && let Some(user_id) = subscription.user_id
    {
        account_id = Some(user_id);
    }

    if account_id.is_none() {
        account_id = parse_optional_uuid_field(&object["metadata"]["userId"]);
    }

    if account_id.is_none() {
        warn!(
            realm_id = %realm_id,
            event_id = %event_id,
            stripe_invoice_id = %stripe_invoice_id,
            "Could not resolve account_id for Stripe invoice event - creating with account_id=None"
        );
    }

    info!(
        realm_id = %realm_id,
        event_id = %event_id,
        stripe_invoice_id = %stripe_invoice_id,
        stripe_status = %stripe_status,
        status = %status.as_str(),
        total,
        currency = %currency,
        account_id = ?account_id,
        "Processing Stripe invoice event - upserting external invoice"
    );

    let external_data = ExternalInvoiceData {
        realm_id: realm_id.to_string(),
        provider: InvoiceProvider::Stripe,
        payment_provider: Some("stripe".to_string()),
        external_invoice_id: Some(stripe_invoice_id.clone()),
        external_order_id: None,
        external_status: Some(stripe_status.to_string()),
        external_hosted_url: hosted_invoice_url,
        external_pdf_url: invoice_pdf,
        external_payload: Some(object.clone()),
        tax_details: None,
        account_id,
        currency,
        total,
        status,
    };

    app_state
        .invoice_repository
        .upsert_external_invoice(external_data)
        .await?;

    info!(
        realm_id = %realm_id,
        event_id = %event_id,
        stripe_invoice_id = %stripe_invoice_id,
        "Stripe invoice event processed - external invoice upserted"
    );

    Ok(create_placeholder_transaction(
        Uuid::now_v7(),
        realm_id,
        TransactionType::SubscriptionGrant,
    ))
}

async fn find_stripe_subscription_from_metadata(
    app_state: &AppState,
    realm_id: &str,
    object: &Value,
) -> Result<Option<Subscription>, CoreError> {
    // Primary path: look for herald-specific keys in object metadata.
    if let Some(subscription_id) = parse_optional_uuid_field(metadata_value(
        &object["metadata"],
        "herald_subscription_id",
        "subscriptionId",
    )) {
        return app_state
            .billing_repository
            .find_subscription_by_id(subscription_id)
            .await;
    }

    if let Some(external_subscription_id) = object["metadata"]["herald_external_subscription_id"]
        .as_str()
        .or_else(|| object["metadata"]["externalSubscriptionId"].as_str())
    {
        return app_state
            .billing_repository
            .find_by_external_subscription_id(external_subscription_id, "stripe")
            .await;
    }

    // Fallback for dispute objects: their metadata is dispute-level, not the original
    // charge/subscription metadata. Use payment_intent to trace back to the subscription
    // via previously stored payment events (e.g. checkout.session.completed).
    if let Some(payment_intent) = object["payment_intent"].as_str() {
        let stripe_subscription_id = app_state
            .billing_repository
            .find_external_subscription_id_by_payment_intent(payment_intent, "stripe", realm_id)
            .await?;

        if let Some(stripe_sub_id) = stripe_subscription_id
            && let Some(sub) = app_state
                .billing_repository
                .find_by_external_subscription_id(&stripe_sub_id, "stripe")
                .await?
        {
            return Ok(Some(sub));
        }
    }

    Ok(None)
}

async fn handle_charge_dispute_created(
    app_state: AppState,
    event: Value,
    realm_id: &str,
    _idempotency_key: &str,
) -> Result<PointsTransaction, CoreError> {
    let event_id = parse_event_id(&event)?;
    let object = &event["data"]["object"];
    let Some(existing) =
        find_stripe_subscription_from_metadata(&app_state, realm_id, object).await?
    else {
        warn!(
            realm_id = %realm_id,
            event_id = %event_id,
            "Stripe dispute created could not be mapped to a local subscription - ignoring"
        );
        return Ok(create_placeholder_transaction(
            Uuid::now_v7(),
            realm_id,
            TransactionType::SubscriptionGrant,
        ));
    };
    let user_id = existing
        .user_id
        .ok_or_else(|| CoreError::BadRequest("Disputed subscription has no userId".to_string()))?;
    let mut provider_metadata = existing
        .provider_metadata
        .clone()
        .unwrap_or(serde_json::json!({}));
    if let Some(obj) = provider_metadata.as_object_mut() {
        obj.insert("disputeId".to_string(), object["id"].clone());
        obj.insert("charge".to_string(), object["charge"].clone());
        obj.insert(
            "paymentIntent".to_string(),
            object["payment_intent"].clone(),
        );
        obj.insert("dispute_amount".to_string(), object["amount"].clone());
        obj.insert("dispute_reason".to_string(), object["reason"].clone());
    }

    let _synced = sync_subscription_input_with_history_in_txn(
        &app_state,
        SyncSubscriptionInput {
            provider: "stripe",
            realm_id: realm_id.to_string(),
            user_id: Some(user_id),
            external_subscription_id: existing.external_subscription_id.clone(),
            external_product_id: existing.external_product_id.clone(),
            client_app_id: existing.client_app_id,
            entitlement_key: existing.entitlement_key.clone(),
            external_price_id: existing.external_price_id.clone(),
            provider_metadata: Some(provider_metadata),
            status: SubscriptionStatus::Dispute,
            current_period_start: existing.current_period_start,
            current_period_end: existing.current_period_end,
            cancel_at_period_end: existing.cancel_at_period_end,
            cancel_at: existing.cancel_at,
            existing_subscription: Some(existing),
        },
        HistoryEventType::Disputed,
    )
    .await?;

    Ok(create_placeholder_transaction(
        user_id,
        realm_id,
        TransactionType::SubscriptionGrant,
    ))
}

async fn handle_charge_dispute_closed(
    app_state: AppState,
    event: Value,
    realm_id: &str,
    _idempotency_key: &str,
) -> Result<PointsTransaction, CoreError> {
    let event_id = parse_event_id(&event)?;
    let object = &event["data"]["object"];
    let Some(existing) =
        find_stripe_subscription_from_metadata(&app_state, realm_id, object).await?
    else {
        warn!(
            realm_id = %realm_id,
            event_id = %event_id,
            "Stripe dispute closed could not be mapped to a local subscription - ignoring"
        );
        return Ok(create_placeholder_transaction(
            Uuid::now_v7(),
            realm_id,
            TransactionType::SubscriptionGrant,
        ));
    };
    let user_id = existing
        .user_id
        .ok_or_else(|| CoreError::BadRequest("Disputed subscription has no userId".to_string()))?;
    let dispute_status = object["status"].as_str().unwrap_or("");
    let needs_cancel = dispute_status == "lost";
    let target_status = match dispute_status {
        "lost" => SubscriptionStatus::Canceled,
        "won" => SubscriptionStatus::Active,
        // warning_closed, charge_refunded, etc. — log and stay in current state
        _ => {
            warn!(
                realm_id = %realm_id,
                event_id = %event_id,
                dispute_status = %dispute_status,
                "Stripe dispute closed with non-terminal status — not reactivating"
            );
            existing.status.clone()
        }
    };

    let synced = sync_subscription_input_with_detected_history_in_txn(
        &app_state,
        SyncSubscriptionInput {
            provider: "stripe",
            realm_id: realm_id.to_string(),
            user_id: Some(user_id),
            external_subscription_id: existing.external_subscription_id.clone(),
            external_product_id: existing.external_product_id.clone(),
            client_app_id: existing.client_app_id,
            entitlement_key: existing.entitlement_key.clone(),
            external_price_id: existing.external_price_id.clone(),
            provider_metadata: existing.provider_metadata.clone(),
            status: target_status.clone(),
            current_period_start: existing.current_period_start,
            current_period_end: existing.current_period_end,
            cancel_at_period_end: existing.cancel_at_period_end,
            cancel_at: if target_status == SubscriptionStatus::Canceled {
                Some(Utc::now())
            } else {
                existing.cancel_at
            },
            existing_subscription: Some(existing),
        },
    )
    .await?;

    if synced.is_some() && needs_cancel {
        app_state
            .subscription_service
            .handle_subscription_cancel(user_id, realm_id, CancelMode::ImmediateCancel, None)
            .await?;
    }

    Ok(create_placeholder_transaction(
        user_id,
        realm_id,
        TransactionType::SubscriptionGrant,
    ))
}

async fn handle_invoice_payment_action_required(
    event: Value,
    realm_id: &str,
) -> Result<PointsTransaction, CoreError> {
    warn!(
        realm_id = %realm_id,
        event_id = %parse_event_id(&event)?,
        invoice_id = ?event["data"]["object"]["id"].as_str(),
        "Stripe invoice payment requires customer action"
    );

    Ok(create_placeholder_transaction(
        Uuid::now_v7(),
        realm_id,
        TransactionType::SubscriptionGrant,
    ))
}

// ============================================================================
// Main Webhook Handler
// ============================================================================

/// Handle Stripe webhook events
///
/// Verifies signature, checks idempotency, routes to appropriate handler,
/// and returns 200 OK immediately. Processing happens synchronously (for now).
pub async fn handle_stripe_webhook(
    State(app_state): State<AppState>,
    Path(realm_id): Path<String>,
    headers: HeaderMap,
    body: String,
) -> Result<StatusCode, CoreError> {
    // Step 1: Parse event JSON (for logging and processing)
    let event: Value = serde_json::from_str(&body).map_err(|e| {
        error!("Failed to parse webhook JSON: {}", e);
        CoreError::BadRequest(format!("Invalid JSON: {}", e))
    })?;

    // Note: realm_id is now extracted from URL path parameter, not from metadata

    // Step 3: Extract and validate signature header
    let signature = headers
        .get("stripe-signature")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            error!("Missing stripe-signature header");
            CoreError::BadRequest("Missing signature".to_string())
        })?;

    // Step 4: Get webhook secret from database for this realm
    let webhook_secret = sqlx::query_scalar::<_, String>(
        "SELECT config_value
         FROM realm_config
         WHERE realm_id = $1 AND config_type = 'stripe' AND config_key = 'webhook_secret' AND enabled = true
         LIMIT 1"
    )
    .bind(&realm_id)
    .fetch_optional(&app_state.pool)
    .await
    .map_err(|e| {
        error!("Failed to load webhook secret from database: {}", e);
        CoreError::InternalServerError(format!("Database error: {}", e))
    })?
    .ok_or_else(|| {
        error!(
            realm_id = %realm_id,
            "Webhook secret not found in database"
        );
        CoreError::InternalServerError(format!(
            "Webhook secret not configured for realm: {}",
            realm_id
        ))
    })?;

    // Step 5: Verify webhook signature using StripeClient (static method)
    herald_core::infrastructure::stripe::StripeClient::verify_webhook_signature(
        body.as_bytes(),
        signature,
        &webhook_secret,
    )?;

    // Step 6: Extract event metadata
    let event_id = event["id"]
        .as_str()
        .ok_or_else(|| CoreError::BadRequest("Missing event id".to_string()))?
        .to_string();

    let event_type = event["type"]
        .as_str()
        .ok_or_else(|| CoreError::BadRequest("Missing event type".to_string()))?
        .to_string();

    // Step 7: Save payment event to database (for audit trail and idempotency)
    let new_payment_event = PaymentEvent {
        id: Uuid::now_v7(),
        realm_id: realm_id.clone(),
        external_event_id: event_id.clone(),
        payment_provider: "stripe".to_string(),
        event_type: event_type.clone(),
        subscription_id: None,
        payload: event.clone(),
        processed: false,
        processing_started_at: None,
        created_at: chrono::Utc::now(),
    };

    // Check for existing payment event (idempotency check)
    let existing_event = app_state
        .billing_repository
        .find_payment_event_by_external_id(&event_id, "stripe")
        .await?;

    if existing_event.is_some() {
        info!(
            realm_id = %realm_id,
            event_id = %event_id,
            event_type = %event_type,
            "Duplicate webhook event - returning OK"
        );
        return Ok(StatusCode::OK);
    }

    // Save new payment event (handle race condition from concurrent webhooks)
    let saved_event = match app_state
        .billing_repository
        .create_payment_event(new_payment_event)
        .await
    {
        Ok(saved_event) => saved_event,
        Err(CoreError::DatabaseError(msg))
            if msg.contains("unique constraint") || msg.contains("duplicate key") =>
        {
            // Concurrent webhook with same event_id already inserted - treat as success
            info!(
                realm_id = %realm_id,
                event_id = %event_id,
                event_type = %event_type,
                "Concurrent webhook event already inserted - returning OK"
            );
            return Ok(StatusCode::OK);
        }
        Err(e) => return Err(e),
    };

    // Step 8: Build idempotency key
    let idempotency_key = format!("stripe_{}", event_id);

    // Step 9: Check idempotency
    let idempotency_service = &app_state.idempotency_service;

    let idempotency_result = idempotency_service
        .check_or_create(&realm_id, &idempotency_key, &body)
        .await?;

    // If cached result exists, return 200 OK
    if let IdempotencyResult::Cached {
        transaction: PointsTransaction {
            id: transaction_id, ..
        },
    } = idempotency_result
    {
        let _ = app_state
            .billing_repository
            .mark_payment_event_processed(saved_event.id)
            .await;
        info!(
            realm_id = %realm_id,
            event_id = %event_id,
            event_type = %event_type,
            transaction_id = %transaction_id,
            "Returning cached result for duplicate webhook event"
        );
        return Ok(StatusCode::OK);
    }

    // Step 10: Route to appropriate handler, retrying transient handler failures.
    let result = process_stripe_event_with_retries(
        app_state.clone(),
        &event,
        &realm_id,
        &idempotency_key,
        &event_id,
        &event_type,
    )
    .await;

    // Step 11: Handle result
    match result {
        Ok(transaction) => {
            idempotency_service
                .save_result(&realm_id, &idempotency_key, &transaction)
                .await?;
            app_state
                .billing_repository
                .mark_payment_event_processed(saved_event.id)
                .await?;
            info!(
                realm_id = %realm_id,
                event_id = %event_id,
                event_type = %event_type,
                transaction_id = %transaction.id,
                "Stripe webhook processed successfully"
            );
            Ok(StatusCode::OK)
        }
        Err(e) => {
            let _ = idempotency_service
                .mark_failed(&realm_id, &idempotency_key)
                .await;
            error!(
                realm_id = %realm_id,
                event_id = %event_id,
                event_type = %event_type,
                error = %e,
                "Failed to process Stripe webhook event"
            );
            Err(e)
        }
    }
}

async fn process_stripe_event_with_retries(
    app_state: AppState,
    event: &Value,
    realm_id: &str,
    idempotency_key: &str,
    event_id: &str,
    event_type: &str,
) -> Result<PointsTransaction, CoreError> {
    const MAX_ATTEMPTS: u32 = 3;

    for attempt in 1..=MAX_ATTEMPTS {
        let result = process_stripe_event_once(
            app_state.clone(),
            event,
            realm_id,
            idempotency_key,
            event_id,
            event_type,
        )
        .await;

        match result {
            Ok(transaction) => return Ok(transaction),
            Err(e) if attempt < MAX_ATTEMPTS => {
                // Only retry transient errors (database, internal, network), not permanent ones
                let is_transient = matches!(
                    &e,
                    CoreError::DatabaseError(_)
                        | CoreError::InternalServerError(_)
                        | CoreError::RateLimitExceeded
                );
                if !is_transient {
                    return Err(e);
                }
                warn!(
                    realm_id = %realm_id,
                    event_id = %event_id,
                    event_type = %event_type,
                    attempt,
                    error = %e,
                    "Stripe webhook handler failed with transient error, retrying"
                );
                tokio::time::sleep(Duration::from_millis(100 * attempt as u64)).await;
            }
            Err(e) => return Err(e),
        }
    }

    unreachable!("retry loop always returns")
}

async fn process_stripe_event_once(
    app_state: AppState,
    event: &Value,
    realm_id: &str,
    idempotency_key: &str,
    event_id: &str,
    event_type: &str,
) -> Result<PointsTransaction, CoreError> {
    match event_type {
        "checkout.session.completed" => {
            handle_checkout_session_completed(
                app_state.clone(),
                event.clone(),
                realm_id,
                idempotency_key,
            )
            .await
        }
        "checkout.session.expired" => {
            handle_checkout_session_expired(
                app_state.clone(),
                event.clone(),
                realm_id,
                idempotency_key,
            )
            .await
        }
        "checkout.session.async_payment_succeeded" => {
            handle_checkout_session_async_succeeded(
                app_state.clone(),
                event.clone(),
                realm_id,
                idempotency_key,
            )
            .await
        }
        "checkout.session.async_payment_failed" => {
            handle_checkout_session_async_failed(
                app_state.clone(),
                event.clone(),
                realm_id,
                idempotency_key,
            )
            .await
        }
        "customer.subscription.created" => {
            handle_subscription_created(app_state.clone(), event.clone(), realm_id, idempotency_key)
                .await
        }
        "customer.subscription.updated" => {
            handle_subscription_updated(app_state.clone(), event.clone(), realm_id, idempotency_key)
                .await
        }
        "customer.subscription.paused" | "customer.subscription.resumed" => {
            handle_subscription_status_change(
                app_state.clone(),
                event.clone(),
                realm_id,
                idempotency_key,
            )
            .await
        }
        "customer.subscription.deleted" => {
            handle_subscription_deleted(app_state.clone(), event.clone(), realm_id, idempotency_key)
                .await
        }
        "charge.refunded" => {
            handle_charge_refunded(app_state.clone(), event.clone(), realm_id, idempotency_key)
                .await
        }
        "charge.dispute.created" => {
            handle_charge_dispute_created(
                app_state.clone(),
                event.clone(),
                realm_id,
                idempotency_key,
            )
            .await
        }
        "charge.dispute.closed" => {
            handle_charge_dispute_closed(
                app_state.clone(),
                event.clone(),
                realm_id,
                idempotency_key,
            )
            .await
        }
        "invoice.payment_succeeded" => {
            handle_invoice_payment_succeeded(
                app_state.clone(),
                event.clone(),
                realm_id,
                idempotency_key,
            )
            .await
        }
        "invoice.payment_action_required" => {
            handle_invoice_payment_action_required(event.clone(), realm_id).await
        }
        "payment_intent.succeeded" => {
            handle_payment_intent_succeeded(
                app_state.clone(),
                event.clone(),
                realm_id,
                idempotency_key,
            )
            .await
        }
        "payment_intent.payment_failed" | "invoice.payment_failed" => {
            handle_payment_failed(app_state.clone(), event.clone(), realm_id, idempotency_key).await
        }
        "invoice.created" | "invoice.finalized" | "invoice.paid" | "invoice.voided" => {
            handle_stripe_invoice_event(app_state.clone(), event.clone(), realm_id, idempotency_key)
                .await
        }
        _ => {
            warn!(
                realm_id = %realm_id,
                event_id = %event_id,
                event_type = %event_type,
                "Unknown Stripe event type - ignoring"
            );
            Ok(create_placeholder_transaction(
                Uuid::now_v7(),
                realm_id,
                TransactionType::SubscriptionGrant,
            ))
        }
    }
}

/// Reprocess a single Stripe event that Herald missed (compensation path).
///
/// Unlike the normal webhook flow, this:
/// - Skips Redis idempotency checks entirely
/// - Skips signature verification (event comes from Stripe Events API, not webhook)
/// - Uses DB `payment_event` for idempotency only
/// - Reuses the same match routing from `process_stripe_event_once`
pub(crate) async fn reprocess_stripe_event(
    app_state: AppState,
    realm_id: &str,
    event: &Value,
    event_type: &str,
) -> Result<(), CoreError> {
    let event_id = event["id"]
        .as_str()
        .ok_or_else(|| CoreError::BadRequest("Missing event id".to_string()))?;

    // Check existing payment event by external_event_id
    if let Some(existing) = app_state
        .billing_repository
        .find_payment_event_by_external_id(event_id, "stripe")
        .await?
    {
        if existing.processed {
            info!(
                realm_id = %realm_id,
                event_id = %event_id,
                event_type = %event_type,
                "Stripe compensation: event already processed, skipping"
            );
            return Ok(());
        }
        // Exists but not processed -- inconsistent state, do not reprocess
        error!(
            realm_id = %realm_id,
            event_id = %event_id,
            event_type = %event_type,
            "Stripe compensation: event exists but processed=false, skipping inconsistent event"
        );
        return Ok(());
    }

    // Create payment event record
    let new_payment_event = PaymentEvent {
        id: Uuid::now_v7(),
        realm_id: realm_id.to_string(),
        external_event_id: event_id.to_string(),
        payment_provider: "stripe".to_string(),
        event_type: event_type.to_string(),
        subscription_id: None,
        payload: event.clone(),
        processed: false,
        processing_started_at: None,
        created_at: chrono::Utc::now(),
    };

    let saved_event = match app_state
        .billing_repository
        .create_payment_event(new_payment_event)
        .await
    {
        Ok(event) => event,
        Err(CoreError::DatabaseError(ref msg))
            if msg.contains("unique constraint") || msg.contains("duplicate key") =>
        {
            info!(
                realm_id = %realm_id,
                event_id = %event_id,
                event_type = %event_type,
                "Stripe compensation: concurrent insert detected, event already handled"
            );
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    // Synthetic idempotency key (not used for Redis, only passed to handlers)
    let idempotency_key = format!("compensation_stripe_{}", event_id);

    // Route to the same handler match branches
    let result = process_stripe_event_once(
        app_state.clone(),
        event,
        realm_id,
        &idempotency_key,
        event_id,
        event_type,
    )
    .await;

    match result {
        Ok(_transaction) => {
            if let Err(e) = app_state
                .billing_repository
                .mark_payment_event_processed(saved_event.id)
                .await
            {
                tracing::error!(
                    realm_id = %realm_id,
                    event_id = %event_id,
                    event_type = %event_type,
                    error = %e,
                    "Stripe compensation: handler succeeded but failed to mark payment_event as processed — event may be reprocessed on next run"
                );
            } else {
                tracing::info!(
                    realm_id = %realm_id,
                    event_id = %event_id,
                    event_type = %event_type,
                    "Stripe compensation: event reprocessed successfully"
                );
            }
            Ok(())
        }
        Err(e) => {
            error!(
                realm_id = %realm_id,
                event_id = %event_id,
                event_type = %event_type,
                error = %e,
                "Stripe compensation: failed to reprocess event"
            );
            Err(e)
        }
    }
}

/// Strategy for handling points issuance when an async payment method is used
/// (SEPA, ACH, BECS, Bacs) and `payment_status` is `unpaid` at checkout completion.
///
/// - `Conservative` (default): wait for `async_payment_succeeded` before issuing points.
/// - `Eager`: issue points immediately on `checkout.session.completed`; reclaim on failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsyncPointsStrategy {
    Conservative,
    Eager,
}

impl From<&str> for AsyncPointsStrategy {
    fn from(value: &str) -> Self {
        match value {
            "eager" => AsyncPointsStrategy::Eager,
            _ => AsyncPointsStrategy::Conservative,
        }
    }
}

/// Read the `async_points_strategy` config value for a given realm.
///
/// Returns `Conservative` when no config row exists or the value is not `"eager"`.
pub async fn read_async_points_strategy(
    pool: &sqlx::PgPool,
    realm_id: &str,
) -> AsyncPointsStrategy {
    let value = sqlx::query_scalar::<_, String>(
        "SELECT config_value
         FROM realm_config
         WHERE realm_id = $1 AND config_type = 'stripe' AND config_key = 'async_points_strategy' AND enabled = true
         LIMIT 1",
    )
    .bind(realm_id)
    .fetch_optional(pool)
    .await;

    match value {
        Ok(Some(v)) => AsyncPointsStrategy::from(v.as_str()),
        Ok(None) => AsyncPointsStrategy::Conservative,
        Err(e) => {
            warn!(
                realm_id = %realm_id,
                error = %e,
                "Failed to read async_points_strategy from DB, defaulting to Conservative"
            );
            AsyncPointsStrategy::Conservative
        }
    }
}
