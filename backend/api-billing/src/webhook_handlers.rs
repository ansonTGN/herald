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

use crate::webhook_common::{
    create_placeholder_transaction, metadata_value, parse_attempt_id, parse_event_id,
    parse_optional_uuid_field, parse_uuid_field,
};
use crate::webhook_subscription_helpers::{
    SyncSubscriptionInput, resolve_entitlement_key, save_subscription_history, sync_subscription,
};
use crate::webhooks::verify_webhook_signature;
use herald_api_base::application::http::state::AppState;
use herald_core::domain::billing::{
    BillingRepository, BillingType, ExternalInvoiceData, HistoryEventType, InvoiceProvider,
    InvoiceRepository, InvoiceStatus, PaymentEvent, Subscription, SubscriptionStatus,
    detect_change_type,
};
use herald_core::domain::common::entities::app_errors::CoreError;
use herald_core::domain::points::IdempotencyResult;
use herald_core::domain::points::entities::{
    CreditType, PointsTransaction, RevocationType, TransactionType,
};
use herald_core::domain::points::ports::PointsRepository;
use herald_core::domain::points::subscription_service::CancelMode;
use herald_core::domain::purchase::metadata_keys;
use herald_core::domain::purchase::{CompletePaymentAttemptInput, PaymentCompletionSource};

struct CreemCheckoutCompletedPayload {
    event_id: String,
    client_app_id: Uuid,
    entitlement_key: String,
    is_trial: bool,
    creem_product_id: String,
    attempt_id: Option<Uuid>,
}

struct CreemSubscriptionPaidPayload {
    event_id: String,
    user_id: Option<Uuid>,
    entitlement_key: String,
    client_app_id: Option<Uuid>,
    external_subscription_id: String,
    external_product_id: String,
    is_renewal: bool,
    cancel_at_period_end: bool,
    current_period_start: Option<DateTime<Utc>>,
    current_period_end: Option<DateTime<Utc>>,
    status: SubscriptionStatus,
}

struct CreemSubscriptionUpdatedPayload {
    event_id: String,
    user_id: Option<Uuid>,
    previous_entitlement_key: String,
    current_entitlement_key: String,
    client_app_id: Option<Uuid>,
    external_subscription_id: String,
    external_product_id: String,
    cancel_at_period_end: bool,
    current_period_start: Option<DateTime<Utc>>,
    current_period_end: Option<DateTime<Utc>>,
    status: SubscriptionStatus,
}

struct CreemSubscriptionCanceledPayload {
    event_id: String,
    user_id: Option<Uuid>,
    entitlement_key: Option<String>,
    client_app_id: Option<Uuid>,
    external_subscription_id: String,
    external_product_id: String,
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

struct CreemSubscriptionLifecyclePayload {
    event_id: String,
    user_id: Option<Uuid>,
    entitlement_key: Option<String>,
    client_app_id: Option<Uuid>,
    external_subscription_id: String,
    external_product_id: String,
    current_period_start: Option<DateTime<Utc>>,
    current_period_end: Option<DateTime<Utc>>,
}

struct CreemDisputeCreatedPayload {
    event_id: String,
    external_subscription_id: String,
    external_product_id: String,
    amount: i64,
    currency: String,
    dispute_id: String,
}

fn creem_event_object(event: &Value) -> &Value {
    if !event["data"]["object"].is_null() {
        &event["data"]["object"]
    } else {
        &event["object"]
    }
}

fn creem_metadata(object: &Value) -> &Value {
    &object["metadata"]
}

fn parse_creem_user_id(object: &Value) -> Option<Uuid> {
    object
        .get("herald_user_id")
        .or_else(|| object.get("metadata").and_then(|m| m.get("herald_user_id")))
        .or_else(|| object.get("userId"))
        .or_else(|| object.get("metadata").and_then(|m| m.get("userId")))
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
}

fn parse_creem_client_app_id(object: &Value) -> Option<Uuid> {
    parse_optional_uuid_field(metadata_value(
        object,
        "herald_client_app_id",
        "clientAppId",
    ))
    .or_else(|| {
        parse_optional_uuid_field(metadata_value(
            creem_metadata(object),
            "herald_client_app_id",
            "clientAppId",
        ))
    })
}

fn parse_creem_entitlement_key(object: &Value) -> Option<String> {
    object["herald_entitlement_key"]
        .as_str()
        .or_else(|| object["metadata"]["herald_entitlement_key"].as_str())
        .or_else(|| object["entitlementKey"].as_str())
        .or_else(|| object["metadata"]["entitlementKey"].as_str())
        .filter(|key| !key.is_empty())
        .map(str::to_string)
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

/// Extract tax details from a Creem checkout.completed event object.
///
/// Creem (Merchant of Record) includes tax fields in the checkout object.
/// Common field names: tax_amount, tax_rate, tax_country, tax_region.
/// Returns None if no tax fields are present.
fn extract_creem_tax_details(object: &Value) -> Option<serde_json::Value> {
    let tax_amount = object.get("tax_amount").and_then(|v| v.as_i64());
    let tax_rate = object.get("tax_rate").and_then(|v| v.as_f64());
    let tax_country = object
        .get("tax_country")
        .and_then(|v| v.as_str())
        .map(String::from);
    let tax_region = object
        .get("tax_region")
        .and_then(|v| v.as_str())
        .map(String::from);

    if tax_amount.is_none() && tax_rate.is_none() && tax_country.is_none() && tax_region.is_none() {
        return None;
    }

    let mut map = serde_json::Map::new();
    if let Some(amount) = tax_amount {
        map.insert("tax_amount".to_string(), serde_json::Value::from(amount));
    }
    if let Some(rate) = tax_rate {
        map.insert("tax_rate".to_string(), serde_json::json!(rate));
    }
    if let Some(country) = tax_country {
        map.insert(
            "tax_country".to_string(),
            serde_json::Value::String(country),
        );
    }
    if let Some(region) = tax_region {
        map.insert("tax_region".to_string(), serde_json::Value::String(region));
    }

    Some(serde_json::Value::Object(map))
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

    // Resolve entitlement_key from metadata
    let entitlement_key = metadata["herald_entitlement_key"]
        .as_str()
        .or_else(|| metadata["entitlementKey"].as_str())
        .unwrap_or("")
        .to_string();

    Ok(CreemCheckoutCompletedPayload {
        event_id: parse_event_id(event)?,
        client_app_id: parse_uuid_field(
            metadata_value(metadata, "herald_client_app_id", "clientAppId"),
            "clientAppId",
        )?,
        entitlement_key,
        is_trial: metadata["herald_trial_days"]
            .as_u64()
            .or_else(|| metadata["trialDays"].as_u64())
            .is_some_and(|days| days > 0),
        creem_product_id: event["object"]["product"]["id"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_default(),
        attempt_id: parse_attempt_id(&metadata[metadata_keys::ATTEMPT_ID]),
    })
}

/// Normalize Creem billing type strings to domain BillingType.
///
/// Creem uses "onetime" for one-time products. The domain uses "one_time".
/// Also handles "subscription" as an alias for "recurring".
fn normalize_creem_billing_type(raw: &str) -> BillingType {
    match raw.to_ascii_lowercase().as_str() {
        "onetime" | "one_time" => BillingType::OneTime,
        "recurring" | "subscription" => BillingType::Recurring,
        _ => BillingType::Recurring,
    }
}

fn parse_subscription_paid_payload(
    event: &Value,
) -> Result<CreemSubscriptionPaidPayload, CoreError> {
    let object = creem_event_object(event);
    let cancel_at_period_end = object["cancelAtPeriodEnd"].as_bool().unwrap_or(false);

    // Resolve entitlement_key from metadata
    let entitlement_key = object["herald_entitlement_key"]
        .as_str()
        .or_else(|| object["metadata"]["herald_entitlement_key"].as_str())
        .or_else(|| object["entitlementKey"].as_str())
        .or_else(|| object["metadata"]["entitlementKey"].as_str())
        .unwrap_or("")
        .to_string();

    let user_id = object
        .get("herald_user_id")
        .or_else(|| object.get("metadata").and_then(|m| m.get("herald_user_id")))
        .or_else(|| object.get("userId"))
        .or_else(|| object.get("metadata").and_then(|m| m.get("userId")))
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok());

    Ok(CreemSubscriptionPaidPayload {
        event_id: parse_event_id(event)?,
        user_id,
        entitlement_key,
        client_app_id: parse_optional_uuid_field(metadata_value(
            object,
            "herald_client_app_id",
            "clientAppId",
        ))
        .or_else(|| {
            parse_optional_uuid_field(metadata_value(
                &object["metadata"],
                "herald_client_app_id",
                "clientAppId",
            ))
        }),
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
        cancel_at_period_end,
        current_period_start: parse_optional_creem_datetime(&object["currentPeriodStart"])
            .or_else(|_| parse_optional_creem_datetime(&object["current_period_start"]))
            .or_else(|_| parse_optional_creem_datetime(&object["current_period_start_date"]))?,
        current_period_end: parse_optional_creem_datetime(&object["currentPeriodEnd"])
            .or_else(|_| parse_optional_creem_datetime(&object["current_period_end"]))
            .or_else(|_| parse_optional_creem_datetime(&object["current_period_end_date"]))?,
        status: parse_creem_status(object["status"].as_str(), cancel_at_period_end)?,
    })
}

fn parse_subscription_updated_payload(
    event: &Value,
) -> Result<CreemSubscriptionUpdatedPayload, CoreError> {
    let object = creem_event_object(event);
    let previous_attributes = creem_event_data(event, "previousAttributes");
    let cancel_at_period_end = object["cancelAtPeriodEnd"].as_bool().unwrap_or(false);

    // Resolve entitlement_keys from metadata
    let current_entitlement_key = object["herald_entitlement_key"]
        .as_str()
        .or_else(|| object["metadata"]["herald_entitlement_key"].as_str())
        .or_else(|| object["entitlementKey"].as_str())
        .or_else(|| object["metadata"]["entitlementKey"].as_str())
        .unwrap_or("")
        .to_string();

    let previous_entitlement_key = previous_attributes["herald_entitlement_key"]
        .as_str()
        .or_else(|| previous_attributes["entitlementKey"].as_str())
        .unwrap_or("")
        .to_string();

    let user_id = object
        .get("herald_user_id")
        .or_else(|| object.get("metadata").and_then(|m| m.get("herald_user_id")))
        .or_else(|| object.get("userId"))
        .or_else(|| object.get("metadata").and_then(|m| m.get("userId")))
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok());

    Ok(CreemSubscriptionUpdatedPayload {
        event_id: parse_event_id(event)?,
        user_id,
        previous_entitlement_key,
        current_entitlement_key,
        client_app_id: parse_optional_uuid_field(metadata_value(
            object,
            "herald_client_app_id",
            "clientAppId",
        ))
        .or_else(|| {
            parse_optional_uuid_field(metadata_value(
                &object["metadata"],
                "herald_client_app_id",
                "clientAppId",
            ))
        }),
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
        cancel_at_period_end,
        current_period_start: parse_optional_creem_datetime(&object["currentPeriodStart"])
            .or_else(|_| parse_optional_creem_datetime(&object["current_period_start"]))
            .or_else(|_| parse_optional_creem_datetime(&object["current_period_start_date"]))?,
        current_period_end: parse_optional_creem_datetime(&object["currentPeriodEnd"])
            .or_else(|_| parse_optional_creem_datetime(&object["current_period_end"]))
            .or_else(|_| parse_optional_creem_datetime(&object["current_period_end_date"]))?,
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

    // Resolve entitlement_key from metadata
    let entitlement_key = object["herald_entitlement_key"]
        .as_str()
        .or_else(|| object["metadata"]["herald_entitlement_key"].as_str())
        .or_else(|| object["entitlementKey"].as_str())
        .or_else(|| object["metadata"]["entitlementKey"].as_str())
        .map(str::to_string);

    let user_id = object
        .get("herald_user_id")
        .or_else(|| object.get("metadata").and_then(|m| m.get("herald_user_id")))
        .or_else(|| object.get("userId"))
        .or_else(|| object.get("metadata").and_then(|m| m.get("userId")))
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok());

    Ok(CreemSubscriptionCanceledPayload {
        event_id: parse_event_id(event)?,
        user_id,
        entitlement_key,
        client_app_id: parse_optional_uuid_field(metadata_value(
            object,
            "herald_client_app_id",
            "clientAppId",
        ))
        .or_else(|| {
            parse_optional_uuid_field(metadata_value(
                &object["metadata"],
                "herald_client_app_id",
                "clientAppId",
            ))
        }),
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
        user_id: parse_uuid_field(
            metadata_value(&object["metadata"], "herald_user_id", "userId"),
            "userId",
        )?,
        refund_type: object["metadata"]["refundType"]
            .as_str()
            .unwrap_or("subscription")
            .to_string(),
    })
}

fn parse_subscription_lifecycle_payload(
    event: &Value,
) -> Result<CreemSubscriptionLifecyclePayload, CoreError> {
    let object = creem_event_object(event);

    Ok(CreemSubscriptionLifecyclePayload {
        event_id: parse_event_id(event)?,
        user_id: parse_creem_user_id(object),
        entitlement_key: parse_creem_entitlement_key(object),
        client_app_id: parse_creem_client_app_id(object),
        external_subscription_id: object["subscriptionId"]
            .as_str()
            .or_else(|| object["id"].as_str())
            .map(str::to_string)
            .ok_or_else(|| CoreError::BadRequest("Missing subscription id".to_string()))?,
        external_product_id: object["productId"]
            .as_str()
            .or_else(|| object["product"]["id"].as_str())
            .or_else(|| object["product"].as_str())
            .map(str::to_string)
            .ok_or_else(|| CoreError::BadRequest("Missing product id".to_string()))?,
        current_period_start: parse_optional_creem_datetime(&object["currentPeriodStart"])
            .or_else(|_| parse_optional_creem_datetime(&object["current_period_start"]))
            .or_else(|_| parse_optional_creem_datetime(&object["current_period_start_date"]))?,
        current_period_end: parse_optional_creem_datetime(&object["currentPeriodEnd"])
            .or_else(|_| parse_optional_creem_datetime(&object["current_period_end"]))
            .or_else(|_| parse_optional_creem_datetime(&object["current_period_end_date"]))?,
    })
}

fn parse_dispute_created_payload(event: &Value) -> Result<CreemDisputeCreatedPayload, CoreError> {
    let object = creem_event_object(event);
    let subscription = &object["subscription"];

    Ok(CreemDisputeCreatedPayload {
        event_id: parse_event_id(event)?,
        external_subscription_id: subscription["id"]
            .as_str()
            .or_else(|| object["transaction"]["subscription"].as_str())
            .map(str::to_string)
            .ok_or_else(|| CoreError::BadRequest("Missing dispute subscription id".to_string()))?,
        external_product_id: subscription["product"]
            .as_str()
            .or_else(|| subscription["product"]["id"].as_str())
            .map(str::to_string)
            .unwrap_or_default(),
        amount: object["amount"].as_i64().unwrap_or(0),
        currency: object["currency"].as_str().unwrap_or("").to_string(),
        dispute_id: object["id"]
            .as_str()
            .ok_or_else(|| CoreError::BadRequest("Missing dispute id".to_string()))?
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
    entitlement_key: String,
    creem_product_id: String,
    status: SubscriptionStatus,
    current_period_start: Option<DateTime<Utc>>,
    current_period_end: Option<DateTime<Utc>>,
    cancel_at_period_end: bool,
    cancel_at: Option<DateTime<Utc>>,
    existing_subscription: Option<Subscription>,
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
            entitlement_key,
            external_price_id: None,
            provider_metadata: None,
            status,
            current_period_start,
            current_period_end,
            cancel_at_period_end,
            cancel_at,
            existing_subscription,
        },
    )
    .await
}

/// Handle checkout.completed events
///
/// For one-time products, completes payment attempt and grants topup_credit.
/// For recurring products, records audit state and defers to subscription.paid.
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

    // Resolve entitlement_key via fallback chain
    let entitlement_key = if payload.entitlement_key.is_empty() {
        resolve_entitlement_key(
            &app_state,
            realm_id,
            "creem",
            &payload.creem_product_id,
            None,
        )
        .await?
    } else {
        payload.entitlement_key.clone()
    };

    // --- Determine billing_type via 3-tier fallback ---
    let metadata = &event["object"]["metadata"];
    let event_object = &event["object"];

    let billing_type = {
        // Priority 1: metadata herald_billing_kind
        let from_metadata = metadata["herald_billing_kind"]
            .as_str()
            .map(normalize_creem_billing_type);

        // Priority 2: Creem product.billing_type
        let from_product = event_object["product"]["billing_type"]
            .as_str()
            .map(normalize_creem_billing_type);

        // Priority 3: mapping lookup by provider product ID
        let from_mapping = if from_metadata.is_none() && from_product.is_none() {
            app_state
                .entitlement_mapping_service
                .find_mapping_by_provider_product(realm_id, "creem", &payload.creem_product_id)
                .await
                .ok()
                .flatten()
                .and_then(|m| m.billing_type)
        } else {
            None
        };

        from_metadata
            .or(from_product)
            .or(from_mapping)
            .unwrap_or(BillingType::Recurring)
    };

    info!(
        realm_id = %realm_id,
        client_app_id = %payload.client_app_id,
        entitlement_key = %entitlement_key,
        is_trial = payload.is_trial,
        creem_product_id = %payload.creem_product_id,
        billing_type = %billing_type.as_str(),
        event_id = %event_id,
        "Checkout completed -- dispatching by billing_type"
    );

    // --- Creem invoice sync (best-effort, non-blocking) ---
    // Extract amount/currency with fallback: event.object -> event.object.product -> payment_attempts -> skip with warn
    let amount_from_event = event_object["amount"]
        .as_i64()
        .or_else(|| event_object["product"]["price"].as_i64());
    let currency_from_event = event_object["currency"]
        .as_str()
        .or_else(|| event_object["product"]["currency"].as_str())
        .map(String::from);

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
                        "No payment_attempt found for Creem checkout -- skipping invoice sync"
                    );
                    (0, String::new(), None)
                }
                Err(e) => {
                    warn!(
                        realm_id = %realm_id,
                        event_id = %event_id,
                        error = %e,
                        "Failed to query payment_attempt for Creem checkout -- skipping invoice sync"
                    );
                    (0, String::new(), None)
                }
            }
        }
    };

    // Extract tax details from Creem event payload (MoR tax fields)
    let tax_details = extract_creem_tax_details(event_object);

    if !resolved_currency.is_empty() {
        let checkout_id = event_object["id"].as_str().map(String::from);
        let external_data = ExternalInvoiceData {
            realm_id: realm_id.to_string(),
            provider: InvoiceProvider::Creem,
            payment_provider: Some("creem".to_string()),
            external_invoice_id: checkout_id,
            external_order_id: Some(payload.event_id.clone()),
            external_status: Some("completed".to_string()),
            external_hosted_url: None,
            external_pdf_url: None,
            external_payload: Some(event.clone()),
            tax_details,
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
                    "Failed to sync Creem invoice -- payment flow continues"
                );
            }
        }
    }

    // --- Dispatch by billing_type ---
    match billing_type {
        BillingType::OneTime => {
            if let Some(attempt_id) = payload.attempt_id {
                let provider_transaction_id = event_object["id"]
                    .as_str()
                    .ok_or_else(|| CoreError::BadRequest("Missing checkout id".to_string()))?
                    .to_string();
                let completed_at = parse_optional_creem_datetime(&event_object["createdAt"])?
                    .or_else(|| {
                        parse_optional_creem_datetime(&event_object["created_at"])
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
                        billing_type_override: Some(BillingType::OneTime),
                    })
                    .await?;

                info!(
                    realm_id = %realm_id,
                    event_id = %event_id,
                    attempt_id = %attempt_id,
                    billing_type = "one_time",
                    "One-time checkout completed -- payment attempt fulfilled, topup_credit granted"
                );

                Ok(create_placeholder_transaction(
                    attempt_id,
                    realm_id,
                    TransactionType::Recharge,
                ))
            } else {
                warn!(
                    realm_id = %realm_id,
                    event_id = %event_id,
                    billing_type = "one_time",
                    "One-time checkout completed without attemptId -- auditing but not fulfilling"
                );

                Ok(create_placeholder_transaction(
                    payload.client_app_id,
                    realm_id,
                    TransactionType::Recharge,
                ))
            }
        }
        BillingType::Recurring => {
            info!(
                realm_id = %realm_id,
                event_id = %event_id,
                "Recurring checkout completed -- subscription creation deferred to subscription.paid"
            );

            Ok(create_placeholder_transaction(
                payload.client_app_id,
                realm_id,
                TransactionType::SubscriptionGrant,
            ))
        }
    }
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

    if let Some(attempt_id) = parse_attempt_id(&object["metadata"][metadata_keys::ATTEMPT_ID]) {
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
                billing_type_override: None,
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

    // Resolve user_id: prefer payload, fall back to existing subscription
    let user_id = match payload.user_id {
        Some(uid) => uid,
        None => {
            let existing = app_state
                .billing_repository
                .find_by_external_subscription_id(&payload.external_subscription_id, "creem")
                .await?
                .ok_or_else(|| {
                    CoreError::BadRequest(format!(
                        "Cannot resolve userId: no userId in event and no existing subscription for {}",
                        payload.external_subscription_id
                    ))
                })?;
            existing.user_id.ok_or_else(|| {
                CoreError::BadRequest(format!(
                    "Existing subscription {} has no userId",
                    payload.external_subscription_id
                ))
            })?
        }
    };

    info!(
        realm_id = %realm_id,
        event_id = %event_id,
        "Processing subscription.paid event"
    );

    // Resolve entitlement_key via fallback chain
    let entitlement_key = if payload.entitlement_key.is_empty() {
        resolve_entitlement_key(
            &app_state,
            realm_id,
            "creem",
            &payload.external_product_id,
            None,
        )
        .await?
    } else {
        payload.entitlement_key.clone()
    };

    if let Some((subscription, previous)) = sync_creem_subscription(
        &app_state,
        realm_id,
        user_id,
        payload.external_subscription_id.as_str(),
        payload.client_app_id,
        entitlement_key.clone(),
        payload.external_product_id,
        payload.status,
        payload.current_period_start,
        payload.current_period_end,
        payload.cancel_at_period_end,
        None,
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

    let period_end = payload
        .current_period_end
        .unwrap_or_else(|| Utc::now() + chrono::Duration::days(30));

    let grant_result = app_state
        .subscription_service
        .handle_subscription_paid(
            user_id,
            realm_id,
            &entitlement_key,
            payload.is_renewal,
            period_end,
            payload.event_id.clone(),
        )
        .await;

    if let Err(error) = grant_result {
        if matches!(error, CoreError::EntitlementMappingNotFound) {
            info!(
                realm_id = %realm_id,
                user_id = %user_id,
                entitlement_key = %entitlement_key,
                event_id = %event_id,
                "Subscription projection synced; skipping points grant because entitlement mapping is disabled or missing"
            );
        } else {
            return Err(error);
        }
    }

    info!(
        realm_id = %realm_id,
        user_id = %user_id,
        entitlement_key = %entitlement_key,
        event_id = %event_id,
        is_renewal = payload.is_renewal,
        period_end = %period_end,
        "Subscription paid event processed - credit ledger created"
    );

    Ok(create_placeholder_transaction(
        user_id,
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

    // Resolve user_id: prefer payload, fall back to existing subscription
    let user_id = match payload.user_id {
        Some(uid) => uid,
        None => {
            let existing = app_state
                .billing_repository
                .find_by_external_subscription_id(&payload.external_subscription_id, "creem")
                .await?
                .ok_or_else(|| {
                    CoreError::BadRequest(format!(
                        "Cannot resolve userId: no userId in event and no existing subscription for {}",
                        payload.external_subscription_id
                    ))
                })?;
            existing.user_id.ok_or_else(|| {
                CoreError::BadRequest(format!(
                    "Existing subscription {} has no userId",
                    payload.external_subscription_id
                ))
            })?
        }
    };

    info!(
        realm_id = %realm_id,
        event_id = %event_id,
        "Processing subscription.update event"
    );

    // Resolve entitlement_keys via fallback chain
    let current_entitlement_key = if payload.current_entitlement_key.is_empty() {
        resolve_entitlement_key(
            &app_state,
            realm_id,
            "creem",
            &payload.external_product_id,
            None,
        )
        .await?
    } else {
        payload.current_entitlement_key.clone()
    };

    // Fetch existing subscription once — reuse for both entitlement resolution and sync
    let existing_subscription_for_update = if payload.previous_entitlement_key.is_empty() {
        app_state
            .billing_repository
            .find_by_external_subscription_id(&payload.external_subscription_id, "creem")
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
            resolve_entitlement_key(
                &app_state,
                realm_id,
                "creem",
                &payload.external_product_id,
                None,
            )
            .await?
        } else {
            from_db
        }
    } else {
        payload.previous_entitlement_key.clone()
    };

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
        .ok_or_else(|| {
            CoreError::InternalServerError(format!(
                "Entitlement mapping not found for current key '{}' during subscription update",
                current_entitlement_key
            ))
        })?;

    let old_points = old_mapping.points_per_period.unwrap_or(0);
    let new_points = new_mapping.points_per_period.unwrap_or(0);
    if old_points == 0 && new_points == 0 {
        tracing::info!(
            realm_id = %realm_id,
            "Both mappings have no points configured; skipping upgrade/downgrade classification"
        );
        // Still sync the subscription but skip points handling
        if let Some((subscription, previous)) = sync_creem_subscription(
            &app_state,
            realm_id,
            user_id,
            payload.external_subscription_id.as_str(),
            payload.client_app_id,
            current_entitlement_key,
            payload.external_product_id,
            payload.status,
            payload.current_period_start,
            payload.current_period_end,
            payload.cancel_at_period_end,
            None,
            existing_subscription_for_update.clone(),
        )
        .await?
        {
            save_subscription_history(
                &app_state,
                previous.as_ref(),
                &subscription,
                HistoryEventType::EntitlementChanged,
            )
            .await?;
        }
        return Ok(create_placeholder_transaction(
            user_id,
            realm_id,
            TransactionType::SubscriptionUpgrade,
        ));
    }
    let is_upgrade = new_points > old_points;

    let period_end_fallback = payload
        .current_period_end
        .unwrap_or_else(|| Utc::now() + chrono::Duration::days(30));

    let history_event_type = if is_upgrade {
        app_state
            .subscription_service
            .handle_subscription_upgrade(
                user_id,
                realm_id,
                &previous_entitlement_key,
                &current_entitlement_key,
                period_end_fallback,
            )
            .await?;
        HistoryEventType::Upgraded
    } else {
        app_state
            .subscription_service
            .handle_subscription_downgrade(
                user_id,
                realm_id,
                &previous_entitlement_key,
                &current_entitlement_key,
            )
            .await?;
        HistoryEventType::Downgraded
    };

    if let Some((subscription, previous)) = sync_creem_subscription(
        &app_state,
        realm_id,
        user_id,
        payload.external_subscription_id.as_str(),
        payload.client_app_id,
        current_entitlement_key,
        payload.external_product_id,
        payload.status,
        payload.current_period_start,
        payload.current_period_end,
        payload.cancel_at_period_end,
        None,
        existing_subscription_for_update.clone(),
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
        user_id,
        realm_id,
        if is_upgrade {
            TransactionType::SubscriptionUpgrade
        } else {
            TransactionType::SubscriptionDowngrade
        },
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

    // Resolve user_id: prefer payload, fall back to existing subscription.
    // When falling back, keep the fetched subscription to avoid a redundant DB query
    // in sync_creem_subscription below.
    let (user_id, existing_subscription) = match payload.user_id {
        Some(uid) => (uid, None),
        None => {
            let existing = app_state
                .billing_repository
                .find_by_external_subscription_id(&payload.external_subscription_id, "creem")
                .await?
                .ok_or_else(|| {
                    CoreError::BadRequest(format!(
                        "Cannot resolve userId: no userId in event and no existing subscription for {}",
                        payload.external_subscription_id
                    ))
                })?;
            let uid = existing.user_id.ok_or_else(|| {
                CoreError::BadRequest(format!(
                    "Existing subscription {} has no userId",
                    payload.external_subscription_id
                ))
            })?;
            (uid, Some(existing))
        }
    };

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
            user_id = %user_id,
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
            user_id = %user_id,
            event_id = %event_id,
            "Subscription immediate cancel - revoking subscription credits"
        );

        (CancelMode::ImmediateCancel, None, Some(Utc::now()))
    };

    let _output = app_state
        .subscription_service
        .handle_subscription_cancel(user_id, realm_id, cancel_mode, period_end)
        .await?;

    // Resolve entitlement_key via fallback chain
    let entitlement_key = if let Some(key) = &payload.entitlement_key {
        if !key.is_empty() {
            key.clone()
        } else {
            resolve_entitlement_key(
                &app_state,
                realm_id,
                "creem",
                &payload.external_product_id,
                None,
            )
            .await?
        }
    } else {
        resolve_entitlement_key(
            &app_state,
            realm_id,
            "creem",
            &payload.external_product_id,
            None,
        )
        .await?
    };

    if let Some((subscription, previous)) = sync_creem_subscription(
        &app_state,
        realm_id,
        user_id,
        payload.external_subscription_id.as_str(),
        payload.client_app_id,
        entitlement_key,
        payload.external_product_id,
        payload.status,
        payload.current_period_start,
        payload.current_period_end,
        payload.cancel_at_period_end,
        cancel_at,
        existing_subscription,
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
        user_id,
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

async fn resolve_existing_creem_subscription(
    app_state: &AppState,
    external_subscription_id: &str,
) -> Result<Option<Subscription>, CoreError> {
    app_state
        .billing_repository
        .find_by_external_subscription_id(external_subscription_id, "creem")
        .await
}

async fn handle_subscription_lifecycle_status(
    app_state: AppState,
    event: Value,
    realm_id: &str,
    status: SubscriptionStatus,
) -> Result<PointsTransaction, CoreError> {
    let payload = parse_subscription_lifecycle_payload(&event)?;
    let event_id = payload.event_id.as_str();
    let existing =
        resolve_existing_creem_subscription(&app_state, &payload.external_subscription_id).await?;
    let user_id = payload
        .user_id
        .or_else(|| {
            existing
                .as_ref()
                .and_then(|subscription| subscription.user_id)
        })
        .ok_or_else(|| {
            CoreError::BadRequest(format!(
                "Cannot resolve userId for subscription {}",
                payload.external_subscription_id
            ))
        })?;
    let entitlement_key = if let Some(key) = payload.entitlement_key {
        key
    } else if let Some(subscription) = &existing {
        subscription.entitlement_key.clone()
    } else {
        resolve_entitlement_key(
            &app_state,
            realm_id,
            "creem",
            &payload.external_product_id,
            None,
        )
        .await?
    };
    let external_product_id = if payload.external_product_id.is_empty() {
        existing
            .as_ref()
            .map(|subscription| subscription.external_product_id.clone())
            .unwrap_or_default()
    } else {
        payload.external_product_id
    };
    let cancel_at_period_end = status == SubscriptionStatus::ScheduledCancel;
    let cancel_at = if cancel_at_period_end {
        payload.current_period_end
    } else {
        None
    };

    info!(
        realm_id = %realm_id,
        event_id = %event_id,
        external_subscription_id = %payload.external_subscription_id,
        status = %status.as_str(),
        "Processing Creem subscription lifecycle event"
    );

    if let Some((subscription, previous)) = sync_creem_subscription(
        &app_state,
        realm_id,
        user_id,
        &payload.external_subscription_id,
        payload.client_app_id,
        entitlement_key,
        external_product_id,
        status.clone(),
        payload.current_period_start,
        payload.current_period_end,
        cancel_at_period_end,
        cancel_at,
        existing,
    )
    .await?
    {
        let history_event = match previous {
            Some(ref prev) => detect_change_type(prev, &subscription),
            None => HistoryEventType::Created,
        };

        save_subscription_history(&app_state, previous.as_ref(), &subscription, history_event)
            .await?;
    }

    // Revoke credits AFTER subscription status is synced to Expired.
    // If sync fails, no credits are revoked and the webhook retries naturally.
    if status == SubscriptionStatus::Expired {
        let output = app_state
            .points_service
            .revoke_points_by_credit_type(
                realm_id,
                user_id,
                CreditType::SubscriptionCredit,
                RevocationType::ExpireRevoke,
                "Subscription expired".to_string(),
            )
            .await?;

        info!(
            realm_id = %realm_id,
            user_id = %user_id,
            total_revoked = output.total_revoked,
            ledger_count = output.ledger_ids.len(),
            "Subscription expired - revoked all unused subscription credits"
        );
    }

    Ok(create_placeholder_transaction(
        user_id,
        realm_id,
        TransactionType::SubscriptionGrant,
    ))
}

async fn handle_dispute_created(
    app_state: AppState,
    event: Value,
    realm_id: &str,
) -> Result<PointsTransaction, CoreError> {
    let payload = parse_dispute_created_payload(&event)?;
    let event_id = payload.event_id.as_str();
    let existing =
        resolve_existing_creem_subscription(&app_state, &payload.external_subscription_id).await?;
    let existing = existing.ok_or_else(|| {
        CoreError::BadRequest(format!(
            "Cannot resolve disputed subscription {}",
            payload.external_subscription_id
        ))
    })?;
    let user_id = existing
        .user_id
        .ok_or_else(|| CoreError::BadRequest("Disputed subscription has no userId".to_string()))?;
    let provider_metadata = serde_json::json!({
        "disputeId": payload.dispute_id,
        "amount": payload.amount,
        "currency": payload.currency,
    });

    info!(
        realm_id = %realm_id,
        event_id = %event_id,
        external_subscription_id = %payload.external_subscription_id,
        dispute_id = %payload.dispute_id,
        "Processing Creem dispute.created event"
    );

    if let Some((subscription, previous)) = sync_subscription(
        &app_state,
        SyncSubscriptionInput {
            provider: "creem",
            realm_id: realm_id.to_string(),
            user_id: Some(user_id),
            external_subscription_id: payload.external_subscription_id,
            external_product_id: if payload.external_product_id.is_empty() {
                existing.external_product_id.clone()
            } else {
                payload.external_product_id
            },
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
    )
    .await?
    {
        save_subscription_history(
            &app_state,
            previous.as_ref(),
            &subscription,
            HistoryEventType::Disputed,
        )
        .await?;
    }

    Ok(create_placeholder_transaction(
        user_id,
        realm_id,
        TransactionType::SubscriptionGrant,
    ))
}

/// Canonical Creem event-type routing function.
///
/// Called from both the normal webhook handler and `reprocess_creem_event`
/// so that adding a new event type only requires updating one place.
async fn process_creem_event_once(
    app_state: AppState,
    event: &Value,
    realm_id: &str,
    idempotency_key: &str,
    event_id: &str,
    event_type: &str,
) -> Result<PointsTransaction, CoreError> {
    match event_type {
        "checkout.completed" => {
            handle_checkout_completed(app_state.clone(), event.clone(), realm_id, idempotency_key)
                .await
        }
        "subscription.paid" => {
            handle_subscription_paid(app_state.clone(), event.clone(), realm_id, idempotency_key)
                .await
        }
        "subscription.update" => {
            handle_subscription_updated(app_state.clone(), event.clone(), realm_id, idempotency_key)
                .await
        }
        "subscription.canceled" => {
            handle_subscription_canceled(
                app_state.clone(),
                event.clone(),
                realm_id,
                idempotency_key,
            )
            .await
        }
        "subscription.active" => {
            handle_subscription_lifecycle_status(
                app_state.clone(),
                event.clone(),
                realm_id,
                SubscriptionStatus::Active,
            )
            .await
        }
        "subscription.trialing" => {
            handle_subscription_lifecycle_status(
                app_state.clone(),
                event.clone(),
                realm_id,
                SubscriptionStatus::Trialing,
            )
            .await
        }
        "subscription.paused" => {
            handle_subscription_lifecycle_status(
                app_state.clone(),
                event.clone(),
                realm_id,
                SubscriptionStatus::Paused,
            )
            .await
        }
        "subscription.past_due" => {
            handle_subscription_lifecycle_status(
                app_state.clone(),
                event.clone(),
                realm_id,
                SubscriptionStatus::PastDue,
            )
            .await
        }
        "subscription.scheduled_cancel" => {
            handle_subscription_lifecycle_status(
                app_state.clone(),
                event.clone(),
                realm_id,
                SubscriptionStatus::ScheduledCancel,
            )
            .await
        }
        "subscription.expired" => {
            handle_subscription_lifecycle_status(
                app_state.clone(),
                event.clone(),
                realm_id,
                SubscriptionStatus::Expired,
            )
            .await
        }
        "refund.created" => {
            handle_refund_created(app_state.clone(), event.clone(), realm_id, idempotency_key).await
        }
        "dispute.created" => {
            handle_dispute_created(app_state.clone(), event.clone(), realm_id).await
        }
        _ => {
            warn!(
                realm_id = %realm_id,
                event_id = %event_id,
                event_type = %event_type,
                "Unknown Creem event type - ignoring"
            );
            Ok(create_placeholder_transaction(
                Uuid::now_v7(),
                realm_id,
                TransactionType::SubscriptionGrant,
            ))
        }
    }
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

    let result = process_creem_event_once(
        app_state.clone(),
        &event,
        &realm_id,
        &idempotency_key,
        &event_id,
        &event_type,
    )
    .await;

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

/// Reprocess a single Creem event that Herald missed (compensation path).
///
/// Unlike the normal webhook flow, this:
/// - Skips Redis idempotency checks entirely
/// - Skips signature verification (event comes from Creem API, not webhook)
/// - Uses DB `payment_event` for idempotency only
/// - Reuses the same match routing via `process_creem_event_once`
pub(crate) async fn reprocess_creem_event(
    app_state: AppState,
    realm_id: &str,
    event: &Value,
    event_type: &str,
) -> Result<(), CoreError> {
    let event_id = parse_event_id(event)?;

    // Check existing payment event by external_event_id
    if let Some(existing) = app_state
        .billing_repository
        .find_payment_event_by_external_id(&event_id, "creem")
        .await?
    {
        if existing.processed {
            info!(
                realm_id = %realm_id,
                event_id = %event_id,
                event_type = %event_type,
                "Creem compensation: event already processed, skipping"
            );
            return Ok(());
        }
        // Exists but not processed -- inconsistent state, do not reprocess
        error!(
            realm_id = %realm_id,
            event_id = %event_id,
            event_type = %event_type,
            "Creem compensation: event exists but processed=false, skipping inconsistent event"
        );
        return Ok(());
    }

    // Create payment event record
    let new_payment_event = PaymentEvent {
        id: Uuid::now_v7(),
        realm_id: realm_id.to_string(),
        external_event_id: event_id.clone(),
        payment_provider: "creem".to_string(),
        event_type: event_type.to_string(),
        subscription_id: None,
        payload: event.clone(),
        processed: false,
        processing_started_at: None,
        created_at: Utc::now(),
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
                "Creem compensation: concurrent insert detected, event already handled"
            );
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    // Synthetic idempotency key (not used for Redis, only passed to handlers)
    let idempotency_key = format!("compensation_creem_{}", event_id);

    // Route to the same handler match branches via shared routing function
    let result = process_creem_event_once(
        app_state.clone(),
        event,
        realm_id,
        &idempotency_key,
        &event_id,
        event_type,
    )
    .await;

    // Compensation payloads built from REST API lack webhook metadata.
    // When the handler fails with a BadRequest indicating a missing field (e.g.
    // clientAppId, entitlementKey), treat it as a best-effort skip rather than
    // a hard failure that would inflate the failed-event counter.
    let result = match result {
        Err(CoreError::BadRequest(ref msg)) if msg.contains("Missing or invalid") => {
            warn!(
                realm_id = %realm_id,
                event_id = %event_id,
                event_type = %event_type,
                error = %msg,
                "Creem compensation: skipping event due to missing metadata (expected for REST API compensation)"
            );
            // Mark the saved event as processed so we don't retry it indefinitely.
            let _ = app_state
                .billing_repository
                .mark_payment_event_processed(saved_event.id)
                .await;
            return Ok(());
        }
        other => other,
    };

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
                    "Creem compensation: handler succeeded but failed to mark payment_event as processed — event may be reprocessed on next run"
                );
            } else {
                tracing::info!(
                    realm_id = %realm_id,
                    event_id = %event_id,
                    event_type = %event_type,
                    "Creem compensation: event reprocessed successfully"
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
                "Creem compensation: failed to reprocess event"
            );
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_creem_billing_type_maps_known_variants() {
        assert_eq!(
            normalize_creem_billing_type("onetime"),
            BillingType::OneTime
        );
        assert_eq!(
            normalize_creem_billing_type("one_time"),
            BillingType::OneTime
        );
        assert_eq!(
            normalize_creem_billing_type("recurring"),
            BillingType::Recurring
        );
        assert_eq!(
            normalize_creem_billing_type("subscription"),
            BillingType::Recurring
        );
    }

    #[test]
    fn normalize_creem_billing_type_case_insensitive() {
        assert_eq!(
            normalize_creem_billing_type("OneTime"),
            BillingType::OneTime
        );
        assert_eq!(
            normalize_creem_billing_type("ONETIME"),
            BillingType::OneTime
        );
    }

    #[test]
    fn normalize_creem_billing_type_unknown_defaults_recurring() {
        assert_eq!(
            normalize_creem_billing_type("unknown"),
            BillingType::Recurring
        );
        assert_eq!(normalize_creem_billing_type(""), BillingType::Recurring);
    }

    #[test]
    fn parse_checkout_completed_extracts_attempt_id() {
        let event: Value = serde_json::json!({
            "id": "evt_test",
            "object": {
                "metadata": {
                    "herald_client_app_id": "00000000-0000-0000-0000-000000000001",
                    "herald_entitlement_key": "test-key",
                    "attemptId": "11111111-1111-1111-1111-111111111111"
                },
                "product": { "id": "prod_123" }
            }
        });

        let payload = parse_checkout_completed_payload(&event).unwrap();
        assert_eq!(
            payload.attempt_id,
            Some(Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap())
        );
    }

    #[test]
    fn parse_checkout_completed_no_attempt_id_returns_none() {
        let event: Value = serde_json::json!({
            "id": "evt_test",
            "object": {
                "metadata": {
                    "herald_client_app_id": "00000000-0000-0000-0000-000000000001",
                    "herald_entitlement_key": "test-key"
                },
                "product": { "id": "prod_123" }
            }
        });

        let payload = parse_checkout_completed_payload(&event).unwrap();
        assert!(payload.attempt_id.is_none());
    }

    #[test]
    fn parse_checkout_completed_nil_attempt_id_treated_as_absent() {
        let event: Value = serde_json::json!({
            "id": "evt_test",
            "object": {
                "metadata": {
                    "herald_client_app_id": "00000000-0000-0000-0000-000000000001",
                    "herald_entitlement_key": "test-key",
                    "attemptId": "00000000-0000-0000-0000-000000000000"
                },
                "product": { "id": "prod_123" }
            }
        });

        let payload = parse_checkout_completed_payload(&event).unwrap();
        assert!(payload.attempt_id.is_none());
    }
}
