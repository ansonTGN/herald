// Webhook Event Handlers for Creem Billing Events
//
// Handles subscription lifecycle events (paid, update, canceled) and refund events.
// All handlers return 202 Accepted immediately and process events asynchronously.

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use chrono::{DateTime, Utc};
use serde_json::Value;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::webhook_common::create_placeholder_transaction;
use crate::webhook_subscription_helpers::{
    SyncSubscriptionInput, save_subscription_history, sync_subscription,
};
use crate::webhooks::verify_webhook_signature;
use herald_api_base::application::http::state::AppState;
use herald_core::domain::billing::{
    BillingPeriod, BillingRepository, ExternalInvoiceData, HistoryEventType, InvoiceProvider,
    InvoiceRepository, InvoiceStatus, PaymentEvent, Subscription, SubscriptionStatus,
};
use herald_core::domain::common::entities::app_errors::CoreError;
use herald_core::domain::points::IdempotencyResult;
use herald_core::domain::points::entities::{PointsTransaction, TransactionType};
use herald_core::domain::points::ports::PointsRepository;
use herald_core::domain::points::subscription_service::CancelMode;
use herald_core::domain::purchase::{CompletePaymentAttemptInput, PaymentCompletionSource};

struct CreemCheckoutCompletedPayload {
    event_id: String,
    client_app_id: Uuid,
    plan_id: Uuid,
    billing_period: BillingPeriod,
    is_trial: bool,
    creem_product_id: String,
}

struct CreemSubscriptionPaidPayload {
    event_id: String,
    user_id: Uuid,
    plan_id: Uuid,
    client_app_id: Option<Uuid>,
    external_subscription_id: String,
    external_product_id: String,
    is_renewal: bool,
    billing_period: BillingPeriod,
    cancel_at_period_end: bool,
    current_period_start: Option<DateTime<Utc>>,
    current_period_end: DateTime<Utc>,
    status: SubscriptionStatus,
}

struct CreemSubscriptionUpdatedPayload {
    event_id: String,
    user_id: Uuid,
    previous_plan_id: Uuid,
    current_plan_id: Uuid,
    client_app_id: Option<Uuid>,
    external_subscription_id: String,
    external_product_id: String,
    billing_period: BillingPeriod,
    cancel_at_period_end: bool,
    current_period_start: Option<DateTime<Utc>>,
    current_period_end: DateTime<Utc>,
    status: SubscriptionStatus,
}

struct CreemSubscriptionCanceledPayload {
    event_id: String,
    user_id: Uuid,
    plan_id: Option<Uuid>,
    client_app_id: Option<Uuid>,
    external_subscription_id: String,
    external_product_id: String,
    billing_period: BillingPeriod,
    cancel_at_period_end: bool,
    current_period_start: Option<DateTime<Utc>>,
    current_period_end: Option<DateTime<Utc>>,
    status: SubscriptionStatus,
}

struct CreemRefundCreatedPayload {
    event_id: String,
    refund_id: String,
    payment_id: String,
    amount: i64,
    original_amount: i64,
    user_id: Uuid,
    refund_type: String,
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

fn creem_event_object(event: &Value) -> &Value {
    if !event["data"]["object"].is_null() {
        &event["data"]["object"]
    } else {
        &event["object"]
    }
}

fn creem_event_data<'a>(event: &'a Value, field: &str) -> &'a Value {
    if !event["data"][field].is_null() {
        &event["data"][field]
    } else {
        &Value::Null
    }
}

fn parse_creem_datetime(value: &Value, field_name: &str) -> Result<DateTime<Utc>, CoreError> {
    if let Some(raw) = value.as_str() {
        return DateTime::parse_from_rfc3339(raw)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|_| {
                CoreError::BadRequest(format!("Invalid RFC3339 timestamp for {}", field_name))
            });
    }

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

    Err(CoreError::BadRequest(format!(
        "Missing or invalid {}",
        field_name
    )))
}

fn parse_optional_creem_datetime(value: &Value) -> Result<Option<DateTime<Utc>>, CoreError> {
    if value.is_null() {
        return Ok(None);
    }

    parse_creem_datetime(value, "timestamp").map(Some)
}

fn parse_billing_period(primary: Option<&str>, fallback: Option<&str>) -> BillingPeriod {
    match primary.or(fallback).unwrap_or("monthly") {
        "yearly" => BillingPeriod::Yearly,
        _ => BillingPeriod::Monthly,
    }
}

fn parse_creem_status(
    status: Option<&str>,
    cancel_at_period_end: bool,
) -> Result<SubscriptionStatus, CoreError> {
    let parsed = match status.unwrap_or("active").to_lowercase().as_str() {
        "active" => SubscriptionStatus::Active,
        "trialing" => SubscriptionStatus::Trialing,
        "canceled" => SubscriptionStatus::Canceled,
        "expired" => SubscriptionStatus::Expired,
        "incomplete" => SubscriptionStatus::Incomplete,
        "paused" => SubscriptionStatus::Paused,
        "past_due" => SubscriptionStatus::PastDue,
        "scheduled_cancel" => SubscriptionStatus::ScheduledCancel,
        "dispute" => SubscriptionStatus::Dispute,
        other => {
            return Err(CoreError::BadRequest(format!(
                "Invalid subscription status: {}",
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
) -> Result<CreemCheckoutCompletedPayload, CoreError> {
    let metadata = &event["object"]["metadata"];

    Ok(CreemCheckoutCompletedPayload {
        event_id: parse_event_id(event)?,
        client_app_id: parse_uuid_field(&metadata["clientAppId"], "clientAppId")?,
        plan_id: parse_uuid_field(&metadata["planId"], "planId")?,
        billing_period: parse_billing_period(
            metadata["billingPeriod"].as_str(),
            metadata["billing_period"].as_str(),
        ),
        is_trial: metadata["trialDays"].as_u64().is_some_and(|days| days > 0),
        creem_product_id: event["object"]["product"]["id"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_default(),
    })
}

fn parse_subscription_paid_payload(
    event: &Value,
) -> Result<CreemSubscriptionPaidPayload, CoreError> {
    let object = creem_event_object(event);
    let cancel_at_period_end = object["cancelAtPeriodEnd"].as_bool().unwrap_or(false);

    Ok(CreemSubscriptionPaidPayload {
        event_id: parse_event_id(event)?,
        user_id: parse_uuid_field(&object["userId"], "userId")?,
        plan_id: parse_uuid_field(&object["planId"], "planId")?,
        client_app_id: parse_optional_uuid_field(&object["clientAppId"])
            .or_else(|| parse_optional_uuid_field(&object["metadata"]["clientAppId"])),
        external_subscription_id: object["subscriptionId"]
            .as_str()
            .or_else(|| object["id"].as_str())
            .map(str::to_string)
            .ok_or_else(|| CoreError::BadRequest("Missing subscriptionId".to_string()))?,
        external_product_id: object["productId"]
            .as_str()
            .or_else(|| object["product"]["id"].as_str())
            .map(str::to_string)
            .ok_or_else(|| CoreError::BadRequest("Missing productId".to_string()))?,
        is_renewal: object["isRenewal"].as_bool().unwrap_or(false),
        billing_period: parse_billing_period(
            object["billingPeriod"].as_str(),
            object["billing_period"].as_str(),
        ),
        cancel_at_period_end,
        current_period_start: parse_optional_creem_datetime(&object["currentPeriodStart"])
            .or_else(|_| parse_optional_creem_datetime(&object["current_period_start"]))?,
        current_period_end: parse_creem_datetime(
            if object["currentPeriodEnd"].is_null() {
                &object["current_period_end"]
            } else {
                &object["currentPeriodEnd"]
            },
            "currentPeriodEnd",
        )?,
        status: parse_creem_status(object["status"].as_str(), cancel_at_period_end)?,
    })
}

fn parse_subscription_updated_payload(
    event: &Value,
) -> Result<CreemSubscriptionUpdatedPayload, CoreError> {
    let object = creem_event_object(event);
    let previous_attributes = creem_event_data(event, "previousAttributes");
    let cancel_at_period_end = object["cancelAtPeriodEnd"].as_bool().unwrap_or(false);

    Ok(CreemSubscriptionUpdatedPayload {
        event_id: parse_event_id(event)?,
        user_id: parse_uuid_field(&object["userId"], "userId")?,
        previous_plan_id: parse_uuid_field(&previous_attributes["planId"], "previous planId")?,
        current_plan_id: parse_uuid_field(&object["planId"], "current planId")?,
        client_app_id: parse_optional_uuid_field(&object["clientAppId"])
            .or_else(|| parse_optional_uuid_field(&object["metadata"]["clientAppId"])),
        external_subscription_id: object["subscriptionId"]
            .as_str()
            .or_else(|| object["id"].as_str())
            .map(str::to_string)
            .ok_or_else(|| CoreError::BadRequest("Missing subscriptionId".to_string()))?,
        external_product_id: object["productId"]
            .as_str()
            .or_else(|| object["product"]["id"].as_str())
            .map(str::to_string)
            .ok_or_else(|| CoreError::BadRequest("Missing productId".to_string()))?,
        billing_period: parse_billing_period(
            object["billingPeriod"].as_str(),
            object["billing_period"].as_str(),
        ),
        cancel_at_period_end,
        current_period_start: parse_optional_creem_datetime(&object["currentPeriodStart"])
            .or_else(|_| parse_optional_creem_datetime(&object["current_period_start"]))?,
        current_period_end: parse_creem_datetime(
            if object["currentPeriodEnd"].is_null() {
                &object["current_period_end"]
            } else {
                &object["currentPeriodEnd"]
            },
            "currentPeriodEnd",
        )?,
        status: parse_creem_status(object["status"].as_str(), cancel_at_period_end)?,
    })
}

fn parse_subscription_canceled_payload(
    event: &Value,
) -> Result<CreemSubscriptionCanceledPayload, CoreError> {
    let object = creem_event_object(event);
    let cancel_at_period_end = object["cancelAtPeriodEnd"].as_bool().unwrap_or(false);
    let status = if cancel_at_period_end {
        SubscriptionStatus::ScheduledCancel
    } else {
        SubscriptionStatus::Canceled
    };

    Ok(CreemSubscriptionCanceledPayload {
        event_id: parse_event_id(event)?,
        user_id: parse_uuid_field(&object["userId"], "userId")?,
        plan_id: parse_optional_uuid_field(&object["planId"]),
        client_app_id: parse_optional_uuid_field(&object["clientAppId"])
            .or_else(|| parse_optional_uuid_field(&object["metadata"]["clientAppId"])),
        external_subscription_id: object["subscriptionId"]
            .as_str()
            .or_else(|| object["id"].as_str())
            .map(str::to_string)
            .ok_or_else(|| CoreError::BadRequest("Missing subscriptionId".to_string()))?,
        external_product_id: object["productId"]
            .as_str()
            .or_else(|| object["product"]["id"].as_str())
            .map(str::to_string)
            .ok_or_else(|| CoreError::BadRequest("Missing productId".to_string()))?,
        billing_period: parse_billing_period(
            object["billingPeriod"].as_str(),
            object["billing_period"].as_str(),
        ),
        cancel_at_period_end,
        current_period_start: parse_optional_creem_datetime(&object["currentPeriodStart"])
            .or_else(|_| parse_optional_creem_datetime(&object["current_period_start"]))?,
        current_period_end: parse_optional_creem_datetime(&object["currentPeriodEnd"])
            .or_else(|_| parse_optional_creem_datetime(&object["current_period_end"]))?,
        status,
    })
}

fn parse_refund_created_payload(event: &Value) -> Result<CreemRefundCreatedPayload, CoreError> {
    let object = creem_event_object(event);

    Ok(CreemRefundCreatedPayload {
        event_id: parse_event_id(event)?,
        refund_id: object["id"]
            .as_str()
            .ok_or_else(|| CoreError::BadRequest("Missing refund id".to_string()))?
            .to_string(),
        payment_id: object["paymentId"]
            .as_str()
            .ok_or_else(|| CoreError::BadRequest("Missing paymentId".to_string()))?
            .to_string(),
        amount: object["amount"]
            .as_i64()
            .ok_or_else(|| CoreError::BadRequest("Missing or invalid amount".to_string()))?,
        original_amount: object["originalAmount"].as_i64().ok_or_else(|| {
            CoreError::BadRequest("Missing or invalid originalAmount".to_string())
        })?,
        user_id: parse_uuid_field(&object["metadata"]["userId"], "userId")?,
        refund_type: object["metadata"]["refundType"]
            .as_str()
            .unwrap_or("subscription")
            .to_string(),
    })
}

#[allow(clippy::too_many_arguments)]
async fn sync_creem_subscription(
    app_state: &AppState,
    realm_id: &str,
    user_id: Uuid,
    creem_subscription_id: &str,
    client_app_id: Option<Uuid>,
    plan_id: Option<Uuid>,
    creem_product_id: String,
    status: SubscriptionStatus,
    billing_period: BillingPeriod,
    current_period_start: Option<DateTime<Utc>>,
    current_period_end: Option<DateTime<Utc>>,
    cancel_at_period_end: bool,
    cancel_at: Option<DateTime<Utc>>,
) -> Result<Option<(Subscription, Option<Subscription>)>, CoreError> {
    sync_subscription(
        app_state,
        SyncSubscriptionInput {
            provider: "creem",
            realm_id: realm_id.to_string(),
            user_id: Some(user_id),
            external_subscription_id: creem_subscription_id.to_string(),
            external_product_id: creem_product_id,
            client_app_id,
            plan_id,
            status,
            billing_period,
            current_period_start,
            current_period_end,
            cancel_at_period_end,
            cancel_at,
        },
    )
    .await
}

/// Handle checkout.completed events
///
/// Validates checkout metadata and records audit state.
///
/// Subscription creation is deferred until `subscription.paid`, when Creem sends
/// a stable provider subscription ID.
async fn handle_checkout_completed(
    app_state: AppState,
    event: Value,
    realm_id: &str,
    _idempotency_key: &str,
) -> Result<PointsTransaction, CoreError> {
    let payload = parse_checkout_completed_payload(&event)?;
    let event_id = payload.event_id.as_str();

    info!(
        realm_id = %realm_id,
        event_id = %event_id,
        "Processing checkout.completed event"
    );

    let _plan_config = app_state
        .points_repository
        .find_plan_config(realm_id, payload.plan_id)
        .await?
        .ok_or(CoreError::NotFound)?;

    info!(
        realm_id = %realm_id,
        client_app_id = %payload.client_app_id,
        plan_id = %payload.plan_id,
        billing_period = ?payload.billing_period,
        is_trial = payload.is_trial,
        creem_product_id = %payload.creem_product_id,
        event_id = %event_id,
        "Checkout completed audited; subscription creation deferred to subscription.paid"
    );

    // --- Creem invoice sync (best-effort, non-blocking) ---
    // Extract amount/currency with fallback: event.object → payment_attempts → skip with warn
    let event_object = &event["object"];
    let amount_from_event = event_object["amount"].as_i64();
    let currency_from_event = event_object["currency"].as_str().map(String::from);

    let (resolved_amount, resolved_currency, resolved_account_id) = match (
        amount_from_event,
        currency_from_event.as_deref(),
    ) {
        (Some(amt), Some(cur)) => {
            // Priority 1: amount and currency from event.object
            (amt, cur.to_string(), None)
        }
        _ => {
            // Priority 2: query payment_attempts via checkout session ID as provider_reference
            let checkout_id = event_object["id"].as_str().unwrap_or_default();
            match app_state
                .payment_attempt_service
                .get_payment_attempt_by_provider_reference("creem", checkout_id)
                .await
            {
                Ok(Some(attempt)) => (
                    attempt.amount,
                    attempt.currency.clone(),
                    Some(attempt.user_id),
                ),
                Ok(None) => {
                    // Priority 3: not found, skip sync
                    warn!(
                        realm_id = %realm_id,
                        event_id = %event_id,
                        checkout_id = %checkout_id,
                        "No payment_attempt found for Creem checkout — skipping invoice sync"
                    );
                    (0, String::new(), None)
                }
                Err(e) => {
                    warn!(
                        realm_id = %realm_id,
                        event_id = %event_id,
                        error = %e,
                        "Failed to query payment_attempt for Creem checkout — skipping invoice sync"
                    );
                    (0, String::new(), None)
                }
            }
        }
    };

    if !resolved_currency.is_empty() {
        let external_data = ExternalInvoiceData {
            realm_id: realm_id.to_string(),
            provider: InvoiceProvider::Creem,
            payment_provider: Some("creem".to_string()),
            external_invoice_id: None,
            external_order_id: Some(payload.event_id.clone()),
            external_status: Some("completed".to_string()),
            external_hosted_url: None,
            external_pdf_url: None,
            external_payload: Some(event.clone()),
            tax_details: None,
            account_id: resolved_account_id,
            currency: resolved_currency,
            total: resolved_amount,
            status: InvoiceStatus::Paid,
        };

        match app_state
            .invoice_repository
            .upsert_external_invoice(external_data)
            .await
        {
            Ok(_) => {
                info!(
                    realm_id = %realm_id,
                    event_id = %event_id,
                    external_order_id = %payload.event_id,
                    "Creem invoice synced successfully"
                );
            }
            Err(e) => {
                warn!(
                    realm_id = %realm_id,
                    event_id = %event_id,
                    error = %e,
                    "Failed to sync Creem invoice — payment flow continues"
                );
            }
        }
    }

    Ok(create_placeholder_transaction(
        payload.client_app_id,
        realm_id,
        TransactionType::SubscriptionGrant,
    ))
}

/// Handle subscription.paid events
///
/// Grants subscription points to user on initial subscription or renewal.
async fn handle_subscription_paid(
    app_state: AppState,
    event: Value,
    realm_id: &str,
    _idempotency_key: &str,
) -> Result<PointsTransaction, CoreError> {
    let object = creem_event_object(&event);

    if let Some(attempt_id) = parse_attempt_id(&object["metadata"]["attemptId"]) {
        let provider_transaction_id = object["subscriptionId"]
            .as_str()
            .or_else(|| object["id"].as_str())
            .ok_or_else(|| CoreError::BadRequest("Missing provider transaction id".to_string()))?
            .to_string();
        let completed_at = parse_optional_creem_datetime(&object["currentPeriodStart"])?
            .or_else(|| {
                parse_optional_creem_datetime(&object["current_period_start"])
                    .ok()
                    .flatten()
            })
            .unwrap_or_else(Utc::now);

        app_state
            .purchase_service
            .complete_succeeded_payment_attempt(CompletePaymentAttemptInput {
                attempt_id,
                provider_status: "succeeded".to_string(),
                provider_transaction_id,
                completed_at,
                source: PaymentCompletionSource::ProviderWebhook {
                    provider: "creem".to_string(),
                },
            })
            .await?;

        return Ok(create_placeholder_transaction(
            attempt_id,
            realm_id,
            TransactionType::SubscriptionGrant,
        ));
    }

    let payload = parse_subscription_paid_payload(&event)?;
    let event_id = payload.event_id.as_str();

    info!(
        realm_id = %realm_id,
        event_id = %event_id,
        "Processing subscription.paid event"
    );

    let _ledger = app_state
        .subscription_service
        .handle_subscription_paid(
            payload.user_id,
            realm_id,
            payload.plan_id,
            payload.is_renewal,
            payload.current_period_end,
            payload.event_id.clone(),
        )
        .await?;

    if let Some((subscription, previous)) = sync_creem_subscription(
        &app_state,
        realm_id,
        payload.user_id,
        payload.external_subscription_id.as_str(),
        payload.client_app_id,
        Some(payload.plan_id),
        payload.external_product_id,
        payload.status,
        payload.billing_period,
        payload.current_period_start,
        Some(payload.current_period_end),
        payload.cancel_at_period_end,
        None,
    )
    .await?
    {
        let history_event_type = if payload.is_renewal {
            HistoryEventType::Renewed
        } else {
            HistoryEventType::Created
        };

        save_subscription_history(
            &app_state,
            previous.as_ref(),
            &subscription,
            history_event_type,
        )
        .await?;
    }

    info!(
        realm_id = %realm_id,
        user_id = %payload.user_id,
        plan_id = %payload.plan_id,
        event_id = %event_id,
        is_renewal = payload.is_renewal,
        period_end = %payload.current_period_end,
        "Subscription paid event processed - credit ledger created"
    );

    Ok(create_placeholder_transaction(
        payload.user_id,
        realm_id,
        if payload.is_renewal {
            TransactionType::SubscriptionRenewal
        } else {
            TransactionType::SubscriptionGrant
        },
    ))
}

/// Handle subscription.update events
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
        "Processing subscription.update event"
    );

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

    let is_upgrade = new_plan.points_per_period > old_plan.points_per_period;

    let history_event_type = if is_upgrade {
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
        HistoryEventType::Upgraded
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
        HistoryEventType::Downgraded
    };

    if let Some((subscription, previous)) = sync_creem_subscription(
        &app_state,
        realm_id,
        payload.user_id,
        payload.external_subscription_id.as_str(),
        payload.client_app_id,
        Some(payload.current_plan_id),
        payload.external_product_id,
        payload.status,
        payload.billing_period,
        payload.current_period_start,
        Some(payload.current_period_end),
        payload.cancel_at_period_end,
        None,
    )
    .await?
    {
        save_subscription_history(
            &app_state,
            previous.as_ref(),
            &subscription,
            history_event_type,
        )
        .await?;
    }

    Ok(create_placeholder_transaction(
        payload.user_id,
        realm_id,
        TransactionType::SubscriptionUpgrade,
    ))
}

/// Handle subscription.canceled events
///
/// Handles subscription cancellation (immediate or end-of-period).
async fn handle_subscription_canceled(
    app_state: AppState,
    event: Value,
    realm_id: &str,
    _idempotency_key: &str,
) -> Result<PointsTransaction, CoreError> {
    let payload = parse_subscription_canceled_payload(&event)?;
    let event_id = payload.event_id.as_str();

    info!(
        realm_id = %realm_id,
        event_id = %event_id,
        "Processing subscription.canceled event"
    );

    let (cancel_mode, period_end, cancel_at) = if payload.cancel_at_period_end {
        let period_end = payload.current_period_end.ok_or_else(|| {
            CoreError::BadRequest("Missing currentPeriodEnd for cancel at period end".to_string())
        })?;

        info!(
            realm_id = %realm_id,
            user_id = %payload.user_id,
            period_end = %period_end,
            event_id = %event_id,
            "Subscription cancel at period end - setting expiration"
        );

        (
            CancelMode::DefaultCancel,
            Some(period_end),
            Some(period_end),
        )
    } else {
        info!(
            realm_id = %realm_id,
            user_id = %payload.user_id,
            event_id = %event_id,
            "Subscription immediate cancel - revoking subscription credits"
        );

        (CancelMode::ImmediateCancel, None, Some(Utc::now()))
    };

    let _output = app_state
        .subscription_service
        .handle_subscription_cancel(payload.user_id, realm_id, cancel_mode, period_end)
        .await?;

    if let Some(plan_id) = payload.plan_id
        && let Some((subscription, previous)) = sync_creem_subscription(
            &app_state,
            realm_id,
            payload.user_id,
            payload.external_subscription_id.as_str(),
            payload.client_app_id,
            Some(plan_id),
            payload.external_product_id,
            payload.status,
            payload.billing_period,
            payload.current_period_start,
            payload.current_period_end,
            payload.cancel_at_period_end,
            cancel_at,
        )
        .await?
    {
        save_subscription_history(
            &app_state,
            previous.as_ref(),
            &subscription,
            HistoryEventType::Canceled,
        )
        .await?;
    }

    Ok(create_placeholder_transaction(
        payload.user_id,
        realm_id,
        TransactionType::CancelRevoke,
    ))
}

/// Handle refund.created events
///
/// Revokes unused points based on refund type (topup vs subscription).
async fn handle_refund_created(
    app_state: AppState,
    event: Value,
    realm_id: &str,
    _idempotency_key: &str,
) -> Result<PointsTransaction, CoreError> {
    let payload = parse_refund_created_payload(&event)?;
    let event_id = payload.event_id.as_str();

    info!(
        realm_id = %realm_id,
        event_id = %event_id,
        "Processing refund.created event"
    );

    info!(
        realm_id = %realm_id,
        refund_id = %payload.refund_id,
        payment_id = %payload.payment_id,
        amount = payload.amount,
        original_amount = payload.original_amount,
        refund_type = %payload.refund_type,
        user_id = %payload.user_id,
        event_id = %event_id,
        "Processing refund - revoking points"
    );

    match payload.refund_type.as_str() {
        "topup" => {
            let (current_schema, backend_pid): (Option<String>, Option<i32>) =
                sqlx::query_as("SELECT current_schema(), pg_backend_pid()")
                    .fetch_one(&app_state.pool)
                    .await
                    .ok()
                    .unwrap_or((None, None));
            info!(
                current_schema = ?current_schema,
                backend_pid = ?backend_pid,
                pool_addr = &format!("{:p}", &app_state.pool),
                "Webhook handler: Current database schema before revocation"
            );

            let _output = app_state
                .points_service
                .revoke_topup_proportional(
                    realm_id,
                    payload.user_id,
                    payload.amount,
                    payload.original_amount,
                    &payload.refund_id,
                )
                .await?;

            info!(
                realm_id = %realm_id,
                user_id = %payload.user_id,
                refund_id = %payload.refund_id,
                amount = payload.amount,
                original_amount = payload.original_amount,
                "Topup refund - proportionally revoked topup credits"
            );
        }
        _ => {
            let _output = app_state
                .points_service
                .revoke_subscription_unused(realm_id, payload.user_id, &payload.refund_id)
                .await?;

            info!(
                realm_id = %realm_id,
                user_id = %payload.user_id,
                refund_id = %payload.refund_id,
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

// ============================================================================
// Main Webhook Handler
// ============================================================================

/// Handle Creem webhook events
///
/// Verifies signature, checks idempotency, routes to appropriate handler,
/// and returns 202 Accepted immediately. Processing happens asynchronously.
pub async fn handle_creem_webhook(
    State(app_state): State<AppState>,
    Path(realm_id): Path<String>,
    headers: HeaderMap,
    body: String,
) -> Result<StatusCode, CoreError> {
    let event: Value = serde_json::from_str(&body).map_err(|e| {
        error!("Failed to parse webhook JSON: {}", e);
        CoreError::BadRequest(format!("Invalid JSON: {}", e))
    })?;

    let signature = headers
        .get("creem-signature")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            error!("Missing creem-signature header");
            CoreError::BadRequest("Missing signature".to_string())
        })?;

    let webhook_secret = sqlx::query_scalar::<_, String>(
        "SELECT config_value
         FROM realm_config
         WHERE realm_id = $1 AND config_type = 'creem' AND config_key = 'webhook_secret' AND enabled = true
         LIMIT 1",
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

    verify_webhook_signature(body.as_bytes(), signature, &webhook_secret)?;

    let event_id = parse_event_id(&event)?;
    let event_type = event["eventType"]
        .as_str()
        .ok_or_else(|| CoreError::BadRequest("Missing eventType".to_string()))?
        .to_string();

    let new_payment_event = PaymentEvent {
        id: Uuid::now_v7(),
        realm_id: realm_id.clone(),
        external_event_id: event_id.clone(),
        payment_provider: "creem".to_string(),
        event_type: event_type.clone(),
        subscription_id: None,
        payload: event.clone(),
        processed: false,
        processing_started_at: None,
        created_at: Utc::now(),
    };

    if app_state
        .billing_repository
        .find_payment_event_by_external_id(&event_id, "creem")
        .await?
        .is_some()
    {
        info!(
            realm_id = %realm_id,
            event_id = %event_id,
            event_type = %event_type,
            "Duplicate webhook event - returning OK"
        );
        return Ok(StatusCode::OK);
    }

    let saved_event = match app_state
        .billing_repository
        .create_payment_event(new_payment_event)
        .await
    {
        Ok(saved_event) => saved_event,
        Err(CoreError::DatabaseError(msg))
            if msg.contains("unique constraint") || msg.contains("duplicate key") =>
        {
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

    let idempotency_key = format!("creem_{}", event_id);
    let idempotency_service = &app_state.idempotency_service;

    let idempotency_result = idempotency_service
        .check_or_create(&realm_id, &idempotency_key, &body)
        .await?;

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

    let result = match event_type.as_str() {
        "checkout.completed" => {
            handle_checkout_completed(
                app_state.clone(),
                event.clone(),
                &realm_id,
                &idempotency_key,
            )
            .await
        }
        "subscription.paid" => {
            handle_subscription_paid(
                app_state.clone(),
                event.clone(),
                &realm_id,
                &idempotency_key,
            )
            .await
        }
        "subscription.update" => {
            handle_subscription_updated(
                app_state.clone(),
                event.clone(),
                &realm_id,
                &idempotency_key,
            )
            .await
        }
        "subscription.canceled" => {
            handle_subscription_canceled(
                app_state.clone(),
                event.clone(),
                &realm_id,
                &idempotency_key,
            )
            .await
        }
        "refund.created" => {
            handle_refund_created(
                app_state.clone(),
                event.clone(),
                &realm_id,
                &idempotency_key,
            )
            .await
        }
        _ => {
            warn!(
                realm_id = %realm_id,
                event_id = %event_id,
                event_type = %event_type,
                "Unknown webhook event type - logging and returning OK"
            );
            Ok(create_placeholder_transaction(
                uuid::Uuid::now_v7(),
                &realm_id,
                TransactionType::SubscriptionGrant,
            ))
        }
    };

    match result {
        Ok(transaction) => {
            if let Err(e) = idempotency_service
                .save_result(&realm_id, &idempotency_key, &transaction)
                .await
            {
                error!(
                    realm_id = %realm_id,
                    idempotency_key = %idempotency_key,
                    error = %e,
                    "Failed to save idempotency result"
                );
            }

            let _ = app_state
                .billing_repository
                .mark_payment_event_processed(saved_event.id)
                .await;
        }
        Err(e) => {
            error!(
                realm_id = %realm_id,
                event_id = %event_id,
                event_type = %event_type,
                error = %e,
                "Webhook handler failed"
            );

            let _ = idempotency_service
                .mark_failed(&realm_id, &idempotency_key)
                .await;

            return Err(e);
        }
    }

    info!(
        realm_id = %realm_id,
        event_id = %event_id,
        event_type = %event_type,
        "Webhook processed successfully"
    );

    Ok(StatusCode::OK)
}
