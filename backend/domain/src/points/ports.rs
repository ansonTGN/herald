// Points repository ports

use std::future::Future;

use uuid::Uuid;

use crate::common::entities::app_errors::CoreError;
use crate::points::dtos::RevokePointsOutput;
use crate::points::entities::CreditSourceType;
use crate::points::entities::{
    ConsumptionAllocationView, CreditLedgerStatus, CreditType, Paginated,
    PointsConsumptionAllocation, PointsCreditLedger, PointsRevocationRecord, PointsTransaction,
    PointsWallet, RevocationType, TransactionType,
};
use crate::points::{
    CreateRealmConfigInput, PointsGrantRecord, PointsGrantSchedule, RealmDefaultConfig,
    UpdateRealmConfigInput, UserPointsConfig,
};

/// Row-level locator for pre-grant reclaim.
///
/// `points_credit_ledger` itself carries no `schedule_id`/`period_number`
/// columns — the business idempotency key `(schedule_id, period_number)` lives
/// only in `points_grant_records`. So reclaim must resolve to the unique
/// ledger row through one of:
/// - `BySourceId` — direct lookup on `points_credit_ledger.source_id`.
/// - `BySchedulePeriod` — resolved through the `points_grant_records.ledger_id`
///   FK subquery: the unique ledger row linked to
///   `(schedule_id, period_number)`.
///
/// The trait layer declares only the locator shape; the infra impl
/// owns the resolution SQL.
#[derive(Debug, Clone)]
pub enum ReclaimLocator {
    /// Reclaim the single ledger row whose `source_id` matches.
    BySourceId(String),
    /// Reclaim the unique ledger row linked to `(schedule_id, period_number)`
    /// via `points_grant_records.ledger_id` FK.
    BySchedulePeriod {
        schedule_id: Uuid,
        period_number: u32,
    },
}

/// Transaction filters
#[derive(Debug, Clone, Default)]
pub struct TransactionFilters {
    pub user_id: Option<Uuid>,
    pub bucket_id: Option<Uuid>,
    pub transaction_type: Option<TransactionType>,
    pub client_app_id: Option<Uuid>,
    pub subscription_id: Option<Uuid>,
    pub external_ref_id: String,
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// Account filters
#[derive(Debug, Clone, Default)]
pub struct WalletFilters {
    pub user_id: Option<Uuid>,
    pub bucket_id: Option<Uuid>,
    pub status: Option<String>,
    pub search: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// Ledger filters
#[derive(Debug, Clone, Default)]
pub struct LedgerFilters {
    pub credit_type: Option<CreditType>,
    pub status: Option<CreditLedgerStatus>,
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub pagination: Option<Pagination>,
}

/// Pagination params
#[derive(Debug, Clone, Copy)]
pub struct Pagination {
    pub page: u64,
    pub page_size: u64,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 20,
        }
    }
}

/// Ledger update type
#[derive(Debug, Clone)]
pub enum LedgerUpdate {
    Consumption(i64),
    Revocation(i64),
    SetExpiration(chrono::DateTime<chrono::Utc>),
    SetStatus(CreditLedgerStatus),
}

/// Delta applied to a wallet row's lifetime analytics columns by the single
/// writer `apply_wallet_delta_in_tx`.
///
/// The 5 per-type balance delta fields are gone
/// (the underlying `points_wallets` balance columns were physically dropped;
/// available balance is now a derived SUM over `points_credit_ledger`).
/// Only the 4 monotonic lifetime analytics columns remain Stored, so this
/// delta only describes how to advance them.
///
/// `total_*_granted` and `total_consumed` are monotonic lifetime totals —
/// grant/consume writers add positive deltas; revocation/refund writers leave
/// them unchanged (revocation does not "un-consume" or "un-grant", it only
/// moves `remaining_amount` on the ledger).
#[derive(Debug, Clone, Copy, Default)]
pub struct WalletDelta {
    pub total_recharged: i64,
    pub total_consumed: i64,
    pub total_topup_granted: i64,
    pub total_subscription_granted: i64,
}

impl WalletDelta {
    pub fn zero() -> Self {
        Self::default()
    }

    /// Delta for granting `amount` of `credit_type`. Advances the matching
    /// lifetime total; paid grants (topup / subscription) also accrue
    /// `total_recharged`. No longer touches any balance column.
    pub fn grant(credit_type: CreditType, amount: i64) -> Self {
        let (total_topup_granted, total_subscription_granted) = match credit_type {
            CreditType::TopupCredit => (amount, 0),
            CreditType::SubscriptionCredit => (0, amount),
            _ => (0, 0),
        };
        Self {
            total_recharged: total_topup_granted + total_subscription_granted,
            total_consumed: 0,
            total_topup_granted,
            total_subscription_granted,
        }
    }
}

/// User config update type
#[derive(Debug, Clone)]
pub enum UserConfigUpdate {
    /// Disable daily grant (used when free user upgrades to paid)
    DisableDailyGrant {
        next_grant_time: Option<chrono::DateTime<chrono::Utc>>,
    },
}

/// Grant schedule update type
#[derive(Debug, Clone)]
pub enum GrantScheduleUpdate {
    /// Disable grant schedule
    Disable,
}

/// Repository for points operations
pub trait PointsRepository: Send + Sync {
    /// Find points wallet by user ID
    fn find_by_user_id(
        &self,
        realm_id: &str,
        user_id: Uuid,
    ) -> impl Future<Output = Result<Option<PointsWallet>, CoreError>> + Send;

    /// Find points wallet by ID
    fn find_by_id(
        &self,
        id: Uuid,
    ) -> impl Future<Output = Result<Option<PointsWallet>, CoreError>> + Send;

    /// Create a new points wallet
    fn create_wallet(
        &self,
        account: PointsWallet,
    ) -> impl Future<Output = Result<PointsWallet, CoreError>> + Send;

    /// Create a points transaction
    fn create_transaction(
        &self,
        transaction: PointsTransaction,
    ) -> impl Future<Output = Result<PointsTransaction, CoreError>> + Send;

    /// Find transactions with filters
    fn find_transactions(
        &self,
        realm_id: &str,
        filters: TransactionFilters,
    ) -> impl Future<Output = Result<Paginated<PointsTransaction>, CoreError>> + Send;

    /// Find expired recharge transactions for a user
    ///
    /// Returns all recharge transactions that have expired (expires_at < NOW())
    /// and have not yet been processed for expiration.
    fn find_expired_recharge_transactions(
        &self,
        realm_id: &str,
        user_id: Uuid,
    ) -> impl Future<Output = Result<Vec<PointsTransaction>, CoreError>> + Send;

    /// Find a single transaction by ID
    fn find_transaction_by_id(
        &self,
        realm_id: &str,
        transaction_id: Uuid,
    ) -> impl Future<Output = Result<Option<PointsTransaction>, CoreError>> + Send;

    /// Count transactions for pagination
    fn count_transactions(
        &self,
        realm_id: &str,
        filters: &TransactionFilters,
    ) -> impl Future<Output = Result<u64, CoreError>> + Send;

    fn check_idempotency_key(
        &self,
        realm_id: &str,
        idempotency_key: &str,
    ) -> impl Future<Output = Result<Option<Uuid>, CoreError>> + Send;

    fn record_idempotency_key(
        &self,
        realm_id: &str,
        idempotency_key: &str,
        transaction_id: Uuid,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    /// Find transaction by external reference ID (for idempotency check)
    /// DEPRECATED: Use check_idempotency_key instead
    fn find_transaction_by_ref(
        &self,
        realm_id: &str,
        user_id: Uuid,
        external_ref_id: &str,
    ) -> impl Future<Output = Result<Option<PointsTransaction>, CoreError>> + Send;

    /// Find points policy by entitlement_key
    fn find_points_policy_by_entitlement_key(
        &self,
        realm_id: &str,
        entitlement_key: &str,
    ) -> impl Future<
        Output = Result<Option<crate::billing::entities::EntitlementMapping>, CoreError>,
    > + Send;

    /// List accounts with filters
    fn list_wallets(
        &self,
        realm_id: &str,
        filters: WalletFilters,
    ) -> impl Future<Output = Result<Paginated<PointsWallet>, CoreError>> + Send;

    /// Count accounts for pagination
    fn count_wallets(
        &self,
        realm_id: &str,
        filters: &WalletFilters,
    ) -> impl Future<Output = Result<u64, CoreError>> + Send;

    // ========== Ledger Management ==========

    /// Create a new credit ledger entry
    fn create_ledger(
        &self,
        ledger: PointsCreditLedger,
    ) -> impl Future<Output = Result<PointsCreditLedger, CoreError>> + Send;

    /// Find ledgers by user ID with filters
    fn find_ledgers_by_user_id(
        &self,
        realm_id: &str,
        user_id: Uuid,
        filters: LedgerFilters,
    ) -> impl Future<Output = Result<Paginated<PointsCreditLedger>, CoreError>> + Send;

    /// Find a single ledger by ID
    fn find_ledger_by_id(
        &self,
        ledger_id: Uuid,
    ) -> impl Future<Output = Result<Option<PointsCreditLedger>, CoreError>> + Send;

    /// Find a single ledger by source_id (e.g., payment_attempt_id for idempotency checks)
    fn find_ledger_by_source_id(
        &self,
        realm_id: &str,
        source_id: &str,
    ) -> impl Future<Output = Result<Option<PointsCreditLedger>, CoreError>> + Send;

    /// Update ledger (consumption or revocation)
    fn update_ledger(
        &self,
        ledger_id: Uuid,
        updates: LedgerUpdate,
    ) -> impl Future<Output = Result<PointsCreditLedger, CoreError>> + Send;

    /// Find active ledgers by credit type (ordered by created_at ASC for FIFO)
    fn find_active_ledgers_by_credit_type(
        &self,
        realm_id: &str,
        user_id: Uuid,
        credit_type: CreditType,
    ) -> impl Future<Output = Result<Vec<PointsCreditLedger>, CoreError>> + Send;

    // ========== Consumption Allocations ==========

    /// Create a consumption allocation record
    fn create_consumption_allocation(
        &self,
        allocation: PointsConsumptionAllocation,
    ) -> impl Future<Output = Result<PointsConsumptionAllocation, CoreError>> + Send;

    /// Find consumption allocations by transaction ID
    fn find_consumption_allocations_by_transaction(
        &self,
        transaction_id: Uuid,
    ) -> impl Future<Output = Result<Vec<PointsConsumptionAllocation>, CoreError>> + Send;

    /// Find consumption allocations for ALL transactions sharing a consume
    /// `correlation_id`. Used by the SDK consume response
    /// to surface the ledger-level truth source of a multi-bucket consume without
    /// re-deducting. Legacy single-pool rows (NULL correlation_id) are excluded.
    /// Returns each allocation joined with its ledger's `credit_type` so the
    /// response can populate `AllocationDetail.credit_type`.
    fn find_consumption_allocations_by_correlation_id(
        &self,
        realm_id: &str,
        correlation_id: &str,
    ) -> impl Future<Output = Result<Vec<ConsumptionAllocationView>, CoreError>> + Send;

    // ========== Revocation Records ==========

    /// Create a revocation record
    fn create_revocation_record(
        &self,
        record: PointsRevocationRecord,
    ) -> impl Future<Output = Result<PointsRevocationRecord, CoreError>> + Send;

    /// Find revocation record by idempotency key (for deduplication)
    fn find_revocation_by_idempotency_key(
        &self,
        idempotency_key: &str,
    ) -> impl Future<Output = Result<Option<PointsRevocationRecord>, CoreError>> + Send;

    // ========== Account Management ==========

    /// Find expired ledgers for cleanup
    fn find_expired_ledgers(
        &self,
        expiration_time: chrono::DateTime<chrono::Utc>,
        limit: usize,
    ) -> impl Future<Output = Result<Vec<PointsCreditLedger>, CoreError>> + Send;

    // ========== Realm Config Repository Methods ==========

    /// Find realm default config by realm_id
    fn find_realm_config(
        &self,
        realm_id: &str,
    ) -> impl Future<Output = Result<Option<RealmDefaultConfig>, CoreError>> + Send;

    /// Create realm default config
    fn create_realm_config(
        &self,
        config: CreateRealmConfigInput,
    ) -> impl Future<Output = Result<RealmDefaultConfig, CoreError>> + Send;

    /// Update realm default config
    fn update_realm_config(
        &self,
        realm_id: &str,
        input: UpdateRealmConfigInput,
    ) -> impl Future<Output = Result<RealmDefaultConfig, CoreError>> + Send;

    // ========== User Config Repository Methods ==========

    /// Find user points config by user_id
    fn find_user_config(
        &self,
        user_id: Uuid,
    ) -> impl Future<Output = Result<Option<UserPointsConfig>, CoreError>> + Send;

    /// Create user points config
    fn create_user_config(
        &self,
        config: UserPointsConfig,
    ) -> impl Future<Output = Result<UserPointsConfig, CoreError>> + Send;

    /// Update user points config
    fn update_user_config(
        &self,
        user_id: Uuid,
        next_grant_time: Option<chrono::DateTime<chrono::Utc>>,
        granted_periods: i64,
        grant_schedule_id: Option<Uuid>,
    ) -> impl Future<Output = Result<UserPointsConfig, CoreError>> + Send;

    /// List user configs by realm (for statistics)
    fn list_user_configs_by_realm(
        &self,
        realm_id: &str,
        pagination: &Pagination,
    ) -> impl Future<Output = Result<Paginated<UserPointsConfig>, CoreError>> + Send;

    /// Find user points config by user_id (alias for find_user_config)
    fn find_user_points_config(
        &self,
        user_id: Uuid,
    ) -> impl Future<Output = Result<Option<UserPointsConfig>, CoreError>> + Send {
        self.find_user_config(user_id)
    }

    /// Find user points config by realm_id and user_id (for free user upgrade)
    fn find_user_config_by_realm(
        &self,
        realm_id: &str,
        user_id: Uuid,
    ) -> impl Future<Output = Result<Option<UserPointsConfig>, CoreError>> + Send;

    // ========== Grant Schedule Repository Methods ==========

    /// Find grant schedule by ID
    fn find_grant_schedule(
        &self,
        schedule_id: Uuid,
    ) -> impl Future<Output = Result<Option<PointsGrantSchedule>, CoreError>> + Send;

    /// Find active grant schedules due for granting
    fn find_due_grant_schedules(
        &self,
        before: chrono::DateTime<chrono::Utc>,
        limit: u64,
    ) -> impl Future<Output = Result<Vec<PointsGrantSchedule>, CoreError>> + Send;

    /// Find grant schedules by user_id
    fn find_grant_schedules_by_user(
        &self,
        user_id: Uuid,
    ) -> impl Future<Output = Result<Vec<PointsGrantSchedule>, CoreError>> + Send;

    // ===== New: Free User Upgrade Support =====

    /// Update user points config
    fn update_user_points_config(
        &self,
        config_id: Uuid,
        update: UserConfigUpdate,
    ) -> impl Future<Output = Result<UserPointsConfig, CoreError>> + Send;

    /// Find grant schedules by realm_id and user_id
    fn find_grant_schedules_by_user_realm(
        &self,
        realm_id: &str,
        user_id: Uuid,
    ) -> impl Future<Output = Result<Vec<PointsGrantSchedule>, CoreError>> + Send;

    /// Apply update to grant schedule (for disabling, etc)
    fn apply_grant_schedule_update(
        &self,
        schedule_id: Uuid,
        update: GrantScheduleUpdate,
    ) -> impl Future<Output = Result<PointsGrantSchedule, CoreError>> + Send;

    /// Find grant schedule by subscription_id
    fn find_grant_schedule_by_subscription(
        &self,
        subscription_id: Uuid,
    ) -> impl Future<Output = Result<Option<PointsGrantSchedule>, CoreError>> + Send;

    /// Create grant schedule
    fn create_grant_schedule(
        &self,
        schedule: PointsGrantSchedule,
    ) -> impl Future<Output = Result<PointsGrantSchedule, CoreError>> + Send;

    /// Update grant schedule (increment periods, update next_grant_time)
    fn update_grant_schedule(
        &self,
        schedule_id: Uuid,
        next_grant_time: chrono::DateTime<chrono::Utc>,
        granted_periods: i64,
        is_active: bool,
    ) -> impl Future<Output = Result<PointsGrantSchedule, CoreError>> + Send;

    /// Deactivate grant schedule
    fn deactivate_grant_schedule(
        &self,
        schedule_id: Uuid,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    // ========== Grant Record Repository Methods ==========

    /// Check if a grant record exists for a schedule and period (idempotency)
    fn find_grant_record(
        &self,
        schedule_id: Uuid,
        period_number: i64,
    ) -> impl Future<Output = Result<Option<PointsGrantRecord>, CoreError>> + Send;

    /// Create grant record
    fn create_grant_record(
        &self,
        record: PointsGrantRecord,
    ) -> impl Future<Output = Result<PointsGrantRecord, CoreError>> + Send;

    /// List grant records by schedule
    fn list_grant_records_by_schedule(
        &self,
        schedule_id: Uuid,
        pagination: &Pagination,
    ) -> impl Future<Output = Result<Paginated<PointsGrantRecord>, CoreError>> + Send;

    /// List grant records by user
    fn list_grant_records_by_user(
        &self,
        user_id: Uuid,
        pagination: &Pagination,
    ) -> impl Future<Output = Result<Paginated<PointsGrantRecord>, CoreError>> + Send;

    // ========== Ledger Repository Methods (Consumption Priority) ==========

    /// Find active ledgers sorted by expiration (for FIFO consumption)
    /// Returns ledgers with status = 'active' AND remaining_amount > 0
    /// Sorted by expires_at ASC (NULL last, meaning permanent credits last)
    fn find_active_ledgers_by_expiration(
        &self,
        realm_id: &str,
        user_id: Uuid,
    ) -> impl Future<Output = Result<Vec<PointsCreditLedger>, CoreError>> + Send;

    // ========== Statistics Repository Methods ==========

    /// Count paid users in realm (for upgrade rate calculation)
    /// Returns count of users with active subscriptions in the realm
    fn count_paid_users_in_realm(
        &self,
        realm_id: &str,
        start_date: Option<chrono::DateTime<chrono::Utc>>,
        end_date: Option<chrono::DateTime<chrono::Utc>>,
    ) -> impl Future<Output = Result<u64, CoreError>> + Send;

    // ========== Atomic Write Paths ==========

    fn consume_points_atomic(
        &self,
        realm_id: &str,
        user_id: Uuid,
        client_app_id: Uuid,
        amount: i64,
        description: Option<String>,
        idempotency_key: Option<String>,
    ) -> impl Future<Output = Result<Vec<PointsTransaction>, CoreError>> + Send;

    /// Reassemble a consume result set from its primary transaction id, WITHOUT
    /// re-deducting (idempotency replay). Multi-pool rows share a
    /// `correlation_id` → return all N sibling transactions ordered by
    /// bucket_id. Legacy single-pool rows (NULL `correlation_id`) return just
    /// the primary transaction.
    fn replay_consume_by_primary(
        &self,
        realm_id: &str,
        primary_txn_id: Uuid,
    ) -> impl Future<Output = Result<Vec<PointsTransaction>, CoreError>> + Send;

    fn revoke_points_by_credit_type_atomic(
        &self,
        realm_id: &str,
        user_id: Uuid,
        bucket_id: Uuid,
        credit_type: CreditType,
        revocation_type: RevocationType,
        reason: String,
        reference_id: Option<String>,
        idempotency_key: Option<String>,
    ) -> impl Future<Output = Result<RevokePointsOutput, CoreError>> + Send;

    /// Revoke all remaining points from a specific ledger identified by source_id.
    /// Unlike `revoke_points_by_credit_type_atomic`, this only targets the single
    /// ledger whose `source_id` matches, avoiding over-broad revocation.
    fn revoke_points_by_source_id_atomic(
        &self,
        realm_id: &str,
        user_id: Uuid,
        bucket_id: Uuid,
        source_id: &str,
        revocation_type: RevocationType,
        reason: String,
        reference_id: Option<String>,
        idempotency_key: Option<String>,
    ) -> impl Future<Output = Result<RevokePointsOutput, CoreError>> + Send;

    /// Revoke all active subscription credit ledgers for one entitlement.
    /// This is intentionally separate from source_id revocation because it is a
    /// subscription-domain operation and may revoke multiple ledgers.
    fn revoke_subscription_credits_by_entitlement_atomic(
        &self,
        realm_id: &str,
        user_id: Uuid,
        bucket_id: Uuid,
        entitlement_key: &str,
        revocation_type: RevocationType,
        reason: String,
        reference_id: Option<String>,
        idempotency_key: Option<String>,
    ) -> impl Future<Output = Result<RevokePointsOutput, CoreError>> + Send;

    fn revoke_topup_proportional_atomic(
        &self,
        realm_id: &str,
        user_id: Uuid,
        bucket_id: Uuid,
        refund_amount: i64,
        original_payment_amount: i64,
        refund_id: &str,
    ) -> impl Future<Output = Result<RevokePointsOutput, CoreError>> + Send;

    fn refund_points_atomic(
        &self,
        realm_id: &str,
        user_id: Uuid,
        bucket_id: Uuid,
        refund_reference: String,
        refund_amount: i64,
        reason: String,
    ) -> impl Future<Output = Result<PointsTransaction, CoreError>> + Send;

    fn grant_points_atomic(
        &self,
        realm_id: &str,
        user_id: Uuid,
        bucket_id: Uuid,
        credit_type: CreditType,
        source_type: CreditSourceType,
        amount: i64,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
        // Expected effective time. `None` ⟺ immediately
        // available; `Some(t)` ⟺ enters the available set only when
        // `effective_at <= NOW()`. INSERT writes the column; derived balance
        // and consumption predicates gate on it.
        effective_at: Option<chrono::DateTime<chrono::Utc>>,
        source_id: Option<String>,
        description: Option<String>,
        idempotency_key: Option<String>,
    ) -> impl Future<Output = Result<PointsCreditLedger, CoreError>> + Send;

    fn recharge_points_atomic(
        &self,
        realm_id: &str,
        user_id: Uuid,
        bucket_id: Uuid,
        credit_type: CreditType,
        source_type: CreditSourceType,
        amount: i64,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
        source_id: Option<String>,
        external_ref_id: Option<String>,
    ) -> impl Future<Output = Result<PointsTransaction, CoreError>> + Send;

    fn set_subscription_ledger_expiration_atomic(
        &self,
        realm_id: &str,
        user_id: Uuid,
        period_end: chrono::DateTime<chrono::Utc>,
    ) -> impl Future<Output = Result<Vec<Uuid>, CoreError>> + Send;

    fn handle_subscription_paid_atomic(
        &self,
        realm_id: &str,
        user_id: Uuid,
        bucket_id: Uuid,
        entitlement_key: String,
        points_amount: i64,
        source_type: CreditSourceType,
        // Billing period start. Used as the ledger
        // `effective_at` (`period_start <= now` ⟺ immediately available) and
        // drives the period/schedule business idempotency key
        // (`points_grant_records(schedule_id, period_number)`).
        period_start: chrono::DateTime<chrono::Utc>,
        period_end: chrono::DateTime<chrono::Utc>,
        idempotency_key: String,
        disable_daily_grant: bool,
    ) -> impl Future<Output = Result<PointsCreditLedger, CoreError>> + Send;

    fn scan_and_expire_points_atomic(
        &self,
        batch_size: usize,
    ) -> impl Future<
        Output = Result<crate::points::expiration_service::ExpirationSummary, CoreError>,
    > + Send;

    // ========== point-time: derived balance + pre-grant + reclaim ==========

    /// Derived available balance. SUM(remaining_amount)
    /// over the shared predicate `status='active' AND remaining_amount>0 AND
    /// (effective_at IS NULL OR effective_at<=now) AND (expires_at IS NULL OR
    /// expires_at>now)` grouped by `credit_type`. Same source as consumption
    /// selection — "seen balance == spendable balance". Replaces reading
    /// `points_wallets` Stored balance columns for available-balance semantics.
    ///
    /// `bucket_ids` semantics: empty slice ⟺ aggregate across ALL the user's
    /// buckets (for `get_balance`'s user-total view); non-empty ⟺ restrict to
    /// the listed buckets (for per-bucket grant responses). The infra impl
    /// maps empty to "no `bucket_id` filter".
    fn compute_available_balance(
        &self,
        realm_id: &str,
        user_id: Uuid,
        bucket_ids: &[Uuid],
        now: chrono::DateTime<chrono::Utc>,
    ) -> impl Future<Output = Result<Vec<(CreditType, i64)>, CoreError>> + Send;

    /// Derived available balance broken down by `(bucket_id, credit_type)`.
    /// Same predicate as `compute_available_balance`, used by
    /// bucket overview / bucket delete guard so they no longer read
    /// `points_wallets.total_balance` (avoids future-effective leakage and
    /// bucket mis-judgement).
    fn compute_bucket_available_balances(
        &self,
        realm_id: &str,
        bucket_ids: &[Uuid],
        now: chrono::DateTime<chrono::Utc>,
    ) -> impl Future<Output = Result<Vec<(Uuid, CreditType, i64)>, CoreError>> + Send;

    /// Pre-grant the next period for a schedule. Writes a
    /// ledger row carrying `effective_at`/`expires_at` PLUS a
    /// `points_grant_records(schedule_id, period_number)` row (UNIQUE
    /// idempotency) linked to the new ledger via `ledger_id` FK. Idempotent:
    /// re-call for the same `(schedule_id, period_number)` returns the
    /// existing ledger row without re-writing. Subscription and free-periodic
    /// pre-grant share this port; `effective_at` anchors to period start
    /// (subscription `period_start` / free-period `next_grant_time`).
    fn pregrant_next_period_atomic(
        &self,
        realm_id: &str,
        schedule: &crate::points::PointsGrantSchedule,
        period_number: u32,
        effective_at: Option<chrono::DateTime<chrono::Utc>>,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> impl Future<Output = Result<PointsCreditLedger, CoreError>> + Send;

    /// Scan for schedules whose next pre-grant is due. Returns
    /// candidates whose `next_grant_time` is within the caller's per-row
    /// `lead_time` window; caller (worker `PointsPreGrantJob`) re-checks
    /// per-row against its `lead_time_map`. Used as a belt-and-braces warming
    /// scan — correctness is not gated on this port.
    fn find_schedules_due_for_pregrant(
        &self,
        before: chrono::DateTime<chrono::Utc>,
        limit: u64,
    ) -> impl Future<Output = Result<Vec<crate::points::PointsGrantSchedule>, CoreError>> + Send;

    /// Single-user free-periodic due schedule scan for read-path realization.
    /// `WHERE realm_id AND user_id AND active AND
    /// subscription_id IS NULL AND next_grant_time <= before` (lead_time=0,
    /// only already-due periods). Single-user scope avoids cross-user lock
    /// contention; subscription schedules are excluded so the request path
    /// never guesses paid-grant fulfillment.
    fn find_due_free_grant_schedules_for_user(
        &self,
        realm_id: &str,
        user_id: Uuid,
        before: chrono::DateTime<chrono::Utc>,
        limit: u64,
    ) -> impl Future<Output = Result<Vec<crate::points::PointsGrantSchedule>, CoreError>> + Send;

    /// Row-level reclaim of a pre-granted ledger row. Sets
    /// the resolved ledger row to `status='revoked'` and
    /// `revoked_amount += remaining_amount`; derived balance auto-excludes it,
    /// so no wallet back-adjustment is needed. Returns the number of rows
    /// affected (0 if the locator did not resolve — caller treats as
    /// idempotent no-op or surfaces per reclaim policy). Used by webhook reclaim
    /// via the trait (cross-crate, cannot call infra private
    /// helpers directly).
    fn revoke_pregrant_ledger_row_atomic(
        &self,
        realm_id: &str,
        locator: ReclaimLocator,
        reason: &str,
    ) -> impl Future<Output = Result<usize, CoreError>> + Send;
}
