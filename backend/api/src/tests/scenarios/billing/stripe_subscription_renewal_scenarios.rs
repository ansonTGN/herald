// =============================================================================
// Stripe Subscription Renewal Scenario Tests (payment-invoice-mapping)
// =============================================================================
//
// Tests for Stripe `invoice.payment_succeeded` **subscription renewal** branch
// `payment_attempt` (idempotent by `provider_reference =
// stripe_renewal:{stripe_subscription_id}:{stripe_invoice_id}`) and re-upsert
// the invoice row so that BOTH `subscription_id` and `payment_attempt_id`
// attribution are non-null. Covers idempotency on duplicate delivery, the
// coexistence-with-`invoice.*`-sync path (one row, attribution filled), the
// `payment_succeeded`-without-prior-`invoice.*` INSERT path, and the
// zero-yuan cycle skip.
//
// User Story: US-PM-001 (every renewal records a payment_attempt),
//             US-PM-003 (external invoice attribution to subscription + payment)
//                  §6.1 (`stripe_subscription_renewal_scenarios` cases),
//                  §6.3 (regression: re-upsert must NOT regress
//                         hosted_url / pdf_url written by an earlier invoice.*
//                         sync event)
//
// NOTE (orchestrator correction, verified against migration
// `20260609_points_package_one_time.sql`): the production `target_type` CHECK
// now only allows `'entitlement_mapping'`. These tests query renewal attempts
// BY `provider_reference` and assert `status='Succeeded'`; they do NOT assert
// `target_type='subscription_entitlement'`.
// =============================================================================

#[cfg(test)]
mod tests {
    use crate::tests::helpers::billing_helpers::{
        setup_stripe_config, setup_test_entitlement_mapping_full,
    };
    use crate::tests::helpers::webhook_helpers::{
        assert_webhook_success, generate_test_event_id, send_stripe_webhook_with_signature,
    };
    use crate::tests::schema_test_context::SchemaTestContext;
    use axum::http::StatusCode;
    use serde_json::json;
    use test_context::test_context;
    use uuid::Uuid;

    use SchemaTestContext as RenewalTestContext;

    // =========================================================================
    // Shared Setup Helpers
    // =========================================================================

    /// Create a test user in the test realm and return their UUID.
    async fn create_test_user(ctx: &RenewalTestContext, realm_id: &str, email: &str) -> Uuid {
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

    /// Create a points wallet (lifetime analytics row) for a user.
    async fn create_points_wallet(ctx: &RenewalTestContext, user_id: Uuid, realm_id: &str) {
        let bucket_id = crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm(
            &ctx.app_state.pool,
            realm_id,
        )
        .await;
        sqlx::query(
            "INSERT INTO points_wallets (id, user_id, realm_id, bucket_id, total_topup_granted, total_subscription_granted, total_recharged, total_consumed, status, created_at, updated_at)
             VALUES ($1, $2, $3, $4, 0, 0, 0, 0, 'active', NOW(), NOW())
             ON CONFLICT (realm_id, user_id, bucket_id) DO NOTHING",
        )
        .bind(Uuid::now_v7())
        .bind(user_id)
        .bind(realm_id)
        .bind(bucket_id)
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to create points wallet");
    }

    /// Create a recurring Stripe entitlement mapping (the resolver used by the
    /// renewal branch maps the subscription's product/price to this row).
    async fn create_recurring_mapping(
        ctx: &mut RenewalTestContext,
        realm_id: &str,
        entitlement_key: &str,
        external_product_id: &str,
        external_price_id: Option<&str>,
    ) {
        setup_test_entitlement_mapping_full(
            ctx,
            realm_id,
            "stripe",
            external_product_id,
            external_price_id,
            entitlement_key,
            Some("recurring"),
            Some("every_month"),
            Some(500),
            None,
            None,
            true,
            None,
            true,
            None,
        )
        .await;
    }

    /// Seed the subscription row that `handle_invoice_payment_succeeded` looks
    /// up via `find_by_external_subscription_id` to recover the renewal's
    /// `external_product_id` / `external_price_id`. Real Stripe flows always
    /// create this row on the initial `customer.subscription.created` BEFORE
    /// the first renewal `invoice.payment_succeeded`; these renewal scenarios
    /// exercise the renewal branch only, so the prior subscription row is a
    /// required precondition (the invoice payload itself carries no product
    /// id the resolver can use). Without it the resolver 400s with
    /// "entitlement_key missing and price ambiguous".
    async fn seed_stripe_subscription_for_renewal(
        ctx: &RenewalTestContext,
        realm_id: &str,
        user_id: Uuid,
        entitlement_key: &str,
        external_product_id: &str,
        external_price_id: &str,
        stripe_subscription_id: &str,
    ) {
        let subscription_id = Uuid::now_v7();
        // `subscription.client_app_id` is UNIQUE; use a fresh id per seed so
        // multiple renewal scenarios in the same realm do not collide.
        let client_app_id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO subscription
                (id, realm_id, user_id, client_app_id, status, entitlement_key,
                 external_price_id, external_subscription_id, external_product_id,
                 payment_provider, current_period_start, current_period_end,
                 cancel_at_period_end, created_at, updated_at, billing_type)
             VALUES ($1, $2, $3, $4, 'active', $5,
                     $6, $7, $8,
                     'stripe', NOW(), NOW() + INTERVAL '30 days',
                     false, NOW(), NOW(), 'recurring')",
        )
        .bind(subscription_id)
        .bind(realm_id)
        .bind(user_id)
        .bind(client_app_id)
        .bind(entitlement_key)
        .bind(external_price_id)
        .bind(stripe_subscription_id)
        .bind(external_product_id)
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to seed Stripe subscription row for renewal scenario");
    }

    // =========================================================================
    // Query Helpers (private, file-local)
    // =========================================================================

    /// Find the renewal payment_attempt by its exact `provider_reference`.
    /// Returns (id, status, provider_reference).
    async fn find_renewal_attempt_by_reference(
        ctx: &RenewalTestContext,
        realm_id: &str,
        provider_reference: &str,
    ) -> Option<(Uuid, String, String)> {
        sqlx::query_as(
            "SELECT id, status, provider_reference
             FROM payment_attempts
             WHERE realm_id = $1
               AND payment_provider = 'stripe'
               AND provider_reference = $2",
        )
        .bind(realm_id)
        .bind(provider_reference)
        .fetch_optional(&ctx.app_state.pool)
        .await
        .unwrap()
    }

    /// Count renewal attempts for a Stripe subscription
    /// (any `stripe_renewal:{ext_sub}:*`).
    async fn count_renewal_attempts_by_sub(
        ctx: &RenewalTestContext,
        realm_id: &str,
        stripe_subscription_id: &str,
    ) -> i64 {
        let pattern = format!("stripe_renewal:{}:%", stripe_subscription_id);
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM payment_attempts
             WHERE realm_id = $1
               AND payment_provider = 'stripe'
               AND provider_reference LIKE $2",
        )
        .bind(realm_id)
        .bind(&pattern)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap()
    }

    /// Find an invoice by `external_invoice_id`, including attribution and the
    /// url fields guarded by the §6.3 regression. Returns
    /// (id, provider, status, subscription_id, payment_attempt_id,
    ///  external_hosted_url, external_pdf_url).
    async fn find_invoice_with_attribution_and_urls(
        ctx: &RenewalTestContext,
        realm_id: &str,
        external_invoice_id: &str,
    ) -> Option<(
        Uuid,
        String,
        String,
        Option<Uuid>,
        Option<Uuid>,
        Option<String>,
        Option<String>,
    )> {
        sqlx::query_as(
            "SELECT id, provider, status, subscription_id, payment_attempt_id,
                    external_hosted_url, external_pdf_url
             FROM invoice
             WHERE realm_id = $1 AND external_invoice_id = $2",
        )
        .bind(realm_id)
        .bind(external_invoice_id)
        .fetch_optional(&ctx.app_state.pool)
        .await
        .unwrap()
    }

    /// Count invoices matching a given `external_invoice_id`.
    async fn count_invoices_by_external_id(
        ctx: &RenewalTestContext,
        realm_id: &str,
        external_invoice_id: &str,
    ) -> i64 {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM invoice
             WHERE realm_id = $1 AND external_invoice_id = $2",
        )
        .bind(realm_id)
        .bind(external_invoice_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap()
    }

    /// Build a Stripe `invoice.*` (sync) webhook event payload. Mirrors the
    /// shape used by `handle_stripe_invoice_event` (status mapping +
    /// hosted_url/pdf_url capture). Used by the §6.3 coexistence regression
    /// to pre-populate the invoice row from the sync path.
    fn build_stripe_invoice_sync_event(
        event_type: &str,
        stripe_invoice_id: &str,
        realm_id: &str,
        amount: i64,
        stripe_status: &str,
        hosted_url: Option<&str>,
        pdf_url: Option<&str>,
    ) -> serde_json::Value {
        let mut object = json!({
            "id": stripe_invoice_id,
            "object": "invoice",
            "status": stripe_status,
            "total": amount,
            "currency": "usd",
            "metadata": {
                "realmId": realm_id
            }
        });
        if let Some(url) = hosted_url {
            object["hosted_invoice_url"] = json!(url);
        }
        if let Some(url) = pdf_url {
            object["invoice_pdf"] = json!(url);
        }
        json!({
            "id": format!("evt_{}", Uuid::now_v7()),
            "object": "event",
            "type": event_type,
            "api_version": "2020-08-27",
            "created": chrono::Utc::now().timestamp(),
            "data": {
                "object": object
            }
        })
    }

    // =========================================================================
    // Test 1: renewal creates 1 Succeeded attempt + 1 attributed Paid invoice
    // =========================================================================

    /// User Story: US-PM-001, US-PM-003
    ///         §6.1 case 1 (renewal -> attempt + attributed invoice)
    ///
    /// Given: A realm with a recurring Stripe entitlement mapping + a user
    /// When: Stripe sends `invoice.payment_succeeded` for a subscription
    ///       renewal (`in_R1`, total=2000)
    /// Then: Exactly 1 Succeeded renewal attempt exists keyed by
    ///       `stripe_renewal:{sub}:{in_R1}`, and exactly 1 Paid invoice with
    ///       `external_invoice_id=in_R1` whose `subscription_id` and
    ///       `payment_attempt_id` are both non-null and `provider='stripe'`.
    /// Why: This is the core GAP1+GAP2 closure for Stripe -- a renewal must
    ///      record a payment and produce an attributable invoice, otherwise
    ///      billing cannot answer "which payment renewed this subscription"
    ///      or "which invoice belongs to this renewal".
    #[test_context(RenewalTestContext)]
    #[tokio::test]
    async fn test_stripe_renewal_creates_succeeded_attempt_and_attributed_invoice(
        ctx: &mut RenewalTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "test_stripe_renewal_inv_secret";
        let realm_id = ctx._realm_id.clone();
        let user_id = create_test_user(ctx, &realm_id, "stripe-renewal-inv@test.com").await;
        let entitlement_key = "stripe-renewal-inv";
        let external_product_id = format!("prod_stripe_{}", entitlement_key);
        let external_price_id = format!("price_stripe_{}", entitlement_key);
        let stripe_subscription_id = format!("sub_stripe_inv_{}", generate_test_event_id());
        let stripe_invoice_id = "in_R1".to_string();

        setup_stripe_config(ctx, &realm_id, "sk_test_key", webhook_secret).await;
        create_recurring_mapping(
            ctx,
            &realm_id,
            entitlement_key,
            &external_product_id,
            Some(&external_price_id),
        )
        .await;
        create_points_wallet(ctx, user_id, &realm_id).await;

        // Precondition (matches real Stripe flows): the subscription row must
        // exist before its first renewal `invoice.payment_succeeded`, since the
        // renewal handler recovers product/price from this row.
        seed_stripe_subscription_for_renewal(
            ctx,
            &realm_id,
            user_id,
            entitlement_key,
            &external_product_id,
            &external_price_id,
            &stripe_subscription_id,
        )
        .await;

        let event_id = generate_test_event_id();
        let payload =
            crate::tests::helpers::webhook_helpers::build_stripe_invoice_payment_succeeded_renewal(
                &event_id,
                &stripe_subscription_id,
                &stripe_invoice_id,
                &realm_id,
                user_id,
                entitlement_key,
                2000,
                None,
                None,
            );

        let response =
            send_stripe_webhook_with_signature(&app, &realm_id, payload, webhook_secret).await;
        assert_webhook_success(&response);

        // --- Attempt assertions (GAP1) ---
        let expected_reference = format!(
            "stripe_renewal:{}:{}",
            stripe_subscription_id, stripe_invoice_id
        );
        let attempt = find_renewal_attempt_by_reference(ctx, &realm_id, &expected_reference)
            .await
            .expect("renewal payment_attempt should exist after invoice.payment_succeeded");
        assert_eq!(
            attempt.1, "Succeeded",
            "renewal attempt status must be Succeeded (US-PM-001)"
        );
        assert_eq!(
            attempt.2, expected_reference,
            "renewal attempt provider_reference must encode sub + in_"
        );
        assert_eq!(
            count_renewal_attempts_by_sub(ctx, &realm_id, &stripe_subscription_id).await,
            1,
            "exactly one renewal attempt per renewal invoice"
        );

        // --- Invoice assertions (GAP2 + attribution) ---
        let invoice = find_invoice_with_attribution_and_urls(ctx, &realm_id, &stripe_invoice_id)
            .await
            .expect("renewal invoice should exist with external_invoice_id=in_R1");
        assert_eq!(
            invoice.1, "stripe",
            "renewal invoice provider must be stripe"
        );
        assert_eq!(invoice.2, "paid", "renewal invoice status must be paid");
        assert!(
            invoice.3.is_some(),
            "renewal invoice MUST carry subscription_id attribution (GAP2/G3)"
        );
        assert_eq!(
            invoice.4,
            Some(attempt.0),
            "renewal invoice payment_attempt_id must reference the renewal attempt"
        );
        assert_eq!(
            count_invoices_by_external_id(ctx, &realm_id, &stripe_invoice_id).await,
            1,
            "exactly one invoice per renewal invoice id"
        );
    }

    // =========================================================================
    // Test 2: duplicate same-invoice delivery is idempotent
    // =========================================================================

    /// User Story: US-PM-001, US-PM-003
    ///
    /// Given: An `invoice.payment_succeeded` renewal for `in_R1` has fired
    /// When: The same `in_R1` renewal event is re-delivered (different event_id)
    /// Then: The renewal attempt count stays at 1 (same id), the invoice row
    ///       count stays at 1, and attribution (subscription_id /
    ///       payment_attempt_id) is NOT cleared by the re-upsert.
    /// Why: Webhook delivery is at-least-once; a duplicate renewal must not
    ///      create a second attempt or invoice, nor must it blank the
    ///      attribution that the first delivery established.
    #[test_context(RenewalTestContext)]
    #[tokio::test]
    async fn test_stripe_renewal_duplicate_same_invoice_is_idempotent(
        ctx: &mut RenewalTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "test_stripe_renewal_idem_secret";
        let realm_id = ctx._realm_id.clone();
        let user_id = create_test_user(ctx, &realm_id, "stripe-renewal-idem@test.com").await;
        let entitlement_key = "stripe-renewal-idem";
        let external_product_id = format!("prod_stripe_{}", entitlement_key);
        let external_price_id = format!("price_stripe_{}", entitlement_key);
        let stripe_subscription_id = format!("sub_stripe_idem_{}", generate_test_event_id());
        let stripe_invoice_id = "in_R1".to_string();

        setup_stripe_config(ctx, &realm_id, "sk_test_key", webhook_secret).await;
        create_recurring_mapping(
            ctx,
            &realm_id,
            entitlement_key,
            &external_product_id,
            Some(&external_price_id),
        )
        .await;
        create_points_wallet(ctx, user_id, &realm_id).await;

        // Precondition (matches real Stripe flows): the subscription row must
        // exist before its first renewal `invoice.payment_succeeded`, since the
        // renewal handler recovers product/price from this row.
        seed_stripe_subscription_for_renewal(
            ctx,
            &realm_id,
            user_id,
            entitlement_key,
            &external_product_id,
            &external_price_id,
            &stripe_subscription_id,
        )
        .await;

        let build_payload = |event_id: String| {
            crate::tests::helpers::webhook_helpers::build_stripe_invoice_payment_succeeded_renewal(
                &event_id,
                &stripe_subscription_id,
                &stripe_invoice_id,
                &realm_id,
                user_id,
                entitlement_key,
                2000,
                None,
                None,
            )
        };

        // First delivery.
        let first_event = generate_test_event_id();
        let response = send_stripe_webhook_with_signature(
            &app,
            &realm_id,
            build_payload(first_event),
            webhook_secret,
        )
        .await;
        assert_webhook_success(&response);

        let expected_reference = format!(
            "stripe_renewal:{}:{}",
            stripe_subscription_id, stripe_invoice_id
        );
        let attempt_after_first =
            find_renewal_attempt_by_reference(ctx, &realm_id, &expected_reference)
                .await
                .expect("attempt must exist after first delivery");
        let invoice_after_first =
            find_invoice_with_attribution_and_urls(ctx, &realm_id, &stripe_invoice_id)
                .await
                .expect("invoice must exist after first delivery");
        assert_eq!(
            count_invoices_by_external_id(ctx, &realm_id, &stripe_invoice_id).await,
            1
        );

        // Re-deliver with a NEW event_id but the SAME in_.
        let second_event = generate_test_event_id();
        let response = send_stripe_webhook_with_signature(
            &app,
            &realm_id,
            build_payload(second_event),
            webhook_secret,
        )
        .await;
        assert_webhook_success(&response);

        // Idempotency: same attempt id, still exactly 1 row, still exactly 1
        // invoice, attribution NOT cleared.
        let attempt_after_second =
            find_renewal_attempt_by_reference(ctx, &realm_id, &expected_reference)
                .await
                .expect("attempt must still exist after re-delivery");
        assert_eq!(
            attempt_after_second.0, attempt_after_first.0,
            "duplicate delivery must resolve to the SAME attempt id (find-or-create)"
        );
        assert_eq!(
            count_renewal_attempts_by_sub(ctx, &realm_id, &stripe_subscription_id).await,
            1,
            "duplicate delivery must NOT create a second renewal attempt"
        );
        assert_eq!(
            count_invoices_by_external_id(ctx, &realm_id, &stripe_invoice_id).await,
            1,
            "duplicate delivery must NOT create a second invoice (upsert idempotency)"
        );

        let invoice_after_second =
            find_invoice_with_attribution_and_urls(ctx, &realm_id, &stripe_invoice_id)
                .await
                .expect("invoice must still exist after re-delivery");
        assert_eq!(
            invoice_after_second.3, invoice_after_first.3,
            "duplicate delivery must NOT clear subscription_id attribution"
        );
        assert_eq!(
            invoice_after_second.4,
            Some(attempt_after_first.0),
            "duplicate delivery must NOT clear payment_attempt_id attribution"
        );
    }

    // =========================================================================
    // Test 3 (§6.3 regression): invoice.finalized THEN invoice.payment_succeeded
    //   same in_ -> one row, attribution filled, hosted_url/pdf_url preserved
    // =========================================================================

    /// User Story: US-PM-003
    ///         §6.1 case 3 (coexistence -> one row + attribution filled),
    ///         §6.3 (REGRESSION: re-upsert must NOT regress hosted_url/pdf_url)
    ///
    /// Given: An `invoice.finalized` sync event already created the invoice row
    ///        for `in_R1` with hosted_url + pdf_url populated
    /// When: `invoice.payment_succeeded` for the same `in_R1` renewal arrives
    ///       and re-upserts the row to backfill attribution
    /// Then: There is still EXACTLY ONE invoice row for `in_R1`, its
    ///       `subscription_id` + `payment_attempt_id` are now non-empty, AND
    ///       `external_hosted_url` / `external_pdf_url` are byte-for-byte
    ///       unchanged from the values written by `invoice.finalized`.
    /// Why: The renewal re-upsert reuses the full field construction so that
    ///      the ON CONFLICT branches do not regress fields written by the
    ///      regressed, hosted/PDF links written by `invoice.finalized` would
    ///      silently disappear after the renewal event.
    #[test_context(RenewalTestContext)]
    #[tokio::test]
    async fn test_stripe_renewal_coexists_with_invoice_sync_row(ctx: &mut RenewalTestContext) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "test_stripe_renewal_coexist_secret";
        let realm_id = ctx._realm_id.clone();
        let user_id = create_test_user(ctx, &realm_id, "stripe-renewal-coexist@test.com").await;
        let entitlement_key = "stripe-renewal-coexist";
        let external_product_id = format!("prod_stripe_{}", entitlement_key);
        let external_price_id = format!("price_stripe_{}", entitlement_key);
        let stripe_subscription_id = format!("sub_stripe_coexist_{}", generate_test_event_id());
        let stripe_invoice_id = "in_R1".to_string();
        let hosted_url = "https://invoice.stripe.com/hosted/coexist_R1".to_string();
        let pdf_url = "https://invoice.stripe.com/pdf/coexist_R1".to_string();

        setup_stripe_config(ctx, &realm_id, "sk_test_key", webhook_secret).await;
        create_recurring_mapping(
            ctx,
            &realm_id,
            entitlement_key,
            &external_product_id,
            Some(&external_price_id),
        )
        .await;
        create_points_wallet(ctx, user_id, &realm_id).await;

        // Precondition (matches real Stripe flows): the subscription row must
        // exist before its first renewal `invoice.payment_succeeded`, since the
        // renewal handler recovers product/price from this row.
        seed_stripe_subscription_for_renewal(
            ctx,
            &realm_id,
            user_id,
            entitlement_key,
            &external_product_id,
            &external_price_id,
            &stripe_subscription_id,
        )
        .await;

        // Step 1: invoice.finalized sync event establishes the row with
        // hosted_url + pdf_url (the §6.3 "written by invoice.*" baseline).
        let finalized_payload = build_stripe_invoice_sync_event(
            "invoice.finalized",
            &stripe_invoice_id,
            &realm_id,
            2000,
            "open",
            Some(&hosted_url),
            Some(&pdf_url),
        );
        let response =
            send_stripe_webhook_with_signature(&app, &realm_id, finalized_payload, webhook_secret)
                .await;
        assert_eq!(response.status(), StatusCode::OK);

        let invoice_after_finalized =
            find_invoice_with_attribution_and_urls(ctx, &realm_id, &stripe_invoice_id)
                .await
                .expect("invoice row must exist after invoice.finalized");
        // Baseline: sync path does not yet know attribution for a renewal
        // (payment_attempt is created by invoice.payment_succeeded).
        let hosted_after_finalized = invoice_after_finalized.5.clone();
        let pdf_after_finalized = invoice_after_finalized.6.clone();
        assert_eq!(hosted_after_finalized.as_deref(), Some(hosted_url.as_str()));
        assert_eq!(pdf_after_finalized.as_deref(), Some(pdf_url.as_str()));

        // Step 2: invoice.payment_succeeded renewal for the SAME in_ ->
        // re-upsert backfills attribution.
        let event_id = generate_test_event_id();
        let payload =
            crate::tests::helpers::webhook_helpers::build_stripe_invoice_payment_succeeded_renewal(
                &event_id,
                &stripe_subscription_id,
                &stripe_invoice_id,
                &realm_id,
                user_id,
                entitlement_key,
                2000,
                None,
                None,
            );
        let response =
            send_stripe_webhook_with_signature(&app, &realm_id, payload, webhook_secret).await;
        assert_webhook_success(&response);

        // --- One row for in_R1 (coexistence -> single row) ---
        assert_eq!(
            count_invoices_by_external_id(ctx, &realm_id, &stripe_invoice_id).await,
            1,
            "invoice.finalized + invoice.payment_succeeded for the same in_ must yield exactly ONE row"
        );

        let invoice_after_renewal =
            find_invoice_with_attribution_and_urls(ctx, &realm_id, &stripe_invoice_id)
                .await
                .expect("invoice row must still exist after renewal re-upsert");

        // --- Attribution backfilled (GAP2/G3) ---
        assert!(
            invoice_after_renewal.3.is_some(),
            "renewal re-upsert must fill subscription_id on the coexisting row"
        );
        assert!(
            invoice_after_renewal.4.is_some(),
            "renewal re-upsert must fill payment_attempt_id on the coexisting row"
        );

        // --- §6.3 regression core assertion: urls UNCHANGED (per-field) ---
        assert_eq!(
            invoice_after_renewal.5, hosted_after_finalized,
            "§6.3 REGRESSION: external_hosted_url must NOT regress after renewal re-upsert"
        );
        assert_eq!(
            invoice_after_renewal.6, pdf_after_finalized,
            "§6.3 REGRESSION: external_pdf_url must NOT regress after renewal re-upsert"
        );
        // Strong per-field equality against the exact values written by
        // invoice.finalized (not just "non-empty").
        assert_eq!(
            invoice_after_renewal.5.as_deref(),
            Some(hosted_url.as_str()),
            "§6.3 REGRESSION: external_hosted_url must equal the invoice.finalized value verbatim"
        );
        assert_eq!(
            invoice_after_renewal.6.as_deref(),
            Some(pdf_url.as_str()),
            "§6.3 REGRESSION: external_pdf_url must equal the invoice.finalized value verbatim"
        );
    }

    // =========================================================================
    // Test 4: payment_succeeded without prior invoice.* still creates the row
    // =========================================================================

    /// User Story: US-PM-001, US-PM-003
    ///         §6.3 (re-upsert uses full field construction on INSERT too)
    ///
    /// Given: NO prior `invoice.*` sync event for `in_R1`
    /// When: `invoice.payment_succeeded` for the renewal arrives
    /// Then: The invoice row is created (re-upsert INSERT branch) with
    ///       `external_invoice_id=in_R1`, `provider=stripe`, and BOTH
    ///       `subscription_id` and `payment_attempt_id` non-empty.
    /// Why: Stripe event ordering is not guaranteed -- the renewal event may
    ///      arrive before any `invoice.*` sync event. The re-upsert must
    ///      create the row with full attribution rather than no-op, otherwise
    ///      the renewal would be recorded as a payment_attempt with NO
    ///      attributable invoice.
    #[test_context(RenewalTestContext)]
    #[tokio::test]
    async fn test_stripe_renewal_payment_succeeded_without_prior_invoice_sync_creates_invoice(
        ctx: &mut RenewalTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "test_stripe_renewal_no_sync_secret";
        let realm_id = ctx._realm_id.clone();
        let user_id = create_test_user(ctx, &realm_id, "stripe-renewal-nosync@test.com").await;
        let entitlement_key = "stripe-renewal-nosync";
        let external_product_id = format!("prod_stripe_{}", entitlement_key);
        let external_price_id = format!("price_stripe_{}", entitlement_key);
        let stripe_subscription_id = format!("sub_stripe_nosync_{}", generate_test_event_id());
        // Distinct in_ to prove no pre-existing row is being relied upon.
        let stripe_invoice_id = format!("in_nosync_{}", Uuid::now_v7());

        setup_stripe_config(ctx, &realm_id, "sk_test_key", webhook_secret).await;
        create_recurring_mapping(
            ctx,
            &realm_id,
            entitlement_key,
            &external_product_id,
            Some(&external_price_id),
        )
        .await;
        create_points_wallet(ctx, user_id, &realm_id).await;

        // Precondition (matches real Stripe flows): the subscription row must
        // exist before its first renewal `invoice.payment_succeeded`, since the
        // renewal handler recovers product/price from this row.
        seed_stripe_subscription_for_renewal(
            ctx,
            &realm_id,
            user_id,
            entitlement_key,
            &external_product_id,
            &external_price_id,
            &stripe_subscription_id,
        )
        .await;

        // Precondition: no row for this in_.
        assert_eq!(
            count_invoices_by_external_id(ctx, &realm_id, &stripe_invoice_id).await,
            0,
            "precondition: no invoice row for this in_"
        );

        let event_id = generate_test_event_id();
        let payload =
            crate::tests::helpers::webhook_helpers::build_stripe_invoice_payment_succeeded_renewal(
                &event_id,
                &stripe_subscription_id,
                &stripe_invoice_id,
                &realm_id,
                user_id,
                entitlement_key,
                2000,
                None,
                None,
            );
        let response =
            send_stripe_webhook_with_signature(&app, &realm_id, payload, webhook_secret).await;
        assert_webhook_success(&response);

        // INSERT path: row created, attribution non-empty.
        let invoice = find_invoice_with_attribution_and_urls(ctx, &realm_id, &stripe_invoice_id)
            .await
            .expect("invoice row must be created by payment_succeeded re-upsert (INSERT path)");
        assert_eq!(invoice.1, "stripe", "provider must be stripe");
        assert!(
            invoice.3.is_some(),
            "re-upsert INSERT path must fill subscription_id"
        );
        assert!(
            invoice.4.is_some(),
            "re-upsert INSERT path must fill payment_attempt_id"
        );
    }

    // =========================================================================
    // Test 5: zero-yuan cycle skips renewal attempt + invoice attribution
    // =========================================================================

    /// User Story: US-PM-001, US-PM-003
    ///
    /// Given: An `invoice.payment_succeeded` renewal with `total=0`
    ///        (100%-off / free-tier cycle)
    /// When: The event is processed
    /// Then: No renewal `payment_attempt` is created (DB CHECK `amount > 0`
    ///       forbids a zero row anyway) and no renewal invoice attribution is
    ///       written, while the subscription cycle / points grant still runs.
    /// Why: A zero-yuan cycle is not a real charge, so it must NOT inflate the
    ///      payment ledger with a Succeeded 0-amount attempt (which the DB
    ///      rejects anyway) and must NOT produce a 0-amount attributed invoice.
    ///      Points / cycle advancement are orthogonal and still proceed.
    #[test_context(RenewalTestContext)]
    #[tokio::test]
    async fn test_stripe_renewal_zero_amount_skips_attempt_and_invoice(
        ctx: &mut RenewalTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "test_stripe_renewal_zero_secret";
        let realm_id = ctx._realm_id.clone();
        let user_id = create_test_user(ctx, &realm_id, "stripe-renewal-zero@test.com").await;
        let entitlement_key = "stripe-renewal-zero";
        let external_product_id = format!("prod_stripe_{}", entitlement_key);
        let external_price_id = format!("price_stripe_{}", entitlement_key);
        let stripe_subscription_id = format!("sub_stripe_zero_{}", generate_test_event_id());
        let stripe_invoice_id = "in_zero".to_string();

        setup_stripe_config(ctx, &realm_id, "sk_test_key", webhook_secret).await;
        create_recurring_mapping(
            ctx,
            &realm_id,
            entitlement_key,
            &external_product_id,
            Some(&external_price_id),
        )
        .await;
        create_points_wallet(ctx, user_id, &realm_id).await;

        // Precondition (matches real Stripe flows): the subscription row must
        // exist before its first renewal `invoice.payment_succeeded`, since the
        // renewal handler recovers product/price from this row.
        seed_stripe_subscription_for_renewal(
            ctx,
            &realm_id,
            user_id,
            entitlement_key,
            &external_product_id,
            &external_price_id,
            &stripe_subscription_id,
        )
        .await;

        let event_id = generate_test_event_id();
        let payload =
            crate::tests::helpers::webhook_helpers::build_stripe_invoice_payment_succeeded_renewal(
                &event_id,
                &stripe_subscription_id,
                &stripe_invoice_id,
                &realm_id,
                user_id,
                entitlement_key,
                0, // zero-yuan cycle
                None,
                None,
            );
        let response =
            send_stripe_webhook_with_signature(&app, &realm_id, payload, webhook_secret).await;
        assert_webhook_success(&response);

        // Zero-yuan cycle: no renewal attempt.
        assert_eq!(
            count_renewal_attempts_by_sub(ctx, &realm_id, &stripe_subscription_id).await,
            0,
            "zero-yuan renewal must NOT create a payment_attempt (amount > 0 CHECK)"
        );
        // Zero-yuan cycle: no renewal invoice attribution row was created by
        // the renewal branch. (The renewal branch skips the upsert entirely on
        // total==0.)
        let invoice =
            find_invoice_with_attribution_and_urls(ctx, &realm_id, &stripe_invoice_id).await;
        assert!(
            invoice.is_none(),
            "zero-yuan renewal must NOT create an attributed invoice row (got {:?})",
            invoice
        );
    }
}
