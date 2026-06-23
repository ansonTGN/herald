// PostgreSQL implementation of Points Repository

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use sqlx::{FromRow, PgPool, Postgres, Row, Transaction};
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

use herald_domain::common::entities::app_errors::CoreError;
use herald_domain::points::{
    dtos::RevokePointsOutput,
    entities::{
        ConsumptionAllocationView, CreditLedgerStatus, CreditSourceType, CreditType, Paginated,
        PointsConsumptionAllocation, PointsCreditLedger, PointsRevocationRecord, PointsTransaction,
        PointsWallet, RevocationType, TransactionType, WalletStatus,
    },
    errors::PointsErrorExt,
    expiration_service::ExpirationSummary,
    ports::{
        LedgerFilters, LedgerUpdate, PointsRepository, ReclaimLocator, TransactionFilters,
        WalletDelta, WalletFilters,
    },
};
// Import mapping functions for ORM conversions
use crate::points::{
    points_consumption_allocation_from_model, points_consumption_allocation_to_active_model,
    points_credit_ledger_from_model, points_credit_ledger_to_active_model,
    points_revocation_record_from_model, points_revocation_record_to_active_model,
};
use herald_entity::{
    account, points_consumption_allocation, points_credit_ledger, points_grant_record,
    points_grant_schedule, points_revocation_record, points_transaction, points_wallet,
    realm_default_config, user_points_config,
};

/// Custom struct for SQLx query results from points_wallets table
/// Implements FromRow to work with sqlx::query_as
///
/// Design §1.3 / A7 (BE-D11): the 5 per-type balance fields and
/// `total_balance` are gone; only the 4 lifetime analytics columns remain.
#[derive(Debug, FromRow)]
struct PointsWalletRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub realm_id: String,
    pub bucket_id: Uuid,
    pub total_topup_granted: i64,
    pub total_subscription_granted: i64,
    pub total_recharged: i64,
    pub total_consumed: i64,
    pub status: String,
    pub created_at: sea_orm::prelude::DateTimeWithTimeZone,
    pub updated_at: sea_orm::prelude::DateTimeWithTimeZone,
}

/// Custom struct for SQLx query results from points_transactions table
/// Implements FromRow to work with sqlx::query_as
#[allow(dead_code)]
#[derive(Debug, FromRow)]
struct PointsTransactionRow {
    pub id: Uuid,
    pub realm_id: String,
    pub wallet_id: Uuid,
    pub user_id: Uuid,
    pub bucket_id: Uuid,
    pub r#type: String,
    pub amount: i64,
    pub balance_after: i64,
    pub topup_balance_after: Option<i64>,
    pub subscription_balance_after: Option<i64>,
    pub credit_type: Option<String>,
    pub description: Option<String>,
    pub client_app_id: Option<Uuid>,
    pub subscription_id: Option<Uuid>,
    pub external_ref_id: Option<String>,
    pub correlation_id: Option<String>,
    /// Sourced via LEFT JOIN to `points_credit_ledger.effective_at` for
    /// grant-type transactions (design §4.2 / §5.1, BE-D10/BE-D11). `None`
    /// for consume/refund/revoke/expiration or when the ledger row's
    /// `effective_at` is NULL (immediately available). The domain
    /// `PointsTransaction.effective_at` is populated from this column.
    ///
    /// `#[sqlx(default)]` so `SELECT * FROM points_transactions` (which cannot
    /// see the JOIN column) still deserializes — the value defaults to `None`
    /// when absent, which is the correct semantics for the write-side replay
    /// paths (`replay_consume_by_primary`, refund idempotency check) that do
    /// not need effective_at. Only the admin/audit `find_transactions` read
    /// path supplies the column via an explicit LEFT JOIN.
    #[sqlx(default)]
    pub effective_at: Option<sea_orm::prelude::DateTimeWithTimeZone>,
    pub created_at: sea_orm::prelude::DateTimeWithTimeZone,
    pub updated_at: sea_orm::prelude::DateTimeWithTimeZone,
    pub expires_at: Option<sea_orm::prelude::DateTimeWithTimeZone>,
}

#[derive(Debug, FromRow)]
struct PointsCreditLedgerRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub realm_id: String,
    pub bucket_id: Uuid,
    pub credit_type: String,
    pub source_type: String,
    pub source_id: String,
    pub granted_amount: i64,
    pub used_amount: i64,
    pub revoked_amount: i64,
    pub remaining_amount: i64,
    pub expires_at: Option<sea_orm::prelude::DateTimeWithTimeZone>,
    /// Expected effective time (design §4.3.2 / §5.1). NULL ⟺ immediately
    /// available; non-null ⟺ gated by the `(effective_at IS NULL OR
    /// effective_at <= NOW())` predicate in consumption selection and derived
    /// balance.
    pub effective_at: Option<sea_orm::prelude::DateTimeWithTimeZone>,
    pub status: String,
    pub created_at: sea_orm::prelude::DateTimeWithTimeZone,
    pub updated_at: sea_orm::prelude::DateTimeWithTimeZone,
}

/// Row captured during pre-grant reclaim (BE-TR production fix). `yanked` is
/// the row's pre-update `remaining_amount` — the unused portion this reclaim
/// moves into `revoked_amount`. `points_credit_ledger.remaining_amount` is a
/// GENERATED column (`granted_amount - used_amount - revoked_amount`), so it
/// regenerates to 0 the instant `revoked_amount` increases; reading it back
/// from `RETURNING *` would yield 0 and the shortfall record would be written
/// with `revoked_amount = 0`, violating
/// `points_revocation_records.revoked_amount > 0`. The reclaim UPDATE uses a
/// CTE that locks + captures the pre-update value so the debt record carries
/// the real yanked portion (design A4).
#[derive(Debug, FromRow)]
struct ReclaimTargetRow {
    id: Uuid,
    user_id: Uuid,
    realm_id: String,
    source_id: String,
    used_amount: i64,
    yanked: i64,
}

/// Pure output of [`PostgresPointsRepository::plan_consume_allocation`].
/// Describes how a consume request splits across the locked ledgers, independent
/// of wallet/transaction writes so it can be unit-tested without Postgres.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ConsumePlan {
    allocations: Vec<PlannedAllocation>,
    fully_covers: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlannedAllocation {
    /// Index into the ledgers slice passed to `plan_consume_allocation`.
    ledger_index: usize,
    amount: i64,
}

/// PostgreSQL implementation of PointsRepository
pub struct PostgresPointsRepository {
    db: Arc<DatabaseConnection>,
    pool: PgPool,
}

/// Unique constraint names in the database
mod constraints {
    pub const UK_POINTS_WALLETS_USER_ID: &str = "uk_points_wallets_user_id";
}

impl PostgresPointsRepository {
    pub fn new(db: Arc<DatabaseConnection>, pool: PgPool) -> Self {
        Self { db, pool }
    }

    /// Convert database model to domain PointsWallet
    fn model_to_points_wallet(model: points_wallet::Model) -> Result<PointsWallet, CoreError> {
        // Design §1.3 / A7 (BE-D11): balance columns gone; analytics only.
        Ok(PointsWallet {
            id: model.id,
            user_id: model.user_id,
            realm_id: model.realm_id,
            bucket_id: Some(model.bucket_id),
            total_topup_granted: model.total_topup_granted,
            total_subscription_granted: model.total_subscription_granted,
            total_recharged: model.total_recharged,
            total_consumed: model.total_consumed,
            status: model.status.parse()?,
            created_at: chrono::DateTime::from(model.created_at),
            updated_at: chrono::DateTime::from(model.updated_at),
        })
    }

    /// Convert PointsWalletRow to domain PointsWallet
    fn row_to_points_wallet(row: PointsWalletRow) -> Result<PointsWallet, CoreError> {
        use herald_domain::points::entities::WalletStatus;

        let status = WalletStatus::from_str(&row.status)
            .map_err(|_| CoreError::BadRequest(format!("Invalid wallet status: {}", row.status)))?;

        Ok(PointsWallet {
            id: row.id,
            user_id: row.user_id,
            realm_id: row.realm_id,
            bucket_id: Some(row.bucket_id),
            total_topup_granted: row.total_topup_granted,
            total_subscription_granted: row.total_subscription_granted,
            total_recharged: row.total_recharged,
            total_consumed: row.total_consumed,
            status,
            created_at: chrono::DateTime::from(row.created_at),
            updated_at: chrono::DateTime::from(row.updated_at),
        })
    }

    /// Convert domain PointsWallet to database active model
    fn points_wallet_to_active_model(account: PointsWallet) -> points_wallet::ActiveModel {
        points_wallet::ActiveModel {
            id: Set(account.id),
            user_id: Set(account.user_id),
            realm_id: Set(account.realm_id.clone()),
            bucket_id: Set(account
                .bucket_id
                .expect("bucket_id is required for wallet persistence")),
            total_topup_granted: Set(account.total_topup_granted),
            total_subscription_granted: Set(account.total_subscription_granted),
            total_recharged: Set(account.total_recharged),
            total_consumed: Set(account.total_consumed),
            status: Set(account.status.as_str().to_string()),
            created_at: Set(sea_orm::prelude::DateTimeWithTimeZone::from(
                account.created_at,
            )),
            updated_at: Set(sea_orm::prelude::DateTimeWithTimeZone::from(
                account.updated_at,
            )),
        }
    }

    /// Convert database model to domain PointsTransaction
    fn model_to_points_transaction(
        model: points_transaction::Model,
    ) -> Result<PointsTransaction, CoreError> {
        use herald_domain::points::entities::CreditType;

        let credit_type = match model.credit_type {
            Some(ref ct) => Some(CreditType::from_str(ct)?),
            None => None,
        };

        Ok(PointsTransaction {
            id: model.id,
            wallet_id: model.wallet_id,
            user_id: model.user_id,
            realm_id: model.realm_id,
            bucket_id: model.bucket_id,
            transaction_type: model.r#type.parse()?,
            amount: model.amount,
            balance_after: model.balance_after,
            topup_balance_after: model.topup_balance_after,
            subscription_balance_after: model.subscription_balance_after,
            credit_type,
            description: model.description,
            client_app_id: model.client_app_id,
            subscription_id: model.subscription_id,
            external_ref_id: model.external_ref_id,
            correlation_id: model.correlation_id,
            // SeaORM `points_transaction::Model` has no `effective_at` column
            // (the value lives on `points_credit_ledger`); callers needing it
            // must use the raw-SQL path (`points_transaction_row_to_domain`)
            // which JOINs the ledger. None here is correct for SeaORM-sourced
            // reads (e.g. `find_transaction_by_id`); the admin/audit
            // `list_transactions` path uses the JOIN-aware raw SQL.
            effective_at: None,
            created_at: chrono::DateTime::from(model.created_at),
        })
    }

    /// Convert SQLx query result row to domain PointsTransaction
    fn points_transaction_row_to_domain(
        row: PointsTransactionRow,
    ) -> Result<PointsTransaction, CoreError> {
        use herald_domain::points::entities::CreditType;

        let credit_type = match row.credit_type {
            Some(ref ct) => Some(CreditType::from_str(ct)?),
            None => None,
        };

        Ok(PointsTransaction {
            id: row.id,
            wallet_id: row.wallet_id,
            user_id: row.user_id,
            realm_id: row.realm_id,
            bucket_id: row.bucket_id,
            transaction_type: row.r#type.parse()?,
            amount: row.amount,
            balance_after: row.balance_after,
            topup_balance_after: row.topup_balance_after,
            subscription_balance_after: row.subscription_balance_after,
            credit_type,
            description: row.description,
            client_app_id: row.client_app_id,
            subscription_id: row.subscription_id,
            external_ref_id: row.external_ref_id,
            correlation_id: row.correlation_id.clone(),
            // BE-D10/BE-D11 Part B: sourced via LEFT JOIN to
            // `points_credit_ledger.effective_at` in the raw-SQL queries that
            // populate this row (see `find_transactions`, refund replay).
            effective_at: row.effective_at.map(chrono::DateTime::from),
            created_at: chrono::DateTime::from(row.created_at),
        })
    }

    /// Convert domain PointsTransaction to database active model
    fn points_transaction_to_active_model(
        transaction: PointsTransaction,
    ) -> points_transaction::ActiveModel {
        let credit_type_str = transaction
            .credit_type
            .as_ref()
            .map(|ct| ct.as_str().to_string());

        points_transaction::ActiveModel {
            id: Set(transaction.id),
            wallet_id: Set(transaction.wallet_id),
            user_id: Set(transaction.user_id),
            realm_id: Set(transaction.realm_id),
            bucket_id: Set(transaction.bucket_id),
            r#type: Set(transaction.transaction_type.as_str().to_string()),
            amount: Set(transaction.amount),
            balance_after: Set(transaction.balance_after),
            topup_balance_after: Set(transaction.topup_balance_after),
            subscription_balance_after: Set(transaction.subscription_balance_after),
            credit_type: Set(credit_type_str),
            description: Set(transaction.description),
            client_app_id: Set(transaction.client_app_id),
            subscription_id: Set(transaction.subscription_id),
            external_ref_id: Set(transaction.external_ref_id),
            correlation_id: Set(transaction.correlation_id),
            created_at: Set(sea_orm::prelude::DateTimeWithTimeZone::from(
                transaction.created_at,
            )),
        }
    }

    fn model_to_user_points_config(
        model: user_points_config::Model,
    ) -> herald_domain::points::user_config::UserPointsConfig {
        // Parse grant period type if present
        let free_periodic_grant_period_type = model.free_periodic_grant_period_type.and_then(|s| {
            s.parse::<herald_domain::points::grant_schedule::GrantPeriodType>()
                .map_err(|e| {
                    tracing::error!(
                        "Failed to parse free_periodic_grant_period_type '{}': {}",
                        s,
                        e
                    );
                    e
                })
                .ok()
        });

        herald_domain::points::user_config::UserPointsConfig {
            user_id: model.user_id,
            realm_id: model.realm_id,
            registration_bonus_points: model.registration_bonus_points,
            free_periodic_points_amount: model.free_periodic_points_amount,
            free_periodic_grant_period_type,
            free_periodic_validity_days: model.free_periodic_validity_days,
            next_grant_time: model.next_grant_time.map(chrono::DateTime::from),
            granted_periods: model.granted_periods,
            grant_schedule_id: model.grant_schedule_id,
            created_at: chrono::DateTime::from(model.created_at),
            updated_at: chrono::DateTime::from(model.updated_at),
        }
    }

    fn model_to_grant_schedule(
        model: points_grant_schedule::Model,
    ) -> Result<herald_domain::points::grant_schedule::PointsGrantSchedule, CoreError> {
        let grant_period_type = herald_domain::points::grant_schedule::GrantPeriodType::from_str(
            &model.grant_period_type,
        )?;

        Ok(herald_domain::points::grant_schedule::PointsGrantSchedule {
            id: model.id,
            user_id: model.user_id,
            realm_id: model.realm_id,
            bucket_id: model.bucket_id,
            subscription_id: model.subscription_id,
            entitlement_key: Some(model.entitlement_key),
            grant_period_type,
            base_time: chrono::DateTime::from(model.base_time),
            next_grant_time: chrono::DateTime::from(model.next_grant_time),
            points_per_period: model.points_per_period,
            validity_days: model.validity_days,
            granted_periods: model.granted_periods,
            max_periods: model.max_periods,
            active: model.active,
            created_at: chrono::DateTime::from(model.created_at),
            updated_at: chrono::DateTime::from(model.updated_at),
        })
    }

    /// Apply transaction filters to a query (shared by find_transactions and count_transactions)
    fn apply_transaction_filters(
        mut query: sea_orm::Select<points_transaction::Entity>,
        filters: &TransactionFilters,
    ) -> sea_orm::Select<points_transaction::Entity> {
        if let Some(user_id) = filters.user_id {
            query = query.filter(points_transaction::Column::UserId.eq(user_id));
        }

        if let Some(bucket_id) = filters.bucket_id {
            query = query.filter(points_transaction::Column::BucketId.eq(bucket_id));
        }

        if let Some(transaction_type) = &filters.transaction_type {
            query = query
                .filter(points_transaction::Column::Type.eq(transaction_type.as_str().to_string()));
        }

        if let Some(client_app_id) = filters.client_app_id {
            query = query.filter(points_transaction::Column::ClientAppId.eq(client_app_id));
        }

        if let Some(subscription_id) = filters.subscription_id {
            query = query.filter(points_transaction::Column::SubscriptionId.eq(subscription_id));
        }

        if !filters.external_ref_id.is_empty() {
            query = query
                .filter(points_transaction::Column::ExternalRefId.eq(&filters.external_ref_id));
        }

        if let Some(start_time) = &filters.start_time {
            query = query.filter(
                points_transaction::Column::CreatedAt
                    .gte(sea_orm::prelude::DateTimeWithTimeZone::from(*start_time)),
            );
        }

        if let Some(end_time) = &filters.end_time {
            query = query.filter(
                points_transaction::Column::CreatedAt
                    .lte(sea_orm::prelude::DateTimeWithTimeZone::from(*end_time)),
            );
        }

        query
    }

    async fn apply_wallet_filters(
        &self,
        mut query: sea_orm::Select<points_wallet::Entity>,
        realm_id: &str,
        filters: &WalletFilters,
    ) -> Result<Option<sea_orm::Select<points_wallet::Entity>>, CoreError> {
        if let Some(user_id) = filters.user_id {
            query = query.filter(points_wallet::Column::UserId.eq(user_id));
        }

        if let Some(status) = &filters.status {
            query = query.filter(points_wallet::Column::Status.eq(status.clone()));
        }

        if let Some(bucket_id) = filters.bucket_id {
            query = query.filter(points_wallet::Column::BucketId.eq(bucket_id));
        }

        if let Some(search) = &filters.search {
            if let Ok(user_id) = Uuid::parse_str(search) {
                query = query.filter(points_wallet::Column::UserId.eq(user_id));
            } else {
                let user_id: Option<Uuid> = account::Entity::find()
                    .select_only()
                    .column(account::Column::Id)
                    .filter(account::Column::RealmId.eq(realm_id.to_string()))
                    .filter(account::Column::Email.eq(search.clone()))
                    .into_tuple::<Uuid>()
                    .one(&*self.db)
                    .await
                    .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

                if let Some(uid) = user_id {
                    query = query.filter(points_wallet::Column::UserId.eq(uid));
                } else {
                    return Ok(None);
                }
            }
        }

        Ok(Some(query))
    }
    fn row_to_points_credit_ledger(
        row: PointsCreditLedgerRow,
    ) -> Result<PointsCreditLedger, CoreError> {
        Ok(PointsCreditLedger {
            id: row.id,
            user_id: row.user_id,
            realm_id: row.realm_id,
            bucket_id: row.bucket_id,
            credit_type: row.credit_type.parse()?,
            source_type: row.source_type.parse()?,
            source_id: row.source_id,
            granted_amount: row.granted_amount,
            used_amount: row.used_amount,
            revoked_amount: row.revoked_amount,
            remaining_amount: row.remaining_amount,
            expires_at: row.expires_at.map(chrono::DateTime::from),
            effective_at: row.effective_at.map(chrono::DateTime::from),
            status: row.status.parse()?,
            created_at: chrono::DateTime::from(row.created_at),
            updated_at: chrono::DateTime::from(row.updated_at),
        })
    }

    /// Find a wallet for a specific (user, bucket) pair, locking the row for update
    /// (design §5.2, A4 single-wallet cleanup).
    async fn find_wallet_by_user_bucket_for_update(
        tx: &mut Transaction<'_, Postgres>,
        realm_id: &str,
        user_id: Uuid,
        bucket_id: Uuid,
    ) -> Result<Option<PointsWallet>, CoreError> {
        let row = sqlx::query_as::<_, PointsWalletRow>(
            r#"
            SELECT * FROM points_wallets
            WHERE realm_id = $1 AND user_id = $2 AND bucket_id = $3
            FOR UPDATE
            "#,
        )
        .bind(realm_id)
        .bind(user_id)
        .bind(bucket_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        row.map(Self::row_to_points_wallet).transpose()
    }

    async fn create_wallet_in_tx(
        tx: &mut Transaction<'_, Postgres>,
        realm_id: &str,
        user_id: Uuid,
        bucket_id: Uuid,
    ) -> Result<PointsWallet, CoreError> {
        // Use ON CONFLICT to handle concurrent wallet creation:
        // two concurrent requests for the same (user, bucket) may both see no wallet,
        // then both try to INSERT. ON CONFLICT returns the existing row instead.
        let row = sqlx::query_as::<_, PointsWalletRow>(
            r#"
            INSERT INTO points_wallets (
                id, user_id, realm_id, bucket_id,
                total_recharged, total_consumed, total_topup_granted,
                total_subscription_granted, status, created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, 0, 0, 0, 0, 'active', NOW(), NOW()
            )
            ON CONFLICT (realm_id, user_id, bucket_id) DO NOTHING
            RETURNING *
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(user_id)
        .bind(realm_id)
        .bind(bucket_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        // If ON CONFLICT DO NOTHING suppressed the INSERT, fetch the existing wallet
        let row = match row {
            Some(r) => r,
            None => sqlx::query_as::<_, PointsWalletRow>(
                "SELECT * FROM points_wallets WHERE realm_id = $1 AND user_id = $2 AND bucket_id = $3 FOR UPDATE",
            )
            .bind(realm_id)
            .bind(user_id)
            .bind(bucket_id)
            .fetch_one(&mut **tx)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?,
        };

        Self::row_to_points_wallet(row)
    }

    fn determine_transaction_type(
        credit_type: CreditType,
        source_type: CreditSourceType,
    ) -> TransactionType {
        match (credit_type, source_type) {
            (CreditType::RegistrationCredit, _) => TransactionType::RegistrationGrant,
            (CreditType::FreePeriodicCredit, _) => TransactionType::FreePeriodicGrant,
            (CreditType::SubscriptionCredit, CreditSourceType::SubscriptionInitial) => {
                TransactionType::SubscriptionGrant
            }
            (CreditType::SubscriptionCredit, CreditSourceType::SubscriptionRenewal) => {
                TransactionType::SubscriptionRenewal
            }
            (CreditType::SubscriptionCredit, CreditSourceType::SubscriptionUpgrade) => {
                TransactionType::SubscriptionUpgrade
            }
            (CreditType::SubscriptionCredit, CreditSourceType::SubscriptionDowngrade) => {
                TransactionType::SubscriptionDowngrade
            }
            (CreditType::GrantedCredit, CreditSourceType::AdminGrant)
            | (CreditType::GrantedCredit, CreditSourceType::SdkGrant) => TransactionType::Grant,
            _ => TransactionType::Recharge,
        }
    }

    /// Ensure a wallet exists for the (user, bucket) pair within a transaction
    /// (design §5.2). All write paths operate on an explicit bucket; there is no
    /// single-wallet fallback.
    async fn ensure_wallet_in_tx(
        tx: &mut Transaction<'_, Postgres>,
        realm_id: &str,
        user_id: Uuid,
        bucket_id: Uuid,
    ) -> Result<PointsWallet, CoreError> {
        match Self::find_wallet_by_user_bucket_for_update(tx, realm_id, user_id, bucket_id).await? {
            Some(wallet) => Ok(wallet),
            None => Self::create_wallet_in_tx(tx, realm_id, user_id, bucket_id).await,
        }
    }

    async fn find_active_ledgers_by_expiration_for_update(
        tx: &mut Transaction<'_, Postgres>,
        realm_id: &str,
        user_id: Uuid,
        bucket_ids: &[Uuid],
    ) -> Result<Vec<PointsCreditLedger>, CoreError> {
        // Empty covered set is a caller bug; resolved higher up as
        // NoCoveredPointsPool. Guard against accidental full-table lock.
        if bucket_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = sqlx::query_as::<_, PointsCreditLedgerRow>(
            r#"
            SELECT * FROM points_credit_ledger
            WHERE realm_id = $1
              AND user_id = $2
              AND bucket_id = ANY($3)
              AND status = 'active'
              AND remaining_amount > 0
              AND (expires_at IS NULL OR expires_at > NOW())
              AND (effective_at IS NULL OR effective_at <= NOW())
            ORDER BY expires_at ASC NULLS LAST, created_at ASC
            FOR UPDATE
            "#,
        )
        .bind(realm_id)
        .bind(user_id)
        .bind(bucket_ids)
        .fetch_all(&mut **tx)
        .await
        .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        rows.into_iter()
            .map(Self::row_to_points_credit_ledger)
            .collect()
    }

    /// Pure allocation plan for a multi-pool consume (design §5.1 step 5).
    ///
    /// Inputs: the already-locked active ledgers in `expires_at ASC NULLS LAST,
    /// created_at ASC` order, and the requested `amount`. Output is a sequence of
    /// `(ledger_index, allocated_amount)` covering the request greedily in expiry
    /// order, plus a `fully_covers` flag.
    ///
    /// This is deliberately free of DB / wallet concerns so the cross-pool split,
    /// permanent-pool-last ordering, partial-coverage rejection and exact-amount
    /// boundary can be unit-tested without Postgres (BE-D04 testing requirement).
    fn plan_consume_allocation(ledgers: &[PointsCreditLedger], amount: i64) -> ConsumePlan {
        let mut allocations: Vec<PlannedAllocation> = Vec::new();
        let mut remaining = amount;
        for (index, ledger) in ledgers.iter().enumerate() {
            if remaining <= 0 {
                break;
            }
            if ledger.remaining_amount <= 0 {
                continue;
            }
            let take = ledger.remaining_amount.min(remaining);
            allocations.push(PlannedAllocation {
                ledger_index: index,
                amount: take,
            });
            remaining -= take;
        }
        let fully_covers = remaining <= 0;
        ConsumePlan {
            allocations,
            fully_covers,
        }
    }

    /// Resolve the covered Bucket set for a client app (design §5.1 step 3).
    /// Explicit `credit_bucket_client_apps` rows joined to enabled
    /// `credit_buckets` only — no default-bucket merging (A4). Sorted ascending
    /// for deterministic lock ordering downstream.
    async fn find_covered_bucket_ids_in_tx(
        tx: &mut Transaction<'_, Postgres>,
        realm_id: &str,
        client_app_id: Uuid,
    ) -> Result<Vec<Uuid>, CoreError> {
        let rows: Vec<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT bca.bucket_id
            FROM credit_bucket_client_apps bca
            JOIN credit_buckets b ON b.id = bca.bucket_id
            WHERE bca.realm_id = $1
              AND bca.client_app_id = $2
              AND b.enabled = true
            ORDER BY bca.bucket_id ASC
            "#,
        )
        .bind(realm_id)
        .bind(client_app_id)
        .fetch_all(&mut **tx)
        .await
        .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// Idempotency replay (design §5.1 step 1). Given the primary
    /// transaction_id stored on `idempotency_keys`, reassemble the original
    /// consume result set. Multi-pool rows share a `correlation_id` → fetch all
    /// sibling transactions ordered by bucket_id. Legacy single-pool rows have
    /// NULL `correlation_id` → return just the primary transaction.
    /// Allocations are NOT re-fetched here; this fn returns the original
    /// transaction rows so the caller can hand them back verbatim without
    /// re-deducting.
    async fn replay_consume_by_primary(
        tx: &mut Transaction<'_, Postgres>,
        realm_id: &str,
        primary_txn_id: Uuid,
    ) -> Result<Vec<PointsTransaction>, CoreError> {
        // Read the primary transaction to discover its correlation_id.
        let primary_row = sqlx::query_as::<_, PointsTransactionRow>(
            r#"
            SELECT * FROM points_transactions
            WHERE realm_id = $1 AND id = $2
            "#,
        )
        .bind(realm_id)
        .bind(primary_txn_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|e| CoreError::DatabaseError(e.to_string()))?
        .ok_or(CoreError::NotFound)?;

        let primary = Self::points_transaction_row_to_domain(primary_row)?;

        match &primary.correlation_id {
            // Legacy single-pool consume row (pre multi-pool) → return as-is.
            None => Ok(vec![primary]),
            Some(correlation_id) => {
                let rows = sqlx::query_as::<_, PointsTransactionRow>(
                    r#"
                    SELECT * FROM points_transactions
                    WHERE realm_id = $1 AND correlation_id = $2
                    ORDER BY bucket_id ASC, created_at ASC
                    "#,
                )
                .bind(realm_id)
                .bind(correlation_id)
                .fetch_all(&mut **tx)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
                rows.into_iter()
                    .map(Self::points_transaction_row_to_domain)
                    .collect()
            }
        }
    }

    async fn find_active_ledgers_by_credit_type_for_update(
        tx: &mut Transaction<'_, Postgres>,
        realm_id: &str,
        user_id: Uuid,
        credit_type: CreditType,
    ) -> Result<Vec<PointsCreditLedger>, CoreError> {
        let rows = sqlx::query_as::<_, PointsCreditLedgerRow>(
            r#"
            SELECT * FROM points_credit_ledger
            WHERE realm_id = $1
              AND user_id = $2
              AND credit_type = $3
              AND status = 'active'
              AND remaining_amount > 0
            ORDER BY created_at ASC
            FOR UPDATE
            "#,
        )
        .bind(realm_id)
        .bind(user_id)
        .bind(credit_type.to_string())
        .fetch_all(&mut **tx)
        .await
        .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        rows.into_iter()
            .map(Self::row_to_points_credit_ledger)
            .collect()
    }

    async fn find_active_ledgers_by_credit_type_and_bucket_for_update(
        tx: &mut Transaction<'_, Postgres>,
        realm_id: &str,
        user_id: Uuid,
        bucket_id: Uuid,
        credit_type: CreditType,
    ) -> Result<Vec<PointsCreditLedger>, CoreError> {
        let rows = sqlx::query_as::<_, PointsCreditLedgerRow>(
            r#"
            SELECT * FROM points_credit_ledger
            WHERE realm_id = $1
              AND user_id = $2
              AND bucket_id = $3
              AND credit_type = $4
              AND status = 'active'
              AND remaining_amount > 0
            ORDER BY created_at ASC
            FOR UPDATE
            "#,
        )
        .bind(realm_id)
        .bind(user_id)
        .bind(bucket_id)
        .bind(credit_type.to_string())
        .fetch_all(&mut **tx)
        .await
        .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        rows.into_iter()
            .map(Self::row_to_points_credit_ledger)
            .collect()
    }

    async fn find_expired_ledgers_for_update(
        tx: &mut Transaction<'_, Postgres>,
        expiration_time: chrono::DateTime<chrono::Utc>,
        limit: usize,
    ) -> Result<Vec<PointsCreditLedger>, CoreError> {
        let rows = sqlx::query_as::<_, PointsCreditLedgerRow>(
            r#"
            SELECT * FROM points_credit_ledger
            WHERE expires_at <= $1
              AND status = 'active'
              AND remaining_amount > 0
            ORDER BY expires_at ASC
            LIMIT $2
            FOR UPDATE
            "#,
        )
        .bind(expiration_time)
        .bind(limit as i64)
        .fetch_all(&mut **tx)
        .await
        .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        rows.into_iter()
            .map(Self::row_to_points_credit_ledger)
            .collect()
    }

    /// SINGLE writer of the `points_wallets` lifetime analytics columns.
    ///
    /// Design §1.3 / A7 (BE-D11): the 5 per-type balance columns and
    /// `total_balance` were physically dropped; available balance is now a
    /// derived SUM over `points_credit_ledger`. This fn only advances the 4
    /// monotonic analytics columns (`total_recharged` / `total_consumed` /
    /// `total_topup_granted` / `total_subscription_granted`) inside the same
    /// transaction as the ledger mutation, so analytics stay drift-free.
    ///
    /// MUST be the only writer of these columns — any future direct ledger
    /// write that bypasses this fn re-introduces analytics drift.
    ///
    /// Returns the post-delta row. Callers that need the post-mutation
    /// available balance for `balance_after` on a `points_transactions` row
    /// must call `compute_available_balance_in_tx` (same predicate as the
    /// derived balance) — this fn no longer returns any balance.
    async fn apply_wallet_delta_in_tx(
        tx: &mut Transaction<'_, Postgres>,
        wallet_id: Uuid,
        delta: WalletDelta,
    ) -> Result<PointsWallet, CoreError> {
        let row = sqlx::query_as::<_, PointsWalletRow>(
            r#"
            UPDATE points_wallets
               SET total_recharged            = total_recharged            + $2,
                   total_consumed             = total_consumed             + $3,
                   total_topup_granted        = total_topup_granted        + $4,
                   total_subscription_granted = total_subscription_granted + $5,
                   updated_at                 = NOW()
             WHERE id = $1
             RETURNING *
            "#,
        )
        .bind(wallet_id)
        .bind(delta.total_recharged)
        .bind(delta.total_consumed)
        .bind(delta.total_topup_granted)
        .bind(delta.total_subscription_granted)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|e| CoreError::DatabaseError(e.to_string()))?
        .ok_or(CoreError::NotFound)?;
        Self::row_to_points_wallet(row)
    }

    /// In-transaction derived available balance (design §5.1 / A7, BE-D11).
    /// Same predicate as `compute_available_balance` (the public port uses
    /// `&self.pool`; this variant runs against the caller's open tx so the
    /// post-mutation snapshot reflects uncommitted ledger writes). Used to
    /// fill `points_transactions.balance_after` (+typed snapshots) with the
    /// REAL derived value at grant/consume/refund/revoke write time, replacing
    /// the old `updated_wallet.total_balance` source that no longer exists.
    ///
    /// `bucket_ids` empty ⟺ aggregate across ALL the user's buckets; non-empty
    /// ⟺ restrict to listed buckets (used by per-bucket consume loop).
    async fn compute_available_balance_in_tx(
        tx: &mut Transaction<'_, Postgres>,
        realm_id: &str,
        user_id: Uuid,
        bucket_ids: &[Uuid],
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<(CreditType, i64)>, CoreError> {
        let rows: Vec<(String, i64)> = if bucket_ids.is_empty() {
            sqlx::query_as(
                r#"
                SELECT credit_type, COALESCE(SUM(remaining_amount), 0)::BIGINT AS available
                FROM points_credit_ledger
                WHERE realm_id = $1
                  AND user_id = $2
                  AND status = 'active'
                  AND remaining_amount > 0
                  AND (effective_at IS NULL OR effective_at <= $3)
                  AND (expires_at  IS NULL OR expires_at  >  $3)
                GROUP BY credit_type
                "#,
            )
            .bind(realm_id)
            .bind(user_id)
            .bind(now)
            .fetch_all(&mut **tx)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?
        } else {
            sqlx::query_as(
                r#"
                SELECT credit_type, COALESCE(SUM(remaining_amount), 0)::BIGINT AS available
                FROM points_credit_ledger
                WHERE realm_id = $1
                  AND user_id = $2
                  AND bucket_id = ANY($3)
                  AND status = 'active'
                  AND remaining_amount > 0
                  AND (effective_at IS NULL OR effective_at <= $4)
                  AND (expires_at  IS NULL OR expires_at  >  $4)
                GROUP BY credit_type
                "#,
            )
            .bind(realm_id)
            .bind(user_id)
            .bind(bucket_ids)
            .bind(now)
            .fetch_all(&mut **tx)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?
        };

        rows.into_iter()
            .map(|(credit_type, amount)| {
                let credit_type: CreditType = credit_type.parse().map_err(|_| {
                    CoreError::DatabaseError(format!("invalid credit_type: {credit_type}"))
                })?;
                Ok((credit_type, amount))
            })
            .collect()
    }

    /// Build `(balance_after, topup_balance_after, subscription_balance_after)`
    /// from a derived SUM result, for writing into `points_transactions`.
    /// `balance_after` = total across all credit types; typed snapshots are
    /// populated for topup/subscription and left `None` otherwise (matching
    /// the original write convention where only those two were snapshotted).
    fn derived_to_balance_snapshots(
        derived: &[(CreditType, i64)],
    ) -> (i64, Option<i64>, Option<i64>) {
        let mut total = 0i64;
        let mut topup = Some(0i64);
        let mut subscription = Some(0i64);
        for (credit_type, amount) in derived {
            total += amount;
            match credit_type {
                CreditType::TopupCredit => topup = Some(topup.unwrap_or(0) + amount),
                CreditType::SubscriptionCredit => {
                    subscription = Some(subscription.unwrap_or(0) + amount)
                }
                _ => {}
            }
        }
        (total, topup, subscription)
    }

    async fn update_ledger_in_tx(
        tx: &mut Transaction<'_, Postgres>,
        ledger_id: Uuid,
        updates: LedgerUpdate,
    ) -> Result<PointsCreditLedger, CoreError> {
        let ledger = sqlx::query_as::<_, PointsCreditLedgerRow>(
            "SELECT * FROM points_credit_ledger WHERE id = $1 FOR UPDATE",
        )
        .bind(ledger_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|e| CoreError::DatabaseError(e.to_string()))?
        .map(Self::row_to_points_credit_ledger)
        .transpose()?
        .ok_or_else(|| CoreError::BadRequest(format!("Ledger not found: {}", ledger_id)))?;

        let (used_amount, revoked_amount, expires_at, status) = match updates {
            LedgerUpdate::Consumption(amount) => {
                let used_amount = ledger.used_amount + amount;
                let remaining = ledger.granted_amount - used_amount - ledger.revoked_amount;
                if remaining < 0 {
                    return Err(CoreError::concurrent_modification());
                }
                (
                    used_amount,
                    ledger.revoked_amount,
                    ledger.expires_at,
                    if remaining == 0 {
                        CreditLedgerStatus::FullyUsed
                    } else {
                        ledger.status
                    },
                )
            }
            LedgerUpdate::Revocation(amount) => {
                let revoked_amount = ledger.revoked_amount + amount;
                let remaining = ledger.granted_amount - ledger.used_amount - revoked_amount;
                if remaining < 0 {
                    return Err(CoreError::concurrent_modification());
                }
                (
                    ledger.used_amount,
                    revoked_amount,
                    ledger.expires_at,
                    if remaining == 0 {
                        CreditLedgerStatus::Revoked
                    } else {
                        ledger.status
                    },
                )
            }
            LedgerUpdate::SetExpiration(expires_at) => (
                ledger.used_amount,
                ledger.revoked_amount,
                Some(expires_at),
                ledger.status,
            ),
            LedgerUpdate::SetStatus(status) => (
                ledger.used_amount,
                ledger.revoked_amount,
                ledger.expires_at,
                status,
            ),
        };

        let row = sqlx::query_as::<_, PointsCreditLedgerRow>(
            r#"
            UPDATE points_credit_ledger
            SET used_amount = $2,
                revoked_amount = $3,
                expires_at = $4,
                status = $5,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(ledger_id)
        .bind(used_amount)
        .bind(revoked_amount)
        .bind(expires_at)
        .bind(status.to_string())
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        Self::row_to_points_credit_ledger(row)
    }

    async fn create_transaction_in_tx(
        tx: &mut Transaction<'_, Postgres>,
        transaction: PointsTransaction,
    ) -> Result<PointsTransaction, CoreError> {
        let row = sqlx::query_as::<_, PointsTransactionRow>(
            r#"
            INSERT INTO points_transactions (
                id, wallet_id, user_id, realm_id, bucket_id, type, amount, balance_after,
                topup_balance_after, subscription_balance_after, credit_type, description,
                client_app_id, subscription_id, external_ref_id, correlation_id, created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8,
                $9, $10, $11, $12,
                $13, $14, $15, $16, $17, NOW()
            )
            RETURNING id, realm_id, wallet_id, user_id, bucket_id, type, amount, balance_after,
                      topup_balance_after, subscription_balance_after, credit_type,
                      description, client_app_id, subscription_id, external_ref_id, correlation_id,
                      NULL::timestamptz AS effective_at,
                      created_at, updated_at, expires_at
            "#,
        )
        .bind(transaction.id)
        .bind(transaction.wallet_id)
        .bind(transaction.user_id)
        .bind(transaction.realm_id)
        .bind(transaction.bucket_id)
        .bind(transaction.transaction_type.as_str())
        .bind(transaction.amount)
        .bind(transaction.balance_after)
        .bind(transaction.topup_balance_after)
        .bind(transaction.subscription_balance_after)
        .bind(transaction.credit_type.map(|v| v.to_string()))
        .bind(transaction.description)
        .bind(transaction.client_app_id)
        .bind(transaction.subscription_id)
        .bind(transaction.external_ref_id)
        .bind(transaction.correlation_id.clone())
        .bind(transaction.created_at)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        Self::points_transaction_row_to_domain(row)
    }

    async fn create_ledger_in_tx(
        tx: &mut Transaction<'_, Postgres>,
        ledger: &PointsCreditLedger,
    ) -> Result<PointsCreditLedger, CoreError> {
        let row = sqlx::query_as::<_, PointsCreditLedgerRow>(
            r#"
            INSERT INTO points_credit_ledger (
                id, user_id, realm_id, bucket_id, credit_type, source_type, source_id,
                granted_amount, used_amount, revoked_amount, expires_at, effective_at,
                status, created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6,
                $7, $8, $9, $10, $11, $12,
                $13, $14, $15
            )
            RETURNING *
            "#,
        )
        .bind(ledger.id)
        .bind(ledger.user_id)
        .bind(&ledger.realm_id)
        .bind(ledger.bucket_id)
        .bind(ledger.credit_type.to_string())
        .bind(ledger.source_type.to_string())
        .bind(&ledger.source_id)
        .bind(ledger.granted_amount)
        .bind(ledger.used_amount)
        .bind(ledger.revoked_amount)
        .bind(ledger.expires_at)
        .bind(ledger.effective_at)
        .bind(ledger.status.to_string())
        .bind(ledger.created_at)
        .bind(ledger.updated_at)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        Self::row_to_points_credit_ledger(row)
    }

    async fn create_consumption_allocation_in_tx(
        tx: &mut Transaction<'_, Postgres>,
        allocation: &PointsConsumptionAllocation,
    ) -> Result<(), CoreError> {
        // wallet_id is NOT NULL on points_consumption_allocations. The consume
        // write path always resolves it; fail loud rather than silently writing NULL.
        let wallet_id = allocation.wallet_id.ok_or_else(|| {
            CoreError::InternalServerError(format!(
                "consumption allocation requires wallet_id (ledger={}, transaction={})",
                allocation.ledger_id, allocation.transaction_id
            ))
        })?;
        sqlx::query(
            r#"
            INSERT INTO points_consumption_allocations (
                id, transaction_id, ledger_id, wallet_id, user_id, realm_id, bucket_id,
                allocated_amount, ledger_remaining_after, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(allocation.id)
        .bind(allocation.transaction_id)
        .bind(allocation.ledger_id)
        .bind(wallet_id)
        .bind(allocation.user_id)
        .bind(&allocation.realm_id)
        .bind(allocation.bucket_id)
        .bind(allocation.allocated_amount)
        .bind(allocation.ledger_remaining_after)
        .bind(allocation.created_at)
        .execute(&mut **tx)
        .await
        .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    async fn create_revocation_record_in_tx(
        tx: &mut Transaction<'_, Postgres>,
        record: &PointsRevocationRecord,
    ) -> Result<(), CoreError> {
        sqlx::query(
            r#"
            INSERT INTO points_revocation_records (
                id, ledger_id, user_id, realm_id, revocation_type,
                revoked_amount, reason, reference_id, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(record.id)
        .bind(record.ledger_id)
        .bind(record.user_id)
        .bind(&record.realm_id)
        .bind(record.revocation_type.to_string())
        .bind(record.revoked_amount)
        .bind(&record.reason)
        .bind(&record.reference_id)
        .bind(record.created_at)
        .execute(&mut **tx)
        .await
        .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    async fn check_completed_idempotency_in_tx(
        tx: &mut Transaction<'_, Postgres>,
        realm_id: &str,
        idempotency_key: &str,
    ) -> Result<Option<Uuid>, CoreError> {
        let row = sqlx::query("SELECT transaction_id FROM idempotency_keys WHERE realm_id = $1 AND idempotency_key = $2 AND status = 'completed' AND expires_at > NOW() LIMIT 1")
            .bind(realm_id)
            .bind(idempotency_key)
            .fetch_optional(&mut **tx)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        Ok(row.and_then(|row| row.try_get("transaction_id").ok()))
    }

    async fn record_completed_idempotency_in_tx(
        tx: &mut Transaction<'_, Postgres>,
        realm_id: &str,
        idempotency_key: &str,
        transaction_id: Uuid,
    ) -> Result<(), CoreError> {
        sqlx::query(
            r#"
            INSERT INTO idempotency_keys (
                id, realm_id, idempotency_key, status, request_data,
                response_data, transaction_id, created_at, expires_at, updated_at
            ) VALUES (
                $1, $2, $3, 'completed', '{}', '{}', $4, NOW(), NOW() + INTERVAL '24 hours', NOW()
            )
            ON CONFLICT (realm_id, idempotency_key)
            DO UPDATE SET transaction_id = EXCLUDED.transaction_id,
                          status = 'completed',
                          expires_at = NOW() + INTERVAL '24 hours',
                          updated_at = NOW()
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(realm_id)
        .bind(idempotency_key)
        .bind(transaction_id)
        .execute(&mut **tx)
        .await
        .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    async fn disable_periodic_grant_in_tx(
        tx: &mut Transaction<'_, Postgres>,
        realm_id: &str,
        user_id: Uuid,
    ) -> Result<(), CoreError> {
        sqlx::query(
            r#"
            UPDATE user_points_configs
            SET free_periodic_points_amount = 0,
                next_grant_time = NULL,
                updated_at = NOW()
            WHERE realm_id = $1 AND user_id = $2
            "#,
        )
        .bind(realm_id)
        .bind(user_id)
        .execute(&mut **tx)
        .await
        .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        sqlx::query(
            r#"
            UPDATE points_grant_schedules
            SET active = false,
                updated_at = NOW()
            WHERE realm_id = $1 AND user_id = $2 AND subscription_id IS NULL AND active = true
            "#,
        )
        .bind(realm_id)
        .bind(user_id)
        .execute(&mut **tx)
        .await
        .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn revoke_ledger_list_in_tx(
        tx: &mut Transaction<'_, Postgres>,
        realm_id: &str,
        user_id: Uuid,
        ledgers: Vec<(Uuid, i64)>,
        revocation_type: RevocationType,
        reason: &str,
        reference_id: Option<&str>,
    ) -> Result<(i64, Vec<Uuid>), CoreError> {
        let mut total_revoked = 0i64;
        let mut ledger_ids = Vec::new();
        for (ledger_id, amount_to_revoke) in ledgers {
            let updated_ledger = Self::update_ledger_in_tx(
                tx,
                ledger_id,
                LedgerUpdate::Revocation(amount_to_revoke),
            )
            .await?;
            let record = PointsRevocationRecord {
                id: Uuid::now_v7(),
                ledger_id: updated_ledger.id,
                user_id,
                realm_id: realm_id.to_string(),
                revocation_type,
                revoked_amount: amount_to_revoke,
                reason: reason.to_string(),
                reference_id: reference_id.map(|s| s.to_string()),
                created_at: chrono::Utc::now(),
            };
            Self::create_revocation_record_in_tx(tx, &record).await?;
            total_revoked += amount_to_revoke;
            ledger_ids.push(updated_ledger.id);
        }
        Ok((total_revoked, ledger_ids))
    }
}

impl PointsRepository for PostgresPointsRepository {
    /// User-total wallet view (read path for `get_balance` / `get_wallet`).
    ///
    /// A user may hold one wallet row per Bucket; this returns a single
    /// aggregated `PointsWallet` with `bucket_id = None` whose balance fields
    /// are the SUM across the user's per-bucket wallet rows (review #5 chimera
    /// fix). For a single-bucket user the result equals that bucket's row.
    ///
    /// O(1) per row: reads the maintained projection columns, never aggregates
    /// the ledger. Returns `None` if the user has no wallet row.
    async fn find_by_user_id(
        &self,
        realm_id: &str,
        user_id: Uuid,
    ) -> Result<Option<PointsWallet>, CoreError> {
        let rows: Vec<PointsWalletRow> = sqlx::query_as::<_, PointsWalletRow>(
            "SELECT * FROM points_wallets WHERE realm_id = $1 AND user_id = $2 ORDER BY created_at ASC",
        )
        .bind(realm_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        if rows.is_empty() {
            return Ok(None);
        }

        // Aggregate into a single user-total view. bucket_id = None signals
        // "not tied to a specific pool" (matches the synthesized zero-balance
        // wallet used when no row exists). Status reflects the most restrictive
        // pool (any non-active row dominates); created_at/updated_at from the
        // oldest / newest row respectively so the view is monotonic.
        // Design §1.3 / A7 (BE-D11): only analytics columns are aggregated;
        // available balance is derived separately via `compute_available_balance`.
        let mut agg = PointsWallet {
            id: rows[0].id,
            user_id,
            realm_id: realm_id.to_string(),
            bucket_id: None,
            total_topup_granted: 0,
            total_subscription_granted: 0,
            total_recharged: 0,
            total_consumed: 0,
            status: WalletStatus::Active,
            created_at: chrono::DateTime::from(rows[0].created_at),
            updated_at: chrono::DateTime::from(rows[0].updated_at),
        };
        for row in rows {
            agg.total_recharged += row.total_recharged;
            agg.total_consumed += row.total_consumed;
            agg.total_topup_granted += row.total_topup_granted;
            agg.total_subscription_granted += row.total_subscription_granted;
            let created = chrono::DateTime::from(row.created_at);
            let updated = chrono::DateTime::from(row.updated_at);
            if created < agg.created_at {
                agg.created_at = created;
            }
            if updated > agg.updated_at {
                agg.updated_at = updated;
            }
            let status = WalletStatus::from_str(&row.status).map_err(|_| {
                CoreError::BadRequest(format!("Invalid wallet status: {}", row.status))
            })?;
            if !matches!(status, WalletStatus::Active) {
                agg.status = status;
            }
        }
        Ok(Some(agg))
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<PointsWallet>, CoreError> {
        let result = points_wallet::Entity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        result.map(Self::model_to_points_wallet).transpose()
    }

    async fn create_wallet(&self, account: PointsWallet) -> Result<PointsWallet, CoreError> {
        let active_model = Self::points_wallet_to_active_model(account);

        let result = active_model.insert(&*self.db).await.map_err(|e| {
            // Check for unique constraint violation
            if e.to_string()
                .contains(constraints::UK_POINTS_WALLETS_USER_ID)
            {
                CoreError::BadRequest("Points wallet already exists for this user".to_string())
            } else {
                CoreError::DatabaseError(e.to_string())
            }
        })?;

        Self::model_to_points_wallet(result)
    }

    async fn create_transaction(
        &self,
        transaction: PointsTransaction,
    ) -> Result<PointsTransaction, CoreError> {
        let active_model = Self::points_transaction_to_active_model(transaction);

        let result = active_model
            .insert(&*self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        Self::model_to_points_transaction(result)
    }

    async fn find_transaction_by_id(
        &self,
        realm_id: &str,
        transaction_id: Uuid,
    ) -> Result<Option<PointsTransaction>, CoreError> {
        let result = points_transaction::Entity::find()
            .filter(points_transaction::Column::RealmId.eq(realm_id))
            .filter(points_transaction::Column::Id.eq(transaction_id))
            .one(&*self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        result.map(Self::model_to_points_transaction).transpose()
    }

    async fn find_expired_recharge_transactions(
        &self,
        realm_id: &str,
        user_id: Uuid,
    ) -> Result<Vec<PointsTransaction>, CoreError> {
        // Use raw SQL to find expired recharge transactions
        // We need to filter by expires_at < NOW() which is not easily expressed in SeaORM
        let query = r#"
            SELECT id, realm_id, wallet_id, user_id, type, amount, balance_after,
                   topup_balance_after, subscription_balance_after, credit_type,
                   description, client_app_id, subscription_id, external_ref_id, correlation_id,
                   created_at, updated_at, expires_at
            FROM points_transactions
            WHERE realm_id = $1
              AND user_id = $2
              AND type = 'recharge'
              AND expires_at < NOW()
              AND expires_at IS NOT NULL
            ORDER BY expires_at ASC
        "#;

        let rows = sqlx::query_as::<_, PointsTransactionRow>(query)
            .bind(realm_id)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                CoreError::DatabaseError(format!("Failed to find expired transactions: {}", e))
            })?;

        rows.into_iter()
            .map(Self::points_transaction_row_to_domain)
            .collect::<Result<Vec<_>, _>>()
    }

    async fn find_transactions(
        &self,
        realm_id: &str,
        filters: TransactionFilters,
    ) -> Result<Paginated<PointsTransaction>, CoreError> {
        let page = filters.page.unwrap_or(1);
        let page_size = filters.page_size.unwrap_or(20).min(100);

        // Count via the existing SeaORM filter builder (no JOIN needed).
        let count_query = Self::apply_transaction_filters(
            points_transaction::Entity::find()
                .filter(points_transaction::Column::RealmId.eq(realm_id)),
            &filters,
        );
        let total = count_query
            .count(&*self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        // BE-D10 / BE-D11 Part B: raw-SQL paged fetch sourcing `effective_at`
        // for grant-type transactions (design §4.2 / §5.1). The
        // transaction→ledger relation has no FK, so we resolve the matching
        // ledger row via a LATERAL subquery that picks EXACTLY ONE row per
        // transaction. This fixes two issues with the prior plain LEFT JOIN:
        //
        //   R1 (fan-out): a plain LEFT JOIN on the OR-based LIKE/equality
        //   conditions could match >1 ledger row per transaction (e.g. when
        //   the same `source_id` is reused across multiple grants), duplicating
        //   `points_transactions` rows and breaking pagination (`total` vs
        //   `len(data)` and LIMIT/OFFSET). LATERAL + LIMIT 1 guarantees at
        //   most one ledger row per transaction.
        //
        //   R2 (subscription-grant miss): subscription grants write
        //   `ledger.source_id = "<entitlement_key>:<idempotency_key>"` and
        //   `txn.external_ref_id = "<idempotency_key>"`. The prior third
        //   branch `l.source_id LIKE (t.external_ref_id || ':%')` only matched
        //   when the entitlement_key prefix was absent. The corrected
        //   suffix-match branch `l.source_id LIKE '%:' || t.external_ref_id`
        //   matches the `<entitlement_key>:<idempotency_key>` form regardless
        //   of the entitlement-key prefix, so `points.manage` now returns the
        //   real `effective_at` for subscription grants (closes BE-D10/BE-A03).
        //
        // Match precedence (deterministic single pick via ORDER BY):
        //   1. exact equality `t.external_ref_id = l.source_id` (strongest),
        //   2. prefix form `t.external_ref_id LIKE l.source_id || ':%'`
        //      (grant_points_atomic / pregrant: txn ref = `<source_id>:<tx_id>`),
        //   3. suffix form `l.source_id LIKE '%:' || t.external_ref_id`
        //      (subscription grant: ledger source_id = `<ek>:<idem_key>`),
        //   then tie-break on `created_at DESC` for determinism.
        // Non-grant rows (consume/refund/expiration) have NULL or non-matching
        // external_ref_id and yield NULL effective_at (correct semantics).
        //
        // The filter clauses mirror `apply_transaction_filters` exactly so
        // count and page stay consistent.
        let mut sql = String::from(
            r#"
            SELECT t.id, t.realm_id, t.wallet_id, t.user_id, t.bucket_id, t.type, t.amount,
                   t.balance_after, t.topup_balance_after, t.subscription_balance_after,
                   t.credit_type, t.description, t.client_app_id, t.subscription_id,
                   t.external_ref_id, t.correlation_id, l.effective_at,
                   t.created_at, t.updated_at, t.expires_at
            FROM points_transactions t
            LEFT JOIN LATERAL (
                SELECT l.effective_at, l.created_at, l.source_id
                FROM points_credit_ledger l
                WHERE l.realm_id = t.realm_id
                  AND l.user_id = t.user_id
                  AND l.bucket_id = t.bucket_id
                  AND t.external_ref_id IS NOT NULL
                  AND (
                      t.external_ref_id = l.source_id
                      OR t.external_ref_id LIKE (l.source_id || ':%')
                      OR l.source_id LIKE ('%:' || t.external_ref_id)
                  )
                ORDER BY
                    (t.external_ref_id = l.source_id) DESC,
                    (t.external_ref_id LIKE (l.source_id || ':%')) DESC,
                    l.created_at DESC
                LIMIT 1
            ) l ON true
            WHERE t.realm_id = $1
            "#,
        );
        let mut param_idx = 2u32;

        if filters.user_id.is_some() {
            sql.push_str(&format!(" AND t.user_id = ${}", param_idx));
            param_idx += 1;
        }
        if filters.bucket_id.is_some() {
            sql.push_str(&format!(" AND t.bucket_id = ${}", param_idx));
            param_idx += 1;
        }
        if filters.transaction_type.is_some() {
            sql.push_str(&format!(" AND t.type = ${}", param_idx));
            param_idx += 1;
        }
        if filters.client_app_id.is_some() {
            sql.push_str(&format!(" AND t.client_app_id = ${}", param_idx));
            param_idx += 1;
        }
        if filters.subscription_id.is_some() {
            sql.push_str(&format!(" AND t.subscription_id = ${}", param_idx));
            param_idx += 1;
        }
        if !filters.external_ref_id.is_empty() {
            sql.push_str(&format!(" AND t.external_ref_id = ${}", param_idx));
            param_idx += 1;
        }
        if filters.start_time.is_some() {
            sql.push_str(&format!(" AND t.created_at >= ${}", param_idx));
            param_idx += 1;
        }
        if filters.end_time.is_some() {
            sql.push_str(&format!(" AND t.created_at <= ${}", param_idx));
            param_idx += 1;
        }

        sql.push_str(" ORDER BY t.created_at DESC");
        sql.push_str(&format!(" LIMIT ${} OFFSET ${}", param_idx, param_idx + 1));
        let offset = (page.saturating_sub(1)) * page_size;

        // Bind in the same order as the WHERE clauses were appended, then the
        // trailing LIMIT/OFFSET. Re-evaluating each condition keeps the bind
        // type concrete (sqlx::Query::bind is monomorphic per call).
        let mut query = sqlx::query_as::<_, PointsTransactionRow>(&sql).bind(realm_id);
        if let Some(user_id) = filters.user_id {
            query = query.bind(user_id);
        }
        if let Some(bucket_id) = filters.bucket_id {
            query = query.bind(bucket_id);
        }
        if let Some(transaction_type) = &filters.transaction_type {
            query = query.bind(transaction_type.as_str());
        }
        if let Some(client_app_id) = filters.client_app_id {
            query = query.bind(client_app_id);
        }
        if let Some(subscription_id) = filters.subscription_id {
            query = query.bind(subscription_id);
        }
        if !filters.external_ref_id.is_empty() {
            query = query.bind(&filters.external_ref_id);
        }
        if let Some(start_time) = &filters.start_time {
            query = query.bind(start_time);
        }
        if let Some(end_time) = &filters.end_time {
            query = query.bind(end_time);
        }
        let rows = query
            .bind(page_size as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        let transactions = rows
            .into_iter()
            .map(Self::points_transaction_row_to_domain)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Paginated {
            total,
            page,
            page_size,
            data: transactions,
        })
    }

    async fn count_transactions(
        &self,
        realm_id: &str,
        filters: &TransactionFilters,
    ) -> Result<u64, CoreError> {
        let query = Self::apply_transaction_filters(
            points_transaction::Entity::find()
                .filter(points_transaction::Column::RealmId.eq(realm_id)),
            filters,
        );

        let count = query
            .count(&*self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        Ok(count)
    }

    async fn check_idempotency_key(
        &self,
        realm_id: &str,
        idempotency_key: &str,
    ) -> Result<Option<Uuid>, CoreError> {
        // Query the idempotency_keys table
        let query = r#"
            SELECT transaction_id
            FROM idempotency_keys
            WHERE realm_id = $1 AND idempotency_key = $2
              AND status = 'completed'
            LIMIT 1
        "#;

        let result = sqlx::query_as::<_, (Uuid,)>(query)
            .bind(realm_id)
            .bind(idempotency_key)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                CoreError::DatabaseError(format!("Failed to check idempotency key: {}", e))
            })?;

        Ok(result.map(|(transaction_id,)| transaction_id))
    }

    async fn record_idempotency_key(
        &self,
        realm_id: &str,
        idempotency_key: &str,
        transaction_id: Uuid,
    ) -> Result<(), CoreError> {
        // Insert into idempotency_keys table with ON CONFLICT DO NOTHING
        // This ensures idempotency at the database level
        let query = r#"
            INSERT INTO idempotency_keys (
                realm_id,
                idempotency_key,
                status,
                request_data,
                response_data,
                transaction_id,
                created_at,
                updated_at,
                expires_at
            ) VALUES (
                $1, $2, 'completed', '{}', '{}', $3, NOW(), NOW(), NOW() + INTERVAL '24 hours'
            )
            ON CONFLICT (realm_id, idempotency_key) DO NOTHING
        "#;

        sqlx::query(query)
            .bind(realm_id)
            .bind(idempotency_key)
            .bind(transaction_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                CoreError::DatabaseError(format!("Failed to record idempotency key: {}", e))
            })?;

        tracing::debug!(
            realm_id = %realm_id,
            idempotency_key = %idempotency_key,
            transaction_id = %transaction_id,
            "Recorded idempotency key"
        );

        Ok(())
    }

    async fn find_transaction_by_ref(
        &self,
        realm_id: &str,
        user_id: Uuid,
        external_ref_id: &str,
    ) -> Result<Option<PointsTransaction>, CoreError> {
        let result = points_transaction::Entity::find()
            .filter(points_transaction::Column::RealmId.eq(realm_id))
            .filter(points_transaction::Column::UserId.eq(user_id))
            .filter(points_transaction::Column::ExternalRefId.eq(external_ref_id))
            .one(&*self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        result.map(Self::model_to_points_transaction).transpose()
    }

    async fn find_points_policy_by_entitlement_key(
        &self,
        realm_id: &str,
        entitlement_key: &str,
    ) -> Result<Option<herald_domain::billing::entities::EntitlementMapping>, CoreError> {
        use herald_entity::provider_entitlement_mapping;

        let result = provider_entitlement_mapping::Entity::find()
            .filter(provider_entitlement_mapping::Column::RealmId.eq(realm_id))
            .filter(provider_entitlement_mapping::Column::EntitlementKey.eq(entitlement_key))
            .filter(provider_entitlement_mapping::Column::Enabled.eq(true))
            .one(&*self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        Ok(result.map(
            |model| herald_domain::billing::entities::EntitlementMapping {
                id: model.id,
                realm_id: model.realm_id,
                payment_provider: model.payment_provider,
                external_product_id: model.external_product_id,
                external_price_id: model.external_price_id,
                bucket_id: model.bucket_id,
                entitlement_key: model.entitlement_key,
                billing_type: model.billing_type.and_then(|s| s.parse().ok()),
                billing_period: model.billing_period,
                points_per_period: model.points_per_period.map(|v| v as i64),
                grant_period_type: model.grant_period_type,
                validity_days: model.validity_days.map(|v| v as i64),
                grant_on_subscribe: model.grant_on_subscribe,
                max_periods: model.max_periods.map(|v| v as i64),
                enabled: model.enabled,
                provider_product_info: model.provider_product_info,
                synced_at: model.synced_at.map(chrono::DateTime::from),
                created_at: chrono::DateTime::from(model.created_at),
                updated_at: chrono::DateTime::from(model.updated_at),
            },
        ))
    }

    async fn list_wallets(
        &self,
        realm_id: &str,
        filters: WalletFilters,
    ) -> Result<Paginated<PointsWallet>, CoreError> {
        let page = filters.page.unwrap_or(1);
        let page_size = filters.page_size.unwrap_or(20).min(100);

        let query =
            points_wallet::Entity::find().filter(points_wallet::Column::RealmId.eq(realm_id));

        let query = self.apply_wallet_filters(query, realm_id, &filters).await?;

        let mut query = match query {
            Some(q) => q,
            None => {
                return Ok(Paginated {
                    total: 0,
                    page,
                    page_size,
                    data: vec![],
                });
            }
        };

        // Get total count
        let total = query
            .clone()
            .count(&*self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        // Order by created_at DESC
        query = query.order_by_desc(points_wallet::Column::CreatedAt);

        // Apply pagination
        let results = query
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        let accounts = results
            .into_iter()
            .map(Self::model_to_points_wallet)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Paginated {
            total,
            page,
            page_size,
            data: accounts,
        })
    }

    async fn count_wallets(
        &self,
        realm_id: &str,
        filters: &WalletFilters,
    ) -> Result<u64, CoreError> {
        let query =
            points_wallet::Entity::find().filter(points_wallet::Column::RealmId.eq(realm_id));

        let query = self.apply_wallet_filters(query, realm_id, filters).await?;

        let query = match query {
            Some(q) => q,
            None => return Ok(0),
        };

        let count = query
            .count(&*self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        Ok(count)
    }

    // ========== Ledger Management ==========

    fn create_ledger(
        &self,
        ledger: PointsCreditLedger,
    ) -> impl std::future::Future<Output = Result<PointsCreditLedger, CoreError>> + Send {
        let db = self.db.clone();
        async move {
            let active_model = points_credit_ledger_to_active_model(&ledger);
            let result = active_model
                .insert(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            Ok(points_credit_ledger_from_model(result))
        }
    }

    fn find_ledgers_by_user_id(
        &self,
        realm_id: &str,
        user_id: Uuid,
        filters: LedgerFilters,
    ) -> impl std::future::Future<Output = Result<Paginated<PointsCreditLedger>, CoreError>> + Send
    {
        let db = self.db.clone();
        let realm_id = realm_id.to_string();
        async move {
            let mut query = points_credit_ledger::Entity::find()
                .filter(points_credit_ledger::Column::RealmId.eq(&realm_id))
                .filter(points_credit_ledger::Column::UserId.eq(user_id));

            if let Some(credit_type) = filters.credit_type {
                query = query
                    .filter(points_credit_ledger::Column::CreditType.eq(credit_type.to_string()));
            }

            if let Some(status) = filters.status {
                query = query.filter(points_credit_ledger::Column::Status.eq(status.to_string()));
            }

            if let Some(start_time) = filters.start_time {
                query = query.filter(points_credit_ledger::Column::CreatedAt.gte(start_time));
            }

            if let Some(end_time) = filters.end_time {
                query = query.filter(points_credit_ledger::Column::CreatedAt.lte(end_time));
            }

            // Always order by created_at DESC for ledger queries (newest first)
            query = query.order_by_desc(points_credit_ledger::Column::CreatedAt);

            let pagination = filters.pagination.unwrap_or_default();

            let total = query
                .clone()
                .count(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            let items = query
                .paginate(&*db, pagination.page_size)
                .fetch_page((pagination.page - 1) * pagination.page_size)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            let items: Result<Vec<PointsCreditLedger>, CoreError> = items
                .into_iter()
                .map(|m| Ok(points_credit_ledger_from_model(m)))
                .collect();
            let items = items?;

            Ok(Paginated {
                data: items,
                total,
                page: pagination.page,
                page_size: pagination.page_size,
            })
        }
    }

    fn find_ledger_by_id(
        &self,
        ledger_id: Uuid,
    ) -> impl std::future::Future<Output = Result<Option<PointsCreditLedger>, CoreError>> + Send
    {
        let db = self.db.clone();
        async move {
            let result = points_credit_ledger::Entity::find_by_id(ledger_id)
                .one(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            Ok(result.map(points_credit_ledger_from_model))
        }
    }

    fn find_ledger_by_source_id(
        &self,
        realm_id: &str,
        source_id: &str,
    ) -> impl std::future::Future<Output = Result<Option<PointsCreditLedger>, CoreError>> + Send
    {
        let db = self.db.clone();
        let realm_id = realm_id.to_string();
        let source_id = source_id.to_string();
        async move {
            let result = points_credit_ledger::Entity::find()
                .filter(points_credit_ledger::Column::RealmId.eq(&realm_id))
                .filter(points_credit_ledger::Column::SourceId.eq(&source_id))
                .one(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            Ok(result.map(points_credit_ledger_from_model))
        }
    }

    fn update_ledger(
        &self,
        ledger_id: Uuid,
        updates: LedgerUpdate,
    ) -> impl std::future::Future<Output = Result<PointsCreditLedger, CoreError>> + Send {
        let db = self.db.clone();
        async move {
            let ledger = points_credit_ledger::Entity::find_by_id(ledger_id)
                .one(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?
                .ok_or_else(|| CoreError::BadRequest(format!("Ledger not found: {}", ledger_id)))?;

            let mut active =
                points_credit_ledger_to_active_model(&points_credit_ledger_from_model(ledger));

            match updates {
                LedgerUpdate::Consumption(amount) => {
                    let current_used = active.used_amount.clone().take();
                    let current_granted = active.granted_amount.clone().take();
                    let current_revoked = active.revoked_amount.clone().take();

                    let new_used = match current_used {
                        Some(v) => v + amount,
                        None => amount,
                    };

                    // remaining_amount is a generated column: granted_amount - used_amount - revoked_amount
                    // We cannot update it directly; the database computes it automatically
                    let new_remaining = match (&current_granted, &current_used, &current_revoked) {
                        (Some(g), Some(u), Some(r)) => g - (u + amount) - r,
                        (Some(g), None, Some(r)) => g - amount - r,
                        (Some(g), Some(u), None) => g - (u + amount),
                        (Some(g), None, None) => g - amount,
                        _ => -amount, // This should not happen
                    };

                    active.used_amount = Set(new_used);

                    // Update status if fully used
                    if new_remaining <= 0 {
                        active.status = Set(CreditLedgerStatus::FullyUsed.to_string());
                    }
                }
                LedgerUpdate::Revocation(amount) => {
                    let current_revoked = active.revoked_amount.clone().take();
                    let current_granted = active.granted_amount.clone().take();
                    let current_used = active.used_amount.clone().take();

                    let new_revoked = match current_revoked {
                        Some(v) => v + amount,
                        None => amount,
                    };

                    // remaining_amount is a generated column: granted_amount - used_amount - revoked_amount
                    // We cannot update it directly; the database computes it automatically
                    let new_remaining = match (&current_granted, &current_used, &current_revoked) {
                        (Some(g), Some(u), Some(r)) => g - u - (r + amount),
                        (Some(g), None, Some(r)) => g - (r + amount),
                        (Some(g), Some(u), None) => g - u - amount,
                        (Some(g), None, None) => g - amount,
                        _ => -amount, // This should not happen
                    };

                    active.revoked_amount = Set(new_revoked);

                    // Update status if revoked
                    if new_remaining <= 0 {
                        active.status = Set(CreditLedgerStatus::Revoked.to_string());
                    }
                }
                LedgerUpdate::SetExpiration(expires_at) => {
                    active.expires_at = Set(Some(sea_orm::prelude::DateTimeWithTimeZone::from(
                        expires_at,
                    )));
                }
                LedgerUpdate::SetStatus(status) => {
                    active.status = Set(status.to_string());
                }
            }

            active.updated_at = Set(chrono::Utc::now().into());

            let result = active
                .update(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            Ok(points_credit_ledger_from_model(result))
        }
    }

    fn find_active_ledgers_by_credit_type(
        &self,
        realm_id: &str,
        user_id: Uuid,
        credit_type: CreditType,
    ) -> impl std::future::Future<Output = Result<Vec<PointsCreditLedger>, CoreError>> + Send {
        let pool = self.pool.clone();
        let realm_id = realm_id.to_string();

        async move {
            let credit_type_str = credit_type.to_string();
            let status_str = CreditLedgerStatus::Active.to_string();

            let rows = sqlx::query_as::<
                _,
                (
                    Uuid,
                    Uuid,
                    String,
                    String,
                    String,
                    String,
                    i64,
                    i64,
                    i64,
                    i64,
                    Option<chrono::DateTime<chrono::Utc>>,
                    String,
                    chrono::DateTime<chrono::Utc>,
                    chrono::DateTime<chrono::Utc>,
                ),
            >(
                "SELECT id, user_id, realm_id, credit_type, source_type, source_id,
                        granted_amount, used_amount, revoked_amount, remaining_amount,
                        expires_at, status, created_at, updated_at
                 FROM points_credit_ledger
                 WHERE realm_id = $1
                   AND user_id = $2
                   AND credit_type = $3
                   AND status = $4
                   AND remaining_amount > 0
                 ORDER BY created_at ASC",
            )
            .bind(&realm_id)
            .bind(user_id)
            .bind(&credit_type_str)
            .bind(&status_str)
            .fetch_all(&pool)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            // Convert rows to domain entities
            let ledgers: Result<Vec<_>, _> = rows
                .into_iter()
                .map(
                    |(
                        id,
                        user_id,
                        realm_id,
                        credit_type,
                        source_type,
                        source_id,
                        granted_amount,
                        used_amount,
                        revoked_amount,
                        remaining_amount,
                        expires_at,
                        status,
                        created_at,
                        updated_at,
                    )| {
                        let credit_type: CreditType = credit_type.parse()?;
                        let source_type: CreditSourceType = source_type.parse()?;
                        let status: CreditLedgerStatus = status.parse()?;

                        Ok(PointsCreditLedger {
                            id,
                            user_id,
                            realm_id,
                            bucket_id: Uuid::nil(),
                            credit_type,
                            source_type,
                            source_id,
                            granted_amount,
                            used_amount,
                            revoked_amount,
                            remaining_amount,
                            expires_at,
                            effective_at: None,
                            status,
                            created_at,
                            updated_at,
                        })
                    },
                )
                .collect();

            ledgers
        }
    }

    // ========== Consumption Allocations ==========

    fn create_consumption_allocation(
        &self,
        allocation: PointsConsumptionAllocation,
    ) -> impl std::future::Future<Output = Result<PointsConsumptionAllocation, CoreError>> + Send
    {
        let db = self.db.clone();
        async move {
            let active_model = points_consumption_allocation_to_active_model(&allocation);
            let result = active_model
                .insert(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            Ok(points_consumption_allocation_from_model(result))
        }
    }

    fn find_consumption_allocations_by_transaction(
        &self,
        transaction_id: Uuid,
    ) -> impl std::future::Future<Output = Result<Vec<PointsConsumptionAllocation>, CoreError>> + Send
    {
        let db = self.db.clone();
        async move {
            let allocations = points_consumption_allocation::Entity::find()
                .filter(points_consumption_allocation::Column::TransactionId.eq(transaction_id))
                .all(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            allocations
                .into_iter()
                .map(|m| Ok(points_consumption_allocation_from_model(m)))
                .collect()
        }
    }

    fn find_consumption_allocations_by_correlation_id(
        &self,
        realm_id: &str,
        correlation_id: &str,
    ) -> impl std::future::Future<Output = Result<Vec<ConsumptionAllocationView>, CoreError>> + Send
    {
        let db = self.db.clone();
        let realm_id = realm_id.to_string();
        let correlation_id = correlation_id.to_string();
        async move {
            // Two-step lookup: transactions sharing the correlation_id → their
            // allocations, each joined with its ledger to surface credit_type
            // (needed for the SDK consume response AllocationDetail).
            let txn_ids: Vec<Uuid> = points_transaction::Entity::find()
                .filter(points_transaction::Column::RealmId.eq(&realm_id))
                .filter(points_transaction::Column::CorrelationId.eq(&correlation_id))
                .all(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?
                .into_iter()
                .map(|m| m.id)
                .collect();

            if txn_ids.is_empty() {
                return Ok(Vec::new());
            }

            let rows = points_consumption_allocation::Entity::find()
                .filter(points_consumption_allocation::Column::TransactionId.is_in(txn_ids))
                .find_also_related(points_credit_ledger::Entity)
                .all(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            rows.into_iter()
                .map(|(alloc_model, ledger_model)| {
                    let ledger = ledger_model.ok_or_else(|| {
                        CoreError::DatabaseError(format!(
                            "consumption allocation {} missing parent ledger {}",
                            alloc_model.id, alloc_model.ledger_id
                        ))
                    })?;
                    let credit_type = CreditType::from_str(&ledger.credit_type)?;
                    Ok(ConsumptionAllocationView {
                        allocation: points_consumption_allocation_from_model(alloc_model),
                        credit_type,
                    })
                })
                .collect()
        }
    }

    // ========== Revocation Records ==========

    fn create_revocation_record(
        &self,
        record: PointsRevocationRecord,
    ) -> impl std::future::Future<Output = Result<PointsRevocationRecord, CoreError>> + Send {
        let db = self.db.clone();
        async move {
            let active_model = points_revocation_record_to_active_model(&record);
            let result = active_model
                .insert(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            Ok(points_revocation_record_from_model(result))
        }
    }

    fn find_revocation_by_idempotency_key(
        &self,
        idempotency_key: &str,
    ) -> impl std::future::Future<Output = Result<Option<PointsRevocationRecord>, CoreError>> + Send
    {
        let db = self.db.clone();
        let idempotency_key = idempotency_key.to_string();
        async move {
            // Note: This assumes idempotency_key is stored in the reference_id field
            // or we need to add it to the revocation_records table
            // For now, we'll use reference_id as the idempotency key
            let result = points_revocation_record::Entity::find()
                .filter(points_revocation_record::Column::ReferenceId.eq(&idempotency_key))
                .one(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            Ok(result.map(points_revocation_record_from_model))
        }
    }

    // ========== Account Management ==========

    fn find_expired_ledgers(
        &self,
        expiration_time: chrono::DateTime<chrono::Utc>,
        limit: usize,
    ) -> impl std::future::Future<Output = Result<Vec<PointsCreditLedger>, CoreError>> + Send {
        let db = self.db.clone();
        async move {
            let ledgers = points_credit_ledger::Entity::find()
                .filter(points_credit_ledger::Column::ExpiresAt.lte(
                    sea_orm::prelude::DateTimeWithTimeZone::from(expiration_time),
                ))
                .filter(
                    points_credit_ledger::Column::Status.eq(CreditLedgerStatus::Active.to_string()),
                )
                .filter(points_credit_ledger::Column::RemainingAmount.gt(0))
                .order_by_asc(points_credit_ledger::Column::ExpiresAt)
                .limit(limit as u64)
                .all(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            ledgers
                .into_iter()
                .map(|m| Ok(points_credit_ledger_from_model(m)))
                .collect()
        }
    }

    // ========== Realm Config Repository Methods (Iteration-2) ==========

    fn find_realm_config(
        &self,
        realm_id: &str,
    ) -> impl std::future::Future<
        Output = Result<Option<herald_domain::points::realm_config::RealmDefaultConfig>, CoreError>,
    > + Send {
        let db = self.db.clone();
        let realm_id = realm_id.to_string();
        async move {
            let result = realm_default_config::Entity::find_by_id(realm_id.clone())
                .one(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            result
                .map(|model| -> Result<_, CoreError> {
                    // Parse grant period type
                    let grant_period_type = model
                        .free_periodic_grant_period_type
                        .parse::<herald_domain::points::grant_schedule::GrantPeriodType>()
                        .map_err(|e| {
                            CoreError::DatabaseError(format!(
                                "Invalid grant period type in database: {}",
                                e
                            ))
                        })?;

                    Ok(herald_domain::points::realm_config::RealmDefaultConfig {
                        realm_id: model.realm_id,
                        registration_bonus_points: model.registration_bonus_points,
                        free_periodic_points_amount: model.free_periodic_points_amount,
                        free_periodic_grant_period_type: grant_period_type,
                        free_periodic_validity_days: model.free_periodic_validity_days,
                        created_at: chrono::DateTime::from(model.created_at),
                        updated_at: chrono::DateTime::from(model.updated_at),
                    })
                })
                .transpose()
        }
    }

    fn create_realm_config(
        &self,
        config: herald_domain::points::CreateRealmConfigInput,
    ) -> impl std::future::Future<
        Output = Result<herald_domain::points::realm_config::RealmDefaultConfig, CoreError>,
    > + Send {
        let db = self.db.clone();
        async move {
            // Parse grant period type
            let grant_period_type = config
                .free_periodic_grant_period_type
                .parse::<herald_domain::points::grant_schedule::GrantPeriodType>()
                .map_err(|e| CoreError::BadRequest(format!("Invalid grant period type: {}", e)))?;

            let now = chrono::Utc::now();
            let active_model = realm_default_config::ActiveModel {
                realm_id: Set(config.realm_id.clone()),
                registration_bonus_points: Set(config.registration_bonus_points),
                free_periodic_points_amount: Set(config.free_periodic_points_amount),
                free_periodic_grant_period_type: Set(config.free_periodic_grant_period_type),
                free_periodic_validity_days: Set(config.free_periodic_validity_days),
                created_at: Set(now.into()),
                updated_at: Set(now.into()),
            };

            let result = active_model
                .insert(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            Ok(herald_domain::points::realm_config::RealmDefaultConfig {
                realm_id: result.realm_id,
                registration_bonus_points: result.registration_bonus_points,
                free_periodic_points_amount: result.free_periodic_points_amount,
                free_periodic_grant_period_type: grant_period_type,
                free_periodic_validity_days: result.free_periodic_validity_days,
                created_at: chrono::DateTime::from(result.created_at),
                updated_at: chrono::DateTime::from(result.updated_at),
            })
        }
    }

    fn update_realm_config(
        &self,
        realm_id: &str,
        input: herald_domain::points::UpdateRealmConfigInput,
    ) -> impl std::future::Future<
        Output = Result<herald_domain::points::realm_config::RealmDefaultConfig, CoreError>,
    > + Send {
        let db = self.db.clone();
        let realm_id = realm_id.to_string();
        async move {
            // Parse grant period type
            let grant_period_type = input
                .free_periodic_grant_period_type
                .parse::<herald_domain::points::grant_schedule::GrantPeriodType>()
                .map_err(|e| CoreError::BadRequest(format!("Invalid grant period type: {}", e)))?;

            let model = realm_default_config::Entity::find_by_id(realm_id.clone())
                .one(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?
                .ok_or(CoreError::NotFound)?;

            let mut active: realm_default_config::ActiveModel = model.into();
            active.registration_bonus_points = Set(input.registration_bonus_points);
            active.free_periodic_points_amount = Set(input.free_periodic_points_amount);
            active.free_periodic_grant_period_type = Set(input.free_periodic_grant_period_type);
            active.free_periodic_validity_days = Set(input.free_periodic_validity_days);
            active.updated_at = Set(chrono::Utc::now().into());

            let result = active
                .update(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            Ok(herald_domain::points::realm_config::RealmDefaultConfig {
                realm_id: result.realm_id,
                registration_bonus_points: result.registration_bonus_points,
                free_periodic_points_amount: result.free_periodic_points_amount,
                free_periodic_grant_period_type: grant_period_type,
                free_periodic_validity_days: result.free_periodic_validity_days,
                created_at: chrono::DateTime::from(result.created_at),
                updated_at: chrono::DateTime::from(result.updated_at),
            })
        }
    }

    // ========== User Config Repository Methods (Iteration-2) ==========

    fn find_user_config(
        &self,
        user_id: Uuid,
    ) -> impl std::future::Future<
        Output = Result<Option<herald_domain::points::user_config::UserPointsConfig>, CoreError>,
    > + Send {
        let db = self.db.clone();
        async move {
            let result = user_points_config::Entity::find_by_id(user_id)
                .one(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            result
                .map(|model| Ok(Self::model_to_user_points_config(model)))
                .transpose()
        }
    }

    fn create_user_config(
        &self,
        config: herald_domain::points::user_config::UserPointsConfig,
    ) -> impl std::future::Future<
        Output = Result<herald_domain::points::user_config::UserPointsConfig, CoreError>,
    > + Send {
        let db = self.db.clone();
        async move {
            let active_model = user_points_config::ActiveModel {
                user_id: Set(config.user_id),
                realm_id: Set(config.realm_id.clone()),
                registration_bonus_points: Set(config.registration_bonus_points),
                free_periodic_points_amount: Set(config.free_periodic_points_amount),
                free_periodic_grant_period_type: Set(config
                    .free_periodic_grant_period_type
                    .map(|pt| pt.to_string())),
                free_periodic_validity_days: Set(config.free_periodic_validity_days),
                next_grant_time: Set(config.next_grant_time.map(|dt| dt.into())),
                granted_periods: Set(config.granted_periods),
                grant_schedule_id: Set(config.grant_schedule_id),
                created_at: Set(config.created_at.into()),
                updated_at: Set(config.updated_at.into()),
            };

            let result = active_model
                .insert(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            Ok(Self::model_to_user_points_config(result))
        }
    }

    fn update_user_config(
        &self,
        user_id: Uuid,
        next_grant_time: Option<chrono::DateTime<chrono::Utc>>,
        granted_periods: i64,
        grant_schedule_id: Option<Uuid>,
    ) -> impl std::future::Future<
        Output = Result<herald_domain::points::user_config::UserPointsConfig, CoreError>,
    > + Send {
        let db = self.db.clone();
        async move {
            let model = user_points_config::Entity::find_by_id(user_id)
                .one(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?
                .ok_or(CoreError::NotFound)?;

            let mut active: user_points_config::ActiveModel = model.into();
            active.next_grant_time = Set(next_grant_time.map(|dt| dt.into()));
            active.granted_periods = Set(granted_periods);
            active.grant_schedule_id = Set(grant_schedule_id);
            active.updated_at = Set(chrono::Utc::now().into());

            let result = active
                .update(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            Ok(Self::model_to_user_points_config(result))
        }
    }

    fn list_user_configs_by_realm(
        &self,
        realm_id: &str,
        pagination: &herald_domain::points::ports::Pagination,
    ) -> impl std::future::Future<
        Output = Result<
            herald_domain::points::entities::Paginated<
                herald_domain::points::user_config::UserPointsConfig,
            >,
            CoreError,
        >,
    > + Send {
        let db = self.db.clone();
        let realm_id = realm_id.to_string();
        let pagination = *pagination;
        async move {
            let page = pagination.page;
            let page_size = pagination.page_size;
            let _offset = (page - 1) * page_size;

            let query = user_points_config::Entity::find()
                .filter(user_points_config::Column::RealmId.eq(&realm_id));

            let total = query
                .clone()
                .count(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            let results = query
                .order_by_asc(user_points_config::Column::CreatedAt)
                .paginate(&*db, page_size)
                .fetch_page(page - 1)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            let configs = results
                .into_iter()
                .map(|model| Ok(Self::model_to_user_points_config(model)))
                .collect::<Result<Vec<_>, CoreError>>()?;

            Ok(Paginated {
                total,
                page,
                page_size,
                data: configs,
            })
        }
    }

    // ========== Grant Schedule Repository Methods (Iteration-2) ==========

    fn find_grant_schedule(
        &self,
        schedule_id: Uuid,
    ) -> impl std::future::Future<
        Output = Result<
            Option<herald_domain::points::grant_schedule::PointsGrantSchedule>,
            CoreError,
        >,
    > + Send {
        let db = self.db.clone();
        async move {
            let result = points_grant_schedule::Entity::find_by_id(schedule_id)
                .one(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            result.map(Self::model_to_grant_schedule).transpose()
        }
    }

    fn find_due_grant_schedules(
        &self,
        before: chrono::DateTime<chrono::Utc>,
        limit: u64,
    ) -> impl std::future::Future<
        Output = Result<Vec<herald_domain::points::grant_schedule::PointsGrantSchedule>, CoreError>,
    > + Send {
        let db = self.db.clone();
        async move {
            let models = points_grant_schedule::Entity::find()
                .filter(
                    points_grant_schedule::Column::NextGrantTime
                        .lte(sea_orm::prelude::DateTimeWithTimeZone::from(before)),
                )
                .filter(points_grant_schedule::Column::Active.eq(true))
                .order_by_asc(points_grant_schedule::Column::NextGrantTime)
                .limit(limit)
                .all(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            models
                .into_iter()
                .map(Self::model_to_grant_schedule)
                .collect()
        }
    }

    fn find_grant_schedules_by_user(
        &self,
        user_id: Uuid,
    ) -> impl std::future::Future<
        Output = Result<Vec<herald_domain::points::grant_schedule::PointsGrantSchedule>, CoreError>,
    > + Send {
        let db = self.db.clone();
        async move {
            let models = points_grant_schedule::Entity::find()
                .filter(points_grant_schedule::Column::UserId.eq(user_id))
                .order_by_asc(points_grant_schedule::Column::CreatedAt)
                .all(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            models
                .into_iter()
                .map(Self::model_to_grant_schedule)
                .collect()
        }
    }

    // ===== New: Free User Upgrade Support =====

    fn find_user_config_by_realm(
        &self,
        realm_id: &str,
        user_id: Uuid,
    ) -> impl std::future::Future<
        Output = Result<Option<herald_domain::points::user_config::UserPointsConfig>, CoreError>,
    > + Send {
        let db = self.db.clone();
        let realm_id = realm_id.to_string();
        async move {
            let result = user_points_config::Entity::find()
                .filter(user_points_config::Column::RealmId.eq(&realm_id))
                .filter(user_points_config::Column::UserId.eq(user_id))
                .one(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            result
                .map(|model| Ok(Self::model_to_user_points_config(model)))
                .transpose()
        }
    }

    fn update_user_points_config(
        &self,
        config_id: Uuid,
        update: herald_domain::points::ports::UserConfigUpdate,
    ) -> impl std::future::Future<
        Output = Result<herald_domain::points::user_config::UserPointsConfig, CoreError>,
    > + Send {
        let db = self.db.clone();
        async move {
            let model = user_points_config::Entity::find_by_id(config_id)
                .one(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?
                .ok_or(CoreError::NotFound)?;

            let mut active: user_points_config::ActiveModel = model.into();

            match update {
                herald_domain::points::ports::UserConfigUpdate::DisableDailyGrant {
                    next_grant_time,
                } => {
                    active.free_periodic_points_amount = Set(0);
                    active.next_grant_time = Set(next_grant_time.map(|dt| dt.into()));
                }
            }

            active.updated_at = Set(chrono::Utc::now().into());

            let result = active
                .update(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            Ok(Self::model_to_user_points_config(result))
        }
    }

    fn find_grant_schedules_by_user_realm(
        &self,
        realm_id: &str,
        user_id: Uuid,
    ) -> impl std::future::Future<
        Output = Result<Vec<herald_domain::points::grant_schedule::PointsGrantSchedule>, CoreError>,
    > + Send {
        let db = self.db.clone();
        let realm_id = realm_id.to_string();
        async move {
            let models = points_grant_schedule::Entity::find()
                .filter(points_grant_schedule::Column::RealmId.eq(&realm_id))
                .filter(points_grant_schedule::Column::UserId.eq(user_id))
                .order_by_asc(points_grant_schedule::Column::CreatedAt)
                .all(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            models
                .into_iter()
                .map(Self::model_to_grant_schedule)
                .collect()
        }
    }

    fn apply_grant_schedule_update(
        &self,
        schedule_id: Uuid,
        update: herald_domain::points::ports::GrantScheduleUpdate,
    ) -> impl std::future::Future<
        Output = Result<herald_domain::points::grant_schedule::PointsGrantSchedule, CoreError>,
    > + Send {
        let db = self.db.clone();
        async move {
            let model = points_grant_schedule::Entity::find_by_id(schedule_id)
                .one(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?
                .ok_or(CoreError::NotFound)?;

            let mut active_model: points_grant_schedule::ActiveModel = model.into();

            match update {
                herald_domain::points::ports::GrantScheduleUpdate::Disable => {
                    active_model.active = Set(false);
                }
            }

            active_model.updated_at = Set(chrono::Utc::now().into());

            let result = active_model
                .update(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            Self::model_to_grant_schedule(result)
        }
    }

    fn find_grant_schedule_by_subscription(
        &self,
        subscription_id: Uuid,
    ) -> impl std::future::Future<
        Output = Result<
            Option<herald_domain::points::grant_schedule::PointsGrantSchedule>,
            CoreError,
        >,
    > + Send {
        let db = self.db.clone();
        async move {
            let result = points_grant_schedule::Entity::find()
                .filter(points_grant_schedule::Column::SubscriptionId.eq(Some(subscription_id)))
                .one(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            result.map(Self::model_to_grant_schedule).transpose()
        }
    }

    fn create_grant_schedule(
        &self,
        schedule: herald_domain::points::grant_schedule::PointsGrantSchedule,
    ) -> impl std::future::Future<
        Output = Result<herald_domain::points::grant_schedule::PointsGrantSchedule, CoreError>,
    > + Send {
        let db = self.db.clone();
        async move {
            let active_model = points_grant_schedule::ActiveModel {
                id: Set(schedule.id),
                user_id: Set(schedule.user_id),
                realm_id: Set(schedule.realm_id.clone()),
                bucket_id: Set(schedule.bucket_id),
                subscription_id: Set(schedule.subscription_id),
                entitlement_key: Set(schedule.entitlement_key.unwrap_or_default()),
                grant_period_type: Set(schedule.grant_period_type.to_string()),
                base_time: Set(schedule.base_time.into()),
                next_grant_time: Set(schedule.next_grant_time.into()),
                points_per_period: Set(schedule.points_per_period),
                validity_days: Set(schedule.validity_days),
                granted_periods: Set(schedule.granted_periods),
                max_periods: Set(schedule.max_periods),
                active: Set(schedule.active),
                created_at: Set(schedule.created_at.into()),
                updated_at: Set(schedule.updated_at.into()),
            };

            let result = active_model
                .insert(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            Self::model_to_grant_schedule(result)
        }
    }

    fn update_grant_schedule(
        &self,
        schedule_id: Uuid,
        next_grant_time: chrono::DateTime<chrono::Utc>,
        granted_periods: i64,
        is_active: bool,
    ) -> impl std::future::Future<
        Output = Result<herald_domain::points::grant_schedule::PointsGrantSchedule, CoreError>,
    > + Send {
        let db = self.db.clone();
        async move {
            let model = points_grant_schedule::Entity::find_by_id(schedule_id)
                .one(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?
                .ok_or(CoreError::NotFound)?;

            let mut active_model: points_grant_schedule::ActiveModel = model.into();
            active_model.next_grant_time = Set(next_grant_time.into());
            active_model.granted_periods = Set(granted_periods);
            active_model.active = Set(is_active);
            active_model.updated_at = Set(chrono::Utc::now().into());

            let result = active_model
                .update(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            Self::model_to_grant_schedule(result)
        }
    }

    fn deactivate_grant_schedule(
        &self,
        schedule_id: Uuid,
    ) -> impl std::future::Future<Output = Result<(), CoreError>> + Send {
        let db = self.db.clone();
        async move {
            let model = points_grant_schedule::Entity::find_by_id(schedule_id)
                .one(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?
                .ok_or(CoreError::NotFound)?;

            let mut active: points_grant_schedule::ActiveModel = model.into();
            active.active = Set(false);
            active.updated_at = Set(chrono::Utc::now().into());

            active
                .update(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            Ok(())
        }
    }

    // ========== Grant Record Repository Methods (Iteration-2) ==========

    fn find_grant_record(
        &self,
        schedule_id: Uuid,
        period_number: i64,
    ) -> impl std::future::Future<
        Output = Result<
            Option<herald_domain::points::grant_schedule::PointsGrantRecord>,
            CoreError,
        >,
    > + Send {
        let db = self.db.clone();
        async move {
            let result = points_grant_record::Entity::find()
                .filter(points_grant_record::Column::ScheduleId.eq(schedule_id))
                .filter(points_grant_record::Column::PeriodNumber.eq(period_number))
                .one(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            result
                .map(|model| -> Result<_, CoreError> {
                    Ok(herald_domain::points::grant_schedule::PointsGrantRecord {
                        id: model.id,
                        schedule_id: model.schedule_id,
                        user_id: model.user_id,
                        realm_id: model.realm_id,
                        period_number: model.period_number,
                        granted_amount: model.granted_amount,
                        grant_time: chrono::DateTime::from(model.grant_time),
                        ledger_id: model.ledger_id,
                        created_at: chrono::DateTime::from(model.created_at),
                    })
                })
                .transpose()
        }
    }

    fn create_grant_record(
        &self,
        record: herald_domain::points::grant_schedule::PointsGrantRecord,
    ) -> impl std::future::Future<
        Output = Result<herald_domain::points::grant_schedule::PointsGrantRecord, CoreError>,
    > + Send {
        let db = self.db.clone();
        async move {
            let active_model = points_grant_record::ActiveModel {
                id: Set(record.id),
                schedule_id: Set(record.schedule_id),
                user_id: Set(record.user_id),
                realm_id: Set(record.realm_id.clone()),
                period_number: Set(record.period_number),
                granted_amount: Set(record.granted_amount),
                grant_time: Set(record.grant_time.into()),
                ledger_id: Set(record.ledger_id),
                created_at: Set(record.created_at.into()),
            };

            let result = active_model
                .insert(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            Ok(herald_domain::points::grant_schedule::PointsGrantRecord {
                id: result.id,
                schedule_id: result.schedule_id,
                user_id: result.user_id,
                realm_id: result.realm_id,
                period_number: result.period_number,
                granted_amount: result.granted_amount,
                grant_time: chrono::DateTime::from(result.grant_time),
                ledger_id: result.ledger_id,
                created_at: chrono::DateTime::from(result.created_at),
            })
        }
    }

    fn list_grant_records_by_schedule(
        &self,
        schedule_id: Uuid,
        pagination: &herald_domain::points::ports::Pagination,
    ) -> impl std::future::Future<
        Output = Result<
            herald_domain::points::entities::Paginated<
                herald_domain::points::grant_schedule::PointsGrantRecord,
            >,
            CoreError,
        >,
    > + Send {
        let db = self.db.clone();
        let pagination = *pagination;
        async move {
            let page = pagination.page;
            let page_size = pagination.page_size;

            let query = points_grant_record::Entity::find()
                .filter(points_grant_record::Column::ScheduleId.eq(schedule_id));

            let total = query
                .clone()
                .count(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            let results = query
                .order_by_desc(points_grant_record::Column::GrantTime)
                .paginate(&*db, page_size)
                .fetch_page(page - 1)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            let records = results
                .into_iter()
                .map(|model| {
                    Ok(herald_domain::points::grant_schedule::PointsGrantRecord {
                        id: model.id,
                        schedule_id: model.schedule_id,
                        user_id: model.user_id,
                        realm_id: model.realm_id,
                        period_number: model.period_number,
                        granted_amount: model.granted_amount,
                        grant_time: chrono::DateTime::from(model.grant_time),
                        ledger_id: model.ledger_id,
                        created_at: chrono::DateTime::from(model.created_at),
                    })
                })
                .collect::<Result<Vec<_>, CoreError>>()?;

            Ok(Paginated {
                total,
                page,
                page_size,
                data: records,
            })
        }
    }

    fn list_grant_records_by_user(
        &self,
        user_id: Uuid,
        pagination: &herald_domain::points::ports::Pagination,
    ) -> impl std::future::Future<
        Output = Result<
            herald_domain::points::entities::Paginated<
                herald_domain::points::grant_schedule::PointsGrantRecord,
            >,
            CoreError,
        >,
    > + Send {
        let db = self.db.clone();
        let pagination = *pagination;
        async move {
            let page = pagination.page;
            let page_size = pagination.page_size;

            let query = points_grant_record::Entity::find()
                .filter(points_grant_record::Column::UserId.eq(user_id));

            let total = query
                .clone()
                .count(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            let results = query
                .order_by_desc(points_grant_record::Column::GrantTime)
                .paginate(&*db, page_size)
                .fetch_page(page - 1)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            let records = results
                .into_iter()
                .map(|model| {
                    Ok(herald_domain::points::grant_schedule::PointsGrantRecord {
                        id: model.id,
                        schedule_id: model.schedule_id,
                        user_id: model.user_id,
                        realm_id: model.realm_id,
                        period_number: model.period_number,
                        granted_amount: model.granted_amount,
                        grant_time: chrono::DateTime::from(model.grant_time),
                        ledger_id: model.ledger_id,
                        created_at: chrono::DateTime::from(model.created_at),
                    })
                })
                .collect::<Result<Vec<_>, CoreError>>()?;

            Ok(Paginated {
                total,
                page,
                page_size,
                data: records,
            })
        }
    }

    // ========== Ledger Repository Methods (Consumption Priority) (Iteration-2) ==========

    fn find_active_ledgers_by_expiration(
        &self,
        realm_id: &str,
        user_id: Uuid,
    ) -> impl std::future::Future<Output = Result<Vec<PointsCreditLedger>, CoreError>> + Send {
        let db = self.db.clone();
        let realm_id = realm_id.to_string();
        async move {
            // Find all active ledgers with remaining balance > 0
            // Sort by expires_at ASC (soonest expiring first), NULL last (permanent credits)
            let ledgers = points_credit_ledger::Entity::find()
                .filter(points_credit_ledger::Column::RealmId.eq(&realm_id))
                .filter(points_credit_ledger::Column::UserId.eq(user_id))
                .filter(
                    points_credit_ledger::Column::Status.eq(CreditLedgerStatus::Active.to_string()),
                )
                .filter(points_credit_ledger::Column::RemainingAmount.gt(0))
                .order_by_asc(points_credit_ledger::Column::ExpiresAt) // NULL values sort last
                .order_by_asc(points_credit_ledger::Column::CreatedAt) // Fallback: FIFO for permanent credits
                .all(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            ledgers
                .into_iter()
                .map(|m| Ok(points_credit_ledger_from_model(m)))
                .collect()
        }
    }

    // ========== Statistics Repository Methods (Iteration-2) ==========

    fn count_paid_users_in_realm(
        &self,
        realm_id: &str,
        start_date: Option<chrono::DateTime<chrono::Utc>>,
        end_date: Option<chrono::DateTime<chrono::Utc>>,
    ) -> impl std::future::Future<Output = Result<u64, CoreError>> + Send {
        let pool = self.pool.clone();
        let realm_id = realm_id.to_string();
        async move {
            let mut query = r#"
                SELECT COUNT(DISTINCT user_id) as count
                FROM subscriptions
                WHERE realm_id = $1
                AND status = 'active'
            "#
            .to_string();

            let mut index = 2;

            if start_date.is_some() {
                query.push_str(&format!(" AND created_at >= ${}", index));
                index += 1;
            }

            if end_date.is_some() {
                query.push_str(&format!(" AND created_at <= ${}", index));
            }

            let mut query_builder = sqlx::query_as::<_, (i64,)>(&query).bind(realm_id);

            if let Some(start) = start_date {
                query_builder =
                    query_builder.bind(sea_orm::prelude::DateTimeWithTimeZone::from(start));
            }

            if let Some(end) = end_date {
                query_builder =
                    query_builder.bind(sea_orm::prelude::DateTimeWithTimeZone::from(end));
            }

            let result = query_builder
                .fetch_optional(&pool)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            result
                .map(|(count,)| count as u64)
                .ok_or(CoreError::NotFound)
        }
    }

    fn consume_points_atomic(
        &self,
        realm_id: &str,
        user_id: Uuid,
        client_app_id: Uuid,
        amount: i64,
        description: Option<String>,
        idempotency_key: Option<String>,
    ) -> impl std::future::Future<Output = Result<Vec<PointsTransaction>, CoreError>> + Send {
        let pool = self.pool.clone();
        let realm_id = realm_id.to_string();
        async move {
            let mut tx = pool
                .begin()
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            // Step 1 — idempotency replay. If the key already maps to a completed
            // consume, reassemble the original result set WITHOUT re-deducting.
            // Primary transaction → correlation_id → all N sibling transactions
            // (ordered by bucket_id) + their allocations. Legacy single-pool rows
            // with NULL correlation_id replay by the single primary transaction_id.
            if let Some(ref key) = idempotency_key
                && let Some(primary_txn_id) =
                    Self::check_completed_idempotency_in_tx(&mut tx, &realm_id, key).await?
            {
                let replayed =
                    Self::replay_consume_by_primary(&mut tx, &realm_id, primary_txn_id).await?;
                tx.commit()
                    .await
                    .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
                return Ok(replayed);
            }

            // Step 2 — correlation_id for THIS consume (shared by its N txns).
            let correlation_id = Uuid::now_v7().to_string();

            // Step 3 — resolve covered Bucket set for client_app_id (explicit rows
            // only, enabled=true; no default-bucket merging per A4).
            let covered_bucket_ids =
                Self::find_covered_bucket_ids_in_tx(&mut tx, &realm_id, client_app_id).await?;
            if covered_bucket_ids.is_empty() {
                return Err(CoreError::NoCoveredPointsPool { client_app_id });
            }

            // Step 4 — lock active ledgers across the WHOLE covered set, in
            // expires_at ASC NULLS LAST order (single FOR UPDATE, design §5.1).
            let ledgers = Self::find_active_ledgers_by_expiration_for_update(
                &mut tx,
                &realm_id,
                user_id,
                &covered_bucket_ids,
            )
            .await?;

            // Step 5 — precheck + pure allocation plan (unit-tested separately).
            let available: i64 = ledgers.iter().map(|l| l.remaining_amount).sum();
            if available < amount {
                return Err(CoreError::insufficient_points(amount, available));
            }
            let plan = Self::plan_consume_allocation(&ledgers, amount);

            // Step 6 — apply ledger deductions and group totals per bucket.
            // `per_bucket` keyed by bucket_id; processed in bucket_id ASC below for
            // deterministic wallet-lock ordering (deadlock avoidance).
            use std::collections::BTreeMap;
            #[derive(Default)]
            struct BucketAccumulator {
                wallet_id: Option<Uuid>,
                total: i64,
            }
            let mut per_bucket: BTreeMap<Uuid, BucketAccumulator> = BTreeMap::new();
            let mut allocations: Vec<PointsConsumptionAllocation> = Vec::new();
            // transaction_id is assigned per-bucket during the write loop below;
            // allocations reference the transaction of their owning bucket.
            let mut bucket_txn_id: BTreeMap<Uuid, Uuid> = BTreeMap::new();

            for planned in &plan.allocations {
                let ledger = &ledgers[planned.ledger_index];
                let updated_ledger = Self::update_ledger_in_tx(
                    &mut tx,
                    ledger.id,
                    LedgerUpdate::Consumption(planned.amount),
                )
                .await?;

                let bucket_id = ledger.bucket_id;
                // Resolve (or create) the wallet for this (user, bucket) inside the
                // same transaction; record it so all allocations in this bucket
                // share the wallet_id and the bucket transaction does too.
                let wallet_id = match per_bucket.get(&bucket_id).and_then(|a| a.wallet_id) {
                    Some(id) => id,
                    None => {
                        let wallet =
                            Self::ensure_wallet_in_tx(&mut tx, &realm_id, user_id, bucket_id)
                                .await?;
                        // Preserve the account-status business rule: a closed
                        // or frozen wallet must not be drained. The bucket
                        // model removed the single-wallet precheck, so the
                        // check lives here at the per-bucket consume write.
                        if wallet.status != WalletStatus::Active {
                            return Err(CoreError::BadRequest(format!(
                                "Cannot consume points from {} wallet",
                                wallet.status.as_str()
                            )));
                        }
                        per_bucket.entry(bucket_id).or_default().wallet_id = Some(wallet.id);
                        wallet.id
                    }
                };

                let acc = per_bucket.entry(bucket_id).or_default();
                acc.total += planned.amount;

                // Defer the transaction_id binding: we allocate it lazily the first
                // time a bucket is touched so allocations can reference it.
                let txn_id = *bucket_txn_id.entry(bucket_id).or_insert_with(Uuid::now_v7);

                allocations.push(PointsConsumptionAllocation {
                    id: Uuid::now_v7(),
                    transaction_id: txn_id,
                    ledger_id: updated_ledger.id,
                    wallet_id: Some(wallet_id),
                    user_id,
                    realm_id: realm_id.clone(),
                    bucket_id,
                    allocated_amount: planned.amount,
                    ledger_remaining_after: updated_ledger.remaining_amount,
                    created_at: chrono::Utc::now(),
                });
            }

            // Sanity: the plan guaranteed full coverage; guard against drift.
            let consumed_total: i64 = per_bucket.values().map(|a| a.total).sum();
            if consumed_total != amount {
                return Err(CoreError::InternalServerError(format!(
                    "consume allocation drift: planned {} but accumulated {}",
                    amount, consumed_total
                )));
            }

            // Step 7 — per-bucket wallet update + transaction write, bucket_id ASC
            // (BTreeMap iterates ascending). One transaction per affected bucket,
            // all sharing correlation_id; external_ref_id NULL (design §4.3.2/A6).
            //
            // BE-D11: `balance_after` is the REAL post-consume derived SUM for
            // this bucket (same predicate as `compute_available_balance`),
            // sourced in-tx so it reflects the just-applied ledger mutations.
            // The per-type consume split is preserved in the ledger `used_amount`
            // increments (Step 5) and in the consumption allocations (Step 8),
            // not in any Stored balance column (dropped, A7).
            let now = chrono::Utc::now();
            let mut transactions: Vec<PointsTransaction> = Vec::new();
            for (bucket_id, acc) in per_bucket.iter() {
                let wallet_id = acc.wallet_id.ok_or_else(|| {
                    CoreError::InternalServerError(format!(
                        "consume: bucket {} accumulator missing wallet_id",
                        bucket_id
                    ))
                })?;
                let delta = WalletDelta {
                    total_recharged: 0,
                    total_consumed: acc.total,
                    total_topup_granted: 0,
                    total_subscription_granted: 0,
                };
                let _updated_wallet =
                    Self::apply_wallet_delta_in_tx(&mut tx, wallet_id, delta).await?;

                // Real post-consume derived snapshot for THIS bucket only.
                let derived = Self::compute_available_balance_in_tx(
                    &mut tx,
                    &realm_id,
                    user_id,
                    std::slice::from_ref(bucket_id),
                    now,
                )
                .await?;
                let (balance_after, topup_after, subscription_after) =
                    Self::derived_to_balance_snapshots(&derived);

                let txn_id = *bucket_txn_id
                    .get(bucket_id)
                    .expect("bucket_txn_id populated alongside per_bucket");

                let transaction = Self::create_transaction_in_tx(
                    &mut tx,
                    PointsTransaction {
                        id: txn_id,
                        wallet_id,
                        user_id,
                        realm_id: realm_id.clone(),
                        bucket_id: *bucket_id,
                        transaction_type: TransactionType::Consume,
                        amount: -acc.total,
                        balance_after,
                        topup_balance_after: topup_after,
                        subscription_balance_after: subscription_after,
                        credit_type: None,
                        description: description.clone(),
                        client_app_id: Some(client_app_id),
                        subscription_id: None,
                        external_ref_id: None,
                        correlation_id: Some(correlation_id.clone()),
                        effective_at: None,
                        created_at: now,
                    },
                )
                .await?;
                transactions.push(transaction);
            }

            // Step 8 — write all allocations (ledger-level truth source, A6).
            for allocation in &allocations {
                Self::create_consumption_allocation_in_tx(&mut tx, allocation).await?;
            }

            // Step 9 — record idempotency. Primary = first transaction by bucket_id
            // ASC (transactions Vec is already in that order from the loop above).
            if let Some(ref key) = idempotency_key {
                let primary_txn_id = transactions.first().map(|t| t.id).ok_or_else(|| {
                    CoreError::InternalServerError("consume produced no transactions".to_string())
                })?;
                Self::record_completed_idempotency_in_tx(&mut tx, &realm_id, key, primary_txn_id)
                    .await?;
            }

            tx.commit()
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
            Ok(transactions)
        }
    }

    fn replay_consume_by_primary(
        &self,
        realm_id: &str,
        primary_txn_id: Uuid,
    ) -> impl std::future::Future<Output = Result<Vec<PointsTransaction>, CoreError>> + Send {
        let pool = self.pool.clone();
        let realm_id = realm_id.to_string();
        async move {
            // Read-only replay. Opens its own short transaction so the HTTP-layer
            // Redis-cache replay path can reassemble the original per-bucket
            // result set without re-deducting (design §5.1). Legacy single-pool
            // rows (NULL correlation_id) replay as a 1-element vec.
            let mut tx = pool
                .begin()
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
            let replayed =
                Self::replay_consume_by_primary(&mut tx, &realm_id, primary_txn_id).await?;
            tx.commit()
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
            Ok(replayed)
        }
    }

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
    ) -> impl std::future::Future<Output = Result<RevokePointsOutput, CoreError>> + Send {
        let pool = self.pool.clone();
        let realm_id = realm_id.to_string();
        async move {
            let mut tx = pool
                .begin()
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            if let Some(ref key) = idempotency_key
                && Self::check_completed_idempotency_in_tx(&mut tx, &realm_id, key)
                    .await?
                    .is_some()
            {
                tx.commit()
                    .await
                    .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
                return Ok(RevokePointsOutput::empty());
            }

            // 如果该 (user, bucket) 钱包不存在，返回"撤销 0 点"的结果（webhook 幂等处理）
            let _wallet = match Self::find_wallet_by_user_bucket_for_update(
                &mut tx, &realm_id, user_id, bucket_id,
            )
            .await?
            {
                Some(acc) => acc,
                None => {
                    // 钱包不存在，没有需要撤销的积分
                    tx.commit()
                        .await
                        .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
                    return Ok(RevokePointsOutput::empty());
                }
            };

            let ledgers = Self::find_active_ledgers_by_credit_type_and_bucket_for_update(
                &mut tx,
                &realm_id,
                user_id,
                bucket_id,
                credit_type,
            )
            .await?;

            let ledger_tuples: Vec<(Uuid, i64)> = ledgers
                .into_iter()
                .filter(|l| l.remaining_amount > 0)
                .map(|l| (l.id, l.remaining_amount))
                .collect();

            let (total_revoked, ledger_ids) = Self::revoke_ledger_list_in_tx(
                &mut tx,
                &realm_id,
                user_id,
                ledger_tuples,
                revocation_type,
                &reason,
                reference_id.as_deref(),
            )
            .await?;

            if let Some(ref key) = idempotency_key {
                Self::record_completed_idempotency_in_tx(&mut tx, &realm_id, key, Uuid::now_v7())
                    .await?;
            }

            tx.commit()
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
            Ok(RevokePointsOutput {
                revocation_id: Uuid::now_v7(),
                ledger_ids,
                total_revoked,
                revoked_at: chrono::Utc::now(),
            })
        }
    }

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
    ) -> impl std::future::Future<Output = Result<RevokePointsOutput, CoreError>> + Send {
        let pool = self.pool.clone();
        let realm_id = realm_id.to_string();
        let source_id = source_id.to_string();
        async move {
            let mut tx = pool
                .begin()
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            if let Some(ref key) = idempotency_key
                && Self::check_completed_idempotency_in_tx(&mut tx, &realm_id, key)
                    .await?
                    .is_some()
            {
                tx.commit()
                    .await
                    .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
                return Ok(RevokePointsOutput::empty());
            }

            let _wallet = match Self::find_wallet_by_user_bucket_for_update(
                &mut tx, &realm_id, user_id, bucket_id,
            )
            .await?
            {
                Some(acc) => acc,
                None => {
                    tx.commit()
                        .await
                        .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
                    return Ok(RevokePointsOutput::empty());
                }
            };

            // Find the specific ledger by source_id (scoped to the target bucket
            // so we never touch credits belonging to a different pool)
            let ledger = sqlx::query_as::<_, (Uuid, i64, String)>(
                "SELECT id, remaining_amount, credit_type FROM points_credit_ledger WHERE realm_id = $1 AND user_id = $2 AND bucket_id = $3 AND source_id = $4 AND remaining_amount > 0 FOR UPDATE"
            )
            .bind(&realm_id)
            .bind(user_id)
            .bind(bucket_id)
            .bind(&source_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            let Some((ledger_id, remaining_amount, credit_type_str)) = ledger else {
                // No active ledger for this source_id — nothing to revoke
                tx.commit()
                    .await
                    .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
                return Ok(RevokePointsOutput::empty());
            };

            let _credit_type: CreditType = credit_type_str.as_str().parse().map_err(|_| {
                CoreError::DatabaseError(format!(
                    "Invalid credit_type in ledger: {}",
                    credit_type_str
                ))
            })?;

            // Update ledger remaining_amount to 0
            let updated_ledger = Self::update_ledger_in_tx(
                &mut tx,
                ledger_id,
                LedgerUpdate::Revocation(remaining_amount),
            )
            .await?;

            // Create revocation record
            let record = PointsRevocationRecord {
                id: Uuid::now_v7(),
                ledger_id: updated_ledger.id,
                user_id,
                realm_id: realm_id.clone(),
                revocation_type,
                revoked_amount: remaining_amount,
                reason,
                reference_id,
                created_at: chrono::Utc::now(),
            };
            Self::create_revocation_record_in_tx(&mut tx, &record).await?;

            if let Some(ref key) = idempotency_key {
                Self::record_completed_idempotency_in_tx(&mut tx, &realm_id, key, Uuid::now_v7())
                    .await?;
            }

            tx.commit()
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            Ok(RevokePointsOutput {
                revocation_id: Uuid::now_v7(),
                ledger_ids: vec![updated_ledger.id],
                total_revoked: remaining_amount,
                revoked_at: chrono::Utc::now(),
            })
        }
    }

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
    ) -> impl std::future::Future<Output = Result<RevokePointsOutput, CoreError>> + Send {
        let pool = self.pool.clone();
        let realm_id = realm_id.to_string();
        let entitlement_key = entitlement_key.to_string();
        async move {
            let mut tx = pool
                .begin()
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            if let Some(ref key) = idempotency_key
                && Self::check_completed_idempotency_in_tx(&mut tx, &realm_id, key)
                    .await?
                    .is_some()
            {
                tx.commit()
                    .await
                    .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
                return Ok(RevokePointsOutput::empty());
            }

            let _wallet = match Self::find_wallet_by_user_bucket_for_update(
                &mut tx, &realm_id, user_id, bucket_id,
            )
            .await?
            {
                Some(acc) => acc,
                None => {
                    tx.commit()
                        .await
                        .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
                    return Ok(RevokePointsOutput::empty());
                }
            };

            // Match exact entitlement_key (initial grants) and entitlement_key:* (renewals).
            // Use colon separator to prevent "tier" matching "tier_premium".
            // Scope by bucket_id so cross-pool subscription credits are never touched.
            let exact = entitlement_key.clone();
            let prefix = format!("{}:%", entitlement_key);
            let ledgers = sqlx::query_as::<_, (Uuid, i64)>(
                "SELECT id, remaining_amount
                 FROM points_credit_ledger
                 WHERE realm_id = $1
                   AND user_id = $2
                   AND bucket_id = $3
                   AND credit_type = 'subscription_credit'
                   AND (source_id = $4 OR source_id LIKE $5)
                   AND status = 'active'
                   AND remaining_amount > 0
                 ORDER BY created_at ASC
                 FOR UPDATE",
            )
            .bind(&realm_id)
            .bind(user_id)
            .bind(bucket_id)
            .bind(&exact)
            .bind(&prefix)
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            let (total_revoked, ledger_ids) = Self::revoke_ledger_list_in_tx(
                &mut tx,
                &realm_id,
                user_id,
                ledgers,
                revocation_type,
                &reason,
                reference_id.as_deref(),
            )
            .await?;

            if let Some(ref key) = idempotency_key {
                Self::record_completed_idempotency_in_tx(&mut tx, &realm_id, key, Uuid::now_v7())
                    .await?;
            }

            tx.commit()
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
            Ok(RevokePointsOutput {
                revocation_id: Uuid::now_v7(),
                ledger_ids,
                total_revoked,
                revoked_at: chrono::Utc::now(),
            })
        }
    }

    fn revoke_topup_proportional_atomic(
        &self,
        realm_id: &str,
        user_id: Uuid,
        bucket_id: Uuid,
        refund_amount: i64,
        original_payment_amount: i64,
        refund_id: &str,
    ) -> impl std::future::Future<Output = Result<RevokePointsOutput, CoreError>> + Send {
        let pool = self.pool.clone();
        let realm_id = realm_id.to_string();
        let refund_id = refund_id.to_string();
        async move {
            let idempotency_key = format!("refund:topup:{}", refund_id);
            let mut tx = pool
                .begin()
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            if Self::check_completed_idempotency_in_tx(&mut tx, &realm_id, &idempotency_key)
                .await?
                .is_some()
            {
                tx.commit()
                    .await
                    .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
                return Ok(RevokePointsOutput::empty());
            }

            let _wallet = match Self::find_wallet_by_user_bucket_for_update(
                &mut tx, &realm_id, user_id, bucket_id,
            )
            .await?
            {
                Some(acc) => acc,
                None => {
                    Self::record_completed_idempotency_in_tx(
                        &mut tx,
                        &realm_id,
                        &idempotency_key,
                        Uuid::now_v7(),
                    )
                    .await?;
                    tx.commit()
                        .await
                        .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
                    return Ok(RevokePointsOutput::empty());
                }
            };

            let ledgers = Self::find_active_ledgers_by_credit_type_for_update(
                &mut tx,
                &realm_id,
                user_id,
                CreditType::TopupCredit,
            )
            .await?;

            let mut total_revoked = 0i64;
            let mut ledger_ids = Vec::new();
            for ledger in ledgers {
                if ledger.remaining_amount <= 0 {
                    continue;
                }
                // Only revoke ledgers in the target bucket — never dip into other pools.
                if ledger.bucket_id != bucket_id {
                    continue;
                }

                // Integer arithmetic with rounding: (a * b + b/2) / b ≈ round(a * b/b)
                let amount_to_revoke = (ledger.remaining_amount * refund_amount
                    + original_payment_amount / 2)
                    / original_payment_amount;
                if amount_to_revoke <= 0 {
                    continue;
                }

                let updated_ledger = Self::update_ledger_in_tx(
                    &mut tx,
                    ledger.id,
                    LedgerUpdate::Revocation(amount_to_revoke),
                )
                .await?;

                let record = PointsRevocationRecord {
                    id: Uuid::now_v7(),
                    ledger_id: updated_ledger.id,
                    user_id,
                    realm_id: realm_id.clone(),
                    revocation_type: RevocationType::RefundRevoke,
                    revoked_amount: amount_to_revoke,
                    reason: format!(
                        "Proportional refund ({}/{})",
                        refund_amount, original_payment_amount
                    ),
                    reference_id: Some(refund_id.clone()),
                    created_at: chrono::Utc::now(),
                };
                Self::create_revocation_record_in_tx(&mut tx, &record).await?;

                total_revoked += amount_to_revoke;
                ledger_ids.push(updated_ledger.id);
            }

            Self::record_completed_idempotency_in_tx(
                &mut tx,
                &realm_id,
                &idempotency_key,
                Uuid::now_v7(),
            )
            .await?;

            tx.commit()
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            Ok(RevokePointsOutput {
                revocation_id: Uuid::now_v7(),
                ledger_ids,
                total_revoked,
                revoked_at: chrono::Utc::now(),
            })
        }
    }

    fn refund_points_atomic(
        &self,
        realm_id: &str,
        user_id: Uuid,
        bucket_id: Uuid,
        refund_reference: String,
        refund_amount: i64,
        reason: String,
    ) -> impl std::future::Future<Output = Result<PointsTransaction, CoreError>> + Send {
        let pool = self.pool.clone();
        let realm_id = realm_id.to_string();
        async move {
            let mut tx = pool
                .begin()
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            if let Some(existing) = sqlx::query_as::<_, PointsTransactionRow>(
                r#"
                SELECT id, realm_id, wallet_id, user_id, bucket_id, type, amount, balance_after,
                       topup_balance_after, subscription_balance_after, credit_type,
                       description, client_app_id, subscription_id, external_ref_id, correlation_id,
                       created_at, updated_at, expires_at
                FROM points_transactions
                WHERE realm_id = $1 AND user_id = $2 AND bucket_id = $3 AND external_ref_id = $4 AND type = 'refund'
                LIMIT 1
                "#,
            )
            .bind(&realm_id)
            .bind(user_id)
            .bind(bucket_id)
            .bind(&refund_reference)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?
            {
                tx.commit()
                    .await
                    .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
                return Self::points_transaction_row_to_domain(existing);
            }

            let wallet =
                Self::find_wallet_by_user_bucket_for_update(&mut tx, &realm_id, user_id, bucket_id)
                    .await?
                    .ok_or(CoreError::NotFound)?;
            // BE-D11: `wallet.total_balance` no longer exists (column dropped,
            // A7). The refund availability precheck uses the derived available
            // balance for this bucket (same predicate as consumption — "seen
            // balance == spendable balance"). The subsequent ledger-drain loop
            // is the authoritative revocation; this precheck is an early
            // user-facing error for the obviously-insufficient case.
            let now = chrono::Utc::now();
            let derived = Self::compute_available_balance_in_tx(
                &mut tx,
                &realm_id,
                user_id,
                std::slice::from_ref(&bucket_id),
                now,
            )
            .await?;
            let available: i64 = derived.iter().map(|(_, amt)| amt).sum();
            if available < refund_amount {
                return Err(CoreError::BadRequest(format!(
                    "Insufficient balance: available={}, required={}",
                    available, refund_amount
                )));
            }

            // Refund = revoke the refunded points from the ledger so the
            // projection (wallet columns) and the source of truth (ledger
            // remaining_amount) never diverge. Drain across all five credit
            // types in a fixed priority so a refund against any balance
            // composition actually revokes — same priority as consume (design
            // §5.1). Bucket-scoped ledger locks prevent cross-bucket contention
            // (review #8) since refunds target a single Bucket.
            let refund_types = [
                CreditType::SubscriptionCredit,
                CreditType::TopupCredit,
                CreditType::GrantedCredit,
                CreditType::RegistrationCredit,
                CreditType::FreePeriodicCredit,
            ];
            let mut total_revoked = 0i64;
            // BE-D11: per-type revoke counters are no longer consumed (the
            // wallet delta is zero and the derived SUM replaces the typed
            // snapshot source), but the per-credit-type drain loop still
            // determines WHICH ledgers to revoke. Counters retained with
            // leading underscore to keep the per-type accounting visible for
            // debugging without tripping dead-code warnings.
            let mut _subscription_revoked = 0i64;
            let mut _topup_revoked = 0i64;
            let mut _granted_revoked = 0i64;
            let mut _registration_revoked = 0i64;
            let mut _free_periodic_revoked = 0i64;
            for credit_type in refund_types {
                let still_to_revoke = refund_amount - total_revoked;
                if still_to_revoke <= 0 {
                    break;
                }
                // Bucket-scoped lock: refund targets the requested bucket only.
                let ledgers = Self::find_active_ledgers_by_credit_type_and_bucket_for_update(
                    &mut tx,
                    &realm_id,
                    user_id,
                    bucket_id,
                    credit_type,
                )
                .await?;
                let mut remaining = still_to_revoke;
                let mut plan: Vec<(Uuid, i64)> = Vec::new();
                for ledger in ledgers.into_iter() {
                    if remaining <= 0 {
                        break;
                    }
                    let take = ledger.remaining_amount.min(remaining);
                    if take <= 0 {
                        continue;
                    }
                    plan.push((ledger.id, take));
                    remaining -= take;
                }
                if plan.is_empty() {
                    continue;
                }
                let (revoked, _) = Self::revoke_ledger_list_in_tx(
                    &mut tx,
                    &realm_id,
                    user_id,
                    plan,
                    RevocationType::RefundRevoke,
                    &reason,
                    Some(&refund_reference),
                )
                .await?;
                total_revoked += revoked;
                match credit_type {
                    CreditType::SubscriptionCredit => _subscription_revoked += revoked,
                    CreditType::TopupCredit => _topup_revoked += revoked,
                    CreditType::GrantedCredit => _granted_revoked += revoked,
                    CreditType::RegistrationCredit => _registration_revoked += revoked,
                    CreditType::FreePeriodicCredit => _free_periodic_revoked += revoked,
                }
            }

            // Fail loud (Rule 12): if the precheck passed but draining across all
            // five types still did not cover the refund (e.g. the balance changed
            // between read and lock, or remaining_amounts did not match the
            // projection), never write a 0/partial transaction silently.
            if total_revoked != refund_amount {
                return Err(CoreError::InternalServerError(format!(
                    "refund revoke drift: requested {} but revoked {}",
                    refund_amount, total_revoked
                )));
            }

            // Apply the net revocation to the wallet analytics (single writer).
            // BE-D11: revocation advances no analytics column (revocation moves
            // `remaining_amount` only), so the delta is zero; kept for explicit
            // single-writer discipline. The post-revoke available balance for
            // `balance_after` is sourced from the in-tx derived SUM below.
            let delta = WalletDelta::zero();
            let _ = Self::apply_wallet_delta_in_tx(&mut tx, wallet.id, delta).await?;

            let now = chrono::Utc::now();
            let derived = Self::compute_available_balance_in_tx(
                &mut tx,
                &realm_id,
                user_id,
                std::slice::from_ref(&bucket_id),
                now,
            )
            .await?;
            let (balance_after, topup_after, subscription_after) =
                Self::derived_to_balance_snapshots(&derived);

            let transaction = Self::create_transaction_in_tx(
                &mut tx,
                PointsTransaction {
                    id: Uuid::now_v7(),
                    wallet_id: wallet.id,
                    user_id,
                    realm_id: realm_id.clone(),
                    bucket_id,
                    transaction_type: TransactionType::Refund,
                    amount: -total_revoked,
                    balance_after,
                    topup_balance_after: topup_after,
                    subscription_balance_after: subscription_after,
                    credit_type: None,
                    description: Some(reason),
                    client_app_id: None,
                    subscription_id: None,
                    external_ref_id: Some(refund_reference),
                    correlation_id: None,
                    effective_at: None,
                    created_at: now,
                },
            )
            .await?;

            tx.commit()
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
            Ok(transaction)
        }
    }

    fn grant_points_atomic(
        &self,
        realm_id: &str,
        user_id: Uuid,
        bucket_id: Uuid,
        credit_type: CreditType,
        source_type: CreditSourceType,
        amount: i64,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
        effective_at: Option<chrono::DateTime<chrono::Utc>>,
        source_id: Option<String>,
        description: Option<String>,
        idempotency_key: Option<String>,
    ) -> impl std::future::Future<Output = Result<PointsCreditLedger, CoreError>> + Send {
        let pool = self.pool.clone();
        let realm_id = realm_id.to_string();
        async move {
            // Verify user exists in the account table before granting points
            let user_exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM account WHERE id = $1 AND realm_id = $2)",
            )
            .bind(user_id)
            .bind(&realm_id)
            .fetch_one(&pool)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            if !user_exists {
                return Err(CoreError::NotFound);
            }

            let mut tx = pool
                .begin()
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            // Idempotency guard: if a completed record exists, return a zero-amount placeholder
            if let Some(ref key) = idempotency_key
                && Self::check_completed_idempotency_in_tx(&mut tx, &realm_id, key)
                    .await?
                    .is_some()
            {
                tx.commit()
                    .await
                    .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
                let now = chrono::Utc::now();
                return Ok(PointsCreditLedger {
                    id: Uuid::now_v7(),
                    user_id,
                    realm_id,
                    bucket_id,
                    credit_type,
                    source_type,
                    source_id: "idempotency".to_string(),
                    granted_amount: 0,
                    used_amount: 0,
                    revoked_amount: 0,
                    remaining_amount: 0,
                    expires_at: None,
                    effective_at,
                    status: CreditLedgerStatus::Active,
                    created_at: now,
                    updated_at: now,
                });
            }

            let wallet = Self::ensure_wallet_in_tx(&mut tx, &realm_id, user_id, bucket_id).await?;
            if wallet.status != WalletStatus::Active {
                return Err(CoreError::BadRequest(format!(
                    "Cannot grant points to {} wallet",
                    wallet.status.as_str()
                )));
            }
            let source_id = source_id.unwrap_or_else(|| "system".to_string());
            let now = chrono::Utc::now();
            let ledger = PointsCreditLedger {
                id: Uuid::now_v7(),
                user_id,
                realm_id: realm_id.clone(),
                bucket_id,
                credit_type,
                source_type,
                source_id: source_id.clone(),
                granted_amount: amount,
                used_amount: 0,
                revoked_amount: 0,
                remaining_amount: amount,
                expires_at,
                effective_at,
                status: CreditLedgerStatus::Active,
                created_at: now,
                updated_at: now,
            };
            let created_ledger = Self::create_ledger_in_tx(&mut tx, &ledger).await?;
            let delta = WalletDelta::grant(credit_type, amount);
            let _ = Self::apply_wallet_delta_in_tx(&mut tx, wallet.id, delta).await?;
            // BE-D11: balance_after = real post-grant derived SUM for this
            // bucket (in-tx, reflects the just-inserted ledger row). For grant
            // rows with a future `effective_at`, the derived SUM predicate
            // excludes the row until effective_at <= NOW(), so balance_after
            // reflects the immediately-available balance (matching the user's
            // visible balance semantics).
            let derived = Self::compute_available_balance_in_tx(
                &mut tx,
                &realm_id,
                user_id,
                std::slice::from_ref(&bucket_id),
                now,
            )
            .await?;
            let (balance_after, topup_after, subscription_after) =
                Self::derived_to_balance_snapshots(&derived);
            let transaction_type = Self::determine_transaction_type(credit_type, source_type);
            let tx_description = description
                .unwrap_or_else(|| format!("{}: {} points granted", source_type.as_str(), amount));
            let transaction_id = Uuid::now_v7();
            // Use a unique external_ref_id by combining source_id with transaction_id
            // to avoid unique constraint violations when the same admin grants points
            // to the same user multiple times
            let external_ref_id = format!("{}:{}", source_id, transaction_id);
            let _ = Self::create_transaction_in_tx(
                &mut tx,
                PointsTransaction {
                    id: transaction_id,
                    wallet_id: wallet.id,
                    user_id,
                    realm_id: realm_id.clone(),
                    bucket_id,
                    transaction_type,
                    amount,
                    balance_after,
                    topup_balance_after: topup_after,
                    subscription_balance_after: subscription_after,
                    credit_type: Some(credit_type),
                    description: Some(tx_description),
                    client_app_id: None,
                    subscription_id: None,
                    external_ref_id: Some(external_ref_id),
                    correlation_id: None,
                    effective_at,
                    created_at: now,
                },
            )
            .await?;

            // Record idempotency key after successful grant
            if let Some(ref key) = idempotency_key {
                Self::record_completed_idempotency_in_tx(&mut tx, &realm_id, key, transaction_id)
                    .await?;
            }

            tx.commit()
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
            Ok(created_ledger)
        }
    }

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
    ) -> impl std::future::Future<Output = Result<PointsTransaction, CoreError>> + Send {
        let pool = self.pool.clone();
        let realm_id = realm_id.to_string();
        async move {
            let mut tx = pool
                .begin()
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            let wallet = Self::ensure_wallet_in_tx(&mut tx, &realm_id, user_id, bucket_id).await?;

            if wallet.status != WalletStatus::Active {
                return Err(CoreError::BadRequest(format!(
                    "Cannot recharge points to {} wallet",
                    wallet.status.as_str()
                )));
            }

            let resolved_source_id = source_id.unwrap_or_else(|| {
                external_ref_id
                    .clone()
                    .unwrap_or_else(|| "system".to_string())
            });
            let now = chrono::Utc::now();

            let ledger = PointsCreditLedger {
                id: Uuid::now_v7(),
                user_id,
                realm_id: realm_id.clone(),
                bucket_id,
                credit_type,
                source_type,
                source_id: resolved_source_id.clone(),
                granted_amount: amount,
                used_amount: 0,
                revoked_amount: 0,
                remaining_amount: amount,
                expires_at,
                effective_at: None,
                status: CreditLedgerStatus::Active,
                created_at: now,
                updated_at: now,
            };

            Self::create_ledger_in_tx(&mut tx, &ledger).await?;

            let delta = WalletDelta::grant(credit_type, amount);
            let _ = Self::apply_wallet_delta_in_tx(&mut tx, wallet.id, delta).await?;

            // BE-D11: balance_after = real post-recharge derived SUM for this
            // bucket (recharge is immediately available: effective_at = NULL).
            let derived = Self::compute_available_balance_in_tx(
                &mut tx,
                &realm_id,
                user_id,
                std::slice::from_ref(&bucket_id),
                now,
            )
            .await?;
            let (balance_after, topup_after, subscription_after) =
                Self::derived_to_balance_snapshots(&derived);

            let transaction_type = Self::determine_transaction_type(credit_type, source_type);

            let transaction = Self::create_transaction_in_tx(
                &mut tx,
                PointsTransaction {
                    id: Uuid::now_v7(),
                    wallet_id: wallet.id,
                    user_id,
                    realm_id: realm_id.clone(),
                    bucket_id,
                    transaction_type,
                    amount,
                    balance_after,
                    topup_balance_after: topup_after,
                    subscription_balance_after: subscription_after,
                    credit_type: Some(credit_type),
                    description: Some(format!("Points recharge ({})", source_type.as_str())),
                    client_app_id: None,
                    subscription_id: None,
                    external_ref_id,
                    correlation_id: None,
                    effective_at: None,
                    created_at: now,
                },
            )
            .await?;

            tx.commit()
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            Ok(transaction)
        }
    }

    fn set_subscription_ledger_expiration_atomic(
        &self,
        realm_id: &str,
        user_id: Uuid,
        period_end: chrono::DateTime<chrono::Utc>,
    ) -> impl std::future::Future<Output = Result<Vec<Uuid>, CoreError>> + Send {
        let pool = self.pool.clone();
        let realm_id = realm_id.to_string();
        async move {
            let mut tx = pool
                .begin()
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
            let ledgers = Self::find_active_ledgers_by_credit_type_for_update(
                &mut tx,
                &realm_id,
                user_id,
                CreditType::SubscriptionCredit,
            )
            .await?;
            let mut ids = Vec::new();
            for ledger in ledgers {
                let updated = Self::update_ledger_in_tx(
                    &mut tx,
                    ledger.id,
                    LedgerUpdate::SetExpiration(period_end),
                )
                .await?;
                ids.push(updated.id);
            }
            tx.commit()
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
            Ok(ids)
        }
    }

    fn handle_subscription_paid_atomic(
        &self,
        realm_id: &str,
        user_id: Uuid,
        bucket_id: Uuid,
        entitlement_key: String,
        points_amount: i64,
        source_type: CreditSourceType,
        period_start: chrono::DateTime<chrono::Utc>,
        period_end: chrono::DateTime<chrono::Utc>,
        idempotency_key: String,
        disable_daily_grant: bool,
    ) -> impl std::future::Future<Output = Result<PointsCreditLedger, CoreError>> + Send {
        let pool = self.pool.clone();
        let realm_id = realm_id.to_string();
        async move {
            let mut tx = pool
                .begin()
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
            if Self::check_completed_idempotency_in_tx(&mut tx, &realm_id, &idempotency_key)
                .await?
                .is_some()
            {
                tx.commit()
                    .await
                    .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
                let now = chrono::Utc::now();
                return Ok(PointsCreditLedger {
                    id: Uuid::now_v7(),
                    user_id,
                    realm_id,
                    bucket_id,
                    credit_type: CreditType::SubscriptionCredit,
                    source_type: CreditSourceType::SubscriptionInitial,
                    source_id: "idempotency".to_string(),
                    granted_amount: 0,
                    used_amount: 0,
                    revoked_amount: 0,
                    remaining_amount: 0,
                    expires_at: None,
                    effective_at: Some(period_start),
                    status: CreditLedgerStatus::Active,
                    created_at: now,
                    updated_at: now,
                });
            }

            // BE-D06: route by subscription.bucket_id (design §5.5) so the grant
            // lands in the subscription's bound Bucket.
            let wallet = Self::ensure_wallet_in_tx(&mut tx, &realm_id, user_id, bucket_id).await?;
            if disable_daily_grant {
                // Free-periodic credits may sit in any of the user's Buckets, not
                // just the subscription's bound Bucket (design §5.2/A4). Revoke
                // them per distinct ledger.bucket_id so we never drive a pool's
                // free_periodic_balance negative. The subscription `wallet` (bound
                // to `bucket_id`) is still the destination for the grant below; we
                // resolve additional pool wallets here as needed.
                let daily_ledgers = Self::find_active_ledgers_by_credit_type_for_update(
                    &mut tx,
                    &realm_id,
                    user_id,
                    CreditType::FreePeriodicCredit,
                )
                .await?;
                for ledger in daily_ledgers {
                    let amount = ledger.remaining_amount;
                    let updated = Self::update_ledger_in_tx(
                        &mut tx,
                        ledger.id,
                        LedgerUpdate::Revocation(amount),
                    )
                    .await?;
                    let record = PointsRevocationRecord {
                        id: Uuid::now_v7(),
                        ledger_id: updated.id,
                        user_id,
                        realm_id: realm_id.clone(),
                        revocation_type: RevocationType::UpgradeRevoke,
                        revoked_amount: amount,
                        reason: "Free user upgraded to paid subscription".to_string(),
                        reference_id: Some(idempotency_key.clone()),
                        created_at: chrono::Utc::now(),
                    };
                    Self::create_revocation_record_in_tx(&mut tx, &record).await?;
                }
                Self::disable_periodic_grant_in_tx(&mut tx, &realm_id, user_id).await?;
            }

            let now = chrono::Utc::now();
            let ledger = PointsCreditLedger {
                id: Uuid::now_v7(),
                user_id,
                realm_id: realm_id.clone(),
                bucket_id,
                credit_type: CreditType::SubscriptionCredit,
                source_type,
                source_id: format!("{}:{}", entitlement_key, idempotency_key),
                granted_amount: points_amount,
                used_amount: 0,
                revoked_amount: 0,
                remaining_amount: points_amount,
                expires_at: Some(period_end),
                effective_at: Some(period_start),
                status: CreditLedgerStatus::Active,
                created_at: now,
                updated_at: now,
            };
            let created_ledger = Self::create_ledger_in_tx(&mut tx, &ledger).await?;
            let delta = WalletDelta::grant(CreditType::SubscriptionCredit, points_amount);
            let _ = Self::apply_wallet_delta_in_tx(&mut tx, wallet.id, delta).await?;
            // BE-D11: balance_after = real post-grant derived SUM for this
            // bucket. The just-written ledger row has effective_at=period_start;
            // when period_start <= now (activation) it is included in the
            // derived SUM, when period_start > now (pre-grant) it is excluded.
            let derived = Self::compute_available_balance_in_tx(
                &mut tx,
                &realm_id,
                user_id,
                std::slice::from_ref(&bucket_id),
                now,
            )
            .await?;
            let (balance_after, topup_after, subscription_after) =
                Self::derived_to_balance_snapshots(&derived);
            let transaction = Self::create_transaction_in_tx(
                &mut tx,
                PointsTransaction {
                    id: Uuid::now_v7(),
                    wallet_id: wallet.id,
                    user_id,
                    realm_id: realm_id.clone(),
                    bucket_id,
                    transaction_type: if source_type == CreditSourceType::SubscriptionRenewal {
                        TransactionType::SubscriptionRenewal
                    } else {
                        TransactionType::SubscriptionGrant
                    },
                    amount: points_amount,
                    balance_after,
                    topup_balance_after: topup_after,
                    subscription_balance_after: subscription_after,
                    credit_type: Some(CreditType::SubscriptionCredit),
                    description: Some(format!(
                        "Subscription {}: {} points",
                        if source_type == CreditSourceType::SubscriptionRenewal {
                            "renewal"
                        } else {
                            "initial"
                        },
                        points_amount
                    )),
                    client_app_id: None,
                    subscription_id: None,
                    external_ref_id: Some(idempotency_key.clone()),
                    correlation_id: None,
                    effective_at: Some(period_start),
                    created_at: now,
                },
            )
            .await?;
            Self::record_completed_idempotency_in_tx(
                &mut tx,
                &realm_id,
                &idempotency_key,
                transaction.id,
            )
            .await?;
            tx.commit()
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
            Ok(created_ledger)
        }
    }

    fn scan_and_expire_points_atomic(
        &self,
        batch_size: usize,
    ) -> impl std::future::Future<Output = Result<ExpirationSummary, CoreError>> + Send {
        let pool = self.pool.clone();
        async move {
            let mut tx = pool
                .begin()
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
            let now = chrono::Utc::now();
            let ledgers = Self::find_expired_ledgers_for_update(&mut tx, now, batch_size).await?;
            if ledgers.is_empty() {
                tx.commit()
                    .await
                    .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
                return Ok(ExpirationSummary {
                    expired_count: 0,
                    total_expired: 0,
                    expired_at: now,
                });
            }

            let mut total_expired = 0i64;
            for ledger in &ledgers {
                let amount = ledger.remaining_amount;
                total_expired += amount;
                let _ = Self::update_ledger_in_tx(
                    &mut tx,
                    ledger.id,
                    LedgerUpdate::SetStatus(CreditLedgerStatus::Expired),
                )
                .await?;
                let record = PointsRevocationRecord {
                    id: Uuid::now_v7(),
                    ledger_id: ledger.id,
                    user_id: ledger.user_id,
                    realm_id: ledger.realm_id.clone(),
                    revocation_type: RevocationType::ExpireRevoke,
                    revoked_amount: amount,
                    reason: "Points expired".to_string(),
                    reference_id: None,
                    created_at: now,
                };
                Self::create_revocation_record_in_tx(&mut tx, &record).await?;
                // Lock this ledger's bound wallet (per-ledger by bucket_id, design
                // §5.2/A4) and assert it exists; the revoke no longer mutates the
                // wallet projection (BE-D11 derived balance).
                let bucket_id = ledger.bucket_id;
                let _wallet = Self::find_wallet_by_user_bucket_for_update(
                    &mut tx,
                    &ledger.realm_id,
                    ledger.user_id,
                    bucket_id,
                )
                .await?
                .ok_or(CoreError::NotFound)?;
            }

            tx.commit()
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
            Ok(ExpirationSummary {
                expired_count: ledgers.len(),
                total_expired,
                expired_at: now,
            })
        }
    }

    // ========== point-time: derived balance + pre-grant + reclaim ==========

    /// Derived available balance SUM(remaining_amount) grouped by credit_type
    /// (design §5.1 A7). Same predicate as consumption selection — "seen
    /// balance == spendable balance" — so future-effective rows are excluded
    /// from the user-visible balance and bucket totals. Replaces reading
    /// `points_wallets` Stored balance columns for available-balance semantics.
    ///
    /// `bucket_ids` empty ⟺ aggregate across ALL the user's buckets (used by
    /// `get_balance`'s user-total view); non-empty ⟺ restrict to listed
    /// buckets (per-bucket grant responses). Empty-slice maps to no
    /// `bucket_id` filter (the predicate still scopes by realm+user).
    fn compute_available_balance(
        &self,
        realm_id: &str,
        user_id: Uuid,
        bucket_ids: &[Uuid],
        now: chrono::DateTime<chrono::Utc>,
    ) -> impl std::future::Future<Output = Result<Vec<(CreditType, i64)>, CoreError>> + Send {
        let pool = self.pool.clone();
        let realm_id = realm_id.to_string();
        let bucket_ids = bucket_ids.to_vec();
        async move {
            // Coalesce to a sentinel-safe form: empty slice ⇒ no bucket_id
            // filter (aggregate across all the user's buckets). ANY() on an
            // empty array evaluates to FALSE, so we branch the SQL.
            let rows: Vec<(String, i64)> = if bucket_ids.is_empty() {
                sqlx::query_as(
                    r#"
                    SELECT credit_type, COALESCE(SUM(remaining_amount), 0)::BIGINT AS available
                    FROM points_credit_ledger
                    WHERE realm_id = $1
                      AND user_id = $2
                      AND status = 'active'
                      AND remaining_amount > 0
                      AND (effective_at IS NULL OR effective_at <= $3)
                      AND (expires_at  IS NULL OR expires_at  >  $3)
                    GROUP BY credit_type
                    "#,
                )
                .bind(&realm_id)
                .bind(user_id)
                .bind(now)
                .fetch_all(&pool)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?
            } else {
                sqlx::query_as(
                    r#"
                    SELECT credit_type, COALESCE(SUM(remaining_amount), 0)::BIGINT AS available
                    FROM points_credit_ledger
                    WHERE realm_id = $1
                      AND user_id = $2
                      AND bucket_id = ANY($3)
                      AND status = 'active'
                      AND remaining_amount > 0
                      AND (effective_at IS NULL OR effective_at <= $4)
                      AND (expires_at  IS NULL OR expires_at  >  $4)
                    GROUP BY credit_type
                    "#,
                )
                .bind(&realm_id)
                .bind(user_id)
                .bind(&bucket_ids)
                .bind(now)
                .fetch_all(&pool)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?
            };

            rows.into_iter()
                .map(|(credit_type, amount)| {
                    let credit_type: CreditType = credit_type.parse().map_err(|_| {
                        CoreError::DatabaseError(format!("invalid credit_type: {credit_type}"))
                    })?;
                    Ok((credit_type, amount))
                })
                .collect()
        }
    }

    /// Derived available balance broken down by `(bucket_id, credit_type)`
    /// (design §5.1). Same predicate as `compute_available_balance`, used by
    /// bucket overview / bucket delete guard / `list_wallets` bulk-derived
    /// assembly so they no longer read `points_wallets.total_balance` (avoids
    /// future-effective leakage and bucket mis-judgement).
    ///
    /// NOTE: `bucket_ids` empty ⟺ no bucket filter (aggregate every bucket in
    /// the realm). Callers that need "current page only" should pass the
    /// concrete bucket_id list.
    fn compute_bucket_available_balances(
        &self,
        realm_id: &str,
        bucket_ids: &[Uuid],
        now: chrono::DateTime<chrono::Utc>,
    ) -> impl std::future::Future<Output = Result<Vec<(Uuid, CreditType, i64)>, CoreError>> + Send
    {
        let pool = self.pool.clone();
        let realm_id = realm_id.to_string();
        let bucket_ids = bucket_ids.to_vec();
        async move {
            let rows: Vec<(Uuid, String, i64)> = if bucket_ids.is_empty() {
                sqlx::query_as(
                    r#"
                    SELECT bucket_id, credit_type, COALESCE(SUM(remaining_amount), 0)::BIGINT AS available
                    FROM points_credit_ledger
                    WHERE realm_id = $1
                      AND status = 'active'
                      AND remaining_amount > 0
                      AND (effective_at IS NULL OR effective_at <= $2)
                      AND (expires_at  IS NULL OR expires_at  >  $2)
                    GROUP BY bucket_id, credit_type
                    "#,
                )
                .bind(&realm_id)
                .bind(now)
                .fetch_all(&pool)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?
            } else {
                sqlx::query_as(
                    r#"
                    SELECT bucket_id, credit_type, COALESCE(SUM(remaining_amount), 0)::BIGINT AS available
                    FROM points_credit_ledger
                    WHERE realm_id = $1
                      AND bucket_id = ANY($2)
                      AND status = 'active'
                      AND remaining_amount > 0
                      AND (effective_at IS NULL OR effective_at <= $3)
                      AND (expires_at  IS NULL OR expires_at  >  $3)
                    GROUP BY bucket_id, credit_type
                    "#,
                )
                .bind(&realm_id)
                .bind(&bucket_ids)
                .bind(now)
                .fetch_all(&pool)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?
            };

            rows.into_iter()
                .map(|(bucket_id, credit_type, amount)| {
                    let credit_type: CreditType = credit_type.parse().map_err(|_| {
                        CoreError::DatabaseError(format!("invalid credit_type: {credit_type}"))
                    })?;
                    Ok((bucket_id, credit_type, amount))
                })
                .collect()
        }
    }

    /// Pre-grant the next period for a schedule (design §5.2 / §5.3). Writes a
    /// ledger row carrying `effective_at`/`expires_at` PLUS a
    /// `points_grant_records(schedule_id, period_number)` row (UNIQUE
    /// idempotency) linked to the new ledger via `ledger_id` FK. Idempotent:
    /// a re-call for an already-written `(schedule_id, period_number)` returns
    /// the existing ledger without re-writing.
    ///
    /// **BE-D05 scope.** Real transactional pre-grant: `FOR UPDATE schedule`
    /// (serializes concurrent callers), check
    /// `points_grant_records(schedule_id, period_number)` existence (HIT →
    /// return existing ledger via `ledger_id` FK bridge), else create ledger +
    /// wallet delta + transaction record (mirroring `grant_points_atomic`
    /// in-tx), INSERT `points_grant_records(... ledger_id = ledger.id ...)`
    /// (BE-D01 FK), and advance the schedule's `next_grant_time` /
    /// `granted_periods` to the just-granted period.
    fn pregrant_next_period_atomic(
        &self,
        realm_id: &str,
        schedule: &herald_domain::points::grant_schedule::PointsGrantSchedule,
        period_number: u32,
        effective_at: Option<chrono::DateTime<chrono::Utc>>,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> impl std::future::Future<Output = Result<PointsCreditLedger, CoreError>> + Send {
        let pool = self.pool.clone();
        let realm_id = realm_id.to_string();
        let schedule = schedule.clone();
        async move {
            let period_number_i64 = i64::from(period_number);
            let mut tx = pool
                .begin()
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            // 1) Lock the schedule row for the duration of this pre-grant so
            //    concurrent callers serialize on the (schedule_id, period_number)
            //    idempotency check. Re-read current state inside the lock.
            let row =
                sqlx::query("SELECT active FROM points_grant_schedules WHERE id = $1 FOR UPDATE")
                    .bind(schedule.id)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
            let active: bool = row
                .as_ref()
                .and_then(|r| r.try_get("active").ok())
                .ok_or_else(|| {
                    CoreError::InternalServerError(format!(
                        "pregrant: schedule {} not found (cannot lock)",
                        schedule.id
                    ))
                })?;
            if !active {
                return Err(CoreError::BadRequest(format!(
                    "pregrant: schedule {} is not active",
                    schedule.id
                )));
            }

            // 2) Idempotency: if a grant_record already exists for this
            //    (schedule_id, period_number), return its ledger row via the
            //    ledger_id FK bridge (design §4.3.2 / §5.2).
            let existing_ledger_id: Option<Uuid> = sqlx::query_scalar(
                "SELECT ledger_id FROM points_grant_records WHERE schedule_id = $1 AND period_number = $2 LIMIT 1",
            )
            .bind(schedule.id)
            .bind(period_number_i64)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            if let Some(ledger_id) = existing_ledger_id {
                let row = sqlx::query_as::<_, PointsCreditLedgerRow>(
                    "SELECT * FROM points_credit_ledger WHERE id = $1",
                )
                .bind(ledger_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?
                .ok_or_else(|| {
                    CoreError::InternalServerError(format!(
                        "grant_record references missing ledger {} (schedule {} period {})",
                        ledger_id, schedule.id, period_number
                    ))
                })?;
                tx.commit()
                    .await
                    .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
                return Self::row_to_points_credit_ledger(row);
            }

            // 3) No prior grant_record — create the ledger row, apply wallet
            //    delta, and write a points_transactions row (mirroring
            //    `grant_points_atomic` in-tx). `source_type` /
            //    `credit_type` come from the schedule's grant semantics.
            let credit_type = match schedule.subscription_id {
                Some(_) => CreditType::SubscriptionCredit,
                None => CreditType::FreePeriodicCredit,
            };
            let source_type = match schedule.subscription_id {
                Some(_) => CreditSourceType::SubscriptionRenewal,
                None => CreditSourceType::FreePeriodicGrant,
            };
            let user_id = schedule.user_id;
            let bucket_id = schedule.bucket_id;
            let amount = schedule.points_per_period;
            let source_id = format!("schedule:{}:period:{}", schedule.id, period_number);

            let wallet = Self::ensure_wallet_in_tx(&mut tx, &realm_id, user_id, bucket_id).await?;
            if wallet.status != WalletStatus::Active {
                return Err(CoreError::BadRequest(format!(
                    "Cannot pre-grant points to {} wallet",
                    wallet.status.as_str()
                )));
            }
            let now = chrono::Utc::now();
            let ledger = PointsCreditLedger {
                id: Uuid::now_v7(),
                user_id,
                realm_id: realm_id.clone(),
                bucket_id,
                credit_type,
                source_type,
                source_id: source_id.clone(),
                granted_amount: amount,
                used_amount: 0,
                revoked_amount: 0,
                remaining_amount: amount,
                expires_at,
                effective_at,
                status: CreditLedgerStatus::Active,
                created_at: now,
                updated_at: now,
            };
            let created_ledger = Self::create_ledger_in_tx(&mut tx, &ledger).await?;
            let delta = WalletDelta::grant(credit_type, amount);
            let _ = Self::apply_wallet_delta_in_tx(&mut tx, wallet.id, delta).await?;
            // BE-D11: balance_after = real post-pregrant derived SUM for this
            // bucket. Pre-grant rows carry a future `effective_at`, so the
            // derived SUM predicate excludes them until the period starts —
            // `balance_after` reflects the currently-available balance, not the
            // pre-granted total.
            let derived = Self::compute_available_balance_in_tx(
                &mut tx,
                &realm_id,
                user_id,
                std::slice::from_ref(&bucket_id),
                now,
            )
            .await?;
            let (balance_after, topup_after, subscription_after) =
                Self::derived_to_balance_snapshots(&derived);
            let transaction_type = Self::determine_transaction_type(credit_type, source_type);
            let transaction_id = Uuid::now_v7();
            let external_ref_id = format!("{}:{}", source_id, transaction_id);
            Self::create_transaction_in_tx(
                &mut tx,
                PointsTransaction {
                    id: transaction_id,
                    wallet_id: wallet.id,
                    user_id,
                    realm_id: realm_id.clone(),
                    bucket_id,
                    transaction_type,
                    amount,
                    balance_after,
                    topup_balance_after: topup_after,
                    subscription_balance_after: subscription_after,
                    credit_type: Some(credit_type),
                    description: Some(format!(
                        "{}: {} points pre-granted (period {})",
                        source_type.as_str(),
                        amount,
                        period_number
                    )),
                    client_app_id: None,
                    subscription_id: schedule.subscription_id,
                    external_ref_id: Some(external_ref_id),
                    correlation_id: None,
                    effective_at,
                    created_at: now,
                },
            )
            .await?;

            // 4) Insert the grant_record linking to the ledger row (BE-D01 FK
            //    bridge). The UNIQUE(schedule_id, period_number) constraint is
            //    the period-level business idempotency guarantee (design §5.2).
            // 4b) Insert the grant_record via raw SQL (the `&mut` sqlx tx
            //     cannot be shared with sea-orm). Column order matches
            //     `create_revocation_record_in_tx` for consistency.
            let grant_record_id = Uuid::now_v7();
            sqlx::query(
                r#"
                INSERT INTO points_grant_records (
                    id, schedule_id, user_id, realm_id, period_number,
                    granted_amount, grant_time, ledger_id, created_at
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                "#,
            )
            .bind(grant_record_id)
            .bind(schedule.id)
            .bind(user_id)
            .bind(&realm_id)
            .bind(period_number_i64)
            .bind(amount)
            .bind(now)
            .bind(created_ledger.id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                // Surface UNIQUE violations loudly: a duplicate means a
                // concurrent pre-grant won the race between our existence
                // check and this INSERT; the caller's retry will hit the
                // idempotency branch.
                CoreError::DatabaseError(format!(
                    "pregrant grant_record insert failed (schedule {} period {}): {}",
                    schedule.id, period_number, e
                ))
            })?;

            // 5) Advance the schedule. `granted_periods` becomes the latest
            //    period granted; `next_grant_time` becomes the start of the
            //    next nominal period (design §5.2). Using the domain's own
            //    `next_grant_time(base, n)` arithmetic keeps a single source
            //    of truth for period cadence.
            let advanced_granted_periods = period_number_i64.max(schedule.granted_periods);
            let next_period_index = advanced_granted_periods + 1;
            let next_grant_time = schedule
                .grant_period_type
                .next_grant_time(schedule.base_time, next_period_index);
            sqlx::query(
                r#"
                UPDATE points_grant_schedules
                   SET next_grant_time = $2,
                       granted_periods = $3,
                       updated_at = NOW()
                 WHERE id = $1
                "#,
            )
            .bind(schedule.id)
            .bind(next_grant_time)
            .bind(advanced_granted_periods)
            .execute(&mut *tx)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            tx.commit()
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
            Ok(created_ledger)
        }
    }

    /// Scan for schedules whose next pre-grant is due (design §5.3). Returns
    /// **candidates** whose `active=TRUE AND next_grant_time <= $before`; this
    /// method intentionally does NOT do a SQL-side `NOT EXISTS` against
    /// `points_grant_records` (P2-2): `period_number` derivation depends on
    /// `first_period_start`/`nominal_period_duration` (domain
    /// `derive_period_number`) and duplicating it inside SQL would drift from
    /// the domain's single source of truth. The caller (worker
    /// `PointsPreGrantJob` / domain) re-derives `period_number` per-row and
    /// checks `points_grant_records` absence before calling
    /// `pregrant_next_period_atomic`. This is a best-effort warming scan
    /// (design A9); returning extra candidates is harmless.
    fn find_schedules_due_for_pregrant(
        &self,
        before: chrono::DateTime<chrono::Utc>,
        limit: u64,
    ) -> impl std::future::Future<
        Output = Result<Vec<herald_domain::points::grant_schedule::PointsGrantSchedule>, CoreError>,
    > + Send {
        let db = self.db.clone();
        async move {
            let models = points_grant_schedule::Entity::find()
                .filter(points_grant_schedule::Column::Active.eq(true))
                .filter(
                    points_grant_schedule::Column::NextGrantTime
                        .lte(sea_orm::prelude::DateTimeWithTimeZone::from(before)),
                )
                .order_by_asc(points_grant_schedule::Column::NextGrantTime)
                .limit(limit)
                .all(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            models
                .into_iter()
                .map(Self::model_to_grant_schedule)
                .collect()
        }
    }

    /// Single-user free-periodic due schedule scan for read-path realization
    /// (design §5.3.1). `WHERE realm_id AND user_id AND active AND
    /// subscription_id IS NULL AND next_grant_time <= before` (lead_time=0,
    /// only already-due periods).
    fn find_due_free_grant_schedules_for_user(
        &self,
        realm_id: &str,
        user_id: Uuid,
        before: chrono::DateTime<chrono::Utc>,
        limit: u64,
    ) -> impl std::future::Future<
        Output = Result<Vec<herald_domain::points::grant_schedule::PointsGrantSchedule>, CoreError>,
    > + Send {
        let db = self.db.clone();
        let realm_id = realm_id.to_string();
        async move {
            let models = points_grant_schedule::Entity::find()
                .filter(points_grant_schedule::Column::RealmId.eq(realm_id))
                .filter(points_grant_schedule::Column::UserId.eq(user_id))
                .filter(points_grant_schedule::Column::Active.eq(true))
                .filter(points_grant_schedule::Column::SubscriptionId.is_null())
                .filter(
                    points_grant_schedule::Column::NextGrantTime
                        .lte(sea_orm::prelude::DateTimeWithTimeZone::from(before)),
                )
                .order_by_asc(points_grant_schedule::Column::NextGrantTime)
                .limit(limit)
                .all(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            models
                .into_iter()
                .map(Self::model_to_grant_schedule)
                .collect()
        }
    }

    /// Row-level reclaim of a pre-granted ledger row (design §5.2 A4). Sets the
    /// resolved ledger row to `status='revoked'` and
    /// `revoked_amount += remaining_amount`; derived balance auto-excludes it,
    /// so no wallet back-adjustment is performed. Returns the number of rows
    /// affected (0 ⟺ locator did not resolve an active row, caller may treat
    /// as idempotent no-op). For partially-consumed rows (`used_amount > 0`),
    /// a `PointsRevocationRecord(reason='subscription_pre_grant_reclaim',
    /// revocation_type=CancelRevoke)` is written so the shortfall is auditable.
    ///
    /// **BE-D05 scope.** `ReclaimLocator::BySourceId(src)` filters
    /// `points_credit_ledger.source_id` directly;
    /// `ReclaimLocator::BySchedulePeriod{..}` is resolved to the unique ledger
    /// row via the `points_grant_records.ledger_id` FK subquery
    /// (BE-D01 bridge — `points_credit_ledger` has no `schedule_id` /
    /// `period_number` columns).
    fn revoke_pregrant_ledger_row_atomic(
        &self,
        realm_id: &str,
        locator: ReclaimLocator,
        reason: &str,
    ) -> impl std::future::Future<Output = Result<usize, CoreError>> + Send {
        let pool = self.pool.clone();
        let realm_id = realm_id.to_string();
        let reason = reason.to_string();
        async move {
            let mut tx = pool
                .begin()
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            // Build the WHERE-clause filter for the locator. `BySchedulePeriod`
            // resolves via the `points_grant_records.ledger_id` FK subquery
            // (UNIQUE(schedule_id, period_number) guarantees at most one row).
            // `points_credit_ledger.remaining_amount` is a GENERATED column
            // (`granted_amount - used_amount - revoked_amount`), so it
            // regenerates to 0 the moment `revoked_amount` increases. A plain
            // `UPDATE ... RETURNING *` would therefore hand back
            // `remaining_amount = 0`, and the shortfall record below would be
            // written with `revoked_amount = 0` (violating
            // `points_revocation_records.revoked_amount > 0`). The CTE locks +
            // captures the pre-update `remaining_amount` as `yanked` so the
            // debt record carries the real unused portion this reclaim removed.
            // `BySchedulePeriod` resolves via the `points_grant_records.ledger_id`
            // FK subquery (UNIQUE(schedule_id, period_number) ⟹ ≤1 row).
            let rows: Vec<ReclaimTargetRow> = match locator {
                ReclaimLocator::BySourceId(src) => sqlx::query_as::<_, ReclaimTargetRow>(
                    r#"
                        WITH target AS (
                            SELECT id, remaining_amount AS yanked
                              FROM points_credit_ledger
                             WHERE realm_id = $1
                               AND source_id = $2
                               AND status = 'active'
                               AND remaining_amount > 0
                             FOR UPDATE
                        )
                        UPDATE points_credit_ledger AS l
                           SET status        = 'revoked',
                               revoked_amount = l.revoked_amount + t.yanked,
                               updated_at     = NOW()
                          FROM target t
                         WHERE l.id = t.id
                        RETURNING l.id, l.user_id, l.realm_id, l.source_id,
                                  l.used_amount, t.yanked
                        "#,
                )
                .bind(&realm_id)
                .bind(&src)
                .fetch_all(&mut *tx)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?,
                ReclaimLocator::BySchedulePeriod {
                    schedule_id,
                    period_number,
                } => sqlx::query_as::<_, ReclaimTargetRow>(
                    r#"
                        WITH target AS (
                            SELECT id, remaining_amount AS yanked
                              FROM points_credit_ledger
                             WHERE realm_id = $1
                               AND status = 'active'
                               AND remaining_amount > 0
                               AND id IN (
                                    SELECT g.ledger_id
                                      FROM points_grant_records g
                                     WHERE g.schedule_id = $2
                                       AND g.period_number = $3
                               )
                             FOR UPDATE
                        )
                        UPDATE points_credit_ledger AS l
                           SET status        = 'revoked',
                               revoked_amount = l.revoked_amount + t.yanked,
                               updated_at     = NOW()
                          FROM target t
                         WHERE l.id = t.id
                        RETURNING l.id, l.user_id, l.realm_id, l.source_id,
                                  l.used_amount, t.yanked
                        "#,
                )
                .bind(&realm_id)
                .bind(schedule_id)
                .bind(i64::from(period_number))
                .fetch_all(&mut *tx)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?,
            };

            // Record shortfall for any partially-consumed row (design A4).
            // `used_amount > 0` means the pre-granted period was already spent
            // before reclaim; the revocation record makes that debt visible to
            // audit / billing. Fully-unused rows (used_amount == 0) need no
            // debt record — the ledger row itself is sufficient.
            let now = chrono::Utc::now();
            for row in &rows {
                if row.used_amount > 0 {
                    let record = PointsRevocationRecord {
                        id: Uuid::now_v7(),
                        ledger_id: row.id,
                        user_id: row.user_id,
                        realm_id: row.realm_id.clone(),
                        revocation_type: RevocationType::CancelRevoke,
                        // `yanked` is the pre-update remaining_amount (the
                        // unused portion this reclaim just moved into
                        // revoked_amount); `used_amount` is preserved on the
                        // ledger row itself for audit.
                        revoked_amount: row.yanked,
                        reason: reason.clone(),
                        reference_id: Some(format!(
                            "subscription_pre_grant_reclaim:{}",
                            row.source_id
                        )),
                        created_at: now,
                    };
                    Self::create_revocation_record_in_tx(&mut tx, &record).await?;
                }
            }

            let affected = rows.len();
            tx.commit()
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;
            Ok(affected)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_conversion() {
        let model = points_wallet::Model {
            id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            realm_id: "test-realm".to_string(),
            bucket_id: Uuid::now_v7(),
            total_topup_granted: 100,
            total_subscription_granted: 0,
            total_recharged: 1000,
            total_consumed: 900,
            status: "active".to_string(),
            created_at: sea_orm::prelude::DateTimeWithTimeZone::from(chrono::Utc::now()),
            updated_at: sea_orm::prelude::DateTimeWithTimeZone::from(chrono::Utc::now()),
        };

        let result = PostgresPointsRepository::model_to_points_wallet(model);
        assert!(result.is_ok());
        let account = result.unwrap();
        assert_eq!(account.total_topup_granted, 100);
    }

    // ===== BE-D04: multi-pool consume allocation planner (pure) =====
    //
    // The allocate-by-expiry loop is the heart of design §5.1. It is extracted as
    // `plan_consume_allocation` so the cross-bucket split, permanent-pool-last
    // ordering, partial-coverage rejection and exact-amount boundary can be
    // verified without a database. These tests encode WHY the split matters:
    // wrong totals → wrong per-bucket transaction amounts / wallet balances;
    // wrong ordering → permanent credits drained before expiring ones (value loss
    // for the user); insufficient rejection → over-spending / negative balances.

    use herald_domain::points::entities::{
        CreditLedgerStatus, CreditSourceType, CreditType, PointsCreditLedger,
    };

    /// Helper: build a ledger with the given remaining amount and expiry.
    /// `bucket_id` distinguishes pools; ordering in the input slice is what the
    /// planner consumes (caller already sorted via the SQL ORDER BY).
    fn ledger(
        index_bucket: usize,
        credit_type: CreditType,
        remaining: i64,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> PointsCreditLedger {
        PointsCreditLedger {
            id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            realm_id: "realm".to_string(),
            // Two distinct buckets: even index → bucket A, odd → bucket B.
            bucket_id: if index_bucket.is_multiple_of(2) {
                Uuid::from_u128(0xA)
            } else {
                Uuid::from_u128(0xB)
            },
            credit_type,
            source_type: CreditSourceType::Topup,
            source_id: "src".to_string(),
            granted_amount: remaining,
            used_amount: 0,
            revoked_amount: 0,
            remaining_amount: remaining,
            expires_at,
            effective_at: None,
            status: CreditLedgerStatus::Active,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn plan_consume_splits_across_two_buckets_in_expiry_order() {
        // Bucket A: 30 expiring soon; Bucket B: 50 expiring later.
        // Request 50 → take all 30 from A, then 20 from B.
        let soon = chrono::Utc::now() + chrono::Duration::hours(1);
        let later = chrono::Utc::now() + chrono::Duration::days(7);
        let ledgers = vec![
            ledger(0, CreditType::TopupCredit, 30, Some(soon)),
            ledger(1, CreditType::TopupCredit, 50, Some(later)),
        ];

        let plan = PostgresPointsRepository::plan_consume_allocation(&ledgers, 50);

        assert!(plan.fully_covers);
        assert_eq!(
            plan.allocations,
            vec![
                PlannedAllocation {
                    ledger_index: 0,
                    amount: 30
                },
                PlannedAllocation {
                    ledger_index: 1,
                    amount: 20
                },
            ]
        );
        // Caller groups per bucket: A=30, B=20 — distinct transactions.
        let mut per_bucket = std::collections::BTreeMap::new();
        for p in &plan.allocations {
            let bid = ledgers[p.ledger_index].bucket_id;
            *per_bucket.entry(bid).or_insert(0i64) += p.amount;
        }
        let totals: Vec<_> = per_bucket.values().copied().collect();
        assert_eq!(totals, vec![30, 20]);
    }

    #[test]
    fn plan_consume_allocates_permanent_pool_null_expires_last() {
        // The SQL ORDER BY is `expires_at ASC NULLS LAST`, so the caller hands the
        // planner ledgers already in that order. Verify the planner respects the
        // given order: expiring ledger is consumed before the permanent one even
        // when the permanent one has more remaining.
        let soon = chrono::Utc::now() + chrono::Duration::minutes(5);
        let ledgers = vec![
            // expiring bucket, 10 left
            ledger(0, CreditType::SubscriptionCredit, 10, Some(soon)),
            // permanent (NULL expires_at) bucket, 1000 left
            ledger(1, CreditType::GrantedCredit, 1000, None),
        ];

        let plan = PostgresPointsRepository::plan_consume_allocation(&ledgers, 15);

        assert!(plan.fully_covers);
        // First 10 from the expiring ledger, remaining 5 from permanent.
        assert_eq!(plan.allocations[0].ledger_index, 0);
        assert_eq!(plan.allocations[0].amount, 10);
        assert_eq!(plan.allocations[1].ledger_index, 1);
        assert_eq!(plan.allocations[1].amount, 5);
        // Without correct ordering the planner would drain 15 from permanent and
        // let the 10 expiring credits lapse — this assertion guards that.
    }

    #[test]
    fn plan_consume_rejects_when_remaining_is_insufficient() {
        // Sum of remaining (30 + 10) < requested 50 → not fully covered.
        // The repository precheck rejects with `insufficient_points`; the plan
        // surfaces `fully_covers=false` so the caller can fail loud rather than
        // write a partial / negative-balance consume.
        let soon = chrono::Utc::now() + chrono::Duration::hours(1);
        let ledgers = vec![
            ledger(0, CreditType::TopupCredit, 30, Some(soon)),
            ledger(1, CreditType::TopupCredit, 10, Some(soon)),
        ];

        let plan = PostgresPointsRepository::plan_consume_allocation(&ledgers, 50);

        assert!(!plan.fully_covers);
        // Still records the partial take so the caller's precheck can report
        // have/need accurately if desired.
        let taken: i64 = plan.allocations.iter().map(|p| p.amount).sum();
        assert_eq!(taken, 40);
    }

    #[test]
    fn plan_consume_exact_amount_boundary_consumes_no_more_than_needed() {
        let soon = chrono::Utc::now() + chrono::Duration::hours(1);
        let ledgers = vec![
            ledger(0, CreditType::TopupCredit, 20, Some(soon)),
            ledger(1, CreditType::TopupCredit, 20, Some(soon)),
        ];

        // Request exactly 20 → only the first ledger is touched; the second is
        // left intact. Guards against over-consuming (negative balance bug).
        let plan = PostgresPointsRepository::plan_consume_allocation(&ledgers, 20);

        assert!(plan.fully_covers);
        assert_eq!(
            plan.allocations,
            vec![PlannedAllocation {
                ledger_index: 0,
                amount: 20
            }]
        );
    }

    #[test]
    fn plan_consume_single_pool_request_yields_single_allocation() {
        // Single-pool hit must produce a length-1 transaction downstream
        // (completion criterion: single-pool → Vec len 1, structurally uniform).
        let soon = chrono::Utc::now() + chrono::Duration::days(1);
        let ledgers = vec![ledger(0, CreditType::TopupCredit, 100, Some(soon))];

        let plan = PostgresPointsRepository::plan_consume_allocation(&ledgers, 40);

        assert!(plan.fully_covers);
        assert_eq!(plan.allocations.len(), 1);
        assert_eq!(plan.allocations[0].amount, 40);
    }

    #[test]
    fn plan_consume_empty_ledger_set_cannot_cover_nonzero_request() {
        // Mirrors the NoCoveredPointsPool / no-active-ledger path: an empty
        // covered set (or a user with no active ledgers in any covered bucket)
        // must NOT silently produce a zero-amount consume.
        let ledgers: Vec<PointsCreditLedger> = vec![];
        let plan = PostgresPointsRepository::plan_consume_allocation(&ledgers, 10);
        assert!(!plan.fully_covers);
        assert!(plan.allocations.is_empty());
    }
}
