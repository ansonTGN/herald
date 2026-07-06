use herald_core::domain::common::entities::app_errors::CoreError;
use herald_core::domain::points::entities::{PointsTransaction, TransactionType};
use herald_core::domain::purchase::metadata_keys;
use serde_json::Value;
use uuid::Uuid;

pub fn create_placeholder_transaction(
    user_id: uuid::Uuid,
    realm_id: &str,
    transaction_type: TransactionType,
) -> PointsTransaction {
    let description = format!("Placeholder for {:?}", transaction_type);
    PointsTransaction {
        id: uuid::Uuid::now_v7(),
        wallet_id: uuid::Uuid::now_v7(),
        user_id,
        realm_id: realm_id.to_string(),
        bucket_id: Uuid::nil(),
        transaction_type,
        amount: 0,
        // Pure idempotency/no-op placeholder — no ledger
        // mutation, no real wallet (synthetic wallet_id, nil bucket_id), amount
        // = 0. `balance_after`/typed snapshots legitimately read 0 (no points
        // moved); computing a derived balance against the nil bucket would also
        // yield 0. effective_at is None (no grant ledger row).
        balance_after: 0,
        topup_balance_after: Some(0),
        subscription_balance_after: Some(0),
        credit_type: None,
        description: Some(description),
        client_app_id: None,
        subscription_id: None,
        external_ref_id: None,
        correlation_id: None,
        effective_at: None,
        created_at: chrono::Utc::now(),
    }
}

/// Parse event ID from webhook event JSON.
pub fn parse_event_id(event: &Value) -> Result<String, CoreError> {
    event["id"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| CoreError::BadRequest("Missing event id".to_string()))
}

/// Parse a required UUID field from JSON.
pub fn parse_uuid_field(value: &Value, field_name: &str) -> Result<Uuid, CoreError> {
    value
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| CoreError::BadRequest(format!("Missing or invalid {}", field_name)))
}

/// Parse an optional UUID field from JSON.
pub fn parse_optional_uuid_field(value: &Value) -> Option<Uuid> {
    value.as_str().and_then(|s| Uuid::parse_str(s).ok())
}

/// Look up a metadata value by primary key, falling back to an alternate key.
pub fn metadata_value<'a>(metadata: &'a Value, primary: &str, fallback: &str) -> &'a Value {
    metadata.get(primary).unwrap_or(&metadata[fallback])
}

/// Resolve the Herald user id from provider webhook metadata.
///
/// Stripe metadata key naming is inconsistent across write paths:
/// - `purchase_service` (Checkout Session path) writes `heraldUserId` (camelCase)
/// - `infra-stripe/client.rs` and `api-billing/handlers.rs` write `herald_user_id` (snake_case)
/// - legacy/fallback readers expect `userId`
///
/// Stripe merges metadata from the Checkout Session + PaymentIntent onto the
/// generated Invoice, so all three keys are typically present on `invoice.*`
/// payloads. This helper tries them in order of recency and returns the first
/// parseable UUID.
pub fn metadata_user_id(metadata: &Value) -> Option<Uuid> {
    metadata
        .get(metadata_keys::HERALD_USER_ID)
        .or_else(|| metadata.get("herald_user_id"))
        .or_else(|| metadata.get("userId"))
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
}

/// Parse attempt_id from JSON, treating nil UUID as absent.
pub fn parse_attempt_id(value: &Value) -> Option<Uuid> {
    value
        .as_str()
        .and_then(|raw| Uuid::parse_str(raw).ok())
        .filter(|id| *id != Uuid::nil())
}
