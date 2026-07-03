use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Points account response.
///
/// The GET /wallets/{user} endpoint is a read-only user-total balance view:
/// for users with multiple Credit Buckets, `find_by_user_id` synthesizes a
/// single aggregated `PointsWallet` with `bucket_id = None`. In that case
/// `id` is `None` too — there is no single concrete wallet to point at, and a
/// fabricated id would be a misleading handle a client could mistake for "the
/// wallet". `id` is `Some` only for a single-bucket user, where it is the
/// wallet row id of that one bucket (still not a writable handle here).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PointsWalletResponse {
    pub id: Option<Uuid>,
    pub user_id: Uuid,
    pub realm_id: String,
    /// Credit Bucket this wallet belongs to. `None` for the
    /// aggregate user-total view (multi-bucket user).
    pub bucket_id: Option<Uuid>,
    pub balance: i64,
    /// Total points granted through paid topups and subscription entitlements.
    pub total_paid_granted: i64,
    /// Deprecated compatibility alias for total_paid_granted.
    pub total_recharged: i64,
    pub total_consumed: i64,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub unit: String,
    pub currency: String,
}

/// Per-credit-type balances (`balancesByType` / `byCreditType`).
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BalancesByType {
    pub topup: i64,
    pub subscription: i64,
    pub registration: i64,
    pub free_periodic: i64,
    pub granted: i64,
}

impl BalancesByType {
    /// Sum of all credit-type balances (the bucket total).
    pub fn total(&self) -> i64 {
        self.topup
            .saturating_add(self.subscription)
            .saturating_add(self.registration)
            .saturating_add(self.free_periodic)
            .saturating_add(self.granted)
    }
}

/// Quota window read view (`QuotaWindowView`), mirrors the domain entity
/// `herald_core::domain::points::QuotaWindowView` (design §4.2.2).
///
/// One row per distinct window `key` for a (user, bucket). `key` is the stable
/// display identity derived from the window length (e.g. `5h`/`week`/`month`),
/// NOT a row ordinal — re-renders / config edits keep the same key.
/// `isTightest` flags the minimum-remaining window (the spendable-from-quota
/// constraint); `exhausted` flags `remaining == 0`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuotaWindowViewResponse {
    /// Stable display key (config-derived, not row ordinal).
    pub key: String,
    pub limit: i64,
    pub used: i64,
    pub remaining: i64,
    /// Sliding window length in seconds (month ≈ 30d).
    pub window_seconds: i64,
    /// Approximate next reset point of the window (design D1 — precise
    /// oldest-consume reset is deferred). `None` when no consume has occurred
    /// in the window yet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resets_at: Option<chrono::DateTime<chrono::Utc>>,
    /// True if this window is the minimum-remaining (tightest) constraint.
    pub is_tightest: bool,
    /// True if `remaining == 0`.
    pub exhausted: bool,
}

impl QuotaWindowViewResponse {
    /// Map a domain `QuotaWindowView` into the HTTP response (1:1; the only
    /// job is crossing the domain/API boundary so the API contract is not
    /// bound to the domain entity).
    pub fn from_domain(view: herald_core::domain::points::QuotaWindowView) -> Self {
        Self {
            key: view.key,
            limit: view.limit,
            used: view.used,
            remaining: view.remaining,
            window_seconds: view.window_seconds,
            resets_at: view.resets_at,
            is_tightest: view.is_tightest,
            exhausted: view.exhausted,
        }
    }
}

/// Free-periodic quota window **request** shape (design §4.2.2 / §4.4.3).
///
/// Carries only the editable fields; `key` is derived by the backend from
/// `windowSeconds` (via `derive_window_key`) before persistence, so callers
/// cannot drift the stable window identity.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, validator::Validate)]
#[serde(rename_all = "camelCase")]
pub struct QuotaWindowInput {
    /// Sliding window length in seconds. Must be > 0.
    #[validate(range(min = 1))]
    pub window_seconds: i64,
    /// Quota limit. Must be >= 0 (0 = window grants nothing but is a valid
    /// config edge case).
    #[validate(range(min = 0))]
    pub limit: i64,
}

/// Wallet balances grouped by Credit Bucket (`WalletByBucket`).
///
/// For the admin (`billing/points/wallets`) view, `user_id` is populated and
/// the response groups per `(user, bucket)`. For the `users/me/points/wallets`
/// view, `user_id` is the calling user and rows group their own wallets by
/// bucket.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WalletByBucketResponse {
    pub bucket_id: Option<Uuid>,
    /// Display name (currently unset; filled when bucket directory is wired).
    pub name: Option<String>,
    /// Whether the bucket is enabled (currently unset; filled when bucket
    /// directory is wired).
    pub enabled: Option<bool>,
    /// User who owns these wallet rows (always present; the admin view spans
    /// users, the user view repeats the calling user).
    pub user_id: Uuid,
    pub balances_by_type: BalancesByType,
    /// Currently spendable total for this bucket = window-available
    /// (`spendable_from_quota`) + pool balance (`spendable_from_pool`).
    /// Semantically extended (design §4.2.2): for a pool-only bucket this
    /// equals the pool sum (zero-regression — `quota_windows` /
    /// `spendable_from_quota` are `None`); for a window bucket it folds in the
    /// tightest window's remaining.
    pub bucket_total: i64,
    /// Per-window quota view for this (user, bucket) (design §4.2.2).
    /// `None` for a pool-only bucket (no active subscription / free-periodic
    /// quota entitlement). `Some([])` is avoided — pool-only stays `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota_windows: Option<Vec<QuotaWindowViewResponse>>,
    /// Window-quota available amount = minimum `remaining` across
    /// `quota_windows` (the tightest constraint). `None` for pool-only buckets.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spendable_from_quota: Option<i64>,
    /// Pool-side balance sum (topup + registration + granted credit types)
    /// for this bucket. `None` for window-only buckets with no pool balance
    /// component; otherwise the pool contribution to `bucket_total`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spendable_from_pool: Option<i64>,
}

/// Aggregated wallets-by-bucket list response.
///
/// `cross_bucket_total` is the sum of every bucket's `bucketTotal`. The field
/// is always present; for a single-bucket realm it equals `bucketTotal` of the
/// sole row.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListWalletsByBucketResponse {
    pub items: Vec<WalletByBucketResponse>,
    pub cross_bucket_total: i64,
}

/// Points balance response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PointsBalanceResponse {
    pub user_id: Uuid,
    pub balance: i64,
    /// Total points granted through paid topups and subscription entitlements.
    pub total_paid_granted: i64,
    /// Deprecated compatibility alias for total_paid_granted.
    pub total_recharged: i64,
    pub total_consumed: i64,
    pub unit: String,
    pub currency: String,
    pub updated_at: String,
}

/// Points transaction response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PointsTransactionResponse {
    pub id: Uuid,
    pub wallet_id: Uuid,
    pub user_id: Uuid,
    pub realm_id: String,
    /// Credit Bucket the transaction landed in.
    pub bucket_id: Option<Uuid>,
    pub transaction_type: String,
    pub amount: i64,
    pub balance_after: i64,
    pub description: Option<String>,
    pub client_app_id: Option<Uuid>,
    pub subscription_id: Option<Uuid>,
    pub external_ref_id: Option<String>,
    pub created_at: String,
    /// Expected effective time (P1-2). Read-only, for
    /// admin/audit reconciliation of pre-generated vs already-effective rows.
    /// `None` on the `points.view` (regular user) path — the handler forces it
    /// to `None` and `skip_serializing_if` omits the key from JSON entirely, so
    /// the field never appears in regular-user responses. Populated (when
    /// available) only on the `points.manage` path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Points plan config response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PointsPlanConfigResponse {
    pub config_id: Uuid,
    pub realm_id: String,
    pub plan_id: Uuid,
    pub grant_period_type: String,
    pub points_per_period: i64,
    pub validity_days: i64,
    pub grant_on_subscribe: bool,
    pub max_periods: Option<i64>,
    pub active: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// List plan configs response wrapper
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListPlanConfigsResponse {
    pub configs: Vec<PointsPlanConfigResponse>,
}

/// Consume points request
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConsumePointsRequest {
    pub user_id: String,
    pub client_app_id: String,
    pub amount: i64,
    pub description: Option<String>,
    pub idempotency_key: Option<String>,
}

/// Consume points response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConsumePointsResponse {
    pub transaction_id: Uuid,
    pub wallet_id: Uuid,
    pub user_id: Uuid,
    pub amount: i64,
    pub balance_after: i64,
}

/// Create plan config request
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatePlanConfigRequest {
    pub plan_id: String,
    pub grant_period_type: String,
    pub points_per_period: i64,
    pub validity_days: i64,
    pub grant_on_subscribe: bool,
    pub max_periods: Option<i64>,
}

/// Update plan config request
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UpdatePlanConfigRequest {
    pub grant_period_type: Option<String>,
    pub points_per_period: Option<i64>,
    pub validity_days: Option<i64>,
    pub grant_on_subscribe: Option<bool>,
    pub max_periods: Option<Option<i64>>,
}

/// Query parameters for listing transactions
#[derive(Debug, Clone, Deserialize, utoipa::IntoParams, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListTransactionsQuery {
    pub user_id: Option<String>,
    pub transaction_type: Option<String>,
    pub client_app_id: Option<String>,
    pub subscription_id: Option<String>,
    pub external_ref_id: Option<String>,
    /// Filter by Credit Bucket. Applied at the handler because
    /// `TransactionFilters` does not yet carry `bucket_id`.
    pub bucket_id: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// Query parameters for listing accounts
#[derive(Debug, Clone, Deserialize, utoipa::IntoParams, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListWalletsQuery {
    pub status: Option<String>,
    pub search: Option<String>,
    /// Filter by Credit Bucket. Applied at the handler because
    /// `WalletFilters` does not yet carry `bucket_id`.
    pub bucket_id: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// Realm default config response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RealmDefaultConfigResponse {
    pub realm_id: String,
    pub registration_bonus_points: i64,
    pub free_periodic_points_amount: i64,
    pub free_periodic_grant_period_type: String,
    pub free_periodic_validity_days: i64,
    /// Free-periodic quota window definitions (design §4.2.2 / §4.3.2).
    /// `None` ⟺ no window-model free-periodic grant (the registration path
    /// skips it, fail-safe). Mirrors the stored JSONB column.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub free_periodic_quota_windows: Option<Vec<QuotaWindowInput>>,
    pub created_at: String,
    pub updated_at: String,
}

/// Create realm config request
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, validator::Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateRealmConfigRequest {
    #[validate(range(min = 0))]
    pub registration_bonus_points: i64,
    #[validate(range(min = 0))]
    pub free_periodic_points_amount: i64,
    #[validate(length(min = 1))]
    pub free_periodic_grant_period_type: String,
    #[validate(range(min = 0))]
    pub free_periodic_validity_days: i64,
    /// Free-periodic quota windows (design §4.2.2). Nullable; window count ≤ 8
    /// and per-window `windowSeconds > 0` / `limit >= 0` enforced in the
    /// handler (the `validator` derive covers the per-window ranges; the count
    /// cap is checked explicitly because `validator` has no built-in for
    /// `Option<Vec<_>>` length on the outer Option).
    pub free_periodic_quota_windows: Option<Vec<QuotaWindowInput>>,
}

/// Update realm config request
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, validator::Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRealmConfigRequest {
    #[validate(range(min = 0))]
    pub registration_bonus_points: i64,
    #[validate(range(min = 0))]
    pub free_periodic_points_amount: i64,
    #[validate(length(min = 1))]
    pub free_periodic_grant_period_type: String,
    #[validate(range(min = 0))]
    pub free_periodic_validity_days: i64,
    /// Free-periodic quota windows (design §4.2.2). `None` ⟺ leave the stored
    /// value untouched (partial-update semantics); `Some([])` ⟺ clear;
    /// `Some([...])` ⟺ replace. Same validation as create.
    pub free_periodic_quota_windows: Option<Vec<QuotaWindowInput>>,
}

/// User points config response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserPointsConfigResponse {
    pub user_id: Uuid,
    pub realm_id: String,
    pub registration_bonus_points: i64,
    pub free_periodic_points_amount: i64,
    pub free_periodic_grant_period_type: Option<String>,
    pub free_periodic_validity_days: i64,
    pub next_grant_time: Option<String>,
    pub granted_periods: i64,
    pub grant_schedule_id: Option<Uuid>,
    pub created_at: String,
    pub updated_at: String,
}

/// Grant points request (admin)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GrantPointsRequest {
    pub user_id: String,
    /// Target Credit Bucket. REQUIRED — every grant must
    /// name an explicit bucket; missing → 400 `grant_bucket_required`.
    pub bucket_id: Option<String>,
    pub amount: i64,
    pub reason: String,
    pub validity_days: Option<i64>,
}

/// Grant points response (admin)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GrantPointsResponse {
    pub transaction_id: Uuid,
    pub user_id: Uuid,
    /// Credit Bucket the grant landed in. Mirrors the
    /// api-ext `ExtGrantPointsResponse.bucketId` and SDK `GrantPointsResponse`
    /// contract so consumers see one shape.
    pub bucket_id: Uuid,
    pub amount: i64,
    pub granted_balance: i64,
    pub total_balance: i64,
    pub expires_at: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_points_wallet_response_camelcase() {
        let account = PointsWalletResponse {
            id: Some(uuid::Uuid::now_v7()),
            user_id: uuid::Uuid::now_v7(),
            realm_id: "test-realm".to_string(),
            bucket_id: Some(uuid::Uuid::now_v7()),
            balance: 100,
            total_paid_granted: 100,
            total_recharged: 100,
            total_consumed: 0,
            status: "active".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            unit: "points".to_string(),
            currency: "points".to_string(),
        };

        let json = serde_json::to_string(&account).unwrap();
        println!("Serialized JSON: {}", json);

        assert!(
            json.contains("\"userId\""),
            "Should contain camelCase 'userId'"
        );
        assert!(
            json.contains("\"realmId\""),
            "Should contain camelCase 'realmId'"
        );
        assert!(
            json.contains("\"totalPaidGranted\""),
            "Should contain camelCase 'totalPaidGranted'"
        );
        assert!(
            json.contains("\"totalRecharged\""),
            "Should keep compatibility field 'totalRecharged'"
        );
        assert!(
            json.contains("\"totalConsumed\""),
            "Should contain camelCase 'totalConsumed'"
        );
        assert!(
            json.contains("\"createdAt\""),
            "Should contain camelCase 'createdAt'"
        );
        assert!(
            json.contains("\"updatedAt\""),
            "Should contain camelCase 'updatedAt'"
        );
        assert!(json.contains("\"unit\""), "Should contain 'unit'");
        assert!(json.contains("\"currency\""), "Should contain 'currency'");

        assert!(
            !json.contains("\"user_id\""),
            "Should not contain snake_case 'user_id'"
        );
        assert!(
            !json.contains("\"realm_id\""),
            "Should not contain snake_case 'realm_id'"
        );
        assert!(
            !json.contains("\"total_paid_granted\""),
            "Should not contain snake_case 'total_paid_granted'"
        );
        assert!(
            !json.contains("\"total_recharged\""),
            "Should not contain snake_case 'total_recharged'"
        );
        assert!(
            !json.contains("\"total_consumed\""),
            "Should not contain snake_case 'total_consumed'"
        );
        assert!(
            !json.contains("\"created_at\""),
            "Should not contain snake_case 'created_at'"
        );
        assert!(
            !json.contains("\"updated_at\""),
            "Should not contain snake_case 'updated_at'"
        );
    }

    #[test]
    fn test_points_balance_response_camelcase() {
        let balance = PointsBalanceResponse {
            user_id: uuid::Uuid::now_v7(),
            balance: 50,
            total_paid_granted: 100,
            total_recharged: 100,
            total_consumed: 50,
            unit: "points".to_string(),
            currency: "points".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&balance).unwrap();
        println!("Serialized JSON: {}", json);

        assert!(
            json.contains("\"userId\""),
            "Should contain camelCase 'userId'"
        );
        assert!(
            json.contains("\"totalPaidGranted\""),
            "Should contain camelCase 'totalPaidGranted'"
        );
        assert!(
            json.contains("\"totalRecharged\""),
            "Should keep compatibility field 'totalRecharged'"
        );
        assert!(
            json.contains("\"totalConsumed\""),
            "Should contain camelCase 'totalConsumed'"
        );
        assert!(
            json.contains("\"updatedAt\""),
            "Should contain camelCase 'updatedAt'"
        );
        assert!(json.contains("\"unit\""), "Should contain 'unit'");
        assert!(json.contains("\"currency\""), "Should contain 'currency'");

        assert!(
            !json.contains("\"user_id\""),
            "Should not contain snake_case 'user_id'"
        );
        assert!(
            !json.contains("\"total_paid_granted\""),
            "Should not contain snake_case 'total_paid_granted'"
        );
        assert!(
            !json.contains("\"total_recharged\""),
            "Should not contain snake_case 'total_recharged'"
        );
        assert!(
            !json.contains("\"total_consumed\""),
            "Should not contain snake_case 'total_consumed'"
        );
        assert!(
            !json.contains("\"updated_at\""),
            "Should not contain snake_case 'updated_at'"
        );
    }

    #[test]
    fn test_points_transaction_response_camelcase() {
        let transaction = PointsTransactionResponse {
            id: uuid::Uuid::now_v7(),
            wallet_id: uuid::Uuid::now_v7(),
            user_id: uuid::Uuid::now_v7(),
            realm_id: "test-realm".to_string(),
            bucket_id: Some(uuid::Uuid::now_v7()),
            transaction_type: "recharge".to_string(),
            amount: 100,
            balance_after: 100,
            description: Some("Test recharge".to_string()),
            client_app_id: Some(uuid::Uuid::now_v7()),
            subscription_id: Some(uuid::Uuid::now_v7()),
            external_ref_id: Some("ref-123".to_string()),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            // None (regular-user / `points.view` shape) — `skip_serializing_if`
            // omits the key entirely (P1-2: the field must not
            // appear in regular-user JSON).
            effective_at: None,
        };

        let json = serde_json::to_string(&transaction).unwrap();
        println!("Serialized JSON: {}", json);

        assert!(
            json.contains("\"walletId\""),
            "Should contain camelCase 'walletId'"
        );
        assert!(
            json.contains("\"userId\""),
            "Should contain camelCase 'userId'"
        );
        assert!(
            json.contains("\"realmId\""),
            "Should contain camelCase 'realmId'"
        );
        assert!(
            json.contains("\"transactionType\""),
            "Should contain camelCase 'transactionType'"
        );
        assert!(
            json.contains("\"balanceAfter\""),
            "Should contain camelCase 'balanceAfter'"
        );
        assert!(
            json.contains("\"description\""),
            "Should contain camelCase 'description'"
        );
        assert!(
            json.contains("\"clientAppId\""),
            "Should contain camelCase 'clientAppId'"
        );
        assert!(
            json.contains("\"subscriptionId\""),
            "Should contain camelCase 'subscriptionId'"
        );
        assert!(
            json.contains("\"externalRefId\""),
            "Should contain camelCase 'externalRefId'"
        );
        assert!(
            json.contains("\"createdAt\""),
            "Should contain camelCase 'createdAt'"
        );

        // when `effective_at` is None the key must be ABSENT
        // from the JSON (skip_serializing_if). This is the regular-user
        // (`points.view`) response shape — the field must never leak. If this
        // assertion fails, someone removed the `skip_serializing_if` attribute
        // and regular users would see an `effectiveAt: null` key.
        assert!(
            !json.contains("effective_at"),
            "Should not contain snake_case 'effective_at'"
        );
        assert!(
            !json.contains("effectiveAt"),
            "effectiveAt must be omitted (not null) when effective_at is None (P1-2)"
        );

        assert!(
            !json.contains("\"wallet_id\""),
            "Should not contain snake_case 'wallet_id'"
        );
        assert!(
            !json.contains("\"user_id\""),
            "Should not contain snake_case 'user_id'"
        );
        assert!(
            !json.contains("\"realm_id\""),
            "Should not contain snake_case 'realm_id'"
        );
        assert!(
            !json.contains("\"transaction_type\""),
            "Should not contain snake_case 'transaction_type'"
        );
        assert!(
            !json.contains("\"balance_after\""),
            "Should not contain snake_case 'balance_after'"
        );
        assert!(
            !json.contains("\"client_app_id\""),
            "Should not contain snake_case 'client_app_id'"
        );
        assert!(
            !json.contains("\"subscription_id\""),
            "Should not contain snake_case 'subscription_id'"
        );
        assert!(
            !json.contains("\"external_ref_id\""),
            "Should not contain snake_case 'external_ref_id'"
        );
        assert!(
            !json.contains("\"created_at\""),
            "Should not contain snake_case 'created_at'"
        );
    }

    #[test]
    fn test_points_plan_config_response_camelcase() {
        let plan_config = PointsPlanConfigResponse {
            config_id: uuid::Uuid::now_v7(),
            realm_id: "test-realm".to_string(),
            plan_id: uuid::Uuid::now_v7(),
            grant_period_type: "monthly".to_string(),
            points_per_period: 1000,
            validity_days: 30,
            grant_on_subscribe: true,
            max_periods: Some(12),
            active: true,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&plan_config).unwrap();
        println!("Serialized JSON: {}", json);

        assert!(
            json.contains("\"configId\""),
            "Should contain camelCase 'configId'"
        );
        assert!(
            json.contains("\"realmId\""),
            "Should contain camelCase 'realmId'"
        );
        assert!(
            json.contains("\"planId\""),
            "Should contain camelCase 'planId'"
        );
        assert!(
            json.contains("\"grantPeriodType\""),
            "Should contain camelCase 'grantPeriodType'"
        );
        assert!(
            json.contains("\"pointsPerPeriod\""),
            "Should contain camelCase 'pointsPerPeriod'"
        );
        assert!(
            json.contains("\"validityDays\""),
            "Should contain camelCase 'validityDays'"
        );
        assert!(
            json.contains("\"grantOnSubscribe\""),
            "Should contain camelCase 'grantOnSubscribe'"
        );
        assert!(
            json.contains("\"maxPeriods\""),
            "Should contain camelCase 'maxPeriods'"
        );
        assert!(
            json.contains("\"createdAt\""),
            "Should contain camelCase 'createdAt'"
        );
        assert!(
            json.contains("\"updatedAt\""),
            "Should contain camelCase 'updatedAt'"
        );

        assert!(
            !json.contains("\"config_id\""),
            "Should not contain snake_case 'config_id'"
        );
        assert!(
            !json.contains("\"realm_id\""),
            "Should not contain snake_case 'realm_id'"
        );
        assert!(
            !json.contains("\"plan_id\""),
            "Should not contain snake_case 'plan_id'"
        );
        assert!(
            !json.contains("\"grant_period_type\""),
            "Should not contain snake_case 'grant_period_type'"
        );
        assert!(
            !json.contains("\"points_per_period\""),
            "Should not contain snake_case 'points_per_period'"
        );
        assert!(
            !json.contains("\"validity_days\""),
            "Should not contain snake_case 'validity_days'"
        );
        assert!(
            !json.contains("\"grant_on_subscribe\""),
            "Should not contain snake_case 'grant_on_subscribe'"
        );
        assert!(
            !json.contains("\"max_periods\""),
            "Should not contain snake_case 'max_periods'"
        );
        assert!(
            !json.contains("\"created_at\""),
            "Should not contain snake_case 'created_at'"
        );
        assert!(
            !json.contains("\"updated_at\""),
            "Should not contain snake_case 'updated_at'"
        );
    }

    #[test]
    fn test_consume_points_response_camelcase() {
        let response = ConsumePointsResponse {
            transaction_id: uuid::Uuid::now_v7(),
            wallet_id: uuid::Uuid::now_v7(),
            user_id: uuid::Uuid::now_v7(),
            amount: 50,
            balance_after: 50,
        };

        let json = serde_json::to_string(&response).unwrap();
        println!("Serialized JSON: {}", json);

        assert!(
            json.contains("\"transactionId\""),
            "Should contain camelCase 'transactionId'"
        );
        assert!(
            json.contains("\"walletId\""),
            "Should contain camelCase 'walletId'"
        );
        assert!(
            json.contains("\"userId\""),
            "Should contain camelCase 'userId'"
        );
        assert!(
            json.contains("\"balanceAfter\""),
            "Should contain camelCase 'balanceAfter'"
        );

        assert!(
            !json.contains("\"transaction_id\""),
            "Should not contain snake_case 'transaction_id'"
        );
        assert!(
            !json.contains("\"wallet_id\""),
            "Should not contain snake_case 'wallet_id'"
        );
        assert!(
            !json.contains("\"user_id\""),
            "Should not contain snake_case 'user_id'"
        );
        assert!(
            !json.contains("\"balance_after\""),
            "Should not contain snake_case 'balance_after'"
        );
    }
}
