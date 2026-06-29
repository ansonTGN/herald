//! Points quota-entitlement expiry cleanup job (points-grant-redesign BE-D11).
//!
//! Under the window-quota model (design §3.1 / §4.1) there is no per-period
//! ledger pre-grant to warm — availability is a pure function of the consume
//! stream + each entitlement's effective interval. This job therefore no
//! longer does full-table pre-grant scheduling; it only sweeps
//! `points_quota_entitlements` rows whose `effective_until` has passed so the
//! active set stays small.
//!
//! This is NOT a correctness boundary: a lapsed-but-not-yet-swept entitlement
//! already contributes nothing to availability (the effective-interval
//! predicate excludes it at read time). The sweep is best-effort telemetry /
//! hygiene only.
//!
//! Legacy Cleanup Checklist (retired in BE-D11):
//! - the full-table pre-grant body (free-periodic `process_due_schedules` +
//!   subscription backstop `find_schedules_due_for_pregrant` /
//!   `pregrant_next_period_atomic`) — replaced by a single
//!   `GrantScheduler::expire_quota_entitlements` call.
//! - the subscription backstop scan + per-row `find_grant_record` idempotency
//!   re-check (no longer reachable; the port methods remain declared for API
//!   stability but have zero callers).

use anyhow::Result;
use herald_core::domain::points::GrantScheduler;
use herald_core::infrastructure::authorization::PermissionBasedPointsPolicy;
use herald_core::infrastructure::points::PostgresPointsRepository;
use std::sync::Arc;

/// Summary of one quota-expiry cleanup tick. `expired_count` mirrors
/// `GrantSummary::processed` (total rows swept across all drain batches). The
/// other `GrantSummary` fields (`skipped` / `failed` / `total_points_granted`)
/// have no meaning under the expiry model and are dropped here.
#[derive(Debug, Default, Clone)]
pub struct QuotaExpirationSummary {
    /// Total expired quota entitlements swept this tick.
    pub expired_count: u64,
}

/// Quota-entitlement expiry cleanup job. Concrete (non-generic) — mirrors how
/// `ExpirationService<PostgresPointsRepository>` is pinned in `WorkerConfig`.
pub struct PointsQuotaExpirationJob {
    grant_scheduler: Arc<GrantScheduler<PostgresPointsRepository, PermissionBasedPointsPolicy>>,
}

impl PointsQuotaExpirationJob {
    pub fn new(
        grant_scheduler: Arc<GrantScheduler<PostgresPointsRepository, PermissionBasedPointsPolicy>>,
    ) -> Self {
        Self { grant_scheduler }
    }

    #[tracing::instrument(
        // Governance: root span — no inbound request context.
        // `self` holds the GrantScheduler (repository handle).
        // Only the low-cardinality job name is recorded.
        skip(self),
        fields(job.name = "points_quota_expiration")
    )]
    pub async fn run(&self) -> Result<QuotaExpirationSummary> {
        let summary = self
            .grant_scheduler
            .expire_quota_entitlements()
            .await
            .map_err(|e| anyhow::anyhow!("expire_quota_entitlements failed: {}", e))?;

        Ok(QuotaExpirationSummary {
            expired_count: summary.processed,
        })
    }
}
