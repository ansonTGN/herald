use chrono::{DateTime, Utc};
use std::sync::Arc;
use uuid::Uuid;

use crate::billing::entities::EntitlementMapping;
use crate::common::entities::app_errors::CoreError;
use crate::common::entities::{generate_uuid_v7, now_utc};
use crate::points::{
    PointsQuotaEntitlement,
    dtos::RevokePointsOutput,
    entities::{
        CreditSourceType, CreditType, PointsCreditLedger, QuotaEntitlementStatus, QuotaSourceType,
        QuotaWindow, RevocationType,
    },
    ports::PointsRepository,
    service::PointsService,
};

const ERROR_ENTITLEMENT_NO_GRANT: &str = "Entitlement does not grant points on subscribe";

/// Cancellation mode for subscriptions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelMode {
    /// Cancel at period end - entitlement stays active until `effective_until`
    /// (no-op here; the entitlement's `effective_until` already encodes the
    /// period end from the grant).
    DefaultCancel,
    /// Cancel immediately - revoke the subscription's active quota entitlement
    /// (consumed amounts NOT reverse-adjusted).
    ImmediateCancel,
}

/// Subscription Service for handling subscription lifecycle events
///
/// This service manages subscription upgrades, downgrades, and cancellations
/// via grant/revoke of window-quota entitlements. It works with
/// the PointsService and Repository.
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

    /// Grant a window-quota entitlement for a subscription / free-periodic
    /// credit. Replaces per-period ledger issuance for the window credit types:
    /// availability is computed from the consume stream and this entitlement's
    /// effective interval, not from a pre-granted ledger row.
    ///
    /// Delegates to `repository.grant_quota_entitlement_atomic`, which is
    /// idempotent on the entitlement's `idempotency_key`
    /// (`UNIQUE(realm_id, user_id, bucket_id, credit_type, idempotency_key)`):
    /// a replayed grant (e.g. redelivered webhook event) returns the
    /// pre-existing entitlement row without re-writing.
    ///
    /// # Idempotency-key semantics
    /// `idempotency_key` MUST be a stable per-grant anchor so a redelivered
    /// webhook event converges on the same entitlement:
    /// - Subscription initial / renewal: the subscription period anchor
    ///   (e.g. `sub:{subscription_id}:period:{period_number}` or the provider
    ///   webhook `event_id`).
    /// - Upgrade: `sub_upgrade:{subscription_id}:{period_end}` so the new-tier
    ///   grant does NOT collide with the revoked old-tier entitlement, and a
    ///   replayed upgrade webhook for the same period converges on the same
    ///   entitlement row.
    /// - Free periodic grant: the registration / schedule anchor.
    ///
    /// `quota_windows` is snapshotted at grant time: later config edits to the
    /// mapping / realm default do not affect this active entitlement.
    pub async fn grant_quota_entitlement(
        &self,
        realm_id: &str,
        user_id: Uuid,
        bucket_id: Uuid,
        credit_type: CreditType,
        source_type: QuotaSourceType,
        source_id: String,
        quota_windows: Vec<QuotaWindow>,
        effective_from: DateTime<Utc>,
        effective_until: Option<DateTime<Utc>>,
        idempotency_key: String,
    ) -> Result<PointsQuotaEntitlement, CoreError> {
        let now = now_utc();
        let entitlement = PointsQuotaEntitlement {
            id: generate_uuid_v7(),
            user_id,
            realm_id: realm_id.to_string(),
            bucket_id,
            credit_type,
            source_type,
            source_id,
            quota_windows,
            effective_from,
            effective_until,
            status: QuotaEntitlementStatus::Active,
            idempotency_key,
            created_at: now,
            updated_at: now,
        };

        let granted = self
            .repo
            .grant_quota_entitlement_atomic(entitlement)
            .await?;

        tracing::info!(
            realm_id = %realm_id,
            user_id = %user_id,
            bucket_id = %bucket_id,
            credit_type = %credit_type.as_str(),
            source_type = %granted.source_type.as_str(),
            entitlement_id = %granted.id,
            "Quota entitlement granted (idempotent on idempotency_key)"
        );

        Ok(granted)
    }

    /// Revoke the active window-quota entitlement identified by
    /// `(realm_id, user_id, bucket_id, credit_type, source_id)`.
    ///
    /// Delegates to `repository.revoke_quota_entitlement_atomic`, which sets
    /// `status = 'revoked'` and `effective_until = revoke_at`. The revoke is
    /// idempotent across replayed webhook events: a no-match returns `Ok(())`
    /// (already-revoked or never-granted ⟹ no-op).
    ///
    /// # Consumed-amount-not-reversed
    /// Already-consumed usage is NOT reverse-adjusted: it ages out naturally
    /// as the sliding window advances. Revocation only ends the entitlement's
    /// effective interval, so window availability drops as the consume stream
    /// stops receiving new window-side deductions — past consumes remain in the
    /// window until they slide out.
    pub async fn revoke_quota_entitlement(
        &self,
        realm_id: &str,
        user_id: Uuid,
        bucket_id: Uuid,
        credit_type: CreditType,
        source_id: &str,
        revoke_at: DateTime<Utc>,
    ) -> Result<(), CoreError> {
        self.repo
            .revoke_quota_entitlement_atomic(
                realm_id,
                user_id,
                bucket_id,
                credit_type,
                source_id,
                revoke_at,
            )
            .await?;

        tracing::info!(
            realm_id = %realm_id,
            user_id = %user_id,
            bucket_id = %bucket_id,
            credit_type = %credit_type.as_str(),
            source_id = %source_id,
            revoke_at = %revoke_at,
            "Quota entitlement revoked (idempotent; consumed amounts not reverse-adjusted)"
        );

        Ok(())
    }

    /// Handle subscription upgrade
    ///
    /// Revokes the old entitlement's quota entitlement and grants a new one
    /// from the new (price-level) mapping's `quota_windows` snapshot. The new
    /// entitlement is effective over the recalculated billing period
    /// (`effective_from = now`, `effective_until = period_end`).
    ///
    /// Already-consumed usage under the old entitlement is NOT reverse-
    /// adjusted; it ages out via window slide.
    ///
    /// `subscription_id` is the entitlement `source_id` locator used by both
    /// the revoke (old entitlement) and the grant (new entitlement). It MUST
    /// match the `source_id` used at the initial/renewal grant so the revoke
    /// resolves the correct active entitlement.
    ///
    /// The new mapping's `quota_windows` is read via `resolve_quota_windows`.
    /// If the field is empty the grant is skipped with a `warn` (consistent
    /// with `handle_subscription_paid`).
    ///
    /// Returns a **non-persisted grant receipt** (`PointsCreditLedger`,
    /// amounts = 0, `id` = new entitlement id) — no ledger row is written for
    /// subscription credit. Production callers discard the `Ok` value; it
    /// exists only to keep the signature stable.
    pub async fn handle_subscription_upgrade(
        &self,
        user_id: Uuid,
        bucket_id: Uuid,
        realm_id: &str,
        subscription_id: Uuid,
        old_mapping: &EntitlementMapping,
        new_mapping: &EntitlementMapping,
        period_end: DateTime<Utc>,
    ) -> Result<PointsCreditLedger, CoreError> {
        let old_entitlement_key = old_mapping.entitlement_key.as_str();
        let new_entitlement_key = new_mapping.entitlement_key.as_str();
        let source_id = subscription_id.to_string();
        let now = now_utc();

        // Revoke the old entitlement. `source_id` = subscription_id matches
        // the initial/renewal grant's `source_id`, so this resolves the
        // currently-active subscription entitlement regardless of which
        // price-level mapping configured it. No-match ⟹ idempotent Ok(()).
        self.revoke_quota_entitlement(
            realm_id,
            user_id,
            bucket_id,
            CreditType::SubscriptionCredit,
            &source_id,
            now,
        )
        .await?;

        // Grant the new entitlement from the new mapping's quota_windows, or
        // fall back to legacy points_per_period ledger semantics for mappings
        // that have not moved to window quotas.
        let quota_windows = resolve_quota_windows(new_mapping);
        if quota_windows.is_empty() {
            let revoke_output = self
                .points_service
                .revoke_points_by_credit_type(
                    realm_id,
                    user_id,
                    bucket_id,
                    CreditType::SubscriptionCredit,
                    RevocationType::UpgradeRevoke,
                    "Subscription upgrade".to_string(),
                )
                .await?;
            let Some(points_per_period) = new_mapping.points_per_period else {
                return Ok(grant_receipt(
                    generate_uuid_v7(),
                    user_id,
                    realm_id,
                    bucket_id,
                    CreditType::SubscriptionCredit,
                    CreditSourceType::SubscriptionUpgrade,
                    &source_id,
                    now,
                    Some(period_end),
                ));
            };
            let ledger_id = self
                .points_service
                .grant_points_internal(
                    realm_id,
                    user_id,
                    bucket_id,
                    CreditType::SubscriptionCredit,
                    CreditSourceType::SubscriptionUpgrade,
                    points_per_period,
                    Some(period_end),
                    None,
                    Some(source_id.clone()),
                    Some(format!(
                        "Subscription upgrade: {} points granted after revoking {}",
                        points_per_period, revoke_output.total_revoked
                    )),
                    Some(format!(
                        "sub_upgrade:{}:{}",
                        subscription_id,
                        period_end.timestamp()
                    )),
                )
                .await?;
            return Ok(grant_receipt(
                ledger_id,
                user_id,
                realm_id,
                bucket_id,
                CreditType::SubscriptionCredit,
                CreditSourceType::SubscriptionUpgrade,
                &source_id,
                now,
                Some(period_end),
            ));
        }

        // Stable idempotency anchor for this upgrade: the subscription + the
        // new entitlement's period end. A replayed webhook event for the same
        // upgrade converges on the same entitlement row; a different upgrade
        // period naturally produces a different key.
        let idempotency_key = format!("sub_upgrade:{}:{}", subscription_id, period_end.timestamp());
        let granted = self
            .grant_quota_entitlement(
                realm_id,
                user_id,
                bucket_id,
                CreditType::SubscriptionCredit,
                QuotaSourceType::SubscriptionUpgrade,
                source_id,
                quota_windows,
                now,
                Some(period_end),
                idempotency_key,
            )
            .await?;

        tracing::info!(
            realm_id = %realm_id,
            user_id = %user_id,
            subscription_id = %subscription_id,
            old_entitlement_key = %old_entitlement_key,
            new_entitlement_key = %new_entitlement_key,
            new_entitlement_id = %granted.id,
            period_end = %period_end,
            "Subscription upgrade: revoked old entitlement and granted new quota entitlement (consumed amounts not reverse-adjusted)"
        );

        Ok(grant_receipt(
            granted.id,
            user_id,
            realm_id,
            bucket_id,
            CreditType::SubscriptionCredit,
            CreditSourceType::SubscriptionUpgrade,
            &granted.source_id,
            granted.effective_from,
            granted.effective_until,
        ))
    }

    /// Handle subscription downgrade
    ///
    /// Logs the downgrade event but does NOT revoke the active entitlement.
    /// The user keeps their current window quota until the entitlement's
    /// `effective_until`; the next renewal webhook grants a fresh entitlement
    /// from the new mapping's `quota_windows`.
    pub async fn handle_subscription_downgrade(
        &self,
        user_id: Uuid,
        subscription_id: Uuid,
        bucket_id: Uuid,
        realm_id: &str,
        old_mapping: &EntitlementMapping,
        new_mapping: &EntitlementMapping,
    ) -> Result<(), CoreError> {
        let old_entitlement_key = old_mapping.entitlement_key.as_str();
        let new_entitlement_key = new_mapping.entitlement_key.as_str();

        tracing::info!(
            realm_id = %realm_id,
            user_id = %user_id,
            subscription_id = %subscription_id,
            bucket_id = %bucket_id,
            old_entitlement_key = %old_entitlement_key,
            new_entitlement_key = %new_entitlement_key,
            "Subscription downgraded - active entitlement untouched; next renewal grants from the new mapping"
        );

        Ok(())
    }

    /// Handle subscription paid event (initial or renewal)
    ///
    /// Grants a window-quota entitlement for the subscription period.
    /// Availability is computed from the consume stream + the entitlement's
    /// effective interval `[effective_from, effective_until]` =
    /// `[period_start, period_end]`; no `points_credit_ledger` row is written
    /// for subscription credit and the next period is NOT pre-granted (grants
    /// happen lazily on each renewal webhook event).
    ///
    /// **Idempotency**: anchored to the subscription period via
    /// `idempotency_key = sub:{subscription_id}:period:{period_start}`.
    /// Redelivered webhook events for the same period converge on the same
    /// entitlement row (infra `UNIQUE` constraint).
    ///
    /// The mapping's `quota_windows` snapshot is read via `resolve_quota_windows`.
    /// If the snapshot is empty the grant is skipped with a `warn` (subscription
    /// credit uniformly uses the window model; empty `quota_windows` ⟹ no
    /// grant, mirroring `create_placeholder_transaction_with_ref`). When
    /// `grant_on_subscribe = false` the entitlement is ignored the same way.
    ///
    /// The grant routes to `subscription.bucket_id`, bound eagerly at
    /// subscription creation. The caller supplies the resolved `bucket_id`.
    ///
    /// Returns a **non-persisted grant receipt** (`PointsCreditLedger`,
    /// amounts = 0, `id` = entitlement id when granted) — no ledger row is
    /// written. Production callers discard the `Ok` value; it exists only to
    /// keep the signature stable.
    pub async fn handle_subscription_paid(
        &self,
        user_id: Uuid,
        subscription_id: Uuid,
        bucket_id: Uuid,
        realm_id: &str,
        mapping: &EntitlementMapping,
        is_renewal: bool,
        // Billing period start / end. Drive the entitlement's
        // `[effective_from, effective_until]`. Provider period
        // normalization is the upstream source; this method does not guess.
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
        event_id: String,
    ) -> Result<PointsCreditLedger, CoreError> {
        let entitlement_key = mapping.entitlement_key.as_str();
        let source_id = subscription_id.to_string();

        // `grant_on_subscribe=false` ⟹ graceful skip: an entitlement configured
        // not to grant on subscribe is ignored, not a data-integrity error.
        if !mapping.grant_on_subscribe {
            tracing::info!(
                realm_id = %realm_id,
                entitlement_key = %entitlement_key,
                "Entitlement does not grant points on subscribe, skipping"
            );
            return self
                .create_placeholder_transaction_with_ref(user_id, realm_id, &event_id)
                .await;
        }

        // Window-quota mappings grant quota entitlements. Mappings without
        // quota_windows keep the legacy points_per_period ledger semantics.
        let quota_windows = resolve_quota_windows(mapping);
        if quota_windows.is_empty() {
            let Some(points_per_period) = mapping.points_per_period else {
                return self
                    .create_placeholder_transaction_with_ref(user_id, realm_id, &event_id)
                    .await;
            };
            let source_type = if is_renewal {
                CreditSourceType::SubscriptionRenewal
            } else {
                CreditSourceType::SubscriptionInitial
            };
            let ledger_id = self
                .points_service
                .grant_points_internal(
                    realm_id,
                    user_id,
                    bucket_id,
                    CreditType::SubscriptionCredit,
                    source_type,
                    points_per_period,
                    Some(period_end),
                    None,
                    Some(source_id.clone()),
                    Some(format!(
                        "{}: {} points granted",
                        source_type.as_str(),
                        points_per_period
                    )),
                    Some(event_id.clone()),
                )
                .await?;
            return Ok(grant_receipt(
                ledger_id,
                user_id,
                realm_id,
                bucket_id,
                CreditType::SubscriptionCredit,
                source_type,
                &source_id,
                period_start,
                Some(period_end),
            ));
        }

        let source_type = if is_renewal {
            QuotaSourceType::SubscriptionRenewal
        } else {
            QuotaSourceType::SubscriptionInitial
        };

        // Idempotency key anchored to the subscription period so a redelivered
        // webhook event for the same period converges on the same entitlement.
        let idempotency_key = format!(
            "sub:{}:period:{}",
            subscription_id,
            period_start.timestamp()
        );

        let granted = self
            .grant_quota_entitlement(
                realm_id,
                user_id,
                bucket_id,
                CreditType::SubscriptionCredit,
                source_type,
                source_id.clone(),
                quota_windows,
                period_start,
                Some(period_end),
                idempotency_key,
            )
            .await?;

        tracing::info!(
            realm_id = %realm_id,
            user_id = %user_id,
            subscription_id = %subscription_id,
            bucket_id = %bucket_id,
            entitlement_key = %entitlement_key,
            is_renewal,
            entitlement_id = %granted.id,
            period_start = %period_start,
            period_end = %period_end,
            event_id = %event_id,
            "Subscription paid: quota entitlement granted (window model; no ledger row, no chained pre-grant)"
        );

        Ok(grant_receipt(
            granted.id,
            user_id,
            realm_id,
            bucket_id,
            CreditType::SubscriptionCredit,
            if is_renewal {
                CreditSourceType::SubscriptionRenewal
            } else {
                CreditSourceType::SubscriptionInitial
            },
            &granted.source_id,
            granted.effective_from,
            granted.effective_until,
        ))
    }

    /// Create placeholder transaction with external ref (for idempotency when
    /// grant_on_subscribe = false or quota_windows is empty). Records the
    /// event id under the historical idempotency namespace and returns the
    /// "no grant" error so the webhook layer can classify the graceful skip
    /// consistently.
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

    /// Handle subscription cancellation
    ///
    /// Two modes:
    /// - `DefaultCancel`: the entitlement's `effective_until` already encodes
    ///   the period end from the grant, so no write is needed — the
    ///   entitlement stays active until it expires naturally.
    /// - `ImmediateCancel`: revoke the subscription's active quota entitlement
    ///   (`source_id = subscription_id`). Already-consumed usage is NOT
    ///   reverse-adjusted.
    ///
    /// `subscription_id` is the entitlement `source_id` locator (REQUIRED for
    /// the revoke semantics). The `entitlement_key` parameter is retained on
    /// the signature for caller compatibility but is not used to locate the
    /// entitlement under the quota model.
    ///
    /// Returns a `RevokePointsOutput` carrier: `ledger_ids` is empty (no
    /// ledger rows are touched) and `total_revoked = 0` (consumed amounts are
    /// not reverse-adjusted). Callers read the revocation fact from the
    /// entitlement row, not from this carrier.
    pub async fn handle_subscription_cancel(
        &self,
        user_id: Uuid,
        bucket_id: Uuid,
        realm_id: &str,
        subscription_id: Uuid,
        cancel_mode: CancelMode,
        _period_end: Option<DateTime<Utc>>,
        _entitlement_key: Option<&str>,
    ) -> Result<RevokePointsOutput, CoreError> {
        let now = now_utc();
        let source_id = subscription_id.to_string();

        match cancel_mode {
            CancelMode::DefaultCancel => {
                // Entitlement `effective_until` already encodes the period end
                // from the grant — nothing to write. Window availability ages
                // out naturally at period end.
                tracing::info!(
                    realm_id = %realm_id,
                    user_id = %user_id,
                    subscription_id = %subscription_id,
                    bucket_id = %bucket_id,
                    "Subscription cancelled at period end - active entitlement expires naturally (effective_until encodes period end)"
                );
                Ok(RevokePointsOutput {
                    revocation_id: generate_uuid_v7(),
                    ledger_ids: Vec::new(),
                    total_revoked: 0,
                    revoked_at: now,
                })
            }
            CancelMode::ImmediateCancel => {
                self.revoke_quota_entitlement(
                    realm_id,
                    user_id,
                    bucket_id,
                    CreditType::SubscriptionCredit,
                    &source_id,
                    now,
                )
                .await?;

                let legacy_revoke = self
                    .points_service
                    .revoke_points_by_credit_type(
                        realm_id,
                        user_id,
                        bucket_id,
                        CreditType::SubscriptionCredit,
                        RevocationType::CancelRevoke,
                        "Subscription cancelled".to_string(),
                    )
                    .await?;

                tracing::info!(
                    realm_id = %realm_id,
                    user_id = %user_id,
                    subscription_id = %subscription_id,
                    bucket_id = %bucket_id,
                    revoked_at = %now,
                    "Subscription cancelled immediately - quota entitlement revoked (idempotent; consumed amounts not reverse-adjusted)"
                );

                Ok(legacy_revoke)
            }
        }
    }
}

/// Resolve the `quota_windows` snapshot from an entitlement mapping.
///
/// `None`/empty ⟹ empty vec (no window-model grant); the caller's grant path
/// treats an empty snapshot as "warn + skip grant" (subscription credit
/// uniformly uses the window model; there is no fallback to the removed
/// `points_per_period` ledger path). Callers do not change.
fn resolve_quota_windows(mapping: &EntitlementMapping) -> Vec<QuotaWindow> {
    mapping.quota_windows.clone().unwrap_or_default()
}

/// Build a **non-persisted grant receipt** shaped as `PointsCreditLedger` so
/// the subscription lifecycle methods keep their existing return type.
///
/// This is NOT a database row: no ledger row is written for subscription
/// credit under the window-quota model. The amounts are zeroed and the `id`
/// carries the granted entitlement's id (or a fresh uuid when no entitlement
/// was granted) so a caller inspecting the receipt can trace it. Production
/// webhook callers discard the `Ok` value.
fn grant_receipt(
    entitlement_id: Uuid,
    user_id: Uuid,
    realm_id: &str,
    bucket_id: Uuid,
    credit_type: CreditType,
    source_type: CreditSourceType,
    source_id: &str,
    effective_at: DateTime<Utc>,
    expires_at: Option<DateTime<Utc>>,
) -> PointsCreditLedger {
    use crate::points::entities::CreditLedgerStatus;
    PointsCreditLedger {
        id: entitlement_id,
        user_id,
        realm_id: realm_id.to_string(),
        bucket_id,
        credit_type,
        source_type,
        source_id: source_id.to_string(),
        // Zeroed: this is a receipt, not a persisted ledger row. The window-
        // quota entitlement is the source of truth for availability.
        granted_amount: 0,
        used_amount: 0,
        revoked_amount: 0,
        remaining_amount: 0,
        expires_at,
        effective_at: Some(effective_at),
        status: CreditLedgerStatus::Active,
        created_at: effective_at,
        updated_at: effective_at,
    }
}
