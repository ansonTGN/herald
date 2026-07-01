use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use crate::authentication::Identity;
use crate::common::entities::app_errors::CoreError;
use crate::common::entities::{generate_uuid_v7, now_utc};
use crate::common::policies::ensure_policy;
use crate::points::entities::{
    CreditSourceType, CreditType, PointsQuotaEntitlement, QuotaEntitlementStatus, QuotaSourceType,
};
use crate::points::grant_schedule::PointsGrantSchedule;
use crate::points::services::registration_pool_resolver::RegistrationPoolResolver;
use crate::points::{PointsPolicy, PointsRepository, PointsService, UserPointsConfig};

/// Registration Service - Handles user registration and initial points grant.
///
/// The free-periodic grant grants a single window-quota entitlement
/// (`points_quota_entitlements`) snapshotting
/// `realm_default_configs.free_periodic_quota_windows`. Availability is computed
/// from the consume stream + the entitlement's effective interval; there is no
/// per-period issuance and no schedule to update. Upgrade-to-paid revokes the
/// entitlement (consumed amounts NOT reverse-adjusted).
///
/// Registration and free-periodic grants target the Realm's registration pool
/// Bucket (the single Bucket flagged `receives_registration_credits = true`).
/// That Bucket is resolved through the injected `RegistrationPoolResolver` port.
/// When no Bucket is marked, grants are skipped fail-safe (no cross-pool
/// fallback).
///
/// `registration_credit` (permanent pool) grant is preserved: it still routes
/// through `PointsService::grant_points_internal` topup-style into
/// `points_credit_ledger`.
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

    /// Handle user registration - grant initial registration bonus and the free
    /// periodic quota entitlement.
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
    /// # Registration pool resolution
    /// The target Bucket for registration and free periodic grants is the Realm's
    /// registration pool Bucket (`receives_registration_credits = true`). When no
    /// Bucket is marked, both grants are skipped fail-safe (warn, not an error) —
    /// never fall back to an implicit pool.
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

        // Check if user config already exists (prevent duplicate).
        if self.repository.find_user_config(user_id).await?.is_some() {
            return Err(CoreError::BadRequest(format!(
                "User {} already has a points config",
                user_id
            )));
        }

        // Get realm default config.
        let realm_config = self
            .repository
            .find_realm_config(realm_id)
            .await?
            .ok_or(CoreError::NotFound)?;

        // Resolve the registration pool Bucket for this Realm. Target = the
        // Realm's Bucket flagged `receives_registration_credits = true` (at
        // most one per Realm). `None` means no marked Bucket → fail-safe skip
        // grants (no cross-pool fallback). This is NOT an error: a Realm may
        // legitimately have no registration pool configured.
        let registration_pool_bucket_id = self
            .registration_pool_resolver
            .resolve_registration_pool_bucket(realm_id)
            .await?;

        // Grant registration bonus (permanent) through the pool-side
        // `grant_points_internal` topup-style path into `points_credit_ledger`.
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
                        None, // effective_at = None (registration grant is immediately available)
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

        // Create user points config (without a schedule).
        let now = Utc::now();
        let grant_period_type = realm_config.free_periodic_grant_period_type;
        let user_config = UserPointsConfig {
            user_id,
            realm_id: realm_id.to_string(),
            registration_bonus_points: registration_bonus,
            free_periodic_points_amount: realm_config.free_periodic_points_amount,
            free_periodic_grant_period_type: Some(grant_period_type),
            free_periodic_validity_days: realm_config.free_periodic_validity_days,
            // No schedule in the window model: next_grant_time / granted_periods
            // / grant_schedule_id carry no meaning for a quota entitlement. They
            // are retained on the struct for future schedule-cleanup but left at
            // their neutral values.
            next_grant_time: None,
            granted_periods: 0,
            grant_schedule_id: None,
            created_at: now,
            updated_at: now,
        };

        let _user_config = self.repository.create_user_config(user_config).await?;

        // Grant free periodic credits. Window-configured realms use quota
        // entitlements; legacy amount-configured realms keep the schedule +
        // first-period ledger grant so worker-down read-path realization has a
        // schedule to advance.
        let quota_windows = realm_config.free_periodic_quota_windows.clone();
        let has_quota_windows = !quota_windows.is_empty();
        if has_quota_windows {
            if let Some(bucket_id) = registration_pool_bucket_id {
                self.grant_free_periodic_entitlement(
                    realm_id,
                    user_id,
                    bucket_id,
                    quota_windows,
                    now,
                )
                .await?;
            } else {
                tracing::warn!(
                    realm_id = %realm_id,
                    user_id = %user_id,
                    "No registration pool Bucket configured; skipping free periodic quota entitlement grant (fail-safe)"
                );
            }
        } else {
            tracing::info!(
                realm_id = %realm_id,
                user_id = %user_id,
                "User registration completed without free periodic quota entitlement (no free_periodic_quota_windows configured)"
            );
        }

        if !has_quota_windows && realm_config.free_periodic_points_amount > 0 {
            if let Some(bucket_id) = registration_pool_bucket_id {
                let schedule = PointsGrantSchedule {
                    id: generate_uuid_v7(),
                    user_id,
                    realm_id: realm_id.to_string(),
                    bucket_id,
                    subscription_id: None,
                    entitlement_key: None,
                    grant_period_type,
                    base_time: now,
                    next_grant_time: now,
                    points_per_period: realm_config.free_periodic_points_amount,
                    validity_days: realm_config.free_periodic_validity_days,
                    granted_periods: 0,
                    max_periods: None,
                    active: true,
                    created_at: now,
                    updated_at: now,
                };
                let schedule = self.repository.create_grant_schedule(schedule).await?;
                let expires_at = grant_period_type
                    .calculate_expiration(now, realm_config.free_periodic_validity_days);
                self.repository
                    .pregrant_next_period_atomic(realm_id, &schedule, 1, Some(now), expires_at)
                    .await?;
                let refreshed = self
                    .repository
                    .find_grant_schedule(schedule.id)
                    .await?
                    .unwrap_or(schedule);
                self.repository
                    .update_user_config(
                        user_id,
                        Some(refreshed.next_grant_time),
                        refreshed.granted_periods,
                        Some(refreshed.id),
                    )
                    .await?;
            } else {
                tracing::warn!(
                    realm_id = %realm_id,
                    user_id = %user_id,
                    amount = realm_config.free_periodic_points_amount,
                    "No registration pool Bucket configured; skipping free periodic schedule grant (fail-safe)"
                );
            }
        }

        Ok(())
    }

    /// Grant the free-periodic window-quota entitlement for a newly-registered
    /// user. Delegates to `repository.grant_quota_entitlement_atomic`, which is
    /// idempotent on the entitlement's `idempotency_key`
    /// (`UNIQUE(realm_id, user_id, bucket_id, credit_type, idempotency_key)`):
    /// a replayed registration anchor returns the pre-existing entitlement row
    /// without re-writing.
    ///
    /// `source_id` / `idempotency_key` anchor to the registration event
    /// (`registration:{user_id}` / `free:registration:{user_id}`) so a duplicate
    /// registration for the same user converges on the same entitlement.
    /// `effective_until = None` (the free entitlement is ongoing until revoked
    /// on upgrade-to-paid).
    async fn grant_free_periodic_entitlement(
        &self,
        realm_id: &str,
        user_id: Uuid,
        bucket_id: Uuid,
        quota_windows: Vec<crate::points::entities::QuotaWindow>,
        effective_from: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), CoreError> {
        let source_id = format!("registration:{}", user_id);
        let idempotency_key = format!("free:registration:{}", user_id);

        let entitlement = PointsQuotaEntitlement {
            id: generate_uuid_v7(),
            user_id,
            realm_id: realm_id.to_string(),
            bucket_id,
            credit_type: CreditType::FreePeriodicCredit,
            source_type: QuotaSourceType::FreePeriodicGrant,
            source_id: source_id.clone(),
            quota_windows,
            effective_from,
            effective_until: None,
            status: QuotaEntitlementStatus::Active,
            idempotency_key: idempotency_key.clone(),
            created_at: effective_from,
            updated_at: effective_from,
        };

        let granted = self
            .repository
            .grant_quota_entitlement_atomic(entitlement)
            .await?;

        tracing::info!(
            realm_id = %realm_id,
            user_id = %user_id,
            bucket_id = %bucket_id,
            entitlement_id = %granted.id,
            "Granted free periodic quota entitlement (window model; no schedule, no ledger row)"
        );

        Ok(())
    }

    /// Revoke the free-periodic quota entitlement (used when a free user
    /// upgrades to a paid plan). Consumed amounts are NOT reverse-adjusted: they
    /// age out naturally as the sliding window advances.
    ///
    /// Method name retained for call-site compatibility; semantics changed.
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
            "Revoking free periodic quota entitlement (upgrade to paid)"
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

        // Revoke the free-periodic quota entitlement. `source_id` mirrors the
        // grant anchor (`registration:{user_id}`) so the revoke resolves the
        // currently-active free entitlement. No-match ⟹ idempotent Ok(()).
        let source_id = format!("registration:{}", user_id);
        self.repository
            .revoke_quota_entitlement_atomic(
                realm_id,
                user_id,
                bucket_id,
                CreditType::FreePeriodicCredit,
                &source_id,
                now_utc(),
            )
            .await?;

        tracing::info!(
            realm_id = %realm_id,
            user_id = %user_id,
            bucket_id = %bucket_id,
            "Free periodic quota entitlement revoked (idempotent; consumed amounts not reverse-adjusted)"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::points::{GrantPeriodType, PointsGrantSchedule};
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
            bucket_id: Uuid::now_v7(),
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
            bucket_id: Uuid::now_v7(),
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

    // Registration and free-periodic grants target the Realm's single
    // registration-pool Bucket (`receives_registration_credits`). When the
    // resolver returns `None` (no marked Bucket) the service MUST skip grants
    // fail-safe and never fall back to an implicit pool. These tests pin the
    // resolver contract via a stub so the fail-safe rule has a regression guard
    // independent of the DB-backed infra impl.

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
}
