// Points domain module - virtual currency and transaction management

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

// Re-export commonly used types
pub use entities::{
    AccountStatus, CreditLedgerStatus, CreditSourceType, CreditType, IdempotencyKey,
    IdempotencyResult, IdempotencyStatus, PointsAccount, PointsBalance,
    PointsConsumptionAllocation, PointsCreditLedger, PointsPlanConfig, PointsRevocationRecord,
    PointsTransaction, RechargeType, RevocationType, SafeArithmetics, TransactionType,
};
pub use errors::PointsErrorExt;
pub use expiration_service::{ExpirationService, ExpirationSummary};
pub use idempotency_service::{IdempotencyService, IdempotencyStore};
pub use policies::PointsPolicy;
pub use ports::{
    AccountFilters, AccountUpdate, GrantScheduleUpdate, LedgerFilters, LedgerUpdate, Pagination,
    PointsRepository, TransactionFilters, UserConfigUpdate,
};
pub use service::PointsService;
pub use subscription_service::{CancelMode, SubscriptionService};

// NEW EXPORTS for credits-plan-split
pub use grant_schedule::{
    GrantPeriodType, GrantSummary, PointsGrantRecord, PointsGrantSchedule, ProcessResult,
};
pub use realm_config::{CreateRealmConfigInput, RealmDefaultConfig, UpdateRealmConfigInput};
pub use services::{GrantScheduler, RealmConfigService, RegistrationService};
pub use user_config::UserPointsConfig;
