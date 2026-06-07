// Points domain entities

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::common::entities::app_errors::CoreError;
// Import Points-specific error extensions
use crate::points::errors::PointsErrorExt;

/// Safe arithmetic operations with overflow protection
pub trait SafeArithmetics: Copy + std::fmt::Display {
    /// Safe addition that returns an error on overflow
    fn safe_add(self, other: Self) -> Result<Self, CoreError>;

    /// Safe subtraction that returns an error on underflow
    fn safe_sub(self, other: Self) -> Result<Self, CoreError>;

    /// Safe multiplication that returns an error on overflow
    fn safe_mul(self, other: Self) -> Result<Self, CoreError>;
}

impl SafeArithmetics for i64 {
    fn safe_add(self, other: Self) -> Result<Self, CoreError> {
        self.checked_add(other).ok_or_else(|| {
            CoreError::overflow_error(&format!("Overflow in addition: {} + {}", self, other))
        })
    }

    fn safe_sub(self, other: Self) -> Result<Self, CoreError> {
        if self < other {
            return Err(CoreError::overflow_error(&format!(
                "Underflow in subtraction: {} - {} = {} (negative result not allowed)",
                self,
                other,
                self - other
            )));
        }
        Ok(self - other)
    }

    fn safe_mul(self, other: Self) -> Result<Self, CoreError> {
        self.checked_mul(other).ok_or_else(|| {
            CoreError::overflow_error(&format!("Overflow in multiplication: {} * {}", self, other))
        })
    }
}

/// Helper for parsing string enums with a default error message
fn parse_enum<T, F>(s: &str, error_prefix: &str, variants: F) -> Result<T, CoreError>
where
    F: Fn(&str) -> Option<T>,
{
    variants(s).ok_or_else(|| CoreError::BadRequest(format!("{}: {}", error_prefix, s)))
}

/// Account status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WalletStatus {
    Active,
    Frozen,
    Closed,
}

impl WalletStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            WalletStatus::Active => "active",
            WalletStatus::Frozen => "frozen",
            WalletStatus::Closed => "closed",
        }
    }
}

impl std::str::FromStr for WalletStatus {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_enum(
            s.to_lowercase().as_str(),
            "Invalid wallet status",
            |s| match s {
                "active" => Some(WalletStatus::Active),
                "frozen" => Some(WalletStatus::Frozen),
                "closed" => Some(WalletStatus::Closed),
                _ => None,
            },
        )
    }
}

impl std::fmt::Display for WalletStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Transaction type enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Recharge,
    Consume,
    SubscriptionGrant,
    SubscriptionRenewal,
    SubscriptionUpgrade,
    RegistrationGrant,
    FreePeriodicGrant,
    RefundRevoke,
    ExpireRevoke,
    CancelRevoke,
    IdempotencyRecord,
    Expiration,
    Refund,
    Grant,
}

impl TransactionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TransactionType::Recharge => "recharge",
            TransactionType::Consume => "consume",
            TransactionType::SubscriptionGrant => "subscription_grant",
            TransactionType::SubscriptionRenewal => "subscription_renewal",
            TransactionType::SubscriptionUpgrade => "subscription_upgrade",
            TransactionType::RegistrationGrant => "registration_grant",
            TransactionType::FreePeriodicGrant => "free_periodic_grant",
            TransactionType::RefundRevoke => "refund_revoke",
            TransactionType::ExpireRevoke => "expire_revoke",
            TransactionType::CancelRevoke => "cancel_revoke",
            TransactionType::IdempotencyRecord => "idempotency_record",
            TransactionType::Expiration => "expiration",
            TransactionType::Refund => "refund",
            TransactionType::Grant => "grant",
        }
    }
}

impl std::str::FromStr for TransactionType {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_enum(
            s.to_lowercase().as_str(),
            "Invalid transaction type",
            |s| match s {
                "recharge" => Some(TransactionType::Recharge),
                "consume" => Some(TransactionType::Consume),
                "subscription_grant" => Some(TransactionType::SubscriptionGrant),
                "subscription_renewal" => Some(TransactionType::SubscriptionRenewal),
                "subscription_upgrade" => Some(TransactionType::SubscriptionUpgrade),
                "registration_grant" => Some(TransactionType::RegistrationGrant),
                "free_periodic_grant" => Some(TransactionType::FreePeriodicGrant),
                "refund_revoke" => Some(TransactionType::RefundRevoke),
                "expire_revoke" => Some(TransactionType::ExpireRevoke),
                "cancel_revoke" => Some(TransactionType::CancelRevoke),
                "idempotency_record" => Some(TransactionType::IdempotencyRecord),
                "expiration" => Some(TransactionType::Expiration),
                "refund" => Some(TransactionType::Refund),
                "grant" => Some(TransactionType::Grant),
                _ => None,
            },
        )
    }
}

impl std::fmt::Display for TransactionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Recharge type enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RechargeType {
    Subscribe,
    Renewal,
}

impl RechargeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RechargeType::Subscribe => "subscribe",
            RechargeType::Renewal => "renewal",
        }
    }
}

impl std::str::FromStr for RechargeType {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_enum(
            s.to_lowercase().as_str(),
            "Invalid recharge type",
            |s| match s {
                "subscribe" => Some(RechargeType::Subscribe),
                "renewal" => Some(RechargeType::Renewal),
                _ => None,
            },
        )
    }
}

impl std::fmt::Display for RechargeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Credit type enum (积分类型) - Extended
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CreditType {
    /// 充值积分（用户主动购买）
    TopupCredit,
    /// 会员积分（订阅套餐赠送）
    SubscriptionCredit,
    /// 注册初始积分（注册时一次性赠送）
    RegistrationCredit,
    /// 免费定期积分（按周期自动发放）
    FreePeriodicCredit,
    /// 主动发放积分（管理员或 SDK 发放）
    GrantedCredit,
}

impl CreditType {
    pub fn as_str(&self) -> &'static str {
        match self {
            CreditType::TopupCredit => "topup_credit",
            CreditType::SubscriptionCredit => "subscription_credit",
            CreditType::RegistrationCredit => "registration_credit",
            CreditType::FreePeriodicCredit => "free_periodic_credit",
            CreditType::GrantedCredit => "granted_credit",
        }
    }

    pub fn is_topup(&self) -> bool {
        matches!(self, CreditType::TopupCredit)
    }

    pub fn is_subscription(&self) -> bool {
        matches!(self, CreditType::SubscriptionCredit)
    }

    pub fn is_free_credit(&self) -> bool {
        matches!(
            self,
            CreditType::RegistrationCredit | CreditType::FreePeriodicCredit
        )
    }

    pub fn is_granted(&self) -> bool {
        matches!(self, CreditType::GrantedCredit)
    }

    /// Returns the (topup_delta, subscription_delta, granted_delta, registration_delta, free_periodic_delta)
    /// balance deltas for this credit type.
    ///
    /// Subscription credits count toward subscription balance, granted credits toward granted balance,
    /// registration credits toward registration balance, free periodic credits toward free_periodic balance,
    /// and topup credits toward topup balance.
    pub fn wallet_balance_delta(&self, amount: i64) -> (i64, i64, i64, i64, i64) {
        match self {
            CreditType::TopupCredit => (amount, 0, 0, 0, 0),
            CreditType::SubscriptionCredit => (0, amount, 0, 0, 0),
            CreditType::GrantedCredit => (0, 0, amount, 0, 0),
            CreditType::RegistrationCredit => (0, 0, 0, amount, 0),
            CreditType::FreePeriodicCredit => (0, 0, 0, 0, amount),
        }
    }
}

impl std::str::FromStr for CreditType {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_enum(
            s.to_lowercase().as_str(),
            "Invalid credit_type",
            |s| match s {
                "topup_credit" => Some(CreditType::TopupCredit),
                "subscription_credit" => Some(CreditType::SubscriptionCredit),
                "registration_credit" => Some(CreditType::RegistrationCredit),
                "free_periodic_credit" => Some(CreditType::FreePeriodicCredit),
                "granted_credit" => Some(CreditType::GrantedCredit),
                _ => None,
            },
        )
    }
}

impl std::fmt::Display for CreditType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Credit source type enum (积分来源类型) - Extended
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CreditSourceType {
    /// 订阅首次赠送
    SubscriptionInitial,
    /// 订阅续费赠送
    SubscriptionRenewal,
    /// 订阅升级（回收老积分，发放新积分）
    SubscriptionUpgrade,
    /// 充值购买
    Topup,
    /// 注册赠送
    Registration,
    /// 免费定期发放
    FreePeriodicGrant,
    /// 系统发放（补偿、奖励等）
    SystemGrant,
    /// 退款回收
    RefundRevoke,
    /// 过期回收
    ExpireRevoke,
    /// 取消订阅回收
    CancelRevoke,
    /// 升级时回收（免费用户升级到付费）
    UpgradeRevoke,
    /// 管理员主动发放
    AdminGrant,
    /// SDK/第三方应用发放
    SdkGrant,
}

impl CreditSourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            CreditSourceType::SubscriptionInitial => "subscription_initial",
            CreditSourceType::SubscriptionRenewal => "subscription_renewal",
            CreditSourceType::SubscriptionUpgrade => "subscription_upgrade",
            CreditSourceType::Topup => "topup",
            CreditSourceType::Registration => "registration",
            CreditSourceType::FreePeriodicGrant => "free_periodic_grant",
            CreditSourceType::SystemGrant => "system_grant",
            CreditSourceType::RefundRevoke => "refund_revoke",
            CreditSourceType::ExpireRevoke => "expire_revoke",
            CreditSourceType::CancelRevoke => "cancel_revoke",
            CreditSourceType::UpgradeRevoke => "upgrade_revoke",
            CreditSourceType::AdminGrant => "admin_grant",
            CreditSourceType::SdkGrant => "sdk_grant",
        }
    }
}

impl std::str::FromStr for CreditSourceType {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_enum(
            s.to_lowercase().as_str(),
            "Invalid source_type",
            |s| match s {
                "subscription_initial" => Some(CreditSourceType::SubscriptionInitial),
                "subscription_renewal" => Some(CreditSourceType::SubscriptionRenewal),
                "subscription_upgrade" => Some(CreditSourceType::SubscriptionUpgrade),
                "topup" => Some(CreditSourceType::Topup),
                "registration" => Some(CreditSourceType::Registration),
                "free_periodic_grant" => Some(CreditSourceType::FreePeriodicGrant),
                "system_grant" => Some(CreditSourceType::SystemGrant),
                "refund_revoke" => Some(CreditSourceType::RefundRevoke),
                "expire_revoke" => Some(CreditSourceType::ExpireRevoke),
                "cancel_revoke" => Some(CreditSourceType::CancelRevoke),
                "upgrade_revoke" => Some(CreditSourceType::UpgradeRevoke),
                "admin_grant" => Some(CreditSourceType::AdminGrant),
                "sdk_grant" => Some(CreditSourceType::SdkGrant),
                _ => None,
            },
        )
    }
}

impl std::fmt::Display for CreditSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Credit ledger status enum (账本状态)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum CreditLedgerStatus {
    Active,
    Revoked,
    Expired,
    FullyUsed,
}

impl CreditLedgerStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            CreditLedgerStatus::Active => "active",
            CreditLedgerStatus::Revoked => "revoked",
            CreditLedgerStatus::Expired => "expired",
            CreditLedgerStatus::FullyUsed => "fully_used",
        }
    }
}

impl std::str::FromStr for CreditLedgerStatus {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_enum(
            s.to_lowercase().as_str(),
            "Invalid ledger status",
            |s| match s {
                "active" => Some(CreditLedgerStatus::Active),
                "revoked" => Some(CreditLedgerStatus::Revoked),
                "expired" => Some(CreditLedgerStatus::Expired),
                "fully_used" => Some(CreditLedgerStatus::FullyUsed),
                _ => None,
            },
        )
    }
}

impl std::fmt::Display for CreditLedgerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Revocation type enum (回收类型)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RevocationType {
    RefundRevoke,
    ExpireRevoke,
    CancelRevoke,
    /// Upgrade revocation (free user upgrading to paid subscription)
    UpgradeRevoke,
}

impl RevocationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RevocationType::RefundRevoke => "refund_revoke",
            RevocationType::ExpireRevoke => "expire_revoke",
            RevocationType::CancelRevoke => "cancel_revoke",
            RevocationType::UpgradeRevoke => "upgrade_revoke",
        }
    }
}

impl std::str::FromStr for RevocationType {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_enum(
            s.to_lowercase().as_str(),
            "Invalid revocation type",
            |s| match s {
                "refund_revoke" => Some(RevocationType::RefundRevoke),
                "expire_revoke" => Some(RevocationType::ExpireRevoke),
                "cancel_revoke" => Some(RevocationType::CancelRevoke),
                "upgrade_revoke" => Some(RevocationType::UpgradeRevoke),
                _ => None,
            },
        )
    }
}

impl std::fmt::Display for RevocationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Points Credit Ledger entity (积分账本实体)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointsCreditLedger {
    pub id: Uuid,
    pub user_id: Uuid,
    pub realm_id: String,
    pub credit_type: CreditType,
    pub source_type: CreditSourceType,
    pub source_id: String,
    pub granted_amount: i64,
    pub used_amount: i64,
    pub revoked_amount: i64,
    pub remaining_amount: i64,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub status: CreditLedgerStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Points Consumption Allocation entity (消费分配实体)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointsConsumptionAllocation {
    pub id: Uuid,
    pub transaction_id: Uuid,
    pub ledger_id: Uuid,
    pub user_id: Uuid,
    pub realm_id: String,
    pub allocated_amount: i64,
    pub ledger_remaining_after: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Points Revocation Record entity (积分回收记录实体)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointsRevocationRecord {
    pub id: Uuid,
    pub ledger_id: Uuid,
    pub user_id: Uuid,
    pub realm_id: String,
    pub revocation_type: RevocationType,
    pub revoked_amount: i64,
    pub reason: String,
    pub reference_id: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Points Wallet entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointsWallet {
    pub id: Uuid,
    pub user_id: Uuid,
    pub realm_id: String,
    pub total_balance: i64, // Computed: topup_balance + subscription_balance + granted_balance + registration_balance + free_periodic_balance
    pub topup_balance: i64,
    pub subscription_balance: i64,
    pub granted_balance: i64,
    pub registration_balance: i64,
    pub free_periodic_balance: i64,
    pub total_topup_granted: i64,
    pub total_subscription_granted: i64,
    // Historical column name. Semantically this is total_topup_granted + total_subscription_granted,
    // not all granted credit types.
    pub total_recharged: i64,
    pub total_consumed: i64,
    pub status: WalletStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Points Transaction entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointsTransaction {
    pub id: Uuid,
    pub wallet_id: Uuid,
    pub user_id: Uuid,
    pub realm_id: String,
    pub transaction_type: TransactionType,
    pub amount: i64,
    pub balance_after: i64,
    pub topup_balance_after: Option<i64>,
    pub subscription_balance_after: Option<i64>,
    pub credit_type: Option<CreditType>,
    pub description: Option<String>,
    pub client_app_id: Option<Uuid>,
    pub subscription_id: Option<Uuid>,
    pub external_ref_id: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Points Balance DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointsBalance {
    pub user_id: Uuid,
    pub balance: i64,
    pub topup_balance: i64,
    pub subscription_balance: i64,
    pub granted_balance: i64,
    pub registration_balance: i64,
    pub free_periodic_balance: i64,
    // Compatibility name for paid topup + subscription grants.
    pub total_recharged: i64,
    pub total_consumed: i64,
    pub unit: String,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<PointsWallet> for PointsBalance {
    fn from(account: PointsWallet) -> Self {
        PointsBalance {
            user_id: account.user_id,
            balance: account.total_balance,
            topup_balance: account.topup_balance,
            subscription_balance: account.subscription_balance,
            granted_balance: account.granted_balance,
            registration_balance: account.registration_balance,
            free_periodic_balance: account.free_periodic_balance,
            total_recharged: account.total_recharged,
            total_consumed: account.total_consumed,
            unit: "points".to_string(),
            updated_at: account.updated_at,
        }
    }
}

/// Idempotency status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum IdempotencyStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

impl IdempotencyStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            IdempotencyStatus::Pending => "pending",
            IdempotencyStatus::Processing => "processing",
            IdempotencyStatus::Completed => "completed",
            IdempotencyStatus::Failed => "failed",
        }
    }
}

impl std::str::FromStr for IdempotencyStatus {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_enum(
            s.to_lowercase().as_str(),
            "Invalid idempotency status",
            |s| match s {
                "pending" => Some(IdempotencyStatus::Pending),
                "processing" => Some(IdempotencyStatus::Processing),
                "completed" => Some(IdempotencyStatus::Completed),
                "failed" => Some(IdempotencyStatus::Failed),
                _ => None,
            },
        )
    }
}

/// Idempotency result enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IdempotencyResult<T> {
    /// New request, should proceed with processing
    New,
    /// Cached response from previous successful request
    Cached { transaction: T },
}

/// Idempotency key entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdempotencyKey {
    pub id: Uuid,
    pub realm_id: String,
    pub idempotency_key: String,
    pub status: IdempotencyStatus,
    pub request_data: Option<String>,
    pub response_data: Option<String>,
    pub transaction_id: Option<Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Paginated response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paginated<T> {
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
    pub data: Vec<T>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_status_from_str() {
        assert_eq!(
            "active".parse::<WalletStatus>().unwrap(),
            WalletStatus::Active
        );
        assert_eq!(
            "frozen".parse::<WalletStatus>().unwrap(),
            WalletStatus::Frozen
        );
        assert_eq!(
            "closed".parse::<WalletStatus>().unwrap(),
            WalletStatus::Closed
        );
        assert!("invalid".parse::<WalletStatus>().is_err());
    }

    #[test]
    fn test_idempotency_status_from_str() {
        assert_eq!(
            "pending".parse::<IdempotencyStatus>().unwrap(),
            IdempotencyStatus::Pending
        );
        assert_eq!(
            "processing".parse::<IdempotencyStatus>().unwrap(),
            IdempotencyStatus::Processing
        );
        assert_eq!(
            "completed".parse::<IdempotencyStatus>().unwrap(),
            IdempotencyStatus::Completed
        );
        assert_eq!(
            "failed".parse::<IdempotencyStatus>().unwrap(),
            IdempotencyStatus::Failed
        );
        assert!("invalid".parse::<IdempotencyStatus>().is_err());
    }

    #[test]
    fn test_safe_arithmetics_add() {
        assert_eq!(5i64.safe_add(10).unwrap(), 15);
        assert!(100i64.safe_add(i64::MAX - 50).is_err());
    }

    #[test]
    fn test_safe_arithmetics_sub() {
        assert_eq!(20i64.safe_sub(5).unwrap(), 15);
        assert!(5i64.safe_sub(10).is_err());
    }

    #[test]
    fn test_safe_arithmetics_mul() {
        assert_eq!(5i64.safe_mul(10).unwrap(), 50);
        assert!(100i64.safe_mul(i64::MAX / 50 + 1).is_err());
    }
}
