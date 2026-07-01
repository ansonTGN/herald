// =============================================================================
// Webhook Grant Idempotency Scenario Tests
// =============================================================================
//
// Tests for verifying webhook duplicate delivery does not cause double point
// grants. When a payment provider (Stripe/Creem) re-delivers a webhook event
// with the same event_id, the system must grant points exactly once.
//
// User Story: US-EM-003, US-EM-004
// Covers: Grant-side idempotency (credit ledger + wallet balance + revocation)
//
// =============================================================================

use crate::tests::helpers::billing_helpers::setup_stripe_config;
use crate::tests::helpers::points_helpers::*;
use crate::tests::helpers::webhook_helpers::*;
use crate::tests::schema_test_context::SchemaTestContext;
use herald_core::domain::points::entities::CreditType;
use test_context::test_context;
use uuid::Uuid;

// ============================================================================
// Test 1: Stripe invoice.payment_succeeded duplicate event no double grant
// ============================================================================

// User Story: US-EM-003, US-EM-004
// Covers: Duplicate Stripe invoice.payment_succeeded with same event_id must not
//         create a second subscription quota entitlement or double window balance.
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_subscription_paid_duplicate_event_no_double_grant(ctx: &mut SchemaTestContext) {
    // Given
    let realm_id = ctx._realm_id.clone();
    let user_id = crate::tests::scenarios::points::fixtures::create_test_user_with_auth(
        &ctx.app_state.pool,
        &realm_id,
        "stripe-idem-grant@test.com",
        "password123",
    )
    .await;
    let entitlement_key = "stripe-idem-plan";
    let event_id = generate_test_event_id();
    let stripe_subscription_id = format!("sub_stripe_idem_{}", event_id);
    let external_product_id = format!("prod_stripe_{}", entitlement_key);
    let webhook_secret = "test_stripe_wh_secret";

    // Setup Stripe config
    setup_stripe_config(ctx, &realm_id, "sk_test_key", webhook_secret).await;

    // Create entitlement mapping so the handler knows how many points to grant
    setup_test_entitlement_mapping_for_webhook(
        ctx,
        &realm_id,
        "stripe",
        &external_product_id,
        entitlement_key,
        1000,
        true,
        true,
    )
    .await;

    // Create points wallet
    create_points_wallet(ctx, user_id, &realm_id).await;

    // Build Stripe invoice.payment_succeeded event
    let event = build_stripe_invoice_with_herald_metadata(
        &event_id,
        &stripe_subscription_id,
        &realm_id,
        user_id,
        entitlement_key,
        2500, // amount_paid
    );

    let app = ctx.create_unified_test_router();

    // When: First processing
    let response1 =
        send_stripe_webhook_with_signature(&app, &realm_id, event.clone(), webhook_secret).await;
    assert_webhook_success(&response1);

    // When: Second processing (same event_id)
    let response2 =
        send_stripe_webhook_with_signature(&app, &realm_id, event, webhook_secret).await;
    assert_webhook_success(&response2);

    // Then: Only one subscription quota entitlement should exist
    let entitlements =
        get_user_quota_entitlements(ctx, user_id, CreditType::SubscriptionCredit).await;
    assert_eq!(
        entitlements.len(),
        1,
        "Duplicate event must not create a second subscription quota entitlement"
    );

    let entitlement = &entitlements[0];
    assert_eq!(
        entitlement.quota_windows[0].limit, 1000,
        "Granted window limit should be exactly one plan allocation"
    );

    // Verify window availability reflects only one grant
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, &realm_id).await;
    let remaining = compute_window_available(
        ctx,
        &realm_id,
        user_id,
        bucket_id,
        CreditType::SubscriptionCredit,
    )
    .await;
    assert_eq!(
        remaining, 1000,
        "Window availability should reflect exactly one grant, not doubled"
    );

    // Verify no revocation records were created
    let revocations = get_revocation_records(ctx, user_id).await;
    assert_eq!(
        revocations.len(),
        0,
        "No revocation records should exist for a grant-only flow"
    );
}

// ============================================================================
// Test 2: Creem checkout.completed duplicate event no double grant
// ============================================================================

// User Story: US-EM-003, US-EM-004
// Covers: Duplicate Creem checkout.completed with same event_id must not
//         create a second topup credit ledger or double wallet balance.
#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_creem_checkout_completed_duplicate_event_no_double_grant(
    ctx: &mut SchemaTestContext,
) {
    // Given
    let realm_id = ctx._realm_id.clone();
    let user_id = crate::tests::scenarios::points::fixtures::create_test_user_with_auth(
        &ctx.app_state.pool,
        &realm_id,
        "creem-idem-grant@test.com",
        "password123",
    )
    .await;
    let client_app_id = Uuid::parse_str(&ctx._client_app_id).unwrap();
    let entitlement_key = "creem-idem-plan";
    let event_id = generate_test_event_id();
    let webhook_secret = "test_creem_wh_secret";

    // Configure Creem webhook for this realm
    ctx.with_creem_config(&realm_id, None, Some(webhook_secret), None)
        .await;

    // Create entitlement mapping for checkout.completed one-time grant
    let external_product_id = format!("prod_creem_{}", entitlement_key);
    let mapping_id = setup_test_entitlement_mapping_for_webhook(
        ctx,
        &realm_id,
        "creem",
        &external_product_id,
        entitlement_key,
        2000,
        true,
        true,
    )
    .await;

    // Set billing_type=one_time so the handler dispatches to the one-time fulfillment path
    sqlx::query("UPDATE provider_entitlement_mappings SET billing_type = 'one_time' WHERE id = $1")
        .bind(mapping_id)
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to set billing_type=one_time");

    // Create a pending payment attempt so the one-time fulfillment can complete it.
    // payment_attempts.bucket_id is NOT NULL — bind the realm's legacy test bucket.
    let bucket_id = crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm(
        &ctx.app_state.pool,
        &realm_id,
    )
    .await;
    let attempt_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO payment_attempts
            (id, realm_id, user_id, payment_provider, target_type, target_id,
             bucket_id, amount, currency, status, expires_at, created_at, updated_at)
         VALUES ($1, $2, $3, 'creem', 'entitlement_mapping', $4,
                 $5, $6, 'USD', 'Pending', NOW() + INTERVAL '2 hours', NOW(), NOW())",
    )
    .bind(attempt_id)
    .bind(&realm_id)
    .bind(user_id)
    .bind(mapping_id)
    .bind(bucket_id)
    .bind(2000i64)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create pending payment attempt");

    // Create points wallet
    create_points_wallet(ctx, user_id, &realm_id).await;

    // Build Creem checkout.completed event with herald_* metadata and attempt_id
    let event = build_creem_checkout_completed_with_herald_metadata(
        &event_id,
        entitlement_key,
        &realm_id,
        user_id,
        client_app_id,
    );
    // Inject attempt_id into metadata so the handler can find and complete the payment attempt
    let mut event = event;
    event["object"]["metadata"]["attemptId"] = serde_json::json!(attempt_id.to_string());

    let app = ctx.create_unified_test_router();

    // When: First processing
    let response1 =
        send_webhook_with_signature(&app, &realm_id, event.clone(), webhook_secret).await;
    assert_webhook_success(&response1);

    // When: Second processing (same event_id)
    let response2 = send_webhook_with_signature(&app, &realm_id, event, webhook_secret).await;
    assert_webhook_success(&response2);

    // Then: Only one topup credit ledger should exist
    let ledgers = get_user_ledgers_by_credit_type(ctx, user_id, CreditType::TopupCredit).await;
    assert_eq!(
        ledgers.len(),
        1,
        "Duplicate event must not create a second topup credit ledger"
    );

    let ledger = &ledgers[0];
    assert_eq!(
        ledger.granted_amount, 2000,
        "Granted amount should be exactly one plan allocation"
    );

    // Verify wallet balance reflects only one grant
    let remaining = get_remaining_credit_by_type(ctx, user_id, CreditType::TopupCredit).await;
    assert_eq!(
        remaining, 2000,
        "Wallet remaining should reflect exactly one grant, not doubled"
    );
}
