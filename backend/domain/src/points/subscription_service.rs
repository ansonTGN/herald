// Subscription Service - Handles subscription lifecycle events
//
// This service manages subscription upgrade, downgrade, and cancellation events
// from the billing system. It follows hexagonal architecture principles and
// uses repository ports directly to avoid circular dependencies.

use chrono::{DateTime, Utc};
use std::sync::Arc;
use uuid::Uuid;

use crate::common::entities::app_errors::CoreError;
use crate::points::{
    dtos::RevokePointsOutput,
    entities::{
        CreditLedgerStatus, CreditSourceType, CreditType, PointsCreditLedger, RevocationType,
        SafeArithmetics,
    },
    ports::PointsRepository,
    service::PointsService,
};

const IDEMPOTENCY_KEY_SUBSCRIPTION_PAID: &str = "sub_paid";
const ERROR_PLAN_NO_GRANT: &str = "Plan does not grant points on subscribe";

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
    /// Grants the difference in points between old and new plans.
    /// The difference points expire at the end of the current billing period.
    ///
    /// # Arguments
    /// * `user_id` - The user ID
    /// * `realm_id` - The realm ID
    /// * `old_plan_id` - The old plan ID
    /// * `new_plan_id` - The new plan ID
    /// * `period_end` - The end of the current billing period
    ///
    /// # Returns
    /// The created credit ledger
    pub async fn handle_subscription_upgrade(
        &self,
        user_id: Uuid,
        realm_id: &str,
        old_plan_id: Uuid,
        new_plan_id: Uuid,
        period_end: DateTime<Utc>,
    ) -> Result<PointsCreditLedger, CoreError> {
        // Query plan configs
        let old_plan = self
            .repo
            .find_plan_config(realm_id, old_plan_id)
            .await?
            .ok_or_else(|| CoreError::PlanNotFound {
                realm_id: realm_id.to_string(),
                plan_id: old_plan_id.to_string(),
            })?;

        let new_plan = self
            .repo
            .find_plan_config(realm_id, new_plan_id)
            .await?
            .ok_or_else(|| CoreError::PlanNotFound {
                realm_id: realm_id.to_string(),
                plan_id: new_plan_id.to_string(),
            })?;

        // Calculate difference using safe arithmetic
        // Using points_per_period as the new field name
        let difference = new_plan
            .points_per_period
            .safe_sub(old_plan.points_per_period)
            .map_err(|e| CoreError::BadRequest(format!("Invalid plan points: {}", e)))?;

        if difference <= 0 {
            return Err(CoreError::BadRequest(
                "New plan must have more points than old plan for upgrade".to_string(),
            ));
        }

        let created_ledger = self
            .repo
            .grant_points_atomic(
                realm_id,
                user_id,
                CreditType::SubscriptionCredit,
                CreditSourceType::SubscriptionUpgrade,
                difference,
                Some(period_end),
                Some(new_plan_id.to_string()),
            )
            .await?;

        tracing::info!(
            realm_id = %realm_id,
            user_id = %user_id,
            old_plan_id = %old_plan_id,
            new_plan_id = %new_plan_id,
            difference,
            period_end = %period_end,
            "Subscription upgrade: granted difference points"
        );

        Ok(created_ledger)
    }

    /// Handle subscription downgrade
    ///
    /// Logs the downgrade event but does NOT revoke any points.
    /// Users keep their existing points; future renewals will use the new plan.
    ///
    /// # Arguments
    /// * `user_id` - The user ID
    /// * `realm_id` - The realm ID
    /// * `old_plan_id` - The old plan ID
    /// * `new_plan_id` - The new plan ID
    pub async fn handle_subscription_downgrade(
        &self,
        user_id: Uuid,
        realm_id: &str,
        old_plan_id: Uuid,
        new_plan_id: Uuid,
    ) -> Result<(), CoreError> {
        // Query plan configs to validate
        let _old_plan = self
            .repo
            .find_plan_config(realm_id, old_plan_id)
            .await?
            .ok_or_else(|| CoreError::PlanNotFound {
                realm_id: realm_id.to_string(),
                plan_id: old_plan_id.to_string(),
            })?;

        let _new_plan = self
            .repo
            .find_plan_config(realm_id, new_plan_id)
            .await?
            .ok_or_else(|| CoreError::PlanNotFound {
                realm_id: realm_id.to_string(),
                plan_id: new_plan_id.to_string(),
            })?;

        // Log downgrade event - no points are revoked
        tracing::info!(
            realm_id = %realm_id,
            user_id = %user_id,
            old_plan_id = %old_plan_id,
            new_plan_id = %new_plan_id,
            "Subscription downgraded - no points revoked, future renewals will use new plan"
        );

        Ok(())
    }

    /// Handle subscription paid event (initial or renewal)
    ///
    /// Creates a credit ledger for subscription points grant.
    /// The ledger will expire at the end of the billing period.
    ///
    /// # Arguments
    /// * `user_id` - The user ID
    /// * `realm_id` - The realm ID
    /// * `plan_id` - The plan ID
    /// * `is_renewal` - Whether this is a renewal event
    /// * `period_end` - The end of the billing period
    /// * `event_id` - The webhook event ID for tracking
    ///
    /// # Returns
    /// The created credit ledger
    pub async fn handle_subscription_paid(
        &self,
        user_id: Uuid,
        realm_id: &str,
        plan_id: Uuid,
        is_renewal: bool,
        period_end: DateTime<Utc>,
        event_id: String,
    ) -> Result<PointsCreditLedger, CoreError> {
        let idempotency_key = format!("{}:{}", IDEMPOTENCY_KEY_SUBSCRIPTION_PAID, event_id);

        if self
            .repo
            .check_idempotency_key(realm_id, &idempotency_key)
            .await?
            .is_some()
        {
            tracing::info!(
                realm_id = %realm_id,
                user_id = %user_id,
                event_id = %event_id,
                "Event already processed, returning cached result"
            );
            return self.create_placeholder_ledger(user_id, realm_id).await;
        }

        let plan_config = self
            .repo
            .find_plan_config(realm_id, plan_id)
            .await?
            .ok_or_else(|| CoreError::PlanNotFound {
                realm_id: realm_id.to_string(),
                plan_id: plan_id.to_string(),
            })?;

        if !plan_config.grant_on_subscribe {
            tracing::info!(
                realm_id = %realm_id,
                plan_id = %plan_id,
                "Plan does not grant points on subscribe, skipping"
            );
            return self
                .create_placeholder_transaction_with_ref(user_id, realm_id, &idempotency_key)
                .await;
        }

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

        let (points_amount, source_type) = if is_renewal {
            (
                plan_config.points_per_period,
                CreditSourceType::SubscriptionRenewal,
            )
        } else {
            (
                plan_config.points_per_period,
                CreditSourceType::SubscriptionInitial,
            )
        };

        let created_ledger = self
            .repo
            .handle_subscription_paid_atomic(
                realm_id,
                user_id,
                plan_id,
                points_amount,
                source_type,
                period_end,
                idempotency_key.clone(),
                disable_daily_grant,
            )
            .await?;

        tracing::info!(
            realm_id = %realm_id,
            user_id = %user_id,
            plan_id = %plan_id,
            is_renewal,
            points_amount,
            period_end = %period_end,
            event_id = %event_id,
            "Subscription paid: credit ledger created"
        );

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

        Err(CoreError::BadRequest(ERROR_PLAN_NO_GRANT.to_string()))
    }

    async fn create_placeholder_ledger(
        &self,
        user_id: Uuid,
        realm_id: &str,
    ) -> Result<PointsCreditLedger, CoreError> {
        let now = crate::common::entities::now_utc();
        Ok(PointsCreditLedger {
            id: crate::common::entities::generate_uuid_v7(),
            user_id,
            realm_id: realm_id.to_string(),
            credit_type: CreditType::SubscriptionCredit,
            source_type: CreditSourceType::SubscriptionInitial,
            source_id: "idempotency".to_string(),
            granted_amount: 0,
            used_amount: 0,
            revoked_amount: 0,
            remaining_amount: 0,
            expires_at: None,
            status: CreditLedgerStatus::Active,
            created_at: now,
            updated_at: now,
        })
    }

    /// Handle subscription cancellation
    ///
    /// Two modes:
    /// - DefaultCancel: Set expiration on existing subscription credits to period_end
    /// - ImmediateCancel: Revoke all unused subscription credits immediately
    ///
    /// # Arguments
    /// * `user_id` - The user ID
    /// * `realm_id` - The realm ID
    /// * `cancel_mode` - The cancellation mode
    /// * `period_end` - Optional period end timestamp for DefaultCancel
    ///
    /// # Returns
    /// Revocation output with details of revoked points
    pub async fn handle_subscription_cancel(
        &self,
        user_id: Uuid,
        realm_id: &str,
        cancel_mode: CancelMode,
        period_end: Option<DateTime<Utc>>,
    ) -> Result<RevokePointsOutput, CoreError> {
        match cancel_mode {
            CancelMode::DefaultCancel => {
                // Set expiration on all active subscription credits
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
                    total_revoked: 0, // No immediate revocation
                    revoked_at: Utc::now(),
                })
            }
            CancelMode::ImmediateCancel => {
                // Revoke all unused subscription credits immediately
                let output = self
                    .points_service
                    .revoke_points_by_credit_type(
                        realm_id,
                        user_id,
                        CreditType::SubscriptionCredit,
                        RevocationType::CancelRevoke,
                        "Immediate subscription cancellation".to_string(),
                    )
                    .await?;

                tracing::info!(
                    realm_id = %realm_id,
                    user_id = %user_id,
                    total_revoked = output.total_revoked,
                    ledger_count = output.ledger_ids.len(),
                    "Subscription cancelled immediately - revoked all unused subscription credits"
                );

                Ok(output)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cancel_mode_equality() {
        assert_eq!(CancelMode::DefaultCancel, CancelMode::DefaultCancel);
        assert_eq!(CancelMode::ImmediateCancel, CancelMode::ImmediateCancel);
        assert_ne!(CancelMode::DefaultCancel, CancelMode::ImmediateCancel);
    }
}
