// =============================================================================
// 通用辅助函数模块
// =============================================================================
//
// 提供跨模块共享的测试辅助函数，减少重复代码。
//
// **Module Organization**:
// - auth_helpers: Authentication and session management
// - billing_helpers: Billing-specific test helpers (plans, subscriptions)
// - client_helpers: Client app helpers
// - rbac_helpers: RBAC/permission helpers
// - test_setup_helpers: Common test setup helpers (authentication, requests, validation)
// - user_helpers: User management helpers
// - test_constants: Common test constants
//
// =============================================================================

pub mod async_payment_helpers;
pub mod auth_helpers;
pub mod billing_helpers;
pub mod client_helpers;
#[cfg(test)]
pub mod credit_bucket_helpers;
#[cfg(test)]
pub mod creem_mocks;
pub mod device_code_helpers;
pub mod email_config_helpers;
pub mod oauth_pkce_helpers;
pub mod oauth_test_helpers;
#[cfg(test)]
pub mod passkey_authenticator;
#[cfg(test)]
pub mod payment_assertions;
pub mod points_grant_helpers;
pub mod points_helpers;
pub mod rbac_helpers;
pub mod subscription_test_helpers;
#[cfg(test)]
pub mod test_setup_helpers;
pub mod user_helpers;
pub mod webhook_helpers;

// 重新导出常用函数
pub use auth_helpers::*;
pub use billing_helpers::*;
pub use client_helpers::*;
pub use rbac_helpers::*;
pub use user_helpers::*;
