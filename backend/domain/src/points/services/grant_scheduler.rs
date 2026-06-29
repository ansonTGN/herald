// Grant Scheduler Service — points-grant-redesign (BE-D07)
// The full-table pre-grant path is GONE. Under the window-quota model
// (design §3.1 / §4.1) availability is a pure function of the consume
// stream + each entitlement's effective interval; there is no per-period
// ledger issuance to "pre-grant" and no `lead_time` window to filter by.
// The scheduler is now a thin wrapper over the repository's
// `expire_quota_entitlements_batch` sweep-expire port: it drains
// `points_quota_entitlements` rows whose `effective_until` has passed,
// in batches, until a batch returns fewer than `batch_size` rows (queue
// drained). This is NOT a correctness backstop — it only reaps already-
// lapsed rows so the active set stays small. Worker (BE-D11) calls this.
// Removed (Legacy Cleanup Checklist):
// - the per-period-type lead-time map field + its due-judgement / max/default
//   lead-time helpers (lead time is meaningless without per-period pre-grant).
// - the full-table pre-grant body (candidate scan + per-row lead-time filter +
//   per-row ledger grant + grant-record insert + schedule advance).
// - `GrantPeriodType` is KEPT (in `grant_schedule.rs`) for window-unit
//   derivation reuse — not deleted this item.

use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::common::entities::app_errors::CoreError;
use crate::points::{GrantSummary, PointsPolicy, PointsRepository, PointsService};

/// Grant Scheduler Service — now a quota-entitlement expiry cleanup job
/// (design §3.1: grant_scheduler full-table scan removed → expiry sweep).
///
/// The per-period-type pre-grant lead-time map is GONE (Legacy Cleanup
/// Checklist): the window model has no notion of a pre-grant lead time. The
/// constructor signature therefore drops the map parameter; BE-D11
/// (worker/app assembly) updates the call sites in `main.rs` / the worker job.
pub struct GrantScheduler<R, P>
where
    R: PointsRepository,
    P: PointsPolicy,
{
    repository: Arc<R>,
    // points-grant-redesign (BE-D07): `points_service` is retained on the
    // constructor signature (BE-D11 / callers still supply it) but is no
    // longer read — expiry cleanup never grants ledger rows. Underscore-
    // prefixed to match the `_grant_scheduler` convention used elsewhere and
    // keep the public constructor signature stable.
    _points_service: Arc<PointsService<R, P>>,
    /// Batch size for the sweep-expire drain loop. Defaults to 1000; BE-D11
    /// may override via env at construction time.
    batch_size: usize,
}

/// Default batch size for the expiry drain loop.
const DEFAULT_EXPIRY_BATCH_SIZE: usize = 1000;

impl<R, P> GrantScheduler<R, P>
where
    R: PointsRepository + Send + Sync,
    P: PointsPolicy,
{
    /// Construct the scheduler. `points_service` is retained for signature
    /// stability but no longer read (no pre-grant path remains).
    pub fn new(repository: Arc<R>, points_service: Arc<PointsService<R, P>>) -> Self {
        Self {
            repository,
            _points_service: points_service,
            batch_size: DEFAULT_EXPIRY_BATCH_SIZE,
        }
    }

    /// Sweep-expire quota entitlements whose `effective_until` has passed
    /// (design §3.1 / §5.5: the scheduler is now an expiry-cleanup job, not a
    /// pre-grant job). Drains `points_quota_entitlements` in batches of
    /// `batch_size` by repeatedly calling
    /// `repository.expire_quota_entitlements_batch(now, batch_size)` until a
    /// batch returns fewer than `batch_size` rows (queue drained).
    ///
    /// This is NOT a correctness backstop: window availability is computed
    /// from the consume stream + each entitlement's effective interval, so a
    /// lapsed-but-not-yet-swept row already contributes nothing to
    /// availability. The sweep only keeps the active set small.
    ///
    /// Returns a `GrantSummary` for caller telemetry shape-compat
    /// (`processed` = total rows expired across all batches; `skipped` /
    /// `failed` / `total_points_granted` stay at their defaults — they have
    /// no meaning under the expiry model but are kept for the summary struct
    /// shape BE-D11 already consumes).
    pub async fn process_due_schedules(&self) -> Result<GrantSummary, CoreError> {
        let summary = self.expire_quota_entitlements().await?;

        tracing::info!(
            expired_count = summary.processed,
            batch_size = self.batch_size,
            "Quota entitlement expiry cleanup completed"
        );

        Ok(summary)
    }

    /// Drain expired quota entitlements in batches until the queue is empty.
    /// Exposed separately so BE-D11 can name the intent at the call site
    /// (the public entry stays `process_due_schedules` for minimal worker
    /// churn; both are equivalent).
    pub async fn expire_quota_entitlements(&self) -> Result<GrantSummary, CoreError> {
        let now = Utc::now();
        let mut summary = GrantSummary::default();

        loop {
            let expired = self
                .repository
                .expire_quota_entitlements_batch(now, self.batch_size)
                .await?;
            summary.processed += u64::try_from(expired).unwrap_or(u64::MAX);

            // A sub-full batch means the queue is drained; stop looping.
            if expired < self.batch_size {
                break;
            }
        }

        Ok(summary)
    }

    /// No-op wrapper kept for the pre-redesign "free user upgrades to paid"
    /// call shape. The free periodic credit is now a quota entitlement
    /// (design §5.4): revocation on upgrade is `revoke_quota_entitlement`
    /// on `SubscriptionService` / `RegistrationService`, NOT a schedule
    /// disable. There are currently no callers (Legacy Cleanup Checklist:
    /// `disable_daily_grant_schedule` had no external callers after the
    /// registration path moved to grant/revoke); the wrapper is retained so
    /// a missed BE-D10/BE-D11 webhook call site still compiles instead of
    /// silently breaking, and logs the no-op so the gap is visible.
    ///
    /// BE-D10/D11 callers SHOULD switch to `revoke_quota_entitlement`
    /// (CreditType::FreePeriodicCredit) on the registration/subscription
    /// service instead of this method.
    pub async fn disable_daily_grant_schedule(
        &self,
        realm_id: &str,
        user_id: Uuid,
        _idempotency_key: Option<String>,
    ) -> Result<(), CoreError> {
        tracing::info!(
            realm_id = %realm_id,
            user_id = %user_id,
            "disable_daily_grant_schedule is a no-op under the window-quota model; revoke the free-periodic quota entitlement via revoke_quota_entitlement instead"
        );
        Ok(())
    }
}
