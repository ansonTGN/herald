//! Points pre-grant warming job (design §5.6).
//!
//! This job is a **pre-warming optimization, NOT a correctness boundary**
//! (design §5.5 / §5.6, decision A9). Correctness comes from:
//! - the availability predicate (`effective_at <= NOW()`) making pre-granted
//!   rows auto-available at period start, and
//! - the realization backstops: subscription chained pre-grant (BE-D03) and
//!   free-periodic read-path realization (`reconcile_due_for_user`, BE-D02).
//!
//! The job runs two belt-and-braces scans each tick:
//! 1. **Free-periodic** via `GrantScheduler::process_due_schedules`, which
//!    uses `find_due_grant_schedules` + per-row `lead_time_map` filtering.
//! 2. **Subscription backstop** via `find_schedules_due_for_pregrant` +
//!    `pregrant_next_period_atomic` (the atomic infra path). This catches
//!    subscription schedules whose chained pre-grant may have been missed.
//!
//! Both scans are best-effort: a single schedule's failure is logged and does
//! NOT abort the batch (fail-loud-per-row, not fail-loud-batch). Idempotency
//! is guaranteed downstream by `points_grant_records(schedule_id, period_number)`
//! UNIQUE + `pregrant_next_period_atomic`'s in-tx schedule lock.
//!
//! The repository and policy are pinned to the concrete
//! `PostgresPointsRepository` / `PermissionBasedPointsPolicy` (matching the
//! `WorkerConfig` shape where `expiration_service` is concrete). This avoids
//! widening `WorkerConfig<R>`'s generic bounds (BE-D07 step 5).

use anyhow::Result;
use chrono::Utc;
use herald_core::domain::points::{GrantScheduler, PointsGrantSchedule, PointsRepository};
use herald_core::infrastructure::authorization::PermissionBasedPointsPolicy;
use herald_core::infrastructure::points::PostgresPointsRepository;
use std::sync::Arc;

const DEFAULT_BATCH_SIZE: usize = 1000;

/// Summary of one pre-grant tick. Counts are best-effort; a non-zero
/// `subscription_failed` does not abort the tick.
#[derive(Debug, Default, Clone)]
pub struct PreGrantSummary {
    /// Schedules processed by `GrantScheduler` (free + any subscription it
    /// picks up via the non-atomic path).
    pub free_processed: u64,
    pub free_skipped: u64,
    pub free_failed: u64,
    /// Subscription schedules picked up by the backstop scan.
    pub subscription_attempted: u64,
    pub subscription_granted: u64,
    pub subscription_skipped: u64,
    pub subscription_failed: u64,
}

/// Pre-grant warming job. Concrete (non-generic) — mirrors how
/// `ExpirationService<PostgresPointsRepository>` is pinned in `WorkerConfig`.
pub struct PointsPreGrantJob {
    grant_scheduler: Arc<GrantScheduler<PostgresPointsRepository, PermissionBasedPointsPolicy>>,
    points_repo: Arc<PostgresPointsRepository>,
    batch_size: usize,
}

impl PointsPreGrantJob {
    pub fn new(
        grant_scheduler: Arc<GrantScheduler<PostgresPointsRepository, PermissionBasedPointsPolicy>>,
        points_repo: Arc<PostgresPointsRepository>,
    ) -> Self {
        let batch_size = std::env::var("WORKER_PRE_GRANT_BATCH_SIZE")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(DEFAULT_BATCH_SIZE);
        Self {
            grant_scheduler,
            points_repo,
            batch_size,
        }
    }

    pub async fn run(&self) -> Result<PreGrantSummary> {
        let mut summary = PreGrantSummary::default();

        // 1) Free-periodic warming via the domain GrantScheduler. This call
        //    also processes subscription schedules through the non-atomic
        //    `grant_points_internal` path; the counts here are lumped under
        //    `free_*` because the scheduler does not distinguish.
        match self.grant_scheduler.process_due_schedules().await {
            Ok(g) => {
                summary.free_processed = g.processed;
                summary.free_skipped = g.skipped;
                summary.free_failed = g.failed;
            }
            Err(e) => {
                // The scheduler already logs per-schedule failures; a
                // top-level error here is logged but does NOT abort the
                // backstop scan (pre-warming is best-effort, not correctness).
                tracing::error!(error = %e, "GrantScheduler::process_due_schedules failed");
            }
        }

        // 2) Subscription backstop scan. `find_schedules_due_for_pregrant`
        //    returns ALL active schedules whose next_grant_time is within the
        //    caller's lead window; we only act on subscription schedules here
        //    (free-periodic are already handled above) and re-check per-row
        //    absence via `find_grant_record` before the atomic grant.
        let now = Utc::now();
        // Loose upper bound: subscription pre-grant lead defaults to 24h
        // (design §5.5). The scan is belt-and-braces; over-scanning is safe
        // because `pregrant_next_period_atomic` is idempotent on
        // (schedule_id, period_number).
        let before = now + chrono::Duration::hours(subscription_pre_grant_lead_hours());

        let candidates = self
            .points_repo
            .find_schedules_due_for_pregrant(before, self.batch_size as u64)
            .await
            .map_err(|e| anyhow::anyhow!("find_schedules_due_for_pregrant failed: {}", e))?;

        for schedule in candidates {
            // Only subscription schedules; free-periodic handled by the
            // GrantScheduler arm above.
            if schedule.subscription_id.is_none() {
                continue;
            }
            summary.subscription_attempted += 1;

            match self.process_subscription_schedule(&schedule).await {
                Ok(SubscriptionOutcome::Granted) => summary.subscription_granted += 1,
                Ok(SubscriptionOutcome::Skipped) => summary.subscription_skipped += 1,
                Err(e) => {
                    summary.subscription_failed += 1;
                    tracing::error!(
                        schedule_id = %schedule.id,
                        user_id = %schedule.user_id,
                        subscription_id = ?schedule.subscription_id,
                        error = %e,
                        "Subscription pre-grant backstop failed for schedule"
                    );
                    // continue — one schedule's failure must not abort the batch.
                }
            }
        }

        Ok(summary)
    }

    /// Process a single subscription schedule through the atomic infra path.
    /// Mirrors the BE-D05 worker contract: period_number = granted_periods + 1,
    /// re-check grant-record absence, then `pregrant_next_period_atomic`.
    async fn process_subscription_schedule(
        &self,
        schedule: &PointsGrantSchedule,
    ) -> Result<SubscriptionOutcome> {
        // period_number mirrors GrantScheduler::process_schedule: the schedule
        // row's granted_periods is authoritative for the warming scan.
        let next_period = schedule.granted_periods + 1;
        let period_number = u32::try_from(next_period).map_err(|_| {
            anyhow::anyhow!(
                "schedule {} granted_periods {} overflowed u32",
                schedule.id,
                next_period
            )
        })?;

        // Idempotency re-check (pregrant_next_period_atomic is itself
        // idempotent, but this avoids a redundant tx for already-granted
        // periods and matches the BE-D05 worker contract).
        if self
            .points_repo
            .find_grant_record(schedule.id, next_period)
            .await
            .map_err(|e| anyhow::anyhow!("find_grant_record failed: {}", e))?
            .is_some()
        {
            return Ok(SubscriptionOutcome::Skipped);
        }

        let effective_at = if schedule.next_grant_time > Utc::now() {
            Some(schedule.next_grant_time)
        } else {
            None
        };
        let expires_at = schedule.calculate_next_expiration();

        self.points_repo
            .pregrant_next_period_atomic(
                &schedule.realm_id,
                schedule,
                period_number,
                effective_at,
                expires_at,
            )
            .await
            .map_err(|e| anyhow::anyhow!("pregrant_next_period_atomic failed: {}", e))?;

        tracing::info!(
            schedule_id = %schedule.id,
            user_id = %schedule.user_id,
            subscription_id = ?schedule.subscription_id,
            period_number,
            "Subscription pre-grant backstop granted"
        );

        Ok(SubscriptionOutcome::Granted)
    }
}

enum SubscriptionOutcome {
    Granted,
    Skipped,
}

/// Read the subscription pre-grant lead (design §5.5 default 24h) from env.
/// Free-periodic lead times are owned by the `GrantScheduler`'s `lead_time_map`
/// (injected at construction in `main.rs`); this is only for the subscription
/// backstop's loose scan window.
fn subscription_pre_grant_lead_hours() -> i64 {
    std::env::var("WORKER_SUBSCRIPTION_PRE_GRANT_LEAD_HOURS")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(24)
}
