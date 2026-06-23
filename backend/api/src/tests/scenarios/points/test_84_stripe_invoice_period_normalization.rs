// =============================================================================
// point-time BE-T05 (renewal path): Stripe invoice.payment_succeeded period
// normalization (design §6.1, A8 P0)
// =============================================================================
//
// SCENARIO-LAYER coverage of the renewal grant on a REALISTIC Stripe Invoice
// payload. Sibling to `test_81_provider_period_normalization.rs`, which covers
// `customer.subscription.created`. This file covers the renewal event that
// the A8 strictness regression silently broke:
//
//   Stripe `invoice.payment_succeeded` carries a Stripe **Invoice** object as
//   `data.object`. A Stripe Invoice has NO top-level `current_period_*`
//   (those are Subscription/SubscriptionItem fields) and exposes its line
//   items under `lines.data` (NOT `items.data`). For a subscription renewal
//   invoice, each line's `period.{start,end}` IS the subscription billing
//   period being paid (Stripe docs: "For subscription line items, this is the
//   subscription period.").
//
// Before the fix, `handle_invoice_payment_succeeded` called
// `normalize_stripe_period(&event["data"]["object"])`, which reads
// `items.data[].current_period_*` / top-level — both absent on an Invoice →
// the handler took the `else` branch (`warn!(reason =
// "period_uniquely_unresolvable")`) and renewal credits were SILENTLY NEVER
// GRANTED. The fix swaps in `normalize_stripe_invoice_period`, which reads
// `lines.data[].period.{start,end}`.
//
// These tests exercise the renewal path END-TO-END via the webhook HTTP path
// (the invoice normalizer is private to herald-api-billing, so the only way
// to observe it is through `handle_subscription_paid` being invoked or
// skipped). They are NOT duplicates of the `normalize_stripe_invoice_period`
// `#[cfg(test)]` unit tests in
// `backend/api-billing/src/stripe_webhook_handlers.rs` — this file asserts
// the *consequence* of normalization at the scenario layer: ledger rows
// written (Some) vs. NOT written (None).
//
// Entry point exercised (read-only, do NOT modify):
//   * Stripe `invoice.payment_succeeded` → `handle_invoice_payment_succeeded`
//     → `normalize_stripe_invoice_period(&event["data"]["object"])`
//     (backend/api-billing/src/stripe_webhook_handlers.rs, renewal handler)
//
// All balance assertions use the BE-T01 derived-predicate helper
// (`assert_derived_balance`), never `points_wallets.total_balance` (BE-D11
// physically removed that column).
//
// A8 P0 quadrants covered:
//   (g) Stripe invoice single-line period              → Some  → grant
//   (h) Stripe invoice with NO line carrying a period  → None  → SKIP grant
//
// =============================================================================

use crate::tests::helpers::points_helpers::{
    assert_derived_balance, get_user_ledgers_by_credit_type,
};
use crate::tests::helpers::webhook_helpers::{
    assert_webhook_success, generate_test_event_id, send_stripe_webhook_with_signature,
    setup_test_entitlement_mapping_for_webhook,
};
use crate::tests::schema_test_context::SchemaTestContext;
use herald_core::domain::points::entities::CreditType;
use test_context::test_context;
use uuid::Uuid;

// ----------------------------------------------------------------------------
// Shared local helpers (mirror test_81 — kept self-contained per sibling test
// convention; avoids pulling in cross-test import paths that drift)
// ----------------------------------------------------------------------------

/// Create a test account row directly. Mirrors test_81's local `create_user`
/// to keep this file self-contained.
async fn create_user(ctx: &SchemaTestContext, realm_id: &str, email: &str) -> Uuid {
    let user_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO account (id, realm_id, email, password, status)
         VALUES ($1, $2, $3, $4, 1)
         ON CONFLICT (realm_id, email) DO NOTHING",
    )
    .bind(user_id)
    .bind(realm_id)
    .bind(email)
    .bind("$2a$12$dummy_password_hash")
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create test user");
    user_id
}

/// Build a Stripe `invoice.payment_succeeded` event whose `data.object` is a
/// REALISTIC Stripe Invoice (NO top-level `current_period_*`; subscription
/// line items under `lines.data`, each carrying `period.{start,end}`).
///
/// The caller controls whether the subscription line carries a `period`:
///   * `Some((start_ts, end_ts))` → the line carries the period → resolver
///     returns Some → renewal grant fires.
///   * `None`                     → the line carries no period → resolver
///     returns None → renewal grant is skipped (A8 P0).
fn build_stripe_invoice_payment_succeeded_event(
    event_id: &str,
    realm_id: &str,
    user_id: Uuid,
    entitlement_key: &str,
    stripe_subscription_id: &str,
    line_period: Option<(i64, i64)>,
) -> serde_json::Value {
    let mut line = serde_json::json!({
        "id": format!("il_{}", event_id),
        "object": "line_item",
        "type": "subscription",
        "subscription": stripe_subscription_id,
        "quantity": 1,
        "amount": 2500,
        "currency": "usd",
    });
    if let Some((start, end)) = line_period {
        line["period"] = serde_json::json!({ "start": start, "end": end });
    }

    serde_json::json!({
        "id": event_id,
        "object": "event",
        "type": "invoice.payment_succeeded",
        "api_version": "2020-08-27",
        "created": chrono::Utc::now().timestamp(),
        "data": {
            "object": {
                "id": format!("in_{}", event_id),
                "object": "invoice",
                "status": "paid",
                "subscription": stripe_subscription_id,
                "amount_paid": 2500,
                "currency": "usd",
                // NOTE: deliberately NO top-level current_period_* — Stripe
                // Invoices do not carry those fields (they are Subscription/
                // SubscriptionItem fields). This is exactly the shape that
                // broke the renewal grant before the fix.
                "lines": { "data": [ line ] },
                "metadata": {
                    "herald_realm_id": realm_id,
                    "herald_user_id": user_id.to_string(),
                    "herald_entitlement_key": entitlement_key,
                    "userId": user_id.to_string(),
                }
            }
        }
    })
}

// ============================================================================
// Scenario (g): Stripe invoice single-line period → Some → renewal grant
// ============================================================================

// User Story: US-PU-009 (use this period's credits on time, on renewal).
// Covers design §6.1 P0 "provider 周期归一化" + A8 P0 renewal quadrant (g):
//   a Stripe `invoice.payment_succeeded` renewal invoice carries the billing
//   period on each subscription line's `period.{start,end}`. The normalizer
//   must resolve it to a unique `(period_start, period_end)` and DRIVE
//   `handle_subscription_paid` (Some ⟹ renewal grant). The current-period
//   grant is immediately available (`effective_at = period_start <= now`).
//
// Why this test exists: before the fix, the renewal handler called
// `normalize_stripe_period` against an Invoice object, which always returned
// None (Invoice has no `items.data` / top-level `current_period_*`), so
// renewal credits were SILENTLY NEVER GRANTED. This test is the regression
// guard for the renewal path.
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_stripe_invoice_period_normalized_drives_renewal_grant(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_user(ctx, &realm_id, "be-t05-stripe-inv-ok@example.com").await;

    let entitlement_key = format!("be-t05-stripe-inv-ok-{}", Uuid::now_v7());
    let external_product_id = format!("prod_{}", entitlement_key);
    let event_id = generate_test_event_id();
    let webhook_secret = "test_stripe_wh_secret";
    let stripe_subscription_id = format!("sub_inv_ok_{}", Uuid::now_v7());

    // Stripe provider config so the webhook signature verifies.
    crate::tests::helpers::billing_helpers::setup_stripe_config(
        ctx,
        &realm_id,
        "sk_test_key",
        webhook_secret,
    )
    .await;

    // Entitlement mapping so the renewal grant resolves points_per_period.
    setup_test_entitlement_mapping_for_webhook(
        ctx,
        &realm_id,
        "stripe",
        &external_product_id,
        &entitlement_key,
        1000,
        true,
        true,
    )
    .await;

    // Build a REALISTIC renewal invoice: subscription line carries its
    // `period`. (No top-level current_period_* — that is the bug shape.)
    let now = chrono::Utc::now();
    let period_start = now - chrono::Duration::seconds(10);
    let period_end = now + chrono::Duration::days(30);
    let event = build_stripe_invoice_payment_succeeded_event(
        &event_id,
        &realm_id,
        user_id,
        &entitlement_key,
        &stripe_subscription_id,
        Some((period_start.timestamp(), period_end.timestamp())),
    );

    // When: Stripe webhook fires.
    let app = ctx.create_unified_test_router();
    let response = send_stripe_webhook_with_signature(&app, &realm_id, event, webhook_secret).await;
    assert_webhook_success(&response);

    // Then: a subscription_credit ledger WAS written — the invoice line period
    // resolved and drove the renewal grant.
    let ledgers =
        get_user_ledgers_by_credit_type(ctx, user_id, CreditType::SubscriptionCredit).await;
    assert!(
        !ledgers.is_empty(),
        "Stripe invoice single-line period must normalize to Some and drive a renewal grant; \
         got 0 subscription_credit ledgers"
    );

    // The current-period grant (effective_at = period_start <= now) is
    // immediately available via the derived predicate.
    assert_derived_balance(
        ctx,
        user_id,
        &realm_id,
        CreditType::SubscriptionCredit,
        1000,
    )
    .await;
}

// ============================================================================
// Scenario (h): Stripe invoice with NO line carrying a period → None → SKIP
// ============================================================================

// User Story: US-PU-009 (never grant against a guessed period).
// Covers design §6.1 P0 "provider 周期归一化前置失败" + A8 P0 renewal quadrant (h):
//   when the renewal invoice's subscription line does NOT carry a `period`
//   (a malformed / partial payload, or a provider quirk), the normalizer
//   returns None and the handler MUST skip the renewal grant, emit a
//   structured warning, and await a later webhook / API compensation. Never
//   guess from event time.
//
// Why this test exists: this is the renewal-path counterpart to scenario (d)
// in test_81. It pins down that the A8 P0 "None ⟹ skip" gate applies to the
// renewal event as well — a regression that fell back to "event time as
// period_start" on missing invoice line periods would silently grant against
// the wrong window.
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_stripe_invoice_no_line_period_skips_renewal_grant(ctx: &mut SchemaTestContext) {
    let realm_id = ctx._realm_id.clone();
    let user_id = create_user(ctx, &realm_id, "be-t05-stripe-inv-miss@example.com").await;

    let entitlement_key = format!("be-t05-stripe-inv-miss-{}", Uuid::now_v7());
    let external_product_id = format!("prod_{}", entitlement_key);
    let event_id = generate_test_event_id();
    let webhook_secret = "test_stripe_wh_secret";
    let stripe_subscription_id = format!("sub_inv_miss_{}", Uuid::now_v7());

    crate::tests::helpers::billing_helpers::setup_stripe_config(
        ctx,
        &realm_id,
        "sk_test_key",
        webhook_secret,
    )
    .await;

    setup_test_entitlement_mapping_for_webhook(
        ctx,
        &realm_id,
        "stripe",
        &external_product_id,
        &entitlement_key,
        1000,
        true,
        true,
    )
    .await;

    // Subscription line carries NO `period` field — exercises the
    // "no line with a period ⟹ None" quadrant.
    let event = build_stripe_invoice_payment_succeeded_event(
        &event_id,
        &realm_id,
        user_id,
        &entitlement_key,
        &stripe_subscription_id,
        None,
    );

    let app = ctx.create_unified_test_router();
    let response = send_stripe_webhook_with_signature(&app, &realm_id, event, webhook_secret).await;
    // The handler acknowledges the webhook (so Stripe doesn't redeliver
    // aggressively) even when it skips the grant — this mirrors the
    // EntitlementMappingNotFound graceful-skip behavior.
    assert_webhook_success(&response);

    // Then: NO subscription_credit ledger was written — A8 P0 forbids writing
    // a ledger with an invented period.
    let ledgers =
        get_user_ledgers_by_credit_type(ctx, user_id, CreditType::SubscriptionCredit).await;
    assert!(
        ledgers.is_empty(),
        "Stripe invoice with NO line carrying a period must SKIP the renewal grant (A8 P0 — \
         never guess); got {} subscription_credit ledgers",
        ledgers.len()
    );

    // And: derived available balance is 0 (no ledger row exists).
    assert_derived_balance(ctx, user_id, &realm_id, CreditType::SubscriptionCredit, 0).await;
}
