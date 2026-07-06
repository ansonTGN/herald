pub mod dtos;
pub mod entities;
pub mod errors;
pub mod expiration_service;
pub mod grant_schedule;
pub mod idempotency_service;
pub mod policies;
pub mod ports;
pub mod realm_config;
pub mod service;
pub mod services;
pub mod subscription_service;
pub mod user_config;

pub use entities::{
    CreditLedgerStatus, CreditSourceType, CreditType, IdempotencyKey, IdempotencyResult,
    IdempotencyStatus, PointsBalance, PointsConsumptionAllocation, PointsCreditLedger,
    PointsQuotaEntitlement, PointsRevocationRecord, PointsTransaction, PointsWallet,
    QuotaEntitlementStatus, QuotaSourceType, QuotaWindow, QuotaWindowView, RechargeType,
    RevocationType, SafeArithmetics, TransactionType, WalletStatus,
};
pub use errors::PointsErrorExt;
pub use expiration_service::{ExpirationService, ExpirationSummary};
pub use idempotency_service::{IdempotencyService, IdempotencyStore};
pub use policies::PointsPolicy;
pub use ports::{
    GrantScheduleUpdate, LedgerFilters, LedgerUpdate, Pagination, PointsRepository, ReclaimLocator,
    TransactionFilters, UserConfigUpdate, WalletDelta, WalletFilters,
};
pub use service::PointsService;
/// Stable display-key derivation for quota windows (design §4.2.2 / §4.4.3).
/// Re-exported so the api layer can derive keys when materializing the domain
/// inputs from the request shape (`QuotaWindowInput` carries no key).
pub use service::derive_window_key;
pub use subscription_service::{CancelMode, SubscriptionService};

pub use grant_schedule::{
    GrantPeriodType, GrantSummary, PointsGrantRecord, PointsGrantSchedule, ProcessResult,
};
pub use realm_config::{
    CreateRealmConfigInput, FREE_PERIODIC_QUOTA_WINDOWS_MAX, RealmDefaultConfig,
    UpdateRealmConfigInput, validate_free_periodic_quota_windows,
};
pub use services::{GrantScheduler, RealmConfigService, RegistrationService};
pub use user_config::UserPointsConfig;
