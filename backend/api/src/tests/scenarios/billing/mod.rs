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
//
// User Story: docs/user-stories/06-billing-user-stories.md
//
// =============================================================================

// New entitlement mapping CRUD + sync scenario tests
pub mod entitlement_mapping_crud_scenarios;

// Provider product sync metadata scenario tests (sync-payment feature)
// User Story: US-BL-SYNC-001
// Covers: Stripe product/price metadata propagation into provider_product_info
//         JSONB; re-sync takes the latest metadata.
pub mod provider_product_sync_scenarios;

// Webhook entitlement scenarios
pub mod webhook_entitlement_scenarios;

// Creem webhook one-time dispatch scenarios
// User Story: US-PA-003, US-PU-006
// Covers: Design section 5.1
pub mod creem_webhook_one_time_scenarios;

// One-time purchase fulfillment scenarios
// User Story: US-PU-006, US-PA-001, US-PA-003
// Covers: Design section 5.1 "PurchaseService + FulfillmentService"
pub mod one_time_fulfillment_scenarios;

// Entitlement subscription scenarios
pub mod entitlement_subscription_scenarios;

// Stripe webhook mode dispatch scenarios (payment vs subscription)
// User Story: US-PA-003, US-PU-006
// Covers: Design section 5.1
pub mod stripe_webhook_mode_scenarios;

// Subscription points entitlement scenarios
pub mod subscription_points_entitlement_scenarios;

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

// One-time API endpoint scenario tests
// User Story: US-EM-001, US-PU-006, US-PU-007, US-PA-001
// Covers: Design section 4.2 "API Interface Design"
pub mod one_time_api_scenarios;

// Stripe webhook patch scenario tests (new webhook event handlers)
// Covers: checkout.session.expired/async_*, dispute.created/closed,
//         subscription paused/resumed/updated/deleted
pub mod stripe_webhook_patch_scenarios;

// Creem webhook patch scenario tests (new webhook event handlers)
// Covers: subscription lifecycle events, dispute.created
pub mod creem_webhook_patch_scenarios;

// Webhook compensation job scenario tests
// User Story: US-WC-001, US-WC-002
// Covers: Design section 5.1 (Stripe/Creem missed event compensation)
pub mod webhook_compensation_scenarios;

// Async payment points strategy scenario tests
// User Story: US-AP-001, US-AP-002
// Covers: Design sections 4.1, 5.1 (strategy config + eager fulfillment)
pub mod async_payment_points_strategy_scenarios;

// Async payment revocation scenario tests
// User Story: US-AP-002 (idempotency), US-AP-003 (revocation), US-AP-004 (debt)
// Covers: Design sections 4.1, 4.3, 5.1
pub mod async_payment_revocation_scenarios;

// Payment attempt status guard scenario tests
pub mod payment_attempt_status_guard_scenarios;

// Webhook grant idempotency scenario tests
// Covers: Duplicate webhook delivery must not cause double point grants
pub mod webhook_grant_idempotency_scenarios;

// Manual Credit Note scenario tests
// User Story: US-IF-010 (admin records offline refund), US-IF-008 (refund visibility)
// Covers: Design section 4.2.2 (Manual Credit Note API)
pub mod credit_note_manual_scenarios;

// Stripe credit_note.created Webhook scenario tests
// User Story: US-IF-007 (Stripe credit note sync), US-IF-008 (refund visibility)
// Covers: Design section 4.1 (Stripe Credit Note path)
pub mod credit_note_stripe_webhook_scenarios;

// Invoice refund field query scenario tests
// User Story: US-IF-008 (admin refund visibility), US-IF-009 (user refund annotation)
// Covers: Design sections 1.3 (Creem exclusion), 4.2.2 (refund field extensions)
pub mod invoice_refund_query_scenarios;

// Feature-availability invoice eligibility scenario tests
// User Story: docs/user-stories/billing/invoice-fallback.md
// Covers: P0-2 Phase A -- realm-level invoice eligibility via feature-availability
//         so frontend can gate Create/Apply invoice buttons before submit.
pub mod feature_availability_invoice_eligibility_scenarios;

// Invoice per-resource apply-eligibility scenario tests
// User Story: docs/user-stories/billing/invoice-fallback.md
// Covers: P0-2 Phase B -- read-only, context-level apply-eligibility verdict
//         for a specific payment_attempt/subscription, so frontend can gate the
//         Apply Invoice button on that resource before submit.
pub mod invoice_apply_eligibility_scenarios;

// Creem subscription renewal scenario tests (payment-invoice-mapping)
// User Story: US-PM-001 (renewal payment_attempt), US-PM-002 (renewal invoice)
// Covers: Design §5.2 (Creem renewal invoice sync), §6.1, §7 (P0 dedup / P1 fallback)
pub mod creem_subscription_renewal_scenarios;

// Stripe subscription renewal scenario tests (payment-invoice-mapping)
// User Story: US-PM-001 (renewal payment_attempt), US-PM-003 (invoice attribution)
// Covers: Design §5.3 (Stripe renewal attempt + re-upsert attribution),
//         §6.1 (Stripe renewal cases), §6.3 (hosted_url/pdf_url non-regression)
pub mod stripe_subscription_renewal_scenarios;

// External invoice attribution + regression scenario tests (payment-invoice-mapping)
// User Story: US-PM-003 (external invoice attribution)
// Covers: Design §5.4 (attribution backfill + upsert COALESCE),
//         §6.1 (attribution + regression cases), §6.3 (COALESCE non-clobber)
pub mod external_invoice_attribution_scenarios;
