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

// Creem webhook one-time dispatch scenarios (authored by BE-T03)
// User Story: US-PA-003, US-PU-006
// Covers: Design section 5.1
pub mod creem_webhook_one_time_scenarios;

// One-time purchase fulfillment scenarios (authored by BE-T04)
// User Story: US-PU-006, US-PA-001, US-PA-003
// Covers: Design section 5.1 "PurchaseService + FulfillmentService"
pub mod one_time_fulfillment_scenarios;

// Entitlement subscription scenarios (authored by BE-T03)
pub mod entitlement_subscription_scenarios;

// Subscription checkout session scenarios
pub mod checkout_session_scenarios;

// Stripe checkout mode branching scenarios (one-time vs recurring)
// User Story: US-EM-001, US-PU-006, US-PA-001
// Covers: Design section 5.1
pub mod stripe_checkout_mode_scenarios;

// Stripe webhook mode dispatch scenarios (payment vs subscription)
// User Story: US-PA-003, US-PU-006
// Covers: Design section 5.1
pub mod stripe_webhook_mode_scenarios;

// Subscription points entitlement scenarios (authored by BE-T03)
pub mod subscription_points_entitlement_scenarios;

// Shopify Config AuthZ scenario tests (does not reference deleted types)
pub mod shopify_config_authz_scenarios;

// Shopify Config Encryption scenario tests
pub mod shopify_config_encryption_scenarios;

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

// One-time API endpoint scenario tests (authored by BE-T05)
// User Story: US-EM-001, US-PU-006, US-PU-007, US-PA-001
// Covers: Design section 4.2 "API Interface Design"
pub mod one_time_api_scenarios;
