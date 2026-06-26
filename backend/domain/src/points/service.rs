// Points Service - Business logic for points operations

use std::sync::Arc;
use uuid::Uuid;

use crate::authentication::Identity;
use crate::common::entities::app_errors::CoreError;
use crate::common::policies::ensure_policy;
use crate::points::{
    dtos::{ConsumePointsInput, GrantPointsInput, GrantPointsOutput, RevokePointsOutput},
    entities::{
        ConsumptionAllocationView, CreditSourceType, CreditType, Paginated, PointsBalance,
        PointsTransaction, PointsWallet, RechargeType, RevocationType, WalletStatus,
    },
    errors::PointsErrorExt,
    policies::PointsPolicy,
    ports::{PointsRepository, TransactionFilters, WalletFilters},
};

/// Points Service - Business logic for points management
///
/// Includes permission-based authorization checks using PointsPolicy
pub struct PointsService<R, P>
where
    R: PointsRepository,
    P: PointsPolicy,
{
    repository: Arc<R>,
    policy: Arc<P>,
}

impl<R, P> PointsService<R, P>
where
    R: PointsRepository + Send + Sync,
    P: PointsPolicy,
{
    pub fn new(repository: Arc<R>, policy: Arc<P>) -> Self {
        Self { repository, policy }
    }

    // ===== Account Management =====

    /// Get points wallet for a user
    pub async fn get_wallet(
        &self,
        identity: Identity,
        realm_id: &str,
        user_id: Uuid,
    ) -> Result<PointsWallet, CoreError> {
        // Check view permissions
        ensure_policy(
            self.policy
                .can_view_points(identity.clone(), Some(user_id))
                .await,
            "Insufficient permissions to view points wallet",
        )?;

        // Check realm boundary
        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot access points from a different realm".to_string(),
            ));
        }

        // Credit Buckets model: a wallet is per-(user,
        // bucket); a "get wallet for a user" is a user-total view (sum across
        // the user's per-bucket wallet rows; 0 if none). We must NOT auto-
        // create a `bucket_id = None` wallet row here — `points_wallets.
        // bucket_id` is NOT NULL, and wallets are created lazily only when a
        // grant/consume targets a specific bucket. So when no wallet row
        // exists we return a synthesized zero-balance view.
        match self.repository.find_by_user_id(realm_id, user_id).await? {
            Some(account) => Ok(account),
            None => {
                tracing::info!(
                    "No points wallet for user {}; returning zero-balance user-total view",
                    user_id
                );
                Ok(Self::synthesized_empty_wallet(realm_id, user_id))
            }
        }
    }

    /// Build a zero-balance user-total wallet view (`bucket_id = None`) for a
    /// user who has no wallet row yet. Mirrors the aggregate shape returned by
    /// `find_by_user_id` for an empty user.
    fn synthesized_empty_wallet(realm_id: &str, user_id: Uuid) -> PointsWallet {
        let now = chrono::Utc::now();
        PointsWallet {
            id: Uuid::nil(),
            user_id,
            realm_id: realm_id.to_string(),
            bucket_id: None,
            total_topup_granted: 0,
            total_subscription_granted: 0,
            total_recharged: 0,
            total_consumed: 0,
            status: WalletStatus::Active,
            created_at: now,
            updated_at: now,
        }
    }

    /// Get points balance for a user
    ///
    /// Switched to derived SUM: the 5 typed balances and
    /// `total_balance` are projected from `compute_available_balance` (same
    /// predicate as consumption — "seen balance == spendable balance"), so
    /// future-effective pre-grant rows never leak into the user-visible
    /// balance. `analytics` (`total_recharged` / `total_consumed`) still come
    /// from the wallet Stored columns (lifetime totals, unaffected by
    /// `effective_at`). Read-path realization runs FIRST
    /// (`reconcile_due_for_user`) so a due free-period schedule is fulfilled
    /// before the balance is computed on the same request — this is the
    /// correctness backstop for US-FU-004 scenario 1.1 when the worker never
    /// runs.
    pub async fn get_balance(
        &self,
        identity: Identity,
        realm_id: &str,
        user_id: Uuid,
    ) -> Result<PointsBalance, CoreError> {
        // Permission + realm boundary checks (same gate as before).
        ensure_policy(
            self.policy
                .can_view_points(identity.clone(), Some(user_id))
                .await,
            "Insufficient permissions to view points balance",
        )?;
        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot access points from a different realm".to_string(),
            ));
        }

        let now = chrono::Utc::now();

        // Read-path realization. Independent committed short
        // transaction; the derived SUM below runs in a separate transaction
        // and under the project default READ COMMITTED sees the committed
        // realization rows. Failure is fail-loud (5xx) — never silently
        // degrade to an old balance or `InsufficientBalance`.
        self.reconcile_due_for_user(realm_id, user_id, now).await?;

        // Analytics still from Stored wallet columns (lifetime totals).
        let account = self.get_wallet(identity, realm_id, user_id).await?;

        // Derived SUM by credit_type (same predicate as consumption).
        let derived = self
            .repository
            .compute_available_balance(realm_id, user_id, &[], now)
            .await?;

        Ok(Self::build_balance_from_derived(account, derived))
    }

    /// Build `PointsBalance` from derived SUM + Stored analytics. The 5 typed
    /// balances and `total_balance` come from the derived SUM keyed by
    /// `CreditType`; analytics (`total_recharged` / `total_consumed`) are
    /// passed through from the Stored wallet (analytics remain Stored).
    fn build_balance_from_derived(
        account: PointsWallet,
        derived: Vec<(CreditType, i64)>,
    ) -> PointsBalance {
        let mut topup = 0i64;
        let mut subscription = 0i64;
        let mut granted = 0i64;
        let mut registration = 0i64;
        let mut free_periodic = 0i64;
        for (credit_type, amount) in derived {
            match credit_type {
                CreditType::TopupCredit => topup += amount,
                CreditType::SubscriptionCredit => subscription += amount,
                CreditType::GrantedCredit => granted += amount,
                CreditType::RegistrationCredit => registration += amount,
                CreditType::FreePeriodicCredit => free_periodic += amount,
            }
        }
        let total_balance = topup + subscription + granted + registration + free_periodic;
        PointsBalance {
            user_id: account.user_id,
            balance: total_balance,
            topup_balance: topup,
            subscription_balance: subscription,
            granted_balance: granted,
            registration_balance: registration,
            free_periodic_balance: free_periodic,
            total_recharged: account.total_recharged,
            total_consumed: account.total_consumed,
            unit: "points".to_string(),
            updated_at: account.updated_at,
        }
    }

    /// Read-path realization for free-periodic schedules.
    ///
    /// Single-user, bounded (N=3), idempotent (`points_grant_records`
    /// `(schedule_id, period_number)` UNIQUE + schedule row `FOR UPDATE`),
    /// lead_time=0 (only already-due periods), and restricted to
    /// `subscription_id IS NULL` (free-periodic only — never guess paid-grant
    /// fulfillment on the request path; subscriptions rely on event-driven
    /// chained pre-grant). This is the correctness backstop for
    /// US-FU-004 scenario 1.1 when the worker never runs.
    ///
    /// Fail-loud: on write failure returns the infra `CoreError` unchanged
    /// (caller surfaces 5xx). NEVER downgrades to `InsufficientBalance` or
    /// swallows the error — masking a write fault as "low balance" would hide
    /// system failure behind a user-visible business error.
    ///
    /// Transaction/visibility (P1-2): runs as an independent committed short
    /// transaction on the request's connection; the caller's subsequent
    /// balance/consume query runs in a separate transaction and under the
    /// project default READ COMMITTED sees the committed realization rows.
    /// The two are NOT merged into one long transaction (avoids widening the
    /// ledger `FOR UPDATE` lock range).
    ///
    /// No due schedule ⟹ pure read, no write transaction, no risk.
    pub async fn reconcile_due_for_user(
        &self,
        realm_id: &str,
        user_id: Uuid,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), CoreError> {
        /// Per-request cap on how many past-due periods are realized in one
        /// call. Free-periodic credits do not roll over, so
        /// periods beyond this cap are already expired (predicate-excluded)
        /// and do not affect current availability.
        const RECONCILE_PERIOD_CAP: u64 = 3;

        let due_schedules = self
            .repository
            .find_due_free_grant_schedules_for_user(realm_id, user_id, now, RECONCILE_PERIOD_CAP)
            .await?;

        if due_schedules.is_empty() {
            // Pure read path: no write transaction opened.
            return Ok(());
        }

        for schedule in due_schedules {
            // period_number follows the schedule's own progression
            // (granted_periods + 1) — same convention as GrantScheduler.
            let period_number = u32::try_from(schedule.granted_periods + 1).map_err(|_| {
                CoreError::InternalServerError("schedule.granted_periods overflow".to_string())
            })?;

            // Anchor to the expected period boundary (already-due ⟹ <=now).
            // `Some(next_grant_time)` documents the boundary even though the
            // predicate now contains it; the infra impl may collapse to None
            // when next_grant_time <= now if it prefers immediate semantics.
            let effective_at = Some(schedule.next_grant_time);
            let expires_at = schedule
                .grant_period_type
                .calculate_expiration(schedule.next_grant_time, schedule.validity_days);

            // Idempotent: if a grant_record already exists for this
            // (schedule_id, period_number), the infra impl returns the
            // existing ledger row without re-writing. Schedule row is locked
            // FOR UPDATE inside the impl for concurrent-request dedup.
            self.repository
                .pregrant_next_period_atomic(
                    realm_id,
                    &schedule,
                    period_number,
                    effective_at,
                    expires_at,
                )
                .await
                .map_err(|e| {
                    // Fail-loud: surface infra error verbatim. NEVER rewrite
                    // to InsufficientBalance or swallow.
                    tracing::error!(
                        realm_id = %realm_id,
                        user_id = %user_id,
                        schedule_id = %schedule.id,
                        period_number,
                        error = %e,
                        "reconcile_due_for_user write failed (fail-loud)"
                    );
                    e
                })?;
        }

        Ok(())
    }

    /// List wallets in a realm. `points.manage` holders see all users' wallets;
    /// `points.view`-only callers are hard-scoped to their own (mirrors
    /// `list_transactions`).
    pub async fn list_wallets(
        &self,
        identity: Identity,
        realm_id: &str,
        filters: WalletFilters,
    ) -> Result<Paginated<PointsWallet>, CoreError> {
        // View gate (handler also checks points.view; this is the domain
        // defense-in-depth). can_view_points(_, None) is true for a points.view-only
        // user viewing their own data and for points.manage holders.
        ensure_policy(
            self.policy.can_view_points(identity.clone(), None).await,
            "Insufficient permissions to list wallets",
        )?;

        // Check realm boundary
        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot access points from a different realm".to_string(),
            ));
        }

        // Only points managers list across users; points.view alone is scoped to self.
        // HARD-SCOPE non-managers to their own wallets:
        //   - `user_id` is server-injected (NOT a query param in ListWalletsQuery),
        //     so the client cannot set or override it.
        //   - `search` is the only client field that can target another user; we drop
        //     it so the ONLY user binding on the query is `user_id = caller`.
        // Remaining filters (bucket_id/status/paging) only narrow within the caller's
        // own rows. Mirrors list_transactions below, with explicit search stripping.
        let can_view_all = self.policy.can_manage_points(identity.clone()).await;
        if !can_view_all && let Ok(current_user_id) = identity.user_id().parse::<Uuid>() {
            let mut restricted = filters;
            restricted.user_id = Some(current_user_id);
            restricted.search = None;
            return self.repository.list_wallets(realm_id, restricted).await;
        }

        self.repository.list_wallets(realm_id, filters).await
    }

    // ===== Helper Methods =====

    // ===== Points Consumption =====

    /// Consume points from a user's account using ledger-based consumption
    ///
    /// Consumption priority: expiration-based (soonest expiring first, permanent last)
    /// This is the NEW correct implementation per the design specification.
    #[tracing::instrument(
        // Governance: identity carries user_id/realm_id;
        // input payload references user_id/client_app_id; realm_id is
        // conservatively skipped. Only the low-cardinality operation type and
        // db.system are recorded (no raw SQL, no ids).
        skip(self, identity, realm_id, input),
        fields(db.system = "postgres", db.operation = "consume_points")
    )]
    pub async fn consume_points(
        &self,
        identity: Identity,
        realm_id: &str,
        input: ConsumePointsInput,
    ) -> Result<Vec<PointsTransaction>, CoreError> {
        // Check consume permissions
        ensure_policy(
            self.policy.can_consume_points(identity.clone()).await,
            "Insufficient permissions to consume points",
        )?;

        // Validate input
        let (user_id, client_app_id, amount, description) = input.try_into()?;

        // Check realm boundary
        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot consume points from a different realm".to_string(),
            ));
        }

        // NOTE: The legacy single-wallet precheck (find_by_user_id → status +
        // balance check) is intentionally removed. With the multi-bucket model
        // a user holds one wallet row per Bucket, so
        // `find_by_user_id` (`.one()`) returns an arbitrary row and its
        // `total_balance` reflects a single pool — not the covered set. The
        // authoritative precheck is the infra layer's `consume_points_atomic`,
        // which sums `remaining_amount` across ALL covered-set ledgers
        // (`find_active_ledgers_by_expiration_for_update`) and reports the real
        // coverage-set availability via `insufficient_points`. Per-bucket
        // wallets are also created lazily inside the consume transaction via
        // `ensure_wallet_in_tx`, so no pre-created single wallet is needed.

        // Read-path realization runs as an independent
        // committed short transaction BEFORE the consume transaction opens —
        // the consume transaction then re-queries under READ COMMITTED and
        // sees the realized rows. Reconcile failure is fail-loud (5xx); it
        // MUST NOT be swallowed or rewritten to InsufficientBalance (masking
        // a write fault as "low balance" hides system failure).
        self.reconcile_due_for_user(realm_id, user_id, chrono::Utc::now())
            .await?;

        let saved_transactions = self
            .repository
            .consume_points_atomic(realm_id, user_id, client_app_id, amount, description, None)
            .await?;

        tracing::info!(
            realm_id = %realm_id,
            user_id = %user_id,
            amount,
            txn_count = saved_transactions.len(),
            "Points consumed successfully (expiration-based priority, per-bucket transactions)"
        );

        Ok(saved_transactions)
    }

    /// Idempotency replay. Reassemble the original consume result
    /// set from its primary transaction id WITHOUT re-deducting. Used by the
    /// HTTP-layer Redis-cache replay path when `check_or_create` returns a
    /// cached primary transaction: the primary → correlation_id → all N sibling
    /// per-bucket transactions. Legacy single-pool rows replay as 1 transaction.
    ///
    /// No permission check is performed here — the caller (HTTP layer) has
    /// already authorized the request, and the primary transaction id comes from
    /// our own idempotency cache, not from untrusted input.
    pub async fn replay_consume(
        &self,
        realm_id: &str,
        primary_txn_id: Uuid,
    ) -> Result<Vec<PointsTransaction>, CoreError> {
        self.repository
            .replay_consume_by_primary(realm_id, primary_txn_id)
            .await
    }

    /// Surface the ledger-level allocations of a consume by its `correlation_id`.
    /// Used by the SDK consume response to populate the
    /// `allocations` slice without re-deducting. Legacy single-pool rows (NULL
    /// correlation_id) return an empty vec.
    ///
    /// No permission check: the caller (HTTP layer) has already authorized the
    /// request and the correlation_id comes from our own consume result.
    pub async fn find_consumption_allocations_by_correlation_id(
        &self,
        realm_id: &str,
        correlation_id: &str,
    ) -> Result<Vec<ConsumptionAllocationView>, CoreError> {
        self.repository
            .find_consumption_allocations_by_correlation_id(realm_id, correlation_id)
            .await
    }

    // ===== Transaction Management =====

    /// Get a single transaction by ID
    pub async fn get_transaction(
        &self,
        identity: Identity,
        realm_id: &str,
        transaction_id: Uuid,
    ) -> Result<PointsTransaction, CoreError> {
        // Check view permissions
        ensure_policy(
            self.policy.can_view_points(identity.clone(), None).await,
            "Insufficient permissions to view transaction",
        )?;

        // Check realm boundary
        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot access transaction from a different realm".to_string(),
            ));
        }

        let transaction = self
            .repository
            .find_transaction_by_id(realm_id, transaction_id)
            .await?
            .ok_or(CoreError::NotFound)?;

        Ok(transaction)
    }

    /// List transactions with filters
    #[tracing::instrument(
        // Governance: identity carries user_id/realm_id; filters carry
        // user_id/bucket_id; realm_id conservatively skipped.
        skip(self, identity, realm_id, filters),
        fields(db.system = "postgres", db.operation = "list_transactions")
    )]
    pub async fn list_transactions(
        &self,
        identity: Identity,
        realm_id: &str,
        filters: TransactionFilters,
    ) -> Result<Paginated<PointsTransaction>, CoreError> {
        // Check view permissions
        ensure_policy(
            self.policy
                .can_view_points(identity.clone(), filters.user_id)
                .await,
            "Insufficient permissions to view transactions",
        )?;

        // Check realm boundary
        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot access transactions from a different realm".to_string(),
            ));
        }

        // Only points managers can query across users; points.view alone is scoped to self.
        let can_view_all = self.policy.can_manage_points(identity.clone()).await;
        if !can_view_all && let Ok(current_user_id) = identity.user_id().parse::<Uuid>() {
            // Override filters to only show current user's transactions
            let mut restricted_filters = filters;
            restricted_filters.user_id = Some(current_user_id);
            return self
                .repository
                .find_transactions(realm_id, restricted_filters)
                .await;
        }

        self.repository.find_transactions(realm_id, filters).await
    }

    // ===== Points Grant =====

    /// Grant points to a user (admin endpoint)
    ///
    /// Performs permission check and realm boundary check, validates input,
    /// then delegates to `grant_points_internal`.
    #[tracing::instrument(
        // Governance: identity carries user_id/realm_id; input payload
        // carries the target user_id; realm_id conservatively skipped.
        skip(self, identity, realm_id, input),
        fields(db.system = "postgres", db.operation = "grant_points")
    )]
    pub async fn grant_points(
        &self,
        identity: Identity,
        realm_id: &str,
        input: GrantPointsInput,
    ) -> Result<GrantPointsOutput, CoreError> {
        // Permission check
        ensure_policy(
            self.policy.can_manage_points(identity.clone()).await,
            "Insufficient permissions to grant points",
        )?;

        // Realm boundary check
        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot grant points from a different realm".to_string(),
            ));
        }

        self.execute_grant(realm_id, input).await
    }

    /// Grant points to a user (SDK/ext endpoint)
    ///
    /// Skips identity/policy checks -- those are handled by the caller
    /// at the middleware level (API Key authentication).
    pub async fn grant_points_for_sdk(
        &self,
        realm_id: &str,
        input: GrantPointsInput,
    ) -> Result<GrantPointsOutput, CoreError> {
        self.execute_grant(realm_id, input).await
    }

    /// Shared implementation for granting points
    async fn execute_grant(
        &self,
        realm_id: &str,
        input: GrantPointsInput,
    ) -> Result<GrantPointsOutput, CoreError> {
        // Validate input
        input.validate()?;

        // Compute expires_at from validity_days
        let expires_at = input
            .validity_days
            .map(|days| chrono::Utc::now() + chrono::Duration::days(days));

        // Build description including the user-provided reason
        let description = Some(format!(
            "{}: {} points granted ({})",
            input.source_type.as_str(),
            input.amount,
            input.reason
        ));

        // Grant points via internal method. Admin/SDK grants are immediately
        // available (`effective_at = None`); exposing an `effective_at` entry
        // point on the grant API is explicitly out of scope.
        let ledger_id = self
            .grant_points_internal(
                realm_id,
                input.user_id,
                input.bucket_id,
                CreditType::GrantedCredit,
                input.source_type,
                input.amount,
                expires_at,
                None,
                Some(input.source_id),
                description,
                None,
            )
            .await?;

        // Derived fill: `granted_balance`/`total_balance`
        // come from `compute_available_balance` post-grant (same source as
        // `get_balance`), NOT from the wallet Stored columns. This keeps the
        // grant response consistent with the derived-balance world and
        // prevents any future-effective rows from leaking into the response.
        let now = chrono::Utc::now();
        let derived = self
            .repository
            .compute_available_balance(realm_id, input.user_id, &[input.bucket_id], now)
            .await?;
        let (granted_balance, total_balance) =
            derived
                .into_iter()
                .fold((0i64, 0i64), |(g, t), (credit_type, amount)| {
                    let new_t = t + amount;
                    let new_g = if credit_type == CreditType::GrantedCredit {
                        g + amount
                    } else {
                        g
                    };
                    (new_g, new_t)
                });

        Ok(GrantPointsOutput {
            transaction_id: ledger_id,
            user_id: input.user_id,
            amount: input.amount,
            granted_balance,
            total_balance,
            expires_at,
        })
    }

    // ===== Internal Methods =====

    /// Recharge points for a user (internal method for billing webhooks)
    pub async fn recharge_points_internal(
        &self,
        realm_id: &str,
        user_id: Uuid,
        bucket_id: Uuid,
        amount: i64,
        recharge_type: RechargeType,
        external_ref_id: Option<String>,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<PointsTransaction, CoreError> {
        if amount <= 0 {
            return Err(CoreError::invalid_amount(
                "Recharge amount must be positive",
            ));
        }

        // NOTE: No aggregate wallet-status precheck here. `find_by_user_id` returns
        // the cross-bucket aggregate wallet (most-restrictive status wins), but
        // recharge targets a SPECIFIC bucket — so a frozen sibling bucket would
        // wrongly block a healthy one. The authoritative per-bucket status check
        // is the infra layer's `recharge_points_atomic`, which resolves the
        // bucket wallet via `ensure_wallet_in_tx` and rejects non-active status
        // (same convention as consume — see NOTE above `consume_points`).

        let (credit_type, source_type) = match recharge_type {
            RechargeType::Subscribe => (
                CreditType::SubscriptionCredit,
                CreditSourceType::SubscriptionInitial,
            ),
            RechargeType::Renewal => (
                CreditType::SubscriptionCredit,
                CreditSourceType::SubscriptionRenewal,
            ),
        };

        let transaction = self
            .repository
            .recharge_points_atomic(
                realm_id,
                user_id,
                bucket_id,
                credit_type,
                source_type,
                amount,
                expires_at,
                None,
                external_ref_id,
            )
            .await?;

        tracing::info!(
            realm_id = %realm_id,
            user_id = %user_id,
            amount,
            recharge_type = %recharge_type.as_str(),
            balance_after = %transaction.balance_after,
            "Points recharged successfully"
        );

        Ok(transaction)
    }

    /// Revoke points by credit type (internal method for subscription cancellation and refunds)
    ///
    /// Revokes all unused points of a specific credit type for a user.
    /// This is used for subscription cancellation, refunds, and expiration.
    ///
    /// # Arguments
    /// * `realm_id` - The realm ID
    /// * `user_id` - The user ID
    /// * `credit_type` - The type of credit to revoke (topup or subscription)
    /// * `revocation_type` - The reason for revocation
    /// * `reason` - Human-readable reason
    ///
    /// # Returns
    /// Revocation output with details of revoked points
    pub async fn revoke_points_by_credit_type(
        &self,
        realm_id: &str,
        user_id: Uuid,
        bucket_id: Uuid,
        credit_type: CreditType,
        revocation_type: RevocationType,
        reason: String,
    ) -> Result<RevokePointsOutput, CoreError> {
        let result = self
            .repository
            .revoke_points_by_credit_type_atomic(
                realm_id,
                user_id,
                bucket_id,
                credit_type,
                revocation_type,
                reason,
                None,
                None,
            )
            .await?;

        tracing::info!(
            realm_id = %realm_id,
            user_id = %user_id,
            credit_type = %credit_type.as_str(),
            total_revoked = result.total_revoked,
            ledger_count = result.ledger_ids.len(),
            revocation_type = %revocation_type.as_str(),
            "Points revoked successfully"
        );

        Ok(result)
    }

    /// Revoke remaining points from a specific ledger identified by source_id.
    /// Unlike `revoke_points_by_credit_type`, this targets only the single ledger
    /// whose `source_id` matches, preventing over-broad revocation of unrelated credits.
    pub async fn revoke_points_by_source_id(
        &self,
        realm_id: &str,
        user_id: Uuid,
        bucket_id: Uuid,
        source_id: &str,
        revocation_type: RevocationType,
        reason: String,
    ) -> Result<RevokePointsOutput, CoreError> {
        let result = self
            .repository
            .revoke_points_by_source_id_atomic(
                realm_id,
                user_id,
                bucket_id,
                source_id,
                revocation_type,
                reason,
                None,
                None,
            )
            .await?;

        tracing::info!(
            realm_id = %realm_id,
            user_id = %user_id,
            source_id = %source_id,
            total_revoked = result.total_revoked,
            revocation_type = %revocation_type.as_str(),
            "Points revoked by source_id successfully"
        );

        Ok(result)
    }

    /// Revoke active subscription credits for a specific entitlement key.
    pub async fn revoke_subscription_credits_by_entitlement(
        &self,
        realm_id: &str,
        user_id: Uuid,
        bucket_id: Uuid,
        entitlement_key: &str,
        revocation_type: RevocationType,
        reason: String,
        reference_id: Option<String>,
        idempotency_key: Option<String>,
    ) -> Result<RevokePointsOutput, CoreError> {
        let result = self
            .repository
            .revoke_subscription_credits_by_entitlement_atomic(
                realm_id,
                user_id,
                bucket_id,
                entitlement_key,
                revocation_type,
                reason,
                reference_id,
                idempotency_key,
            )
            .await?;

        tracing::info!(
            realm_id = %realm_id,
            user_id = %user_id,
            entitlement_key = %entitlement_key,
            total_revoked = result.total_revoked,
            ledger_count = result.ledger_ids.len(),
            revocation_type = %revocation_type.as_str(),
            "Subscription credits revoked by entitlement successfully"
        );

        Ok(result)
    }

    /// Revoke all daily free credits for a user (used when free user upgrades to paid)
    ///
    /// **Idempotency Guarantee**:
    /// - If idempotency_key is provided, checks if already processed
    /// - If no active daily credits exist, returns empty result (success)
    /// - Creates idempotency record even when no credits are revoked
    ///
    /// # Arguments
    /// * `realm_id` - The realm ID
    /// * `user_id` - The user ID
    /// * `reason` - Reason for revocation
    /// * `idempotency_key` - Optional idempotency key for deduplication
    ///
    /// # Returns
    /// Revocation output with details of revoked credits
    pub async fn revoke_all_daily_credits(
        &self,
        realm_id: &str,
        user_id: Uuid,
        bucket_id: Uuid,
        reason: String,
        idempotency_key: Option<String>,
    ) -> Result<RevokePointsOutput, CoreError> {
        self.repository
            .revoke_points_by_credit_type_atomic(
                realm_id,
                user_id,
                bucket_id,
                CreditType::FreePeriodicCredit,
                RevocationType::UpgradeRevoke,
                reason,
                None,
                idempotency_key,
            )
            .await
    }

    /// Proportionally revoke topup points based on refund ratio
    ///
    /// When a topup payment is refunded, we need to revoke the proportionate amount
    /// of points based on the refund ratio. This ensures users don't keep points they didn't pay for.
    ///
    /// # Arguments
    /// * `realm_id` - The realm ID
    /// * `user_id` - The user ID
    /// * `refund_amount` - The amount being refunded (in cents)
    /// * `original_payment_amount` - The original payment amount (in cents)
    /// * `refund_id` - The refund ID for reference
    ///
    /// # Returns
    /// Revocation output with details of revoked points
    pub async fn revoke_topup_proportional(
        &self,
        realm_id: &str,
        user_id: Uuid,
        bucket_id: Uuid,
        refund_amount: i64,
        original_payment_amount: i64,
        refund_id: &str,
    ) -> Result<RevokePointsOutput, CoreError> {
        if original_payment_amount <= 0 {
            return Err(CoreError::BadRequest(
                "Original payment amount must be positive".to_string(),
            ));
        }

        if refund_amount <= 0 {
            return Err(CoreError::BadRequest(
                "Refund amount must be positive".to_string(),
            ));
        }

        if refund_amount > original_payment_amount {
            return Err(CoreError::BadRequest(
                "Refund amount cannot exceed original payment".to_string(),
            ));
        }

        let result = self
            .repository
            .revoke_topup_proportional_atomic(
                realm_id,
                user_id,
                bucket_id,
                refund_amount,
                original_payment_amount,
                refund_id,
            )
            .await?;

        if result.total_revoked == 0 {
            tracing::info!(
                realm_id = %realm_id,
                user_id = %user_id,
                refund_id = %refund_id,
                "No active topup ledgers found for proportional revocation"
            );
        } else {
            tracing::info!(
                realm_id = %realm_id,
                user_id = %user_id,
                refund_id = %refund_id,
                refund_amount = refund_amount,
                original_payment_amount = original_payment_amount,
                total_revoked = result.total_revoked,
                ledger_count = result.ledger_ids.len(),
                "Proportionally revoked topup points"
            );
        }

        Ok(result)
    }

    /// Revoke all unused subscription points
    ///
    /// This is a convenience method for refund scenarios where all subscription
    /// points need to be revoked. It simply calls revoke_points_by_credit_type
    /// with SubscriptionCredit and RefundRevoke type.
    ///
    /// # Arguments
    /// * `realm_id` - The realm ID
    /// * `user_id` - The user ID
    /// * `refund_id` - The refund ID for reference
    ///
    /// # Returns
    /// Revocation output with details of revoked points
    pub async fn revoke_subscription_unused(
        &self,
        realm_id: &str,
        user_id: Uuid,
        bucket_id: Uuid,
        refund_id: &str,
    ) -> Result<RevokePointsOutput, CoreError> {
        self.repository
            .revoke_points_by_credit_type_atomic(
                realm_id,
                user_id,
                bucket_id,
                CreditType::SubscriptionCredit,
                crate::points::entities::RevocationType::RefundRevoke,
                format!("Refund {}", refund_id),
                Some(refund_id.to_string()),
                Some(format!("refund:subscription:{}", refund_id)),
            )
            .await
    }

    /// Refund points from a user's account
    ///
    /// Creates a refund transaction to deduct points when a subscription is refunded.
    /// This is typically called when a payment refund is processed.
    ///
    /// # Arguments
    /// * `realm_id` - The realm ID
    /// * `user_id` - The user ID
    /// * `subscription_id` - The subscription ID being refunded
    /// * `refund_amount` - The amount of points to refund (deduct)
    /// * `reason` - The reason for the refund
    ///
    /// # Returns
    /// The created refund transaction
    ///
    /// # Errors
    /// - Account not found
    /// - Invalid amount (must be positive)
    /// - Database errors
    pub async fn refund_points(
        &self,
        realm_id: &str,
        user_id: Uuid,
        bucket_id: Uuid,
        subscription_id: String,
        refund_amount: i64,
        reason: String,
    ) -> Result<PointsTransaction, CoreError> {
        if refund_amount <= 0 {
            return Err(CoreError::BadRequest(
                "Refund amount must be positive".to_string(),
            ));
        }

        let created_transaction = self
            .repository
            .refund_points_atomic(
                realm_id,
                user_id,
                bucket_id,
                subscription_id.clone(),
                refund_amount,
                reason,
            )
            .await?;

        tracing::info!(
            realm_id = %realm_id,
            user_id = %user_id,
            subscription_id = %subscription_id,
            refund_amount,
            new_balance = created_transaction.balance_after,
            "Points refunded successfully"
        );

        Ok(created_transaction)
    }

    // ===== Internal Methods (for Background Services) =====

    /// Internal method to grant points directly to ledger
    ///
    /// This is used by background services (registration, scheduler)
    /// and bypasses the public API layer validation.
    ///
    /// # Arguments
    /// * `realm_id` - The realm ID
    /// * `user_id` - The user ID
    /// * `credit_type` - Type of credit to grant
    /// * `source_type` - Source of the grant
    /// * `amount` - Amount to grant
    /// * `expires_at` - Optional expiration time (None = permanent)
    /// * `source_id` - Optional source ID for traceability
    ///
    /// # Returns
    /// Ok(ledger_id) on success -- the ID of the created credit ledger entry
    ///
    /// # Errors
    /// - InvalidAmount if amount <= 0
    /// - Database errors
    ///
    /// # Security
    /// This is an internal method (NOT an HTTP endpoint) and will only be called
    /// from trusted internal services (RegistrationService, GrantScheduler).
    /// No authorization checks are performed.
    pub async fn grant_points_internal(
        &self,
        realm_id: &str,
        user_id: Uuid,
        bucket_id: Uuid,
        credit_type: CreditType,
        source_type: crate::points::entities::CreditSourceType,
        amount: i64,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
        // Expected effective time. `None` ⟺ immediately
        // available (current behavior); `Some(t)` ⟺ enters the available
        // set only when `effective_at <= NOW()`. Passed through to
        // `grant_points_atomic`.
        effective_at: Option<chrono::DateTime<chrono::Utc>>,
        source_id: Option<String>,
        description: Option<String>,
        idempotency_key: Option<String>,
    ) -> Result<Uuid, CoreError> {
        if amount <= 0 {
            return Err(CoreError::BadRequest(
                "Grant amount must be positive".to_string(),
            ));
        }

        // NOTE: No aggregate wallet-status precheck here. `find_by_user_id` returns
        // the cross-bucket aggregate wallet (most-restrictive status wins), but
        // grant targets a SPECIFIC bucket — so a frozen sibling bucket would
        // wrongly block a healthy one. The authoritative per-bucket status check
        // is the infra layer's `grant_points_atomic`, which resolves the
        // bucket wallet via `ensure_wallet_in_tx` and rejects non-active status
        // (same convention as consume — see NOTE above `consume_points`).

        let saved_ledger = self
            .repository
            .grant_points_atomic(
                realm_id,
                user_id,
                bucket_id,
                credit_type,
                source_type,
                amount,
                expires_at,
                effective_at,
                source_id,
                description,
                idempotency_key,
            )
            .await?;

        tracing::info!(
            realm_id = %realm_id,
            user_id = %user_id,
            amount,
            credit_type = ?credit_type,
            source_type = ?source_type,
            expires_at = ?expires_at,
            effective_at = ?effective_at,
            "Points granted internally"
        );

        Ok(saved_ledger.id)
    }
}

// Governance tests.
//
// Covers: domain points service `consume_points`,
// `list_transactions`, `grant_points` instrument skip correctness.
//
// WHY: these methods take `identity` (carries user_id/realm_id), `realm_id`,
// and `input`/`filters` (reference user_id/bucket_id). If the `#[instrument]`
// macro ever stops skipping those, the identifiers leak into a span field.
// Source-scan baseline, anchored per method to the
// immediately-preceding `#[tracing::instrument(...)]`.
#[cfg(test)]
mod instrument_skip_tests {
    const SRC: &str = include_str!("service.rs");

    fn instrument_body_preceding(fn_name: &str) -> String {
        let needle = format!("fn {fn_name}");
        let fn_pos = SRC
            .find(&needle)
            .unwrap_or_else(|| panic!("fn {fn_name} not found in source"));
        let attr_start = SRC[..fn_pos]
            .rfind("#[tracing::instrument(")
            .unwrap_or_else(|| panic!("no #[tracing::instrument( preceding fn {fn_name}"));
        let body_start = attr_start + "#[tracing::instrument(".len();
        // Find the attribute close: the first line at/after body_start whose
        // trimmed content is exactly `)]`. This handles indented closes (e.g.
        // inside an `impl` block) and ignores inline `))]` sequences such as
        // `#[validate(length(...))]` that appear on struct fields.
        let tail = &SRC[body_start..];
        let mut consumed = 0usize;
        for line in tail.lines() {
            let prev = consumed;
            consumed += line.len() + 1; // +1 for the line separator
            if line.trim() == ")]" {
                return tail[..prev].to_string();
            }
        }
        panic!("unterminated #[tracing::instrument( for fn {fn_name}")
    }

    #[test]
    fn instrument_skip_points_consume_excludes_identity_realm_input() {
        let body = instrument_body_preceding("consume_points");
        for required in ["identity", "realm_id", "input"] {
            assert!(
                body.contains(required),
                "consume_points must skip `{required}`; body was:\n{body}"
            );
        }
        for banned in ["token", "password", "email", "secret", "user_id"] {
            assert!(
                !body.contains(&format!("{banned} ="))
                    && !body.contains(&format!("fields({banned}")),
                "consume_points span must not record a `{banned}` field; body was:\n{body}"
            );
        }
    }

    #[test]
    fn instrument_skip_points_list_transactions_excludes_identity_filters() {
        let body = instrument_body_preceding("list_transactions");
        for required in ["identity", "realm_id", "filters"] {
            assert!(
                body.contains(required),
                "list_transactions must skip `{required}`; body was:\n{body}"
            );
        }
        for banned in ["token", "password", "email", "secret", "user_id"] {
            assert!(
                !body.contains(&format!("{banned} ="))
                    && !body.contains(&format!("fields({banned}")),
                "list_transactions span must not record a `{banned}` field; body was:\n{body}"
            );
        }
    }

    #[test]
    fn instrument_skip_points_grant_excludes_identity_realm_input() {
        let body = instrument_body_preceding("grant_points");
        for required in ["identity", "realm_id", "input"] {
            assert!(
                body.contains(required),
                "grant_points must skip `{required}`; body was:\n{body}"
            );
        }
        for banned in ["token", "password", "email", "secret", "user_id"] {
            assert!(
                !body.contains(&format!("{banned} ="))
                    && !body.contains(&format!("fields({banned}")),
                "grant_points span must not record a `{banned}` field; body was:\n{body}"
            );
        }
    }
}
