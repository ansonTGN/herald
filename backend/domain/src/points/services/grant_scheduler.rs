use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use crate::common::entities::app_errors::CoreError;
use crate::points::{
    CreditSourceType, CreditType, GrantSummary, PointsGrantRecord, PointsGrantSchedule,
    PointsPolicy, PointsRepository, PointsService, ProcessResult,
};

/// Grant Scheduler Service - Background job that grants points based on schedules
pub struct GrantScheduler<R, P>
where
    R: PointsRepository,
    P: PointsPolicy,
{
    repository: Arc<R>,
    points_service: Arc<PointsService<R, P>>,
}

impl<R, P> GrantScheduler<R, P>
where
    R: PointsRepository + Send + Sync,
    P: PointsPolicy,
{
    pub fn new(repository: Arc<R>, points_service: Arc<PointsService<R, P>>) -> Self {
        Self {
            repository,
            points_service,
        }
    }

    /// Process all due grant schedules
    ///
    /// This is called by the background worker (typically every hour)
    pub async fn process_due_schedules(&self) -> Result<GrantSummary, CoreError> {
        let now = Utc::now();
        tracing::info!("Processing grant schedules due before {}", now);

        // Find due schedules
        let schedules = self
            .repository
            .find_due_grant_schedules(now, 1000) // Process max 1000 at a time
            .await?;

        let mut summary = GrantSummary::default();

        for schedule in schedules {
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

        // Grant points
        let expires_at = schedule.calculate_next_expiration();
        self.grant_points_for_schedule(schedule, expires_at).await?;

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
            created_at: Utc::now(),
        };

        let _record = self.repository.create_grant_record(record).await?;

        tracing::info!(
            schedule_id = %schedule.id,
            user_id = %schedule.user_id,
            period = next_period,
            amount = schedule.points_per_period,
            next_grant_time = %next_grant_time,
            "Points granted successfully"
        );

        Ok(ProcessResult::Granted)
    }

    /// Grant points for a schedule
    async fn grant_points_for_schedule(
        &self,
        schedule: &PointsGrantSchedule,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<(), CoreError> {
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

        // Grant points using PointsService
        self.points_service
            .grant_points_internal(
                &schedule.realm_id,
                schedule.user_id,
                credit_type,
                source_type,
                schedule.points_per_period,
                expires_at,
                Some(schedule.id.to_string()),
            )
            .await?;

        Ok(())
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

    #[test]
    fn test_service_creation() {
        // This is a placeholder test
        // Real tests would need mock repository and policy
    }
}
