// Subscription Service - Handles subscription lifecycle events
//
// This service manages subscription upgrade, downgrade, and cancellation events
// from the billing system. It follows hexagonal architecture principles and
// uses repository ports directly to avoid circular dependencies.

use chrono::{DateTime, Duration, Months, Utc};
use std::sync::Arc;
use uuid::Uuid;

use crate::billing::entities::EntitlementMapping;
use crate::common::entities::app_errors::CoreError;
use crate::points::{
    PointsGrantRecord,
    dtos::RevokePointsOutput,
    entities::{CreditSourceType, CreditType, PointsCreditLedger, RevocationType},
    grant_schedule::GrantPeriodType,
    ports::{LedgerUpdate, PointsRepository},
    service::PointsService,
};

const IDEMPOTENCY_KEY_SUBSCRIPTION_PAID: &str = "sub_paid";
const ERROR_ENTITLEMENT_NO_GRANT: &str = "Entitlement does not grant points on subscribe";

/// Resolve the per-period points amount an entitlement grants.
///
/// Extracted from the inline `mapping.points_per_period` read so the
/// pre-grant path and the formal subscription-paid webhook share one amount
/// source. Returns `None` when the entitlement is configured with no points
/// (None / non-positive) so callers can short-circuit with a graceful skip
/// rather than surfacing a data-integrity error for an entitlement that
/// should simply be ignored.
fn resolve_entitlement_points(mapping: &EntitlementMapping) -> Option<i64> {
    match mapping.points_per_period {
        Some(amount) if amount > 0 => Some(amount),
        _ => None,
    }
}

/// Derive the 1-based `period_number` for a subscription period anchored to
/// the schedule's `first_period_start`.
///
/// `period_number = floor((period_start − first_period_start) / nominal_period_duration) + 1`
///
/// `nominal_period_duration` follows the entitlement/plan cycle:
/// `Monthly` uses calendar-month arithmetic (`chrono::Months`), so periods
/// crossing natural-month boundaries (Jan-31 → Feb-28 → Mar-28) stay aligned
/// to the anchor day-of-month. `Weekly`/`Daily` use fixed `Duration`. The
/// first period (`period_start == first_period_start`) is `period_number=1`.
/// Returns an error if `period_start` is strictly before `first_period_start`
/// (clock skew / out-of-order webhook) — callers should treat that as a
/// provider-period-normalization bug, not a silent clamp.
fn derive_period_number(
    first_period_start: DateTime<Utc>,
    period_start: DateTime<Utc>,
    nominal: GrantPeriodType,
) -> Result<u32, CoreError> {
    if period_start < first_period_start {
        return Err(CoreError::BadRequest(format!(
            "period_start {} precedes first_period_start {} (clock skew or out-of-order webhook)",
            period_start, first_period_start
        )));
    }
    match nominal {
        GrantPeriodType::Once => {
            // Once-type subscriptions have a single period; any period_start
            // at/after the anchor is period 1.
            Ok(1)
        }
        GrantPeriodType::Daily => {
            let elapsed = period_start - first_period_start;
            let days = elapsed.num_days();
            if days < 0 {
                return Err(CoreError::BadRequest(
                    "Negative day delta in period derivation".to_string(),
                ));
            }
            Ok(u32::try_from(days).map_err(|_| {
                CoreError::InternalServerError("period_number (daily) overflow".to_string())
            })? + 1)
        }
        GrantPeriodType::Weekly => {
            let elapsed = period_start - first_period_start;
            let weeks = elapsed.num_weeks();
            if weeks < 0 {
                return Err(CoreError::BadRequest(
                    "Negative week delta in period derivation".to_string(),
                ));
            }
            Ok(u32::try_from(weeks).map_err(|_| {
                CoreError::InternalServerError("period_number (weekly) overflow".to_string())
            })? + 1)
        }
        GrantPeriodType::Monthly => {
            // Calendar-month delta. Count whole months between the anchors;
            // any residual (< 1 month) still belongs to the current period
            // because the NEXT month boundary is the period boundary.
            // `checked_*` guards against month-count overflow and the
            // year/month saturation edge (anchor day-of-month > target month
            // length) — saturation collapses onto the last valid day, which
            // is the desired "same day-of-month next month" semantics.
            let mut count: u32 = 0;
            let mut cursor = first_period_start;
            // Walk forward month-by-month until cursor > period_start; the
            // period_number is the count of complete month boundaries crossed
            // plus one. This avoids Duration rounding for month-length
            // variance (28/29/30/31 days).
            while cursor <= period_start {
                let next = cursor.checked_add_months(Months::new(1)).ok_or_else(|| {
                    CoreError::InternalServerError("month add overflow in period derivation".into())
                })?;
                if next <= period_start {
                    count = count.checked_add(1).ok_or_else(|| {
                        CoreError::InternalServerError("period_number overflow".into())
                    })?;
                    cursor = next;
                } else {
                    break;
                }
            }
            Ok(count + 1)
        }
    }
}

/// Estimate the end of the period that starts at `period_start` when the
/// formal webhook has not yet supplied the provider's actual `period_end`.
/// Monthly uses `chrono::Months` calendar arithmetic;
/// `Once` returns `None` (no expiry).
fn estimate_next_period_end(
    period_start: DateTime<Utc>,
    nominal: GrantPeriodType,
) -> Option<DateTime<Utc>> {
    match nominal {
        GrantPeriodType::Once => None,
        GrantPeriodType::Daily => Some(period_start + Duration::days(1)),
        GrantPeriodType::Weekly => Some(period_start + Duration::weeks(1)),
        GrantPeriodType::Monthly => period_start.checked_add_months(Months::new(1)),
    }
}

/// Advance a period_start by one nominal period — used to compute the next
/// period's start for chained pre-grant.
fn advance_one_period(
    period_start: DateTime<Utc>,
    nominal: GrantPeriodType,
) -> Result<DateTime<Utc>, CoreError> {
    match nominal {
        GrantPeriodType::Once => Err(CoreError::BadRequest(
            "Cannot advance a Once-type subscription period".to_string(),
        )),
        GrantPeriodType::Daily => Ok(period_start + Duration::days(1)),
        GrantPeriodType::Weekly => Ok(period_start + Duration::weeks(1)),
        GrantPeriodType::Monthly => {
            period_start
                .checked_add_months(Months::new(1))
                .ok_or_else(|| {
                    CoreError::InternalServerError("month add overflow in period advance".into())
                })
        }
    }
}

/// Parse the entitlement mapping's `grant_period_type` (stored as Option<String>)
/// into the nominal period used for `period_number` derivation and
/// `estimate_next_period_end`. Falls back to `Monthly` when the mapping does
/// not specify one (subscription entitlements are monthly by default per the
/// billing domain convention).
fn nominal_period_for(mapping: &EntitlementMapping) -> GrantPeriodType {
    mapping
        .grant_period_type
        .as_deref()
        .and_then(|s| s.parse::<GrantPeriodType>().ok())
        .unwrap_or(GrantPeriodType::Monthly)
}

/// Cancellation mode for subscriptions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelMode {
    /// Cancel at period end - points remain usable until period ends
    DefaultCancel,
    /// Cancel immediately - revoke all unused subscription points
    ImmediateCancel,
}

/// Subscription Service for handling subscription lifecycle events
///
/// This service manages subscription upgrades, downgrades, and cancellations.
/// It works with the PointsService and Repository to grant and revoke points.
pub struct SubscriptionService<R, P>
where
    R: PointsRepository + Send + Sync,
    P: crate::points::policies::PointsPolicy,
{
    points_service: Arc<PointsService<R, P>>,
    repo: Arc<R>,
    _grant_scheduler: Option<Arc<crate::points::services::GrantScheduler<R, P>>>,
}

impl<R, P> SubscriptionService<R, P>
where
    R: PointsRepository + Send + Sync,
    P: crate::points::policies::PointsPolicy,
{
    /// Create a new SubscriptionService
    pub fn new(
        points_service: Arc<PointsService<R, P>>,
        repo: Arc<R>,
        _grant_scheduler: Option<Arc<crate::points::services::GrantScheduler<R, P>>>,
    ) -> Self {
        Self {
            points_service,
            repo,
            _grant_scheduler,
        }
    }

    /// Handle subscription upgrade
    ///
    /// Revokes subscription credits for the old entitlement, then grants the new entitlement's full points.
    /// The new points expire at the end of the recalculated billing period.
    ///
    /// Strategy source is the **price-level** mapping (US-EM-008):
    /// `old_mapping`/`new_mapping` are the resolved price-level entitlement
    /// mappings the caller (webhook handler) obtained via price-aware
    /// resolution. This kills the shared-`entitlement_key` ambiguity (e.g.
    /// monthly 1000 vs annual 12000 both under `pro-plan`).
    pub async fn handle_subscription_upgrade(
        &self,
        user_id: Uuid,
        bucket_id: Uuid,
        realm_id: &str,
        old_mapping: &EntitlementMapping,
        new_mapping: &EntitlementMapping,
        period_end: DateTime<Utc>,
    ) -> Result<PointsCreditLedger, CoreError> {
        let old_entitlement_key = old_mapping.entitlement_key.as_str();
        let new_entitlement_key = new_mapping.entitlement_key.as_str();

        let new_points = new_mapping.points_per_period.unwrap_or(0);
        if new_points <= 0 {
            return Err(CoreError::BadRequest(
                "New entitlement must grant points for upgrade".to_string(),
            ));
        }

        let revoked = self
            .repo
            .revoke_subscription_credits_by_entitlement_atomic(
                realm_id,
                user_id,
                bucket_id,
                old_entitlement_key,
                RevocationType::UpgradeRevoke,
                "Subscription upgrade replaced old subscription credits".to_string(),
                None,
                None,
            )
            .await?;

        let created_ledger = self
            .repo
            .grant_points_atomic(
                realm_id,
                user_id,
                bucket_id,
                CreditType::SubscriptionCredit,
                CreditSourceType::SubscriptionUpgrade,
                new_points,
                Some(period_end),
                None, // effective_at = None (upgrade grant is immediately available; period anchoring revisited later)
                Some(new_entitlement_key.to_string()),
                None,
                Some(format!("grant:upgrade:{}", new_entitlement_key)),
            )
            .await?;

        tracing::info!(
            realm_id = %realm_id,
            user_id = %user_id,
            old_entitlement_key = %old_entitlement_key,
            new_entitlement_key = %new_entitlement_key,
            revoked_points = revoked.total_revoked,
            new_points,
            period_end = %period_end,
            "Subscription upgrade: reclaimed old subscription credits and granted new points"
        );

        Ok(created_ledger)
    }

    /// Handle subscription downgrade
    ///
    /// Logs the downgrade event but does NOT revoke any points.
    /// Users keep their existing points for the current period; future renewals
    /// (next-period `grant_schedule`) will use the new entitlement and route to
    /// the same `subscription.bucket_id` (bound eagerly at creation, non-null).
    pub async fn handle_subscription_downgrade(
        &self,
        user_id: Uuid,
        subscription_id: Uuid,
        bucket_id: Uuid,
        realm_id: &str,
        old_mapping: &EntitlementMapping,
        new_mapping: &EntitlementMapping,
    ) -> Result<(), CoreError> {
        let old_entitlement_key = old_mapping.entitlement_key.as_str();
        let new_entitlement_key = new_mapping.entitlement_key.as_str();

        tracing::info!(
            realm_id = %realm_id,
            user_id = %user_id,
            subscription_id = %subscription_id,
            bucket_id = %bucket_id,
            old_entitlement_key = %old_entitlement_key,
            new_entitlement_key = %new_entitlement_key,
            "Subscription downgraded - no points revoked this period; next-period grant_schedule keeps the same bucket and uses the new entitlement"
        );

        Ok(())
    }

    /// Handle subscription paid event (initial or renewal)
    ///
    /// Creates a credit ledger for subscription points grant based on entitlement_key.
    /// The ledger carries `effective_at = period_start` (zero-delay availability:
    /// `period_start <= now` ⟺ immediately spendable) and
    /// `expires_at = period_end`.
    ///
    /// **Business idempotency (period-level)**: when the
    /// subscription's grant schedule is resolvable, the grant is deduplicated
    /// by `points_grant_records(schedule_id, period_number)` where
    /// `period_number` is derived from `period_start` anchored to
    /// `schedule.base_time` (`first_period_start`). On a hit:
    /// - The pre-granted ledger row's `expires_at` is **corrected** to the
    ///   provider's actual `period_end` if it differs (pre-grant used an
    ///   estimate; the formal webhook is the truth source), with an audit log.
    /// - No duplicate grant is issued.
    /// - The next period is **chained** via `pregrant_next_period_atomic`
    ///   (`period_number + 1`, future-`effective_at`).
    ///
    /// When no prior grant_record exists, the current period is granted
    /// atomically via `handle_subscription_paid_atomic` (infra path also
    /// writes the grant_record linking the new ledger row), then the next
    /// period is chained the same way.
    ///
    /// The grant routes to `subscription.bucket_id`, bound eagerly at
    /// subscription creation (non-null). The caller supplies the resolved
    /// subscription's `bucket_id`.
    ///
    /// Provider event-level idempotency (`event_id`) is retained at the
    /// webhook handler layer as a defense-in-depth backstop;
    /// the `event_id` passed here is used only for the secondary
    /// `sub_paid:{event_id}` idempotency key (kept for cross-crate
    /// compatibility until provider event-level dedup lands).
    pub async fn handle_subscription_paid(
        &self,
        user_id: Uuid,
        subscription_id: Uuid,
        bucket_id: Uuid,
        realm_id: &str,
        mapping: &EntitlementMapping,
        is_renewal: bool,
        // Billing period start. Drives `effective_at`,
        // `period_number`, and chained pre-grant. Provider period
        // normalization is the upstream source; this method does not guess.
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
        event_id: String,
    ) -> Result<PointsCreditLedger, CoreError> {
        // Strategy source is the **price-level** mapping
        // (US-EM-008): the caller (webhook handler) supplies the mapping
        // resolved via price-aware resolution, so shared-`entitlement_key`
        // ambiguity (monthly 1000 vs annual 12000 under one key) cannot reach
        // the grant. `entitlement_key` is read from the mapping for downstream
        // ledger/revoke bookkeeping.
        let entitlement_key = mapping.entitlement_key.as_str();

        // Secondary (legacy) event-level idempotency key. Period-level
        // business idempotency via `points_grant_records(schedule_id,
        // period_number)` is the primary gate (checked below when a schedule
        // is resolvable). This key is retained so the existing
        // `check_completed_idempotency` infra path keeps working during the
        // rollout period; provider event-level dedup is finalized at the
        // webhook handler layer.
        let idempotency_key = format!("{}:{}", IDEMPOTENCY_KEY_SUBSCRIPTION_PAID, event_id);

        // The mapping is the price-level strategy source supplied by the
        // caller. `grant_on_subscribe=false` means this entitlement should be
        // ignored (graceful skip), matching the prior contract where an
        // absent/disabled mapping raised `EntitlementMappingNotFound` (which
        // the Creem webhook handler swallows). The fail-loud bucket check
        // (`mapping exists but no bucket`) runs at the webhook layer before
        // calling this method, so `subscription_with_unresolved_bucket_fails_loud`
        // still holds.
        if !mapping.grant_on_subscribe {
            tracing::info!(
                realm_id = %realm_id,
                entitlement_key = %entitlement_key,
                "Entitlement does not grant points on subscribe, skipping"
            );
            return self
                .create_placeholder_transaction_with_ref(user_id, realm_id, &idempotency_key)
                .await;
        }

        // Resolve the per-period amount via the shared helper so pre-grant
        // and formal webhook converge on the same amount source.
        let points_amount = match resolve_entitlement_points(mapping) {
            Some(amount) => amount,
            None => {
                tracing::info!(
                    realm_id = %realm_id,
                    entitlement_key = %entitlement_key,
                    "Entitlement has no points_per_period configured, skipping grant"
                );
                return self
                    .create_placeholder_transaction_with_ref(user_id, realm_id, &idempotency_key)
                    .await;
            }
        };

        let disable_daily_grant = self
            .repo
            .find_user_config_by_realm(realm_id, user_id)
            .await?
            .is_some_and(|config| config.free_periodic_points_amount > 0);

        if disable_daily_grant {
            tracing::info!(
                realm_id = %realm_id,
                user_id = %user_id,
                "Free user upgrading to paid subscription"
            );
        }

        let source_type = if is_renewal {
            CreditSourceType::SubscriptionRenewal
        } else {
            CreditSourceType::SubscriptionInitial
        };

        // Nominal period (from entitlement/plan) drives period_number
        // derivation, chained-pregrant effective_at, and estimated expires_at.
        let nominal = nominal_period_for(mapping);

        // Resolve the subscription's grant schedule. When present, the
        // schedule's `base_time` is the `first_period_start` anchor
        // (subscription schedules are created at subscription binding time
        // with `base_time = first period start`). When absent (no schedule
        // row yet — e.g. legacy subscription or provider normalization gap),
        // we fall back to the legacy atomic grant path WITHOUT period-level
        // idempotency or chained pre-grant; provider event-level idempotency
        // remains the backstop.
        let schedule = self
            .repo
            .find_grant_schedule_by_subscription(subscription_id)
            .await?;

        if let Some(schedule_ref) = schedule.as_ref() {
            return self
                .handle_subscription_paid_with_schedule(
                    user_id,
                    subscription_id,
                    bucket_id,
                    realm_id,
                    entitlement_key,
                    is_renewal,
                    period_start,
                    period_end,
                    event_id,
                    idempotency_key,
                    points_amount,
                    source_type,
                    disable_daily_grant,
                    nominal,
                    schedule_ref,
                )
                .await;
        }

        // Fallback: no schedule resolvable — legacy atomic grant path.
        tracing::warn!(
            realm_id = %realm_id,
            user_id = %user_id,
            subscription_id = %subscription_id,
            "No grant schedule found for subscription; skipping period-level idempotency and chained pre-grant (legacy path)"
        );
        let created_ledger = self
            .repo
            .handle_subscription_paid_atomic(
                realm_id,
                user_id,
                bucket_id,
                entitlement_key.to_string(),
                points_amount,
                source_type,
                period_start,
                period_end,
                idempotency_key.clone(),
                disable_daily_grant,
            )
            .await?;

        tracing::info!(
            realm_id = %realm_id,
            user_id = %user_id,
            subscription_id = %subscription_id,
            bucket_id = %bucket_id,
            entitlement_key = %entitlement_key,
            is_renewal,
            points_amount,
            period_start = %period_start,
            period_end = %period_end,
            event_id = %event_id,
            "Subscription paid: credit ledger created (legacy path, no schedule / no chained pre-grant)"
        );

        Ok(created_ledger)
    }

    /// Period-aware subscription grant path. Handles both the
    /// "pre-grant already exists for this period" correction case and the
    /// "no prior pre-grant, grant now" case, then chains the next period.
    #[allow(clippy::too_many_arguments)]
    async fn handle_subscription_paid_with_schedule(
        &self,
        user_id: Uuid,
        subscription_id: Uuid,
        bucket_id: Uuid,
        realm_id: &str,
        entitlement_key: &str,
        is_renewal: bool,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
        event_id: String,
        idempotency_key: String,
        points_amount: i64,
        source_type: CreditSourceType,
        disable_daily_grant: bool,
        nominal: GrantPeriodType,
        schedule: &crate::points::PointsGrantSchedule,
    ) -> Result<PointsCreditLedger, CoreError> {
        let period_number = derive_period_number(schedule.base_time, period_start, nominal)?;

        // Period-level business idempotency. The grant
        // record UNIQUE(schedule_id, period_number) is the primary dedup;
        // pre-grant and formal webhook both converge here for the same
        // (schedule_id, period_number).
        let existing_record = self
            .repo
            .find_grant_record(schedule.id, i64::from(period_number))
            .await?;

        let created_ledger = if let Some(record) = existing_record {
            // Pre-grant already wrote this period's ledger row. Correct its
            // `expires_at` to the provider's actual period_end if the
            // pre-grant estimate differs. No re-grant.
            let existing_ledger = self
                .repo
                .find_ledger_by_id(record.ledger_id)
                .await?
                .ok_or_else(|| {
                    CoreError::InternalServerError(format!(
                        "grant_record {} references missing ledger {}",
                        record.id, record.ledger_id
                    ))
                })?;

            if let Some(existing_expires) = existing_ledger.expires_at {
                if existing_expires != period_end {
                    tracing::info!(
                        realm_id = %realm_id,
                        user_id = %user_id,
                        subscription_id = %subscription_id,
                        schedule_id = %schedule.id,
                        period_number,
                        ledger_id = %existing_ledger.id,
                        old_expires_at = %existing_expires,
                        new_expires_at = %period_end,
                        "Subscription pre-grant expires_at corrected to provider period_end"
                    );
                    self.repo
                        .update_ledger(existing_ledger.id, LedgerUpdate::SetExpiration(period_end))
                        .await?;
                }
            } else if existing_ledger.expires_at.is_none() {
                // Pre-grant left expires_at as None (estimate said Once / no
                // expiry) but the formal webhook now supplies a real period_end.
                tracing::info!(
                    realm_id = %realm_id,
                    user_id = %user_id,
                    subscription_id = %subscription_id,
                    schedule_id = %schedule.id,
                    period_number,
                    ledger_id = %existing_ledger.id,
                    new_expires_at = %period_end,
                    "Subscription pre-grant expires_at set from None to provider period_end"
                );
                self.repo
                    .update_ledger(existing_ledger.id, LedgerUpdate::SetExpiration(period_end))
                    .await?;
            }

            tracing::info!(
                realm_id = %realm_id,
                user_id = %user_id,
                subscription_id = %subscription_id,
                schedule_id = %schedule.id,
                period_number,
                ledger_id = %existing_ledger.id,
                event_id = %event_id,
                "Subscription paid: period already pre-granted, no duplicate grant issued (period-level idempotency)"
            );

            existing_ledger
        } else {
            // No prior pre-grant for this period — grant the current period
            // atomically. `handle_subscription_paid_atomic` writes the
            // ledger with effective_at=period_start, expires_at=period_end
            // and (infra side) records the grant_record linking the
            // new ledger via `ledger_id` so subsequent webhook retries /
            // pre-grant collisions hit the idempotency gate above. The
            // secondary `sub_paid:{event_id}` key remains a backstop for
            // legacy callers.
            let granted = self
                .repo
                .handle_subscription_paid_atomic(
                    realm_id,
                    user_id,
                    bucket_id,
                    entitlement_key.to_string(),
                    points_amount,
                    source_type,
                    period_start,
                    period_end,
                    idempotency_key.clone(),
                    disable_daily_grant,
                )
                .await?;

            // Defensive: if the infra path did not (yet) write the
            // grant_record — e.g. during rollout before the atomic
            // impl is updated — record one here so the period-level
            // idempotency gate has a row to hit on the next retry / pre-grant.
            // The grant_record's UNIQUE(schedule_id, period_number) makes
            // this idempotent: a collision means the infra path already wrote
            // it, which we treat as success.
            let now = crate::common::entities::now_utc();
            let record_id = crate::common::entities::generate_uuid_v7();
            let defensive_record = PointsGrantRecord {
                id: record_id,
                schedule_id: schedule.id,
                user_id,
                realm_id: realm_id.to_string(),
                period_number: i64::from(period_number),
                granted_amount: points_amount,
                grant_time: period_start,
                ledger_id: granted.id,
                created_at: now,
            };
            if let Err(e) = self.repo.create_grant_record(defensive_record).await {
                // Only a UNIQUE-violation is safe to swallow: it means the
                // infra atomic path already wrote the record
                // — the period was successfully recorded. Any OTHER
                // error (DB drop, deadlock, CHECK violation, serialization
                // failure) must surface fail-loud: silently proceeding would
                // leave a granted ledger with NO grant_record, so the next
                // webhook redelivery's `find_grant_record(period)` returns
                // None and the current period is granted AGAIN (double-grant).
                // The sqlx error code is lost through `CoreError::DatabaseError`,
                // so classify via the rendered message (codebase convention,
                // see `classify_from_message` in infra billing repo).
                let is_unique_violation = matches!(&e,
                    CoreError::DatabaseError(m) if m.contains("duplicate key value")
                        || m.contains("23505")
                );
                if is_unique_violation {
                    tracing::warn!(
                        realm_id = %realm_id,
                        user_id = %user_id,
                        subscription_id = %subscription_id,
                        schedule_id = %schedule.id,
                        period_number,
                        error = %e,
                        "create_grant_record for current period hit UNIQUE violation (already written by atomic path); proceeding"
                    );
                } else {
                    return Err(e);
                }
            }

            tracing::info!(
                realm_id = %realm_id,
                user_id = %user_id,
                subscription_id = %subscription_id,
                bucket_id = %bucket_id,
                entitlement_key = %entitlement_key,
                is_renewal,
                points_amount,
                period_start = %period_start,
                period_end = %period_end,
                schedule_id = %schedule.id,
                period_number,
                event_id = %event_id,
                "Subscription paid: credit ledger created (period-level path)"
            );

            granted
        };

        // Chained pre-grant of the NEXT period. The next
        // period's `effective_at` is its `period_start` (= current
        // period_end for monthly; computed via `advance_one_period`); its
        // `expires_at` uses the estimate (`estimate_next_period_end`) since
        // the formal webhook has not arrived yet. The trait impl
        // is idempotent on `(schedule_id, period_number)`, so calling this
        // on every webhook retry is safe. A `Once` nominal period has no
        // "next period" — skip the chained pre-grant.
        if nominal != GrantPeriodType::Once {
            let next_period_start = advance_one_period(period_start, nominal)?;
            let next_period_number = period_number
                .checked_add(1)
                .ok_or_else(|| CoreError::InternalServerError("period_number overflow".into()))?;
            let estimated_expires = estimate_next_period_end(next_period_start, nominal);

            // `pregrant_next_period_atomic` writes a future-effective ledger
            // row (excluded from available balance until the period starts)
            // + a grant_record row. Errors here are surfaced fail-loud — a
            // chained pre-grant failure must not silently degrade the
            // current period's grant (which already succeeded). Caller
            // (webhook handler) decides retry semantics.
            if let Err(e) = self
                .repo
                .pregrant_next_period_atomic(
                    realm_id,
                    schedule,
                    next_period_number,
                    Some(next_period_start),
                    estimated_expires,
                )
                .await
            {
                tracing::error!(
                    realm_id = %realm_id,
                    user_id = %user_id,
                    subscription_id = %subscription_id,
                    schedule_id = %schedule.id,
                    next_period_number,
                    next_period_start = %next_period_start,
                    error = %e,
                    "Chained pre-grant for next period failed (fail-loud); current period grant already committed"
                );
                return Err(e);
            }

            tracing::info!(
                realm_id = %realm_id,
                user_id = %user_id,
                subscription_id = %subscription_id,
                schedule_id = %schedule.id,
                next_period_number,
                next_period_start = %next_period_start,
                "Subscription chained pre-grant: next period ledger written (future-effective)"
            );
        }

        Ok(created_ledger)
    }

    /// Create placeholder transaction with external ref (for idempotency when grant_on_subscribe = false)
    async fn create_placeholder_transaction_with_ref(
        &self,
        _user_id: Uuid,
        realm_id: &str,
        external_ref_id: &str,
    ) -> Result<PointsCreditLedger, CoreError> {
        let dummy_transaction_id = crate::common::entities::generate_uuid_v7();
        self.repo
            .record_idempotency_key(realm_id, external_ref_id, dummy_transaction_id)
            .await?;

        Err(CoreError::BadRequest(
            ERROR_ENTITLEMENT_NO_GRANT.to_string(),
        ))
    }

    /// Handle subscription cancellation
    ///
    /// Two modes:
    /// - DefaultCancel: Set expiration on existing subscription credits to period_end
    /// - ImmediateCancel: Revoke unused subscription credits for the specific entitlement
    ///
    /// When `entitlement_key` is provided with ImmediateCancel, only credits from that
    /// entitlement are revoked. Otherwise falls back to revoking all subscription credits.
    pub async fn handle_subscription_cancel(
        &self,
        user_id: Uuid,
        bucket_id: Uuid,
        realm_id: &str,
        cancel_mode: CancelMode,
        period_end: Option<DateTime<Utc>>,
        entitlement_key: Option<&str>,
    ) -> Result<RevokePointsOutput, CoreError> {
        match cancel_mode {
            CancelMode::DefaultCancel => {
                let period_end = period_end.ok_or_else(|| {
                    CoreError::BadRequest("period_end required for DefaultCancel".to_string())
                })?;

                let ledger_ids = self
                    .repo
                    .set_subscription_ledger_expiration_atomic(realm_id, user_id, period_end)
                    .await?;

                tracing::info!(
                    realm_id = %realm_id,
                    user_id = %user_id,
                    period_end = %period_end,
                    ledger_count = ledger_ids.len(),
                    "Subscription cancelled at period end - updated expiration times"
                );

                Ok(RevokePointsOutput {
                    revocation_id: Uuid::now_v7(),
                    ledger_ids,
                    total_revoked: 0,
                    revoked_at: Utc::now(),
                })
            }
            CancelMode::ImmediateCancel => {
                let output = if let Some(ekey) = entitlement_key {
                    if ekey.is_empty() {
                        self.points_service
                            .revoke_points_by_credit_type(
                                realm_id,
                                user_id,
                                bucket_id,
                                CreditType::SubscriptionCredit,
                                RevocationType::CancelRevoke,
                                "Immediate subscription cancellation".to_string(),
                            )
                            .await?
                    } else {
                        self.repo
                            .revoke_subscription_credits_by_entitlement_atomic(
                                realm_id,
                                user_id,
                                bucket_id,
                                ekey,
                                RevocationType::CancelRevoke,
                                "Immediate subscription cancellation".to_string(),
                                None,
                                None,
                            )
                            .await?
                    }
                } else {
                    self.points_service
                        .revoke_points_by_credit_type(
                            realm_id,
                            user_id,
                            bucket_id,
                            CreditType::SubscriptionCredit,
                            RevocationType::CancelRevoke,
                            "Immediate subscription cancellation".to_string(),
                        )
                        .await?
                };

                tracing::info!(
                    realm_id = %realm_id,
                    user_id = %user_id,
                    entitlement_key = ?entitlement_key,
                    total_revoked = output.total_revoked,
                    ledger_count = output.ledger_ids.len(),
                    "Subscription cancelled immediately - revoked unused subscription credits"
                );

                Ok(output)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::billing::entities::{BillingType, EntitlementMapping};

    fn mapping_fixture(points: Option<i64>, grant_on_subscribe: bool) -> EntitlementMapping {
        let now = crate::common::entities::now_utc();
        EntitlementMapping {
            id: Uuid::now_v7(),
            realm_id: "test-realm".to_string(),
            payment_provider: "stripe".to_string(),
            external_product_id: "prod_test".to_string(),
            external_price_id: None,
            bucket_id: Uuid::now_v7(),
            entitlement_key: "ent_test".to_string(),
            billing_type: Some(BillingType::Recurring),
            billing_period: Some("monthly".to_string()),
            points_per_period: points,
            grant_period_type: Some("monthly".to_string()),
            validity_days: Some(30),
            grant_on_subscribe,
            max_periods: None,
            enabled: true,
            provider_product_info: None,
            synced_at: Some(now),
            created_at: now,
            updated_at: now,
        }
    }

    // --- resolve_entitlement_points: boundaries ---

    #[test]
    fn resolve_entitlement_points_returns_some_when_positive() {
        let m = mapping_fixture(Some(500), true);
        assert_eq!(resolve_entitlement_points(&m), Some(500));
    }

    #[test]
    fn resolve_entitlement_points_returns_none_when_zero_or_negative() {
        let m_zero = mapping_fixture(Some(0), true);
        assert_eq!(resolve_entitlement_points(&m_zero), None);

        let m_neg = mapping_fixture(Some(-10), true);
        assert_eq!(resolve_entitlement_points(&m_neg), None);
    }

    #[test]
    fn resolve_entitlement_points_returns_none_when_unconfigured() {
        let m = mapping_fixture(None, true);
        assert_eq!(resolve_entitlement_points(&m), None);
    }

    // --- derive_period_number: cross-natural-month boundary ---

    #[test]
    fn derive_period_number_first_period_anchors_to_one() {
        // Jan 1 00:00 UTC anchor; same instant is period 1.
        let anchor = DateTime::<Utc>::from_naive_utc_and_offset(
            chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            Utc,
        );
        let n = derive_period_number(anchor, anchor, GrantPeriodType::Monthly).unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn derive_period_number_monthly_advances_on_calendar_months() {
        // Anchor Jan 1; each subsequent natural-month start is a new period.
        let anchor = DateTime::<Utc>::from_naive_utc_and_offset(
            chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            Utc,
        );
        let feb1 = DateTime::<Utc>::from_naive_utc_and_offset(
            chrono::NaiveDate::from_ymd_opt(2026, 2, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            Utc,
        );
        let mar1 = DateTime::<Utc>::from_naive_utc_and_offset(
            chrono::NaiveDate::from_ymd_opt(2026, 3, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            Utc,
        );

        assert_eq!(
            derive_period_number(anchor, feb1, GrantPeriodType::Monthly).unwrap(),
            2
        );
        assert_eq!(
            derive_period_number(anchor, mar1, GrantPeriodType::Monthly).unwrap(),
            3
        );
    }

    #[test]
    fn derive_period_number_monthly_handles_short_months_and_day_of_month_anchor() {
        // Anchor Jan 31 (day-of-month > shortest months). Calendar-month
        // arithmetic saturates Feb → Feb 28 (non-leap 2026). The next period
        // boundary is Feb 28; Mar 28 is the one after that.
        let anchor = DateTime::<Utc>::from_naive_utc_and_offset(
            chrono::NaiveDate::from_ymd_opt(2026, 1, 31)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            Utc,
        );
        let feb28 = DateTime::<Utc>::from_naive_utc_and_offset(
            chrono::NaiveDate::from_ymd_opt(2026, 2, 28)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            Utc,
        );
        let mar28 = DateTime::<Utc>::from_naive_utc_and_offset(
            chrono::NaiveDate::from_ymd_opt(2026, 3, 28)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            Utc,
        );

        // Same anchor day-of-month: period 2 at the saturated Feb 28.
        assert_eq!(
            derive_period_number(anchor, feb28, GrantPeriodType::Monthly).unwrap(),
            2
        );
        // Day BEFORE the Mar 28 boundary still belongs to period 2.
        let mar27 = DateTime::<Utc>::from_naive_utc_and_offset(
            chrono::NaiveDate::from_ymd_opt(2026, 3, 27)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            Utc,
        );
        assert_eq!(
            derive_period_number(anchor, mar27, GrantPeriodType::Monthly).unwrap(),
            2
        );
        // Mar 28 = period 3 boundary.
        assert_eq!(
            derive_period_number(anchor, mar28, GrantPeriodType::Monthly).unwrap(),
            3
        );
    }

    #[test]
    fn derive_period_number_daily_and_weekly_floor_to_periods() {
        let anchor = DateTime::<Utc>::from_naive_utc_and_offset(
            chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            Utc,
        );
        let day1 = anchor + Duration::days(1);
        let day3 = anchor + Duration::days(3);
        let week1 = anchor + Duration::weeks(1);

        assert_eq!(
            derive_period_number(anchor, day1, GrantPeriodType::Daily).unwrap(),
            2
        );
        assert_eq!(
            derive_period_number(anchor, day3, GrantPeriodType::Daily).unwrap(),
            4
        );
        assert_eq!(
            derive_period_number(anchor, week1, GrantPeriodType::Weekly).unwrap(),
            2
        );
    }

    #[test]
    fn derive_period_number_rejects_period_before_anchor() {
        let anchor = DateTime::<Utc>::from_naive_utc_and_offset(
            chrono::NaiveDate::from_ymd_opt(2026, 2, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            Utc,
        );
        let before = DateTime::<Utc>::from_naive_utc_and_offset(
            chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            Utc,
        );
        let err = derive_period_number(anchor, before, GrantPeriodType::Monthly).unwrap_err();
        // Clock skew / out-of-order webhook must surface loudly, not clamp.
        assert!(matches!(err, CoreError::BadRequest(_)), "got {:?}", err);
    }

    // --- estimate_next_period_end / advance_one_period ---

    #[test]
    fn estimate_next_period_end_monthly_uses_calendar_months() {
        let jan31 = DateTime::<Utc>::from_naive_utc_and_offset(
            chrono::NaiveDate::from_ymd_opt(2026, 1, 31)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            Utc,
        );
        let est = estimate_next_period_end(jan31, GrantPeriodType::Monthly).unwrap();
        // Saturates to Feb 28 in non-leap 2026.
        assert_eq!(est.format("%Y-%m-%d").to_string(), "2026-02-28");
    }

    #[test]
    fn advance_one_period_once_errors() {
        let now = crate::common::entities::now_utc();
        assert!(advance_one_period(now, GrantPeriodType::Once).is_err());
    }
}
