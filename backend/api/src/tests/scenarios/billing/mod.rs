// =============================================================================
// Billing Security Tests
// =============================================================================
//
// Tests for:
// 1. Permission checks (billing.view, billing.manage)
// 2. Webhook signature verification
// 3. Entitlement mapping CRUD + sync
// 4. Invoice tests
// 5. Points package tests
// 6. Shopify/WeChat config tests
//
// User Story: docs/user-stories/06-billing-user-stories.md
//
// NOTE: Modules that exclusively test deleted Product/Plan/PointsPlanConfig
// functionality or import deleted types (create_test_plan, ensure_default_product,
// etc.) have been commented out. New scenario files replace their coverage.
// BE-T04 runner step 2 will verify all 4 new modules are registered and
// uncomment any that were commented out.
//
// =============================================================================

// Commented out: reference deleted types (ensure_default_product, create_test_plan, etc.)
// pub mod permission_checks;
// pub mod webhook_tests;

// New entitlement mapping CRUD + sync scenario tests
pub mod entitlement_mapping_crud_scenarios;

// Webhook entitlement scenarios (authored by BE-T02)
pub mod webhook_entitlement_scenarios;

// Entitlement subscription scenarios (authored by BE-T03)
pub mod entitlement_subscription_scenarios;

// Subscription points entitlement scenarios (authored by BE-T03)
pub mod subscription_points_entitlement_scenarios;

// --- Commented out: reference deleted types (create_test_plan, create_test_subscription, etc.)
// These modules will be replaced by the new scenario files above.
// pub mod billing_period_tests;
// pub mod error_recovery_tests;
// pub mod feature_availability_scenarios;
// pub mod integration_e2e_tests;
// pub mod payment_flow_tests;
// pub mod plan_delete_scenarios;
// pub mod plan_update_scenarios;
// pub mod purchase_authz_scenarios;
// pub mod refund_tests;
// pub mod shopify_integration_scenarios;
// pub mod subscription_fulfillment_scenarios;
// pub mod subscription_lifecycle_tests;
// pub mod webhook_infrastructure_tests;
// pub mod wechat_order_authz_scenarios;

// Shopify Config AuthZ scenario tests (does not reference deleted types)
pub mod shopify_config_authz_scenarios;

// Shopify Config Encryption scenario tests
pub mod shopify_config_encryption_scenarios;

// Points Package scenario tests
pub mod points_package_authz_scenarios;
pub mod points_package_promo_scenarios;
pub mod points_package_purchase_scenarios;

// WeChat Pay config scenario tests
pub mod wechat_config_authz_scenarios;

// Invoice Admin scenario tests
pub mod invoice_admin_scenarios;

// Invoice PDF scenario tests
pub mod invoice_pdf_scenarios;

// Invoice Provider & Policy Guard scenario tests
pub mod invoice_provider_policy_scenarios;

// Invoice External Sync scenario tests
pub mod invoice_external_sync_scenarios;

// Invoice User scenario tests
pub mod invoice_user_scenarios;

// Invoice Overdue Job scenario tests
pub mod invoice_overdue_scenarios;
