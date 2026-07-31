//! Free-periodic fixed schedule execution and quota-entitlement expiry.

use anyhow::Result;
use herald_core::domain::points::GrantScheduler;
use herald_core::infrastructure::authorization::PermissionBasedPointsPolicy;
use herald_core::infrastructure::points::PostgresPointsRepository;
use std::sync::Arc;

/// Summary of one quota-expiry cleanup tick. `expired_count` mirrors
/// `GrantSummary::processed` (total rows swept across all drain batches). The
/// other `GrantSummary` fields (`skipped` / `failed` / `total_granted`)
/// have no meaning under the expiry model and are dropped here.
#[derive(Debug, Default, Clone)]
pub struct QuotaExpirationSummary {
    pub schedules_processed: u64,
    pub schedules_failed: u64,
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
        let schedules = self
            .grant_scheduler
            .process_due_schedules()
            .await
            .map_err(|e| anyhow::anyhow!("process_due_schedules failed: {}", e))?;
        let expired = self
            .grant_scheduler
            .expire_quota_entitlements()
            .await
            .map_err(|e| anyhow::anyhow!("expire_quota_entitlements failed: {}", e))?;

        Ok(QuotaExpirationSummary {
            schedules_processed: schedules.processed,
            schedules_failed: schedules.failed,
            expired_count: expired.processed,
        })
    }
}
