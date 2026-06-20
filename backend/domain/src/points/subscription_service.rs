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
    },
    ports::PointsRepository,
    service::PointsService,
};

const IDEMPOTENCY_KEY_SUBSCRIPTION_PAID: &str = "sub_paid";
const ERROR_ENTITLEMENT_NO_GRANT: &str = "Entitlement does not grant points on subscribe";

/// Resolve the routing Bucket for a subscription lifecycle event.
///
/// Per design §5.5 / A8: every subscription grant/revoke routes to
/// `subscription.bucket_id`. A subscription without a bound Bucket is a
/// data-integrity error and must fail loud (`SubscriptionBucketNotResolved`)
/// rather than silently fall back to any implicit pool.
///
/// This is extracted as a pure helper so the fail-loud rule has a single,
/// directly testable source of truth shared by all subscription handlers.
pub fn resolve_subscription_bucket(
    subscription_id: Uuid,
    bucket_id: Option<Uuid>,
) -> Result<Uuid, CoreError> {
    bucket_id.ok_or(CoreError::SubscriptionBucketNotResolved { subscription_id })
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
    pub async fn handle_subscription_upgrade(
        &self,
        user_id: Uuid,
        bucket_id: Uuid,
        realm_id: &str,
        old_entitlement_key: &str,
        new_entitlement_key: &str,
        period_end: DateTime<Utc>,
    ) -> Result<PointsCreditLedger, CoreError> {
        let _old_mapping = self
            .repo
            .find_points_policy_by_entitlement_key(realm_id, old_entitlement_key)
            .await?
            .ok_or(CoreError::EntitlementMappingNotFound)?;

        let new_mapping = self
            .repo
            .find_points_policy_by_entitlement_key(realm_id, new_entitlement_key)
            .await?
            .ok_or(CoreError::EntitlementMappingNotFound)?;

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
    /// the same `subscription.bucket_id`.
    ///
    /// Per design §5.5 / A8: `bucket_id` is the resolved `subscription.bucket_id`.
    /// `None` means the subscription has no bound Bucket and must fail loud
    /// (`SubscriptionBucketNotResolved`).
    pub async fn handle_subscription_downgrade(
        &self,
        user_id: Uuid,
        subscription_id: Uuid,
        bucket_id: Option<Uuid>,
        realm_id: &str,
        old_entitlement_key: &str,
        new_entitlement_key: &str,
    ) -> Result<(), CoreError> {
        let bucket_id = resolve_subscription_bucket(subscription_id, bucket_id)?;

        // Validate that both entitlements exist
        let _old_mapping = self
            .repo
            .find_points_policy_by_entitlement_key(realm_id, old_entitlement_key)
            .await?
            .ok_or(CoreError::EntitlementMappingNotFound)?;

        let _new_mapping = self
            .repo
            .find_points_policy_by_entitlement_key(realm_id, new_entitlement_key)
            .await?
            .ok_or(CoreError::EntitlementMappingNotFound)?;

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
    /// The ledger will expire at the end of the billing period.
    ///
    /// Per design §5.5 / A8: the grant routes to `subscription.bucket_id`. The
    /// caller supplies the resolved subscription's `bucket_id`; `None` means the
    /// subscription has no bound Bucket and must fail loud
    /// (`SubscriptionBucketNotResolved`) rather than fall back to an implicit pool.
    pub async fn handle_subscription_paid(
        &self,
        user_id: Uuid,
        subscription_id: Uuid,
        bucket_id: Option<Uuid>,
        realm_id: &str,
        entitlement_key: &str,
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

        // Resolve the entitlement mapping FIRST. This preserves the domain
        // contract that an absent (or disabled) mapping raises
        // `EntitlementMappingNotFound` — which the Creem webhook handler
        // swallows for a graceful skip (no grant, no retry). Resolving the
        // bucket before the mapping would wrongly surface a missing-bucket
        // data-integrity error (`SubscriptionBucketNotResolved`) for an
        // entitlement that should simply be ignored. The fail-loud bucket
        // check (`mapping exists but no bucket`) runs below, right before the
        // grant, so `subscription_with_unresolved_bucket_fails_loud` still
        // holds.
        let mapping = self
            .repo
            .find_points_policy_by_entitlement_key(realm_id, entitlement_key)
            .await?
            .ok_or(CoreError::EntitlementMappingNotFound)?;

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

        let points_amount = match mapping.points_per_period {
            Some(amount) if amount > 0 => amount,
            _ => {
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
        // Mapping exists and wants to grant points — now resolve the routing
        // bucket fail-loud. A subscription without a bound Bucket is a
        // data-integrity error (`SubscriptionBucketNotResolved`); it must NOT
        // silently route to an implicit pool. This runs after the
        // mapping/skip checks above so an absent mapping still surfaces as
        // `EntitlementMappingNotFound` (graceful skip), not as a bucket error.
        let bucket_id = resolve_subscription_bucket(subscription_id, bucket_id)?;

        let source_type = if is_renewal {
            CreditSourceType::SubscriptionRenewal
        } else {
            CreditSourceType::SubscriptionInitial
        };

        // Route to `subscription.bucket_id` (design §5.5). The resolved bucket
        // (validated fail-loud above) is passed to the atomic write path so the
        // grant lands in the subscription's bound Bucket.
        let created_ledger = self
            .repo
            .handle_subscription_paid_atomic(
                realm_id,
                user_id,
                bucket_id,
                entitlement_key.to_string(),
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
            subscription_id = %subscription_id,
            bucket_id = %bucket_id,
            entitlement_key = %entitlement_key,
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

        Err(CoreError::BadRequest(
            ERROR_ENTITLEMENT_NO_GRANT.to_string(),
        ))
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
            bucket_id: None,
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

    /// A subscription without a bound Bucket must fail loud rather than route the
    /// grant/revoke to an implicit pool (design §5.5 / A8). This encodes WHY the
    /// rule matters: a None bucket is a data-integrity breach that must surface
    /// as `SubscriptionBucketNotResolved` (carrying the offending subscription_id),
    /// not be silently swallowed into the wrong pool.
    #[test]
    fn resolve_subscription_bucket_fails_loud_when_none() {
        let subscription_id = Uuid::now_v7();
        let err = resolve_subscription_bucket(subscription_id, None).unwrap_err();
        match err {
            CoreError::SubscriptionBucketNotResolved {
                subscription_id: sid,
            } => {
                assert_eq!(sid, subscription_id);
            }
            other => panic!("expected SubscriptionBucketNotResolved, got {:?}", other),
        }
    }

    #[test]
    fn resolve_subscription_bucket_passes_through_when_some() {
        let subscription_id = Uuid::now_v7();
        let bucket_id = Uuid::now_v7();
        let resolved = resolve_subscription_bucket(subscription_id, Some(bucket_id)).unwrap();
        assert_eq!(resolved, bucket_id);
    }
}
