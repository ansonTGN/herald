// =============================================================================
// Points System Scenario Tests
// =============================================================================
//
// Comprehensive scenario tests for the Points system backend implementation.
//
// **Test Coverage**:
// - Account creation and management (5 tests)
// - Points consumption (4 tests)
// - Points recharge/webhooks (0 tests)
// - Transaction history (2 tests)
// - Plan configuration (2 tests)
// - Account status (1 test)
// - API key validation (1 test)
// - Unified filter tests (1 test)
//
// **Total Tests**: 16 scenario tests
//
// **User Stories Covered**:
// - US-PO-01: Configure Points Plans (P0)
// - US-PO-02: View All User Wallets (P1)
// - US-PO-03: View User Points Transaction History (P1)
// - US-PO-04: Manage Points Plan Configurations (P2)
// - US-PU-01: View My Points Balance (P0)
// - US-PU-02: View My Transaction History (P1)
// - US-PU-03: Filter Transaction Records (P2)
//
// **Running Tests**:
// ```bash
// cd backend
// uv run scripts/backend-test.py points
// ```
//
// =============================================================================

mod test_01_account_creation;
mod test_02_view_balance;
mod test_03_balance_permission;
mod test_08_consume_success;
mod test_09_consume_exact_balance;
mod test_10_consume_invalid_amount;
mod test_11_consume_edge_cases;
mod test_13_concurrent_consumption;
mod test_14_api_key_validation;
mod test_15_consume_idempotency;
mod test_24_entitlement_points_policy;
mod test_27_api_key_cannot_access_points_admin_configs;
mod test_31_closed_account_consumption;
mod test_32_frozen_account_consumption;

// fix-points-2 test files
mod test_40_webhook_subscription_paid;
mod test_41_webhook_subscription_upgrade;
mod test_42_webhook_subscription_downgrade;
mod test_43_webhook_subscription_cancel;
mod test_44_webhook_refund_created;
mod test_50_points_expiration;
mod test_60_mixed_balance_consumption;

// Credits Plan Split - Batch 1 Tests
mod test_61_free_user_registration;
mod test_62_free_user_upgrade;

// Unified filter test framework (replaces legacy filter tests)
mod unified_filter_tests;

// Consume + Webhook concurrency race condition tests
mod test_70_consume_webhook_race;

// Concurrency gap coverage tests
mod test_71_concurrent_consume_recharge;
mod test_72_mixed_credit_concurrent_consume;
mod test_73_mixed_operations_concurrent;

// Admin grant points tests
mod test_74_admin_grant_points;

// Ext/SDK grant points tests
mod test_75_ext_grant_points;

// Regression tests for code-review fixes
mod test_76_regression_code_review_fixes;

pub mod fixtures;
