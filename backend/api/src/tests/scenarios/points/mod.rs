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

// Idempotency guard tests
mod test_77_idempotency_guards;

// Refund precision tests
mod test_78_refund_precision;

// point-time BE-T02: effective_at semantics — future-effective exclusion,
// zero-delay availability (clock-advance only, no worker), immediate
// availability when effective_at IS NULL.
mod test_80_effective_at_semantics;

// point-time BE-T05: provider period normalization (Stripe top-level +
// item-level + multi-item disagreeing + both absent; Creem symmetric +
// missing). Scenario-layer coverage of design §6.1 P0 + A8 P0. The pure
// `normalize_*` unit tests live in backend/api-billing (owned by BE-D08);
// these tests exercise the consequence end-to-end via the webhook HTTP
// path (grant written vs. skipped).
mod test_81_provider_period_normalization;

// point-time BE-T08: worker-down + read-path realization (US-FU-004 scenario
// 1.1) + realization write-failure fail-loud (P2). Scenario-layer coverage of
// design §6.1 P0 (worker-down still usable) + §5.3.1 (reconcile_due_for_user:
// single-user, N=3, idempotent, lead_time=0, subscription_id IS NULL only,
// fail-loud 5xx). The worker is NEVER started; correctness is exercised
// purely via the read path (`PointsService::get_balance` / `consume_points`)
// and clock-advance SQL UPDATE on ledger rows.
mod test_82_worker_down_read_path_realization;

// point-time BE-T09: response/wallet-list non-leak + DTO effective_at hiding
// (design §6.1 P1 "管理员钱包列表不泄漏未来期积分" + §4.2/§5.1 P1-2 DTO
// `effective_at` permission hiding). Asserts the `skip_serializing_if`
// field-level contract via raw JSON KEY presence/absence (not just value),
// and the cross-user batched derived assembly in `list_wallets`.
mod test_83_response_non_leak_dto_hidden;

// point-time BE-T05 (renewal path): Stripe invoice.payment_succeeded period
// normalization (sibling to test_81). Covers the renewal event that the A8
// strictness regression silently broke — a Stripe Invoice has NO top-level
// `current_period_*` and uses `lines.data` (NOT `items.data`); the invoice
// resolver reads `lines.data[].period.{start,end}`. Exercises the renewal
// grant END-TO-END via the webhook HTTP path: single-line period → grant;
// line without a period → SKIP (A8 P0).
mod test_84_stripe_invoice_period_normalization;

pub mod fixtures;
