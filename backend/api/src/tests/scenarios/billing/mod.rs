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
// =============================================================================

// New entitlement mapping CRUD + sync scenario tests
pub mod entitlement_mapping_crud_scenarios;

// Webhook entitlement scenarios (authored by BE-T02)
pub mod webhook_entitlement_scenarios;

// Entitlement subscription scenarios (authored by BE-T03)
pub mod entitlement_subscription_scenarios;

// Subscription points entitlement scenarios (authored by BE-T03)
pub mod subscription_points_entitlement_scenarios;

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
