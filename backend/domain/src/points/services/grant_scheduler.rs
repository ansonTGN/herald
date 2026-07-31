use std::sync::Arc;

use chrono::Utc;

use crate::common::entities::app_errors::CoreError;
use crate::points::{
    DistributionEvent, DistributionGrantResult, DistributionRuleOwner, DistributionRuleSelection,
    DistributionTrigger, GrantSummary, PointsPolicy, PointsRepository, PointsService,
    event_key_for_free_periodic,
};

const DEFAULT_BATCH_SIZE: usize = 1000;

pub struct GrantScheduler<R, P>
where
    R: PointsRepository,
    P: PointsPolicy,
{
    repository: Arc<R>,
    _points_service: Arc<PointsService<R, P>>,
    batch_size: usize,
}

impl<R, P> GrantScheduler<R, P>
where
    R: PointsRepository + Send + Sync,
    P: PointsPolicy,
{
    pub fn new(repository: Arc<R>, points_service: Arc<PointsService<R, P>>) -> Self {
        Self {
            repository,
            _points_service: points_service,
            batch_size: DEFAULT_BATCH_SIZE,
        }
    }

    /// Execute each due fixed free-periodic schedule through its bound rule.
    /// The schedule snapshots bucket, amount, validity and cadence; current
    /// owner rule selection is never repeated for later periods.
    pub async fn process_due_schedules(&self) -> Result<GrantSummary, CoreError> {
        let due = self
            .repository
            .find_due_grant_schedules(Utc::now(), self.batch_size as u64)
            .await?;
        let mut summary = GrantSummary::default();

        for schedule in due {
            let period_number = schedule.granted_periods + 1;
            let event_period = u32::try_from(period_number).map_err(|_| {
                CoreError::InternalServerError(format!(
                    "invalid free-periodic period number {period_number} for schedule {}",
                    schedule.id
                ))
            })?;
            let event_key = event_key_for_free_periodic(
                schedule.user_id,
                schedule.distribution_rule_id,
                event_period,
            );
            let event = DistributionEvent {
                realm_id: schedule.realm_id.clone(),
                user_id: schedule.user_id,
                owner: DistributionRuleOwner::RealmRegistration,
                trigger: DistributionTrigger::FreePeriodicGrant,
                event_key: event_key.clone(),
                source_id: event_key,
                effective_from: schedule.next_grant_time,
                effective_until: None,
            };

            match self
                .repository
                .execute_distribution_event_atomic(
                    event,
                    DistributionRuleSelection::ScheduledRule(schedule.distribution_rule_id),
                )
                .await
            {
                Ok(results) => {
                    summary.processed += 1;
                    summary.total_granted += results
                        .into_iter()
                        .map(|result| match result {
                            DistributionGrantResult::Fixed { amount, .. } => amount,
                            _ => 0,
                        })
                        .sum::<i64>();
                }
                Err(error) => {
                    summary.failed += 1;
                    tracing::error!(
                        schedule_id = %schedule.id,
                        rule_id = %schedule.distribution_rule_id,
                        %error,
                        "failed to execute free-periodic schedule"
                    );
                }
            }
        }

        Ok(summary)
    }

    pub async fn expire_quota_entitlements(&self) -> Result<GrantSummary, CoreError> {
        let now = Utc::now();
        let mut summary = GrantSummary::default();
        loop {
            let expired = self
                .repository
                .expire_quota_entitlements_batch(now, self.batch_size)
                .await?;
            summary.processed += u64::try_from(expired).unwrap_or(u64::MAX);
            if expired < self.batch_size {
                break;
            }
        }
        Ok(summary)
    }
}
