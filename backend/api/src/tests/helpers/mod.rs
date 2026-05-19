// =============================================================================
// 通用辅助函数模块
// =============================================================================
//
// 提供跨模块共享的测试辅助函数，减少重复代码。
//
// **Module Organization**:
// - assertions: Test assertion helpers
// - auth_helpers: Authentication and session management
// - billing_helpers: Billing-specific test helpers (plans, subscriptions)
// - client_helpers: Client app helpers
// - rbac_helpers: RBAC/permission helpers
// - test_setup_helpers: Common test setup helpers (authentication, requests, validation)
// - user_helpers: User management helpers
// - test_commons: Unified test fixtures (points, user, billing) - Import explicitly
// - test_constants: Common test constants
//
// **Note**: test_commons is not glob-re-exported to avoid name conflicts.
// Tests should import explicitly from test_commons when needed.
//
// =============================================================================

pub mod assertions;
pub mod auth_helpers;
pub mod billing_helpers;
pub mod client_helpers;
#[cfg(test)]
pub mod creem_mocks;
pub mod device_code_helpers;
pub mod email_config_helpers;
pub mod oauth_test_helpers;
#[cfg(test)]
pub mod payment_assertions;
pub mod points_helpers;
#[cfg(test)]
pub mod points_package_helpers;
pub mod rbac_helpers;
#[cfg(test)]
pub mod shopify_helpers;
pub mod test_commons;
#[cfg(test)]
pub mod test_constants;
#[cfg(test)]
pub mod test_setup_helpers;
pub mod user_helpers;
pub mod webhook_helpers;

// WeChat Pay test helpers
#[cfg(test)]
pub mod wechat_helpers;

// 测试模块
#[cfg(test)]
mod auth_helpers_test;

// 重新导出常用函数
pub use auth_helpers::*;
pub use billing_helpers::*;
pub use client_helpers::*;
pub use rbac_helpers::*;
pub use user_helpers::*;
pub use webhook_helpers::*;

// 测试设置辅助函数导出（用于测试场景）
