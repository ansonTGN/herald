// Points API Types

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Points account response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PointsWalletResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub realm_id: String,
    /// Credit Bucket this wallet belongs to (design §4.2.1).
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

/// Per-credit-type balances (design §4.2.3 `balancesByType` / `byCreditType`).
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

/// Wallet balances grouped by Credit Bucket (design §4.2.3 `WalletByBucket`).
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
    /// Sum of `balances_by_type` for this bucket.
    pub bucket_total: i64,
}

/// Aggregated wallets-by-bucket list response (design §4.2.3).
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
    /// Credit Bucket the transaction landed in (design §4.2.3).
    pub bucket_id: Option<Uuid>,
    pub transaction_type: String,
    pub amount: i64,
    pub balance_after: i64,
    pub description: Option<String>,
    pub client_app_id: Option<Uuid>,
    pub subscription_id: Option<Uuid>,
    pub external_ref_id: Option<String>,
    pub created_at: String,
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
    /// Filter by Credit Bucket (design §4.2.3). Applied at the handler because
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
    /// Filter by Credit Bucket (design §4.2.1). Applied at the handler because
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
    /// Target Credit Bucket (design §4.2.4 / A5). REQUIRED — every grant must
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
    /// Credit Bucket the grant landed in (design §4.2.3 / §4.2.4). Mirrors the
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
            id: uuid::Uuid::now_v7(),
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

        // Verify that snake_case fields are converted to camelCase
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

        // Verify that original snake_case is not present
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

        // Verify camelCase conversion
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

        // Verify snake_case is not present
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
        };

        let json = serde_json::to_string(&transaction).unwrap();
        println!("Serialized JSON: {}", json);

        // Verify camelCase conversion
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

        // Verify snake_case is not present
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

        // Verify camelCase conversion
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

        // Verify snake_case is not present
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

        // Verify camelCase conversion
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

        // Verify snake_case is not present
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
