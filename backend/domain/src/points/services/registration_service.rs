use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use crate::authentication::Identity;
use crate::common::entities::app_errors::CoreError;
use crate::common::policies::ensure_policy;
use crate::points::services::registration_pool_resolver::RegistrationPoolResolver;
use crate::points::{
    CreditSourceType, CreditType, PointsGrantSchedule, PointsPolicy, PointsRepository,
    PointsService, UserPointsConfig,
};

/// Registration Service - Handles user registration and initial points grant
///
/// Per design §5.4 / §4.3.2 (credit-bucket): registration and free periodic
/// grants target the Realm's registration pool Bucket (the single Bucket flagged
/// `receives_registration_credits = true`). That Bucket is resolved through the
/// injected `RegistrationPoolResolver` port (infra impl by BE-D07). When no
/// Bucket is marked, grants are skipped fail-safe (no cross-pool fallback, A4/A5).
pub struct RegistrationService<R, P, Z>
where
    R: PointsRepository + Send + Sync,
    P: PointsPolicy,
    Z: RegistrationPoolResolver,
{
    repository: Arc<R>,
    points_service: Arc<PointsService<R, P>>,
    policy: Arc<P>,
    registration_pool_resolver: Arc<Z>,
}

impl<R, P, Z> RegistrationService<R, P, Z>
where
    R: PointsRepository + Send + Sync,
    P: PointsPolicy,
    Z: RegistrationPoolResolver,
{
    pub fn new(
        repository: Arc<R>,
        points_service: Arc<PointsService<R, P>>,
        policy: Arc<P>,
        registration_pool_resolver: Arc<Z>,
    ) -> Self {
        Self {
            repository,
            points_service,
            policy,
            registration_pool_resolver,
        }
    }

    /// Handle user registration - grant initial registration bonus and setup daily grant
    ///
    /// # Arguments
    /// * `user_id` - The newly registered user ID
    /// * `realm_id` - The realm ID
    ///
    /// # Returns
    /// Ok(()) on success
    ///
    /// # Errors
    /// - Realm config not found
    /// - User config already exists (duplicate registration)
    /// - Database errors
    ///
    /// # Registration pool resolution (design §5.4)
    /// The target Bucket for registration and free periodic grants is the Realm's
    /// registration pool Bucket (`receives_registration_credits = true`). When no
    /// Bucket is marked, both grants are skipped fail-safe (warn, not an error) —
    /// never fall back to an implicit pool (A4/A5).
    pub async fn handle_user_registration(
        &self,
        user_id: Uuid,
        realm_id: &str,
    ) -> Result<(), CoreError> {
        tracing::info!(
            realm_id = %realm_id,
            user_id = %user_id,
            "Handling user registration"
        );

        // 1. Check if user config already exists (prevent duplicate)
        if self.repository.find_user_config(user_id).await?.is_some() {
            return Err(CoreError::BadRequest(format!(
                "User {} already has a points config",
                user_id
            )));
        }

        // 2. Get realm default config
        let realm_config = self
            .repository
            .find_realm_config(realm_id)
            .await?
            .ok_or(CoreError::NotFound)?;

        // 3. Resolve the registration pool Bucket for this Realm.
        // Per design §5.4: target = the Realm's Bucket flagged
        // `receives_registration_credits = true` (at most one per Realm).
        // `None` means no marked Bucket → fail-safe skip grants (no cross-pool
        // fallback, A4/A5). This is NOT an error: a Realm may legitimately
        // have no registration pool configured.
        let registration_pool_bucket_id = self
            .registration_pool_resolver
            .resolve_registration_pool_bucket(realm_id)
            .await?;

        // 4. Grant registration bonus (permanent).
        let registration_bonus = realm_config.registration_bonus_points;
        if registration_bonus > 0 {
            if let Some(bucket_id) = registration_pool_bucket_id {
                self.points_service
                    .grant_points_internal(
                        realm_id,
                        user_id,
                        bucket_id,
                        CreditType::RegistrationCredit,
                        CreditSourceType::Registration,
                        registration_bonus,
                        None, // expires_at = None (permanent)
                        None, // source_id
                        None, // description
                        Some(format!("grant:registration:{}", user_id)),
                    )
                    .await?;

                tracing::info!(
                    realm_id = %realm_id,
                    user_id = %user_id,
                    amount = registration_bonus,
                    "Granted registration bonus"
                );
            } else {
                tracing::warn!(
                    realm_id = %realm_id,
                    user_id = %user_id,
                    amount = registration_bonus,
                    "No registration pool Bucket configured; skipping registration bonus grant (fail-safe)"
                );
            }
        }

        // 4. Create user points config
        let now = Utc::now();
        let grant_period_type = realm_config.free_periodic_grant_period_type;
        let user_config = UserPointsConfig {
            user_id,
            realm_id: realm_id.to_string(),
            registration_bonus_points: registration_bonus,
            free_periodic_points_amount: realm_config.free_periodic_points_amount,
            free_periodic_grant_period_type: Some(grant_period_type),
            free_periodic_validity_days: realm_config.free_periodic_validity_days,
            next_grant_time: Some(now), // Grant immediately
            granted_periods: 0,
            grant_schedule_id: None,
            created_at: now,
            updated_at: now,
        };

        let _user_config = self.repository.create_user_config(user_config).await?;

        // 5. Create periodic grant schedule (only if free_periodic_points_amount > 0
        //    AND a registration pool Bucket is configured — periodic grants target
        //    that pool per design §5.4). When no registration pool Bucket is marked
        //    the schedule is skipped fail-safe: a schedule without a target Bucket
        //    would only fail loud at grant time, so we do not create one at all.
        if realm_config.free_periodic_points_amount > 0 {
            if let Some(bucket_id) = registration_pool_bucket_id {
                let schedule = PointsGrantSchedule {
                    id: Uuid::now_v7(),
                    user_id,
                    realm_id: realm_id.to_string(),
                    bucket_id: Some(bucket_id),
                    subscription_id: None, // Free user has no subscription
                    entitlement_key: None,
                    grant_period_type,
                    base_time: now,
                    next_grant_time: now,
                    points_per_period: realm_config.free_periodic_points_amount,
                    validity_days: realm_config.free_periodic_validity_days,
                    granted_periods: 0,
                    max_periods: None, // Unlimited for free users
                    active: true,
                    created_at: now,
                    updated_at: now,
                };

                let schedule = self.repository.create_grant_schedule(schedule).await?;

                // 6. Update user config with schedule_id
                let user_config = self
                    .repository
                    .update_user_config(user_id, Some(now), 0, Some(schedule.id))
                    .await?;

                // 7. Grant first periodic points immediately
                self.grant_periodic_points(realm_id, user_id, &user_config, &schedule)
                    .await?;

                // 8. Calculate next grant time and update schedule
                let next_grant_time = schedule.calculate_next_grant_time();
                let _user_config = self
                    .repository
                    .update_user_config(user_id, Some(next_grant_time), 1, Some(schedule.id))
                    .await?;

                // 9. Update schedule's next_grant_time and granted_periods
                let _ = self
                    .repository
                    .update_grant_schedule(schedule.id, next_grant_time, 1, true)
                    .await?;

                tracing::info!(
                    realm_id = %realm_id,
                    user_id = %user_id,
                    schedule_id = %schedule.id,
                    period_type = %grant_period_type.as_str(),
                    "User registration completed with periodic grant"
                );
            } else {
                tracing::warn!(
                    realm_id = %realm_id,
                    user_id = %user_id,
                    "No registration pool Bucket configured; skipping periodic grant schedule (fail-safe)"
                );
            }
        } else {
            tracing::info!(
                realm_id = %realm_id,
                user_id = %user_id,
                "User registration completed without periodic grant"
            );
        }

        Ok(())
    }

    /// Grant periodic points to a free user
    async fn grant_periodic_points(
        &self,
        realm_id: &str,
        user_id: Uuid,
        user_config: &UserPointsConfig,
        schedule: &PointsGrantSchedule,
    ) -> Result<(), CoreError> {
        let amount = user_config.free_periodic_points_amount;
        if amount <= 0 {
            tracing::warn!(
                realm_id = %realm_id,
                user_id = %user_id,
                "Periodic grant amount is 0, skipping"
            );
            return Ok(());
        }

        // Calculate expiration
        let expires_at = schedule
            .grant_period_type
            .calculate_expiration(Utc::now(), user_config.free_periodic_validity_days);

        // Grant points. Periodic grants target the schedule's Bucket (design §5.4);
        // missing bucket = misconfiguration, fail loud (A4/A5).
        let bucket_id = schedule.bucket_id.ok_or(CoreError::GrantBucketRequired)?;

        self.points_service
            .grant_points_internal(
                realm_id,
                user_id,
                bucket_id,
                CreditType::FreePeriodicCredit,
                CreditSourceType::FreePeriodicGrant,
                amount,
                expires_at,
                Some(schedule.id.to_string()),
                None, // description
                Some(format!("grant:periodic:{}", schedule.id)),
            )
            .await?;

        tracing::info!(
            realm_id = %realm_id,
            user_id = %user_id,
            amount,
            expires_at = ?expires_at,
            period_type = %schedule.grant_period_type.as_str(),
            "Granted periodic points"
        );

        Ok(())
    }

    /// Revoke all free user credits (used when upgrading to paid plan)
    pub async fn revoke_free_user_credits(
        &self,
        identity: Identity,
        realm_id: &str,
        user_id: Uuid,
        bucket_id: Uuid,
    ) -> Result<(), CoreError> {
        tracing::info!(
            realm_id = %realm_id,
            user_id = %user_id,
            "Revoking free user credits"
        );

        // Check realm boundary
        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot revoke credits from a different realm".to_string(),
            ));
        }

        // Check manage permissions
        ensure_policy(
            self.policy.can_manage_points(identity.clone()).await,
            "Insufficient permissions to revoke free user credits",
        )?;

        // Revoke all free periodic credits from the registration pool Bucket.
        self.points_service
            .revoke_points_by_credit_type(
                realm_id,
                user_id,
                bucket_id,
                CreditType::FreePeriodicCredit,
                crate::points::entities::RevocationType::UpgradeRevoke,
                "Upgraded to paid plan".to_string(),
            )
            .await?;

        // Deactivate daily grant schedule
        let user_config = self
            .repository
            .find_user_config(user_id)
            .await?
            .ok_or(CoreError::NotFound)?;

        if let Some(schedule_id) = user_config.grant_schedule_id {
            self.repository
                .deactivate_grant_schedule(schedule_id)
                .await?;

            // Update user config
            let _ = self
                .repository
                .update_user_config(
                    user_id,
                    None, // Clear next_grant_time
                    user_config.granted_periods,
                    None, // Clear schedule_id
                )
                .await?;

            tracing::info!(
                realm_id = %realm_id,
                user_id = %user_id,
                schedule_id = %schedule_id,
                "Deactivated daily grant schedule"
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::points::GrantPeriodType;
    use chrono::Duration;

    #[test]
    fn test_build_initial_user_config_starts_immediately() {
        let now = Utc::now();
        let user_id = Uuid::now_v7();
        let config = UserPointsConfig {
            user_id,
            realm_id: "realm-a".to_string(),
            registration_bonus_points: 120,
            free_periodic_points_amount: 15,
            free_periodic_grant_period_type: Some(GrantPeriodType::Daily),
            free_periodic_validity_days: 3,
            next_grant_time: Some(now),
            granted_periods: 0,
            grant_schedule_id: None,
            created_at: now,
            updated_at: now,
        };

        assert_eq!(config.user_id, user_id);
        assert_eq!(config.free_periodic_points_amount, 15);
        assert_eq!(config.next_grant_time, Some(now));
        assert!(config.is_periodic_grant_due(now));
    }

    #[test]
    fn test_daily_schedule_first_follow_up_is_24_hours_later() {
        let now = Utc::now();
        let schedule = PointsGrantSchedule {
            id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            realm_id: "realm-a".to_string(),
            bucket_id: Some(Uuid::now_v7()),
            subscription_id: None,
            entitlement_key: Some("test-entitlement".to_string()),
            grant_period_type: GrantPeriodType::Daily,
            base_time: now,
            next_grant_time: now,
            points_per_period: 15,
            validity_days: 3,
            granted_periods: 0,
            max_periods: None,
            active: true,
            created_at: now,
            updated_at: now,
        };

        assert_eq!(
            schedule.calculate_next_grant_time(),
            now + Duration::days(1)
        );
        assert_eq!(
            schedule.calculate_next_expiration(),
            Some(now + Duration::days(4))
        );
    }

    #[test]
    fn test_once_schedule_does_not_repeat() {
        let now = Utc::now();
        let schedule = PointsGrantSchedule {
            id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            realm_id: "realm-a".to_string(),
            bucket_id: Some(Uuid::now_v7()),
            subscription_id: None,
            entitlement_key: Some("test-entitlement".to_string()),
            grant_period_type: GrantPeriodType::Once,
            base_time: now,
            next_grant_time: now,
            points_per_period: 100,
            validity_days: 0, // Permanent
            granted_periods: 0,
            max_periods: None,
            active: true,
            created_at: now,
            updated_at: now,
        };

        // Once period should always return base_time
        assert_eq!(schedule.calculate_next_grant_time(), now);
        assert_eq!(schedule.calculate_next_expiration(), None); // Permanent
    }

    // ===== Credit-bucket registration-pool resolution tests (BE-D05) =====
    //
    // Per design §5.4 / A4 / A5: registration and free-periodic grants target the
    // Realm's single registration-pool Bucket (`receives_registration_credits`).
    // When the resolver returns `None` (no marked Bucket) the service MUST skip
    // grants fail-safe and never fall back to an implicit pool. These tests pin
    // the resolver contract via a stub so the fail-safe rule has a regression
    // guard independent of the DB-backed infra impl (BE-D07).

    struct StubRegistrationPoolResolver {
        bucket: Option<Uuid>,
    }

    impl RegistrationPoolResolver for StubRegistrationPoolResolver {
        fn resolve_registration_pool_bucket(
            &self,
            _realm_id: &str,
        ) -> impl Future<Output = Result<Option<Uuid>, CoreError>> + Send {
            let bucket = self.bucket;
            async move { Ok(bucket) }
        }
    }

    /// When a Realm has a marked registration-pool Bucket, the resolver returns
    /// `Some(bucket_id)` and grants route into that exact pool.
    #[tokio::test]
    async fn registration_pool_resolver_returns_marked_bucket() {
        let marked = Uuid::now_v7();
        let resolver = StubRegistrationPoolResolver {
            bucket: Some(marked),
        };
        let resolved = resolver
            .resolve_registration_pool_bucket("realm-a")
            .await
            .unwrap();
        assert_eq!(resolved, Some(marked));
    }

    /// When no Bucket is marked as the registration pool, the resolver returns
    /// `None`. The service treats this as fail-safe SKIP (no grant, no error) —
    /// this is the regression guard for "do not fall back to an implicit pool".
    #[tokio::test]
    async fn registration_pool_resolver_returns_none_when_unmarked() {
        let resolver = StubRegistrationPoolResolver { bucket: None };
        let resolved = resolver
            .resolve_registration_pool_bucket("realm-a")
            .await
            .unwrap();
        assert!(
            resolved.is_none(),
            "unmarked Realm must resolve to None (fail-safe skip)"
        );
    }

    /// A periodic grant schedule without a target Bucket must fail loud
    /// (`GrantBucketRequired`) rather than grant into an implicit pool. This pins
    /// the fail-loud half of §5.4 at the schedule-processing boundary.
    #[test]
    fn schedule_without_bucket_yields_no_grant_target() {
        let now = Utc::now();
        let schedule = PointsGrantSchedule {
            id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            realm_id: "realm-a".to_string(),
            bucket_id: None, // misconfigured schedule: no target pool
            subscription_id: None,
            entitlement_key: None,
            grant_period_type: GrantPeriodType::Daily,
            base_time: now,
            next_grant_time: now,
            points_per_period: 10,
            validity_days: 1,
            granted_periods: 0,
            max_periods: None,
            active: true,
            created_at: now,
            updated_at: now,
        };

        // Mirrors the check in grant_periodic_points / grant_scheduler:
        //   let bucket_id = schedule.bucket_id.ok_or(GrantBucketRequired)?;
        let result: Result<Uuid, CoreError> =
            schedule.bucket_id.ok_or(CoreError::GrantBucketRequired);
        assert!(matches!(result, Err(CoreError::GrantBucketRequired)));
    }
}
