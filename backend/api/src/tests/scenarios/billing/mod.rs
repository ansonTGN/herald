// =============================================================================
// Billing Security Tests
// =============================================================================
//
// Tests for:
// 1. Permission checks (billing.view, billing.manage)
// 2. Webhook signature verification
// 3. Webhook idempotency
// 4. Timestamp validation (replay attack prevention)
// 5. Payment flow tests
// 6. Subscription lifecycle tests
// 7. Billing period tests
// 8. Refund processing tests
// 9. Error recovery tests
// 10. E2E integration tests
//
// User Story: docs/user-stories/06-billing-user-stories.md
//
// =============================================================================

pub mod permission_checks;
pub mod webhook_tests;

// New test modules
pub mod billing_period_tests;
pub mod error_recovery_tests;
pub mod integration_e2e_tests;
pub mod payment_flow_tests;
pub mod plan_delete_scenarios;
pub mod plan_update_scenarios;
pub mod purchase_authz_scenarios;
pub mod refund_tests;
pub mod subscription_lifecycle_tests;
pub mod webhook_infrastructure_tests;

// WeChat Pay scenario tests (removed - see .ai/future/backend_test_delete.md)
pub mod wechat_config_authz_scenarios;
pub mod wechat_order_authz_scenarios;
// pub mod wechat_webhook_scenarios;

// Shopify Config AuthZ scenario tests
pub mod shopify_config_authz_scenarios;

// Shopify Config Encryption scenario tests
pub mod shopify_config_encryption_scenarios;

// Points Package scenario tests
pub mod points_package_authz_scenarios;
pub mod points_package_purchase_scenarios;

// Subscription Fulfillment scenario tests
pub mod subscription_fulfillment_scenarios;

// Shopify Integration scenario tests (end-to-end workflows)
pub mod shopify_integration_scenarios;

// Invoice Admin scenario tests
pub mod invoice_admin_scenarios;

// Invoice PDF scenario tests
pub mod invoice_pdf_scenarios;

// Invoice User scenario tests
pub mod invoice_user_scenarios;

// Invoice Overdue Job scenario tests
pub mod invoice_overdue_scenarios;
