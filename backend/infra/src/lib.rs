// Infrastructure layer - external adapters implementation

pub mod audit;
pub mod authentication;
pub mod authorization;
pub mod billing;
pub mod client;
pub mod client_api_keys;
pub mod creem;
pub mod dashboard;
pub mod oauth;
pub mod payment_attempt;
pub mod points;
pub mod points_package;
pub mod purchase;
pub mod realm;
pub mod realm_config;
pub mod redis;
pub mod shopify;
pub mod stripe;
pub mod totp_key_management;
pub mod user;
pub mod user_totp;
pub mod wechat;

pub mod webhook;

// Re-export commonly used types
pub use audit::PostgresAuditEventRepository;
pub use dashboard::PostgresDashboardRepository;
pub use user::{
    PostgresAdminUserRepository, PostgresRolePolicyRepository, PostgresUserRepository,
    PostgresUserRoleRepository, PostgresVerificationRepository,
};
