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

use crate::webhook_common::create_placeholder_transaction;
use crate::webhook_subscription_helpers::{
    SyncSubscriptionInput, save_subscription_history, sync_subscription,
};
use herald_api_base::application::http::state::AppState;
use herald_core::domain::billing::{
    ACTOR_WEBHOOK, BillingPeriod, BillingRepository, HistoryEventType, PaymentEvent, Subscription,
    SubscriptionHistoryService, SubscriptionStatus, SubscriptionTier,
};
use herald_core::domain::common::entities::app_errors::CoreError;
use herald_core::domain::points::IdempotencyResult;
use herald_core::domain::points::entities::{PointsTransaction, TransactionType};
use herald_core::domain::points::ports::PointsRepository;
use herald_core::domain::points::subscription_service::CancelMode;
use herald_core::domain::purchase::{CompletePaymentAttemptInput, PaymentCompletionSource};

struct StripeCheckoutCompletedPayload {
    event_id: String,
    user_id: Uuid,
    client_app_id: Uuid,
    plan_id: Uuid,
    billing_period: BillingPeriod,
    is_trial: bool,
    stripe_subscription_id: String,
    stripe_product_id: String,
}

struct StripeSubscriptionCreatedPayload {
    event_id: String,
    stripe_subscription_id: String,
    user_id: Uuid,
    plan_id: Uuid,
    client_app_id: Option<Uuid>,
    external_product_id: String,
    billing_period: BillingPeriod,
    cancel_at_period_end: bool,
    current_period_start: Option<DateTime<Utc>>,
    current_period_end: DateTime<Utc>,
    status: SubscriptionStatus,
}

struct StripeSubscriptionUpdatedPayload {
    event_id: String,
    stripe_subscription_id: String,
    user_id: Uuid,
    previous_plan_id: Uuid,
    current_plan_id: Uuid,
    external_product_id: String,
    billing_period: BillingPeriod,
    cancel_at_period_end: bool,
    current_period_start: Option<DateTime<Utc>>,
    current_period_end: DateTime<Utc>,
    status: SubscriptionStatus,
}

struct StripeSubscriptionDeletedPayload {
    event_id: String,
    stripe_subscription_id: String,
    user_id: Uuid,
    plan_id: Option<Uuid>,
    cancel_at_period_end: bool,
    current_period_start: Option<DateTime<Utc>>,
    current_period_end: DateTime<Utc>,
}

struct StripeChargeRefundedPayload {
    event_id: String,
    charge_id: String,
    amount_refunded: i64,
    user_id: Uuid,
    refund_type: String,
}

struct StripeInvoicePaidPayload {
    event_id: String,
    stripe_subscription_id: String,
    user_id: Uuid,
    plan_id: Uuid,
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

fn parse_event_id(event: &Value) -> Result<String, CoreError> {
    event["id"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| CoreError::BadRequest("Missing event id".to_string()))
}

fn parse_uuid_field(value: &Value, field_name: &str) -> Result<Uuid, CoreError> {
    value
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| CoreError::BadRequest(format!("Missing or invalid {}", field_name)))
}

fn parse_optional_uuid_field(value: &Value) -> Option<Uuid> {
    value.as_str().and_then(|s| Uuid::parse_str(s).ok())
}

fn parse_attempt_id(value: &Value) -> Option<Uuid> {
    value
        .as_str()
        .and_then(|raw| Uuid::parse_str(raw).ok())
        .filter(|id| *id != Uuid::nil())
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

fn parse_billing_period(value: Option<&str>) -> BillingPeriod {
    match value.unwrap_or("monthly") {
        "yearly" => BillingPeriod::Yearly,
        _ => BillingPeriod::Monthly,
    }
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

fn parse_checkout_completed_payload(
    event: &Value,
) -> Result<StripeCheckoutCompletedPayload, CoreError> {
    let client_app_id = parse_uuid_field(
        &event["data"]["object"]["metadata"]["clientAppId"],
        "clientAppId",
    )?;
    let user_id = parse_uuid_field(&event["data"]["object"]["metadata"]["userId"], "userId")?;
    let plan_id = parse_uuid_field(&event["data"]["object"]["metadata"]["planId"], "planId")?;

    Ok(StripeCheckoutCompletedPayload {
        event_id: parse_event_id(event)?,
        user_id,
        client_app_id,
        plan_id,
        billing_period: parse_billing_period(
            event["data"]["object"]["metadata"]["billingPeriod"].as_str(),
        ),
        is_trial: event["data"]["object"]["metadata"]["trialDays"]
            .as_u64()
            .is_some_and(|days| days > 0),
        stripe_subscription_id: event["data"]["object"]["subscription"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| CoreError::BadRequest("Missing subscription id".to_string()))?,
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
    let stripe_subscription_id = event["data"]["object"]["id"]
        .as_str()
        .ok_or_else(|| CoreError::BadRequest("Missing subscription id".to_string()))?
        .to_string();
    let user_id = parse_uuid_field(&event["data"]["object"]["metadata"]["userId"], "userId")?;
    let plan_id = parse_uuid_field(&event["data"]["object"]["metadata"]["planId"], "planId")?;
    let cancel_at_period_end = event["data"]["object"]["cancel_at_period_end"]
        .as_bool()
        .unwrap_or(false);
    let status = parse_stripe_subscription_status(
        event["data"]["object"]["status"].as_str(),
        cancel_at_period_end,
    )?;

    Ok(StripeSubscriptionCreatedPayload {
        event_id: parse_event_id(event)?,
        stripe_subscription_id,
        user_id,
        plan_id,
        client_app_id: parse_optional_uuid_field(
            &event["data"]["object"]["metadata"]["clientAppId"],
        ),
        external_product_id: event["data"]["object"]["items"]["data"][0]["price"]["product"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| format!("prod_test_{}", plan_id)),
        billing_period: parse_billing_period(
            event["data"]["object"]["metadata"]["billingPeriod"].as_str(),
        ),
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
    let previous_plan_id = parse_uuid_field(
        &event["data"]["previous_attributes"]["items"]["data"][0]["price"]["metadata"]["planId"],
        "previous planId",
    )?;
    let current_plan_id = parse_uuid_field(
        &event["data"]["object"]["items"]["data"][0]["price"]["metadata"]["planId"],
        "current planId",
    )?;
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
        user_id: parse_uuid_field(&event["data"]["object"]["metadata"]["userId"], "userId")?,
        previous_plan_id,
        current_plan_id,
        external_product_id: event["data"]["object"]["items"]["data"][0]["price"]["product"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| format!("prod_test_{}", current_plan_id)),
        billing_period: parse_billing_period(
            event["data"]["object"]["metadata"]["billingPeriod"].as_str(),
        ),
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
    Ok(StripeSubscriptionDeletedPayload {
        event_id: parse_event_id(event)?,
        stripe_subscription_id: event["data"]["object"]["id"]
            .as_str()
            .ok_or_else(|| CoreError::BadRequest("Missing subscription id".to_string()))?
            .to_string(),
        user_id: parse_uuid_field(&event["data"]["object"]["metadata"]["userId"], "userId")?,
        plan_id: parse_optional_uuid_field(&event["data"]["object"]["metadata"]["planId"]),
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
        amount_refunded: event["data"]["object"]["amount_refunded"]
            .as_i64()
            .ok_or_else(|| CoreError::BadRequest("Missing or invalid amount".to_string()))?,
        user_id: parse_uuid_field(&event["data"]["object"]["metadata"]["userId"], "userId")?,
        refund_type: event["data"]["object"]["metadata"]["refundType"]
            .as_str()
            .unwrap_or("subscription")
            .to_string(),
    })
}

fn parse_invoice_paid_payload(event: &Value) -> Result<StripeInvoicePaidPayload, CoreError> {
    Ok(StripeInvoicePaidPayload {
        event_id: parse_event_id(event)?,
        stripe_subscription_id: event["data"]["object"]["subscription"]
            .as_str()
            .ok_or_else(|| CoreError::BadRequest("Missing subscription id".to_string()))?
            .to_string(),
        user_id: parse_uuid_field(&event["data"]["object"]["metadata"]["userId"], "userId")?,
        plan_id: parse_uuid_field(&event["data"]["object"]["metadata"]["planId"], "planId")?,
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
    let attempt_id = parse_attempt_id(&object["metadata"]["attemptId"]).ok_or_else(|| {
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
    let Some(attempt_id) = parse_attempt_id(&object["metadata"]["attemptId"]) else {
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
async fn sync_stripe_subscription(
    app_state: &AppState,
    realm_id: &str,
    user_id: Uuid,
    external_subscription_id: &str,
    client_app_id: Option<Uuid>,
    plan_id: Uuid,
    external_product_id: String,
    status: SubscriptionStatus,
    billing_period: BillingPeriod,
    current_period_start: Option<DateTime<Utc>>,
    current_period_end: Option<DateTime<Utc>>,
    cancel_at_period_end: bool,
    cancel_at: Option<DateTime<Utc>>,
) -> Result<(Subscription, Option<Subscription>), CoreError> {
    sync_subscription(
        app_state,
        SyncSubscriptionInput {
            provider: "stripe",
            realm_id: realm_id.to_string(),
            user_id: Some(user_id),
            external_subscription_id: external_subscription_id.to_string(),
            external_product_id,
            client_app_id,
            plan_id: Some(plan_id),
            status,
            billing_period,
            current_period_start,
            current_period_end,
            cancel_at_period_end,
            cancel_at,
        },
    )
    .await?
    .ok_or_else(|| {
        CoreError::InternalServerError(
            "stripe subscription sync failed to create or update subscription".to_string(),
        )
    })
}

// ============================================================================
// Webhook Event Handlers
// ============================================================================

/// Handle checkout.session.completed events
///
/// Creates a subscription when a Stripe checkout is completed.
async fn handle_checkout_session_completed(
    app_state: AppState,
    event: Value,
    realm_id: &str,
    _idempotency_key: &str,
) -> Result<PointsTransaction, CoreError> {
    if let Some(attempt_id) = parse_attempt_id(&event["data"]["object"]["metadata"]["attemptId"]) {
        let provider_transaction_id = event["data"]["object"]["subscription"]
            .as_str()
            .or_else(|| event["data"]["object"]["payment_intent"].as_str())
            .or_else(|| event["data"]["object"]["id"].as_str())
            .ok_or_else(|| CoreError::BadRequest("Missing provider transaction id".to_string()))?
            .to_string();
        let completed_at = parse_optional_stripe_datetime(&event["data"]["object"]["created"])?
            .unwrap_or_else(Utc::now);

        fulfill_payment_attempt(
            &app_state,
            attempt_id,
            "succeeded",
            provider_transaction_id,
            completed_at,
        )
        .await?;

        return Ok(create_placeholder_transaction(
            attempt_id,
            realm_id,
            TransactionType::SubscriptionGrant,
        ));
    }

    let payload = parse_checkout_completed_payload(&event)?;
    let event_id = payload.event_id.as_str();

    info!(
        realm_id = %realm_id,
        event_id = %event_id,
        "Processing checkout.session.completed event"
    );

    // Parse event payload
    // Get plan config
    let _plan_config = app_state
        .points_repository
        .find_plan_config(realm_id, payload.plan_id)
        .await?
        .ok_or(CoreError::NotFound)?;

    // Determine status
    let status = if payload.is_trial {
        SubscriptionStatus::Trialing
    } else {
        SubscriptionStatus::Active
    };

    // Create subscription
    let subscription = Subscription {
        id: Uuid::now_v7(),
        realm_id: realm_id.to_string(),
        external_subscription_id: payload.stripe_subscription_id.clone(),
        external_product_id: payload.stripe_product_id.clone(),
        payment_provider: "stripe".to_string(),
        status,
        tier: SubscriptionTier::Starter,
        user_id: Some(payload.user_id),
        current_period_start: Some(chrono::Utc::now()),
        current_period_end: None,
        cancel_at_period_end: false,
        client_app_id: Some(payload.client_app_id),
        plan_id: Some(payload.plan_id),
        billing_period: payload.billing_period,
        cancel_at: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let created_subscription = app_state
        .billing_repository
        .create_subscription(subscription)
        .await?;

    let history_event = SubscriptionHistoryService::create_subscription_created_event(
        &created_subscription,
        Some(ACTOR_WEBHOOK.to_string()),
    );
    app_state
        .billing_repository
        .save_history_event(history_event)
        .await?;

    info!(
        realm_id = %realm_id,
        subscription_id = %created_subscription.id,
        client_app_id = %payload.client_app_id,
        plan_id = %payload.plan_id,
        stripe_subscription_id = ?payload.stripe_subscription_id,
        event_id = %event_id,
        "Checkout completed - subscription created"
    );

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
    )
    .await?;

    Ok(create_placeholder_transaction(
        payload.attempt_id,
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

    let (subscription, previous_subscription) = sync_stripe_subscription(
        &app_state,
        realm_id,
        payload.user_id,
        &payload.stripe_subscription_id,
        payload.client_app_id,
        payload.plan_id,
        payload.external_product_id.clone(),
        payload.status.clone(),
        payload.billing_period.clone(),
        payload.current_period_start,
        Some(payload.current_period_end),
        payload.cancel_at_period_end,
        None,
    )
    .await?;

    app_state
        .subscription_service
        .handle_subscription_paid(
            payload.user_id,
            realm_id,
            payload.plan_id,
            false,
            payload.current_period_end,
            payload.event_id.clone(),
        )
        .await?;

    save_subscription_history(
        &app_state,
        previous_subscription.as_ref(),
        &subscription,
        HistoryEventType::Created,
    )
    .await?;

    info!(
        realm_id = %realm_id,
        user_id = %payload.user_id,
        plan_id = %payload.plan_id,
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

    // Get plan configs to determine if upgrade or downgrade
    let old_plan = app_state
        .points_repository
        .find_plan_config(realm_id, payload.previous_plan_id)
        .await?
        .ok_or_else(|| CoreError::SubscriptionPlanNotFound {
            realm_id: realm_id.to_string(),
            plan_id: payload.previous_plan_id.to_string(),
        })?;

    let new_plan = app_state
        .points_repository
        .find_plan_config(realm_id, payload.current_plan_id)
        .await?
        .ok_or_else(|| CoreError::SubscriptionPlanNotFound {
            realm_id: realm_id.to_string(),
            plan_id: payload.current_plan_id.to_string(),
        })?;

    let (subscription, previous_subscription) = sync_stripe_subscription(
        &app_state,
        realm_id,
        payload.user_id,
        &payload.stripe_subscription_id,
        None,
        payload.current_plan_id,
        payload.external_product_id.clone(),
        payload.status.clone(),
        payload.billing_period.clone(),
        payload.current_period_start,
        Some(payload.current_period_end),
        payload.cancel_at_period_end,
        if payload.cancel_at_period_end {
            Some(payload.current_period_end)
        } else {
            None
        },
    )
    .await?;

    let is_upgrade = new_plan.points_per_period > old_plan.points_per_period;

    if is_upgrade {
        app_state
            .subscription_service
            .handle_subscription_upgrade(
                payload.user_id,
                realm_id,
                payload.previous_plan_id,
                payload.current_plan_id,
                payload.current_period_end,
            )
            .await?;

        save_subscription_history(
            &app_state,
            previous_subscription.as_ref(),
            &subscription,
            HistoryEventType::Upgraded,
        )
        .await?;

        info!(
            realm_id = %realm_id,
            user_id = %payload.user_id,
            stripe_subscription_id = %payload.stripe_subscription_id,
            old_plan_id = %payload.previous_plan_id,
            new_plan_id = %payload.current_plan_id,
            event_id = %event_id,
            "Processed subscription upgrade"
        );

        Ok(create_placeholder_transaction(
            payload.user_id,
            realm_id,
            TransactionType::SubscriptionUpgrade,
        ))
    } else {
        app_state
            .subscription_service
            .handle_subscription_downgrade(
                payload.user_id,
                realm_id,
                payload.previous_plan_id,
                payload.current_plan_id,
            )
            .await?;

        save_subscription_history(
            &app_state,
            previous_subscription.as_ref(),
            &subscription,
            HistoryEventType::Downgraded,
        )
        .await?;

        info!(
            realm_id = %realm_id,
            user_id = %payload.user_id,
            stripe_subscription_id = %payload.stripe_subscription_id,
            old_plan_id = %payload.previous_plan_id,
            new_plan_id = %payload.current_plan_id,
            event_id = %event_id,
            "Processed subscription downgrade"
        );

        Ok(create_placeholder_transaction(
            payload.user_id,
            realm_id,
            TransactionType::SubscriptionUpgrade,
        ))
    }
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
    let plan_id = existing_subscription
        .as_ref()
        .and_then(|subscription| subscription.plan_id)
        .or(payload.plan_id)
        .ok_or_else(|| CoreError::BadRequest("Missing or invalid planId".to_string()))?;
    let external_product_id = existing_subscription
        .as_ref()
        .map(|subscription| subscription.external_product_id.clone())
        .unwrap_or_else(|| format!("prod_test_{}", plan_id));
    let billing_period = existing_subscription
        .as_ref()
        .map(|subscription| subscription.billing_period.clone())
        .unwrap_or(BillingPeriod::Monthly);
    let cancel_mode = if payload.cancel_at_period_end {
        CancelMode::DefaultCancel
    } else {
        CancelMode::ImmediateCancel
    };

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

    let status = if payload.cancel_at_period_end {
        SubscriptionStatus::ScheduledCancel
    } else {
        SubscriptionStatus::Canceled
    };

    let (subscription, previous_subscription) = sync_stripe_subscription(
        &app_state,
        realm_id,
        payload.user_id,
        &payload.stripe_subscription_id,
        existing_subscription
            .as_ref()
            .and_then(|subscription| subscription.client_app_id),
        plan_id,
        external_product_id,
        status,
        billing_period,
        payload.current_period_start,
        Some(payload.current_period_end),
        payload.cancel_at_period_end,
        Some(if payload.cancel_at_period_end {
            payload.current_period_end
        } else {
            Utc::now()
        }),
    )
    .await?;

    save_subscription_history(
        &app_state,
        previous_subscription.as_ref(),
        &subscription,
        HistoryEventType::Canceled,
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
                    payload.amount_refunded,
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
    let external_product_id = existing_subscription
        .as_ref()
        .map(|subscription| subscription.external_product_id.clone())
        .unwrap_or_else(|| format!("prod_test_{}", payload.plan_id));
    let billing_period = existing_subscription
        .as_ref()
        .map(|subscription| subscription.billing_period.clone())
        .unwrap_or(BillingPeriod::Monthly);

    let (subscription, previous_subscription) = sync_stripe_subscription(
        &app_state,
        realm_id,
        payload.user_id,
        &payload.stripe_subscription_id,
        existing_subscription
            .as_ref()
            .and_then(|subscription| subscription.client_app_id),
        payload.plan_id,
        external_product_id,
        SubscriptionStatus::Active,
        billing_period,
        payload.current_period_start,
        Some(payload.current_period_end),
        false,
        None,
    )
    .await?;

    app_state
        .subscription_service
        .handle_subscription_paid(
            payload.user_id,
            realm_id,
            payload.plan_id,
            true,
            payload.current_period_end,
            payload.event_id.clone(),
        )
        .await?;

    save_subscription_history(
        &app_state,
        previous_subscription.as_ref(),
        &subscription,
        HistoryEventType::Renewed,
    )
    .await?;

    info!(
        realm_id = %realm_id,
        user_id = %payload.user_id,
        plan_id = %payload.plan_id,
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
        "customer.subscription.created" => {
            handle_subscription_created(app_state.clone(), event.clone(), realm_id, idempotency_key)
                .await
        }
        "customer.subscription.updated" => {
            handle_subscription_updated(app_state.clone(), event.clone(), realm_id, idempotency_key)
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
        "invoice.payment_succeeded" => {
            handle_invoice_payment_succeeded(
                app_state.clone(),
                event.clone(),
                realm_id,
                idempotency_key,
            )
            .await
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
