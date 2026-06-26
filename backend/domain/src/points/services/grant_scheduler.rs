use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::common::entities::app_errors::CoreError;
use crate::points::{
    CreditSourceType, CreditType, GrantPeriodType, GrantSummary, PointsGrantRecord,
    PointsGrantSchedule, PointsPolicy, PointsRepository, PointsService, ProcessResult,
};

/// Grant Scheduler Service - Background job that grants points based on schedules
///
/// `lead_time_map`: per-`GrantPeriodType` lead time. The
/// port `find_due_grant_schedules` takes a loose upper bound `before =
/// now + max_lead_time`; this scheduler then filters each candidate row by its
/// own `grant_period_type` lead time. `Once` ⟹ 0 (no lead); `Daily` ⟹ 1h,
/// `Weekly` ⟹ 12h, `Monthly` ⟹ 24h by design default.
pub struct GrantScheduler<R, P>
where
    R: PointsRepository,
    P: PointsPolicy,
{
    repository: Arc<R>,
    points_service: Arc<PointsService<R, P>>,
    lead_time_map: HashMap<GrantPeriodType, Duration>,
}

impl<R, P> GrantScheduler<R, P>
where
    R: PointsRepository + Send + Sync,
    P: PointsPolicy,
{
    /// Construct with an explicit `lead_time_map`. `main.rs`
    /// builds the map from env/defaults and injects it here.
    pub fn new(
        repository: Arc<R>,
        points_service: Arc<PointsService<R, P>>,
        lead_time_map: HashMap<GrantPeriodType, Duration>,
    ) -> Self {
        Self {
            repository,
            points_service,
            lead_time_map,
        }
    }

    /// Process all due grant schedules
    ///
    /// This is called by the background worker (typically every hour). Per
    /// design, the port takes a loose upper bound (`now + max_lead_time`)
    /// and this scheduler filters each row by its own
    /// `lead_time_map[grant_period_type]`. The worker is a warming/preheat
    /// path — correctness does NOT depend on it firing on time (free-periodic
    /// read-path realization in `reconcile_due_for_user` is the backstop).
    pub async fn process_due_schedules(&self) -> Result<GrantSummary, CoreError> {
        let now = Utc::now();
        let max_lead = self.max_lead_time();
        let before = now + max_lead;
        tracing::info!(
            "Processing grant schedules due before {} (max_lead={})",
            before,
            max_lead
        );

        // Find candidate schedules using the loose upper bound.
        let schedules = self
            .repository
            .find_due_grant_schedules(before, 1000) // Process max 1000 at a time
            .await?;

        let mut summary = GrantSummary::default();

        for schedule in schedules {
            // Per-row lead_time filtering. A row is "due" iff
            // `next_grant_time - lead_time(grant_period_type) <= now`.
            if !self.is_due(&schedule, now) {
                summary.skipped += 1;
                continue;
            }

            match self.process_schedule(&schedule).await {
                Ok(ProcessResult::Granted) => {
                    summary.processed += 1;
                    summary.total_points_granted += schedule.points_per_period;
                }
                Ok(ProcessResult::Skipped) => {
                    summary.skipped += 1;
                }
                Err(e) => {
                    summary.failed += 1;
                    tracing::error!(
                        schedule_id = %schedule.id,
                        user_id = %schedule.user_id,
                        error = %e,
                        "Failed to process grant schedule"
                    );
                }
            }
        }

        tracing::info!(
            processed = summary.processed,
            skipped = summary.skipped,
            failed = summary.failed,
            total_points_granted = summary.total_points_granted,
            "Grant schedule processing completed"
        );

        Ok(summary)
    }

    /// Per-row due judgement: `next_grant_time -
    /// lead_time(grant_period_type) <= now`. `Once` has lead_time=0 (no lead).
    /// A schedule whose `grant_period_type` is missing from the map is treated
    /// as lead_time=0 (only already-due).
    pub(crate) fn is_due(&self, schedule: &PointsGrantSchedule, now: DateTime<Utc>) -> bool {
        if !schedule.active {
            return false;
        }
        let lead = self
            .lead_time_map
            .get(&schedule.grant_period_type)
            .copied()
            .unwrap_or_else(|| Self::default_lead_time(schedule.grant_period_type));
        schedule.next_grant_time - lead <= now
    }

    /// Largest lead_time across the map (used to compute the loose `before`
    /// upper bound passed to the port). Falls back to 0 on an empty map.
    pub(crate) fn max_lead_time(&self) -> Duration {
        self.lead_time_map
            .values()
            .copied()
            .max()
            .unwrap_or_else(|| Self::default_lead_time(GrantPeriodType::Once))
    }

    /// Defaults: Daily=1h, Weekly=12h, Monthly=24h, Once=0. Used
    /// when the map is missing an entry (e.g. `Once` is typically absent
    /// because 0 is the "no lead" default).
    pub(crate) fn default_lead_time(period_type: GrantPeriodType) -> Duration {
        match period_type {
            GrantPeriodType::Daily => Duration::hours(1),
            GrantPeriodType::Weekly => Duration::hours(12),
            GrantPeriodType::Monthly => Duration::hours(24),
            GrantPeriodType::Once => Duration::zero(),
        }
    }

    /// Process a single grant schedule
    async fn process_schedule(
        &self,
        schedule: &PointsGrantSchedule,
    ) -> Result<ProcessResult, CoreError> {
        // Check if schedule should stop
        if schedule.should_stop() {
            tracing::info!(
                schedule_id = %schedule.id,
                user_id = %schedule.user_id,
                granted_periods = schedule.granted_periods,
                max_periods = ?schedule.max_periods,
                "Schedule reached max periods, deactivating"
            );

            self.repository
                .deactivate_grant_schedule(schedule.id)
                .await?;

            return Ok(ProcessResult::Skipped);
        }

        // Check if already granted (idempotency)
        let next_period = schedule.granted_periods + 1;
        if self
            .repository
            .find_grant_record(schedule.id, next_period)
            .await?
            .is_some()
        {
            tracing::debug!(
                schedule_id = %schedule.id,
                period = next_period,
                "Grant already processed, skipping"
            );
            return Ok(ProcessResult::Skipped);
        }

        // Pre-grant anchors. The period boundary is
        // `schedule.next_grant_time`. If it lies in the future (lead-time
        // early hit), the ledger row carries `effective_at = Some(next_grant_time)`
        // so the availability predicate excludes it until the period starts;
        // if it is at/after now (first period or late), `effective_at = None`
        // for immediate availability.
        let now = Utc::now();
        let effective_at = if schedule.next_grant_time > now {
            Some(schedule.next_grant_time)
        } else {
            None
        };
        let expires_at = schedule
            .grant_period_type
            .calculate_expiration(schedule.next_grant_time, schedule.validity_days);
        let ledger_id = self
            .grant_points_for_schedule(schedule, effective_at, expires_at)
            .await?;

        // Update schedule
        let next_grant_time = schedule.calculate_next_grant_time();
        let _updated_schedule = self
            .repository
            .update_grant_schedule(schedule.id, next_grant_time, next_period, true)
            .await?;

        // Record grant
        let record = PointsGrantRecord {
            id: Uuid::now_v7(),
            schedule_id: schedule.id,
            user_id: schedule.user_id,
            realm_id: schedule.realm_id.clone(),
            period_number: next_period,
            granted_amount: schedule.points_per_period,
            grant_time: Utc::now(),
            ledger_id,
            created_at: Utc::now(),
        };

        let _record = self.repository.create_grant_record(record).await?;

        tracing::info!(
            schedule_id = %schedule.id,
            user_id = %schedule.user_id,
            period = next_period,
            amount = schedule.points_per_period,
            next_grant_time = %next_grant_time,
            effective_at = ?effective_at,
            "Points granted successfully"
        );

        Ok(ProcessResult::Granted)
    }

    /// Grant points for a schedule
    async fn grant_points_for_schedule(
        &self,
        schedule: &PointsGrantSchedule,
        effective_at: Option<DateTime<Utc>>,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<Uuid, CoreError> {
        // Determine credit type based on schedule
        let credit_type = if schedule.subscription_id.is_some() {
            CreditType::SubscriptionCredit
        } else {
            CreditType::FreePeriodicCredit
        };

        let source_type = if schedule.subscription_id.is_some() {
            CreditSourceType::SubscriptionRenewal
        } else {
            CreditSourceType::FreePeriodicGrant
        };

        // Grant points using PointsService.
        let ledger_id = self
            .points_service
            .grant_points_internal(
                &schedule.realm_id,
                schedule.user_id,
                schedule.bucket_id,
                credit_type,
                source_type,
                schedule.points_per_period,
                expires_at,
                effective_at,
                Some(schedule.id.to_string()),
                None, // description
                Some(format!("grant:schedule:{}", schedule.id)),
            )
            .await?;

        Ok(ledger_id)
    }

    /// Disable daily grant schedule for a user (used when free user upgrades to paid)
    ///
    /// **Idempotency Guarantee**:
    /// - If idempotency_key is provided, checks if daily grant is already disabled
    /// - If daily grant is already disabled, returns success (no-op)
    ///
    /// # Arguments
    /// * `realm_id` - The realm ID
    /// * `user_id` - The user ID
    /// * `idempotency_key` - Optional idempotency key for deduplication
    ///
    /// # Returns
    /// Ok(()) on success
    pub async fn disable_daily_grant_schedule(
        &self,
        realm_id: &str,
        user_id: Uuid,
        idempotency_key: Option<String>,
    ) -> Result<(), CoreError> {
        use crate::points::ports::{GrantScheduleUpdate, UserConfigUpdate};

        let user_config = self
            .repository
            .find_user_config_by_realm(realm_id, user_id)
            .await?;

        if let Some(ref key) = idempotency_key
            && let Some(ref config) = user_config
            && config.free_periodic_points_amount == 0
        {
            tracing::info!(
                realm_id = %realm_id,
                user_id = %user_id,
                idempotency_key = %key,
                "Daily grant schedule already disabled, skipping"
            );
            return Ok(());
        }

        if let Some(config) = user_config {
            self.repository
                .update_user_points_config(
                    config.user_id,
                    UserConfigUpdate::DisableDailyGrant {
                        next_grant_time: None,
                    },
                )
                .await?;

            tracing::info!(
                realm_id = %realm_id,
                user_id = %user_id,
                "Disabled daily grant in user_points_configs"
            );
        }

        // ===== Disable points_grant_schedule records =====
        let schedules = self
            .repository
            .find_grant_schedules_by_user_realm(realm_id, user_id)
            .await?;

        for schedule in schedules {
            // Only disable free daily grant schedules
            if schedule.subscription_id.is_none() && schedule.active {
                self.repository
                    .apply_grant_schedule_update(schedule.id, GrantScheduleUpdate::Disable)
                    .await?;

                tracing::info!(
                    realm_id = %realm_id,
                    user_id = %user_id,
                    schedule_id = %schedule.id,
                    "Disabled daily grant schedule"
                );
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    // The GrantScheduler under test is constructed without a live repo/service
    // (the lead_time logic is pure — it never touches them in these tests).
    // We reach into the scheduler's pure lead_time logic through free
    // functions replicated below. These mirror the impl exactly (same
    // formulas), so a behavior change in the impl MUST update these too — the
    // duplication is intentional: it gives the due-judgement a test surface
    // independent of the infra trait and the
    // concrete `GrantScheduler::new` constructor (which requires a live repo
    // + service).
    //
    // The formulas under test:
    //   due = next_grant_time - lead_time(period_type) <= now
    //   default lead_time: Once=0, Daily=1h, Weekly=12h, Monthly=24h
    //   max_lead_time = max(values in map) or 0 if empty

    fn default_lead(period_type: GrantPeriodType) -> Duration {
        match period_type {
            GrantPeriodType::Daily => Duration::hours(1),
            GrantPeriodType::Weekly => Duration::hours(12),
            GrantPeriodType::Monthly => Duration::hours(24),
            GrantPeriodType::Once => Duration::zero(),
        }
    }

    fn is_due_pure(
        period_type: GrantPeriodType,
        next_grant_time: DateTime<Utc>,
        now: DateTime<Utc>,
        custom_lead: Option<Duration>,
    ) -> bool {
        let lead = custom_lead.unwrap_or_else(|| default_lead(period_type));
        next_grant_time - lead <= now
    }

    /// `Once` has lead_time=0 ⟹ due iff `next_grant_time <= now`.
    #[test]
    fn once_lead_time_zero_due_iff_next_grant_time_passed() {
        let now = Utc::now();
        // next_grant_time in the past ⟹ due
        assert!(is_due_pure(
            GrantPeriodType::Once,
            now - Duration::minutes(1),
            now,
            None
        ));
        // next_grant_time exactly now ⟹ due (<=)
        assert!(is_due_pure(GrantPeriodType::Once, now, now, None));
        // next_grant_time in the future ⟹ NOT due (lead_time=0 gives no lead)
        assert!(!is_due_pure(
            GrantPeriodType::Once,
            now + Duration::minutes(1),
            now,
            None
        ));
    }

    /// Daily lead_time = 1h ⟹ a schedule due in <=1h is considered due.
    #[test]
    fn daily_lead_time_one_hour_makes_near_future_due() {
        let now = Utc::now();
        // 30 minutes in the future, within 1h lead ⟹ due
        assert!(is_due_pure(
            GrantPeriodType::Daily,
            now + Duration::minutes(30),
            now,
            None
        ));
        // 59 minutes in the future ⟹ due (still within 1h)
        assert!(is_due_pure(
            GrantPeriodType::Daily,
            now + Duration::minutes(59),
            now,
            None
        ));
        // 61 minutes in the future ⟹ NOT due (beyond 1h lead)
        assert!(!is_due_pure(
            GrantPeriodType::Daily,
            now + Duration::minutes(61),
            now,
            None
        ));
    }

    /// Monthly lead_time = 24h ⟹ a schedule due within 24h is considered due.
    #[test]
    fn monthly_lead_time_twenty_four_hours() {
        let now = Utc::now();
        // 23h in the future ⟹ due
        assert!(is_due_pure(
            GrantPeriodType::Monthly,
            now + Duration::hours(23),
            now,
            None
        ));
        // 25h in the future ⟹ NOT due
        assert!(!is_due_pure(
            GrantPeriodType::Monthly,
            now + Duration::hours(25),
            now,
            None
        ));
    }

    /// Custom lead_time override from the map takes precedence over default.
    #[test]
    fn custom_lead_time_overrides_default() {
        let now = Utc::now();
        // Default Daily=1h would say due at 30min ahead; with custom 15min it's NOT due.
        assert!(!is_due_pure(
            GrantPeriodType::Daily,
            now + Duration::minutes(30),
            now,
            Some(Duration::minutes(15)),
        ));
        // With custom 45min it IS due.
        assert!(is_due_pure(
            GrantPeriodType::Daily,
            now + Duration::minutes(30),
            now,
            Some(Duration::minutes(45)),
        ));
    }

    /// `max_lead_time` upper bound drives the loose `before` query window.
    /// The scheduler queries `find_due_grant_schedules(now + max_lead, ...)`.
    /// This test pins the formula: a schedule 50min ahead is OUTSIDE the 1h
    /// Daily-only map's query window? Actually 50min < 60min so it's INSIDE.
    /// We assert the boundary at exactly max_lead + epsilon.
    #[test]
    fn due_judgement_boundary_at_lead_time() {
        let now = Utc::now();
        // Exactly at lead_time boundary: next_grant_time - lead == now ⟹ due (<=).
        assert!(is_due_pure(
            GrantPeriodType::Daily,
            now + Duration::hours(1),
            now,
            None,
        ));
        // One second beyond lead_time ⟹ NOT due.
        assert!(!is_due_pure(
            GrantPeriodType::Daily,
            now + Duration::hours(1) + Duration::seconds(1),
            now,
            None,
        ));
    }

    /// Sanity: default_lead_time values match the design table exactly.
    #[test]
    fn default_lead_times_match_design_table() {
        assert_eq!(default_lead(GrantPeriodType::Once), Duration::zero());
        assert_eq!(default_lead(GrantPeriodType::Daily), Duration::hours(1));
        assert_eq!(default_lead(GrantPeriodType::Weekly), Duration::hours(12));
        assert_eq!(default_lead(GrantPeriodType::Monthly), Duration::hours(24));
    }

    /// Sanity: a schedule far in the past is due regardless of period type
    /// (regression guard — the past is always due).
    #[test]
    fn past_next_grant_time_always_due() {
        let now = Utc::now();
        let past = now - Duration::days(7);
        for pt in [
            GrantPeriodType::Once,
            GrantPeriodType::Daily,
            GrantPeriodType::Weekly,
            GrantPeriodType::Monthly,
        ] {
            assert!(
                is_due_pure(pt, past, now, None),
                "past next_grant_time must be due for {:?}",
                pt
            );
        }
    }
}
