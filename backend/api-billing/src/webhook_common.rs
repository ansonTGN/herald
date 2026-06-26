use herald_api_base::application::http::state::AppState;
use herald_core::domain::common::entities::app_errors::CoreError;
use herald_core::domain::points::entities::{PointsTransaction, TransactionType};
use herald_core::domain::points::ports::{PointsRepository, ReclaimLocator};
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

/// Parse attempt_id from JSON, treating nil UUID as absent.
pub fn parse_attempt_id(value: &Value) -> Option<Uuid> {
    value
        .as_str()
        .and_then(|raw| Uuid::parse_str(raw).ok())
        .filter(|id| *id != Uuid::nil())
}

/// Reason string recorded on `PointsRevocationRecord` rows produced by
/// pre-grant reclaim. The infra reclaim path
/// (`revoke_pregrant_ledger_row_atomic`) writes the revocation record for
/// partially-consumed rows using this reason; fully-unused rows need no
/// debt record.
pub const RECLAIM_REASON: &str = "subscription_pre_grant_reclaim";

/// Row-level reclaim of a subscription's pre-granted future period.
///
/// Subscription pre-grant is **chained one period ahead**:
/// each `handle_subscription_paid` / activation writes the current period
/// and pre-grants the next period. `pregrant_next_period_atomic` advances
/// `granted_periods` to the pre-granted period number, so the highest
/// period row (the not-yet-current pre-grant) lives AT
/// `granted_periods`. "All future-effective pre-grant rows for this
/// subscription" therefore resolves to that single row, located
/// row-precisely via `ReclaimLocator::BySchedulePeriod { schedule_id,
/// period_number }`.
///
/// Behaviour:
/// - No schedule on file (legacy subscription / pre-D08 gap) → no-op, returns
///   `Ok(0)`. Provider event-level idempotency + the absence of a pre-grant
///   row make this safe.
/// - Row already revoked / never pre-granted → infra returns 0 (idempotent).
/// - Row active & unused → set `revoked`, no revocation record (no shortfall).
/// - Row active & partially consumed → set `revoked` + write
///   `PointsRevocationRecord(reason = subscription_pre_grant_reclaim)`.
///
/// **No wallet back-adjustment**: derived balance auto-excludes revoked
/// rows. **No other active credits touched** (row-precise locator).
///
/// Returns the number of ledger rows revoked (0 ⟹ idempotent no-op).
pub async fn reclaim_pregrant_for_subscription(
    app_state: &AppState,
    realm_id: &str,
    subscription_id: Uuid,
) -> Result<usize, CoreError> {
    // Resolve the subscription's grant schedule. When absent there is no
    // chained pre-grant row to reclaim — return idempotently.
    let Some(schedule) = app_state
        .points_repository
        .find_grant_schedule_by_subscription(subscription_id)
        .await?
    else {
        tracing::debug!(
            realm_id = %realm_id,
            subscription_id = %subscription_id,
            "reclaim: no grant schedule for subscription, no pre-grant row to revoke"
        );
        return Ok(0);
    };

    // The pre-grant to reclaim is the highest-numbered period row, which lives
    // AT `granted_periods`. Reason: `pregrant_next_period_atomic` advances
    // `granted_periods = max(old, period_number)` after writing the chained
    // pre-grant, so once a pre-grant exists for period N, `granted_periods`
    // already equals N (not N-1). The worker backstop still pre-grants
    // `granted_periods + 1` (the next UN-pre-granted period) — that path is
    // untouched. Reclaiming the row at `granted_periods` therefore covers the
    // future-effective pre-grant state for this subscription. (Off-by-one fix:
    // the previous `granted_periods + 1` target pointed one period too high and
    // left the real pre-grant row active after cancel.)
    let next_period_number = u32::try_from(schedule.granted_periods)
        .map_err(|_| CoreError::InternalServerError("granted_periods overflows u32".to_string()))?;
    let locator = ReclaimLocator::BySchedulePeriod {
        schedule_id: schedule.id,
        period_number: next_period_number,
    };

    let affected = app_state
        .points_repository
        .revoke_pregrant_ledger_row_atomic(realm_id, locator, RECLAIM_REASON)
        .await?;

    tracing::info!(
        realm_id = %realm_id,
        subscription_id = %subscription_id,
        schedule_id = %schedule.id,
        period_number = next_period_number,
        affected,
        reason = RECLAIM_REASON,
        "Pre-grant reclaim: future-effective ledger row revoked (row-precise, no wallet back-adjust)"
    );

    Ok(affected)
}
