// =============================================================================
// Stripe credit_note.created Webhook Scenario Tests
// =============================================================================
//
// Tests for the Stripe `credit_note.created` webhook handler covering:
//   - Normal sync (US-IF-007 scenario 1)
//   - Idempotency (US-IF-007 scenario 5)
//   - Missing invoice warn+skip (US-IF-007 scenario 6, edge case)
//   - Provider mismatch warn+skip (US-IF-007 edge case)
//   - Partial refund accumulation (US-IF-007 scenario 2)
//   - Invoice status unchanged (US-IF-007 scenario 3)
//   - No point clawback side effect (US-IF-007 scenario 4)
//
// Refund visibility on the invoice detail response is populated by dev item
// BE-D06 (amountRefunded / amountRemaining / creditNotes fields); the underlying
// invoice refund totals are written by the handler in dev item BE-D04.
//
// User Story: docs/user-stories/billing/invoice-fallback.md
// Covers: US-IF-007 scenarios 1-6, US-IF-008 scenario 1
//
// =============================================================================

#[cfg(test)]
#[allow(dead_code)]
mod tests {
    use crate::tests::helpers::billing_helpers::setup_stripe_config;
    use crate::tests::schema_test_context::SchemaTestContext;
    use axum::http::StatusCode;
    use serde_json::json;
    use test_context::test_context;
    use uuid::Uuid;

    use SchemaTestContext as CreditNoteWebhookTestContext;

    // ---------------------------------------------------------------------------
    // Helpers
    // ---------------------------------------------------------------------------

    /// Build a Stripe `credit_note.created` webhook event payload.
    ///
    /// The amount is carried under `data.object.total` to match the production
    /// payload parser (`parse_credit_note_created_payload`), which reads
    /// `object["total"]` as the credit note amount in the smallest currency unit.
    /// The `metadata.realmId` field follows the realm routing convention used
    /// by other Stripe webhook scenarios.
    fn build_stripe_credit_note_event(
        stripe_credit_note_id: &str,
        stripe_invoice_id: &str,
        realm_id: &str,
        amount: i64,
        currency: &str,
    ) -> serde_json::Value {
        json!({
            "id": format!("evt_{}", Uuid::now_v7()),
            "object": "event",
            "type": "credit_note.created",
            "api_version": "2020-08-27",
            "created": chrono::Utc::now().timestamp(),
            "data": {
                "object": {
                    "id": stripe_credit_note_id,
                    "object": "credit_note",
                    "invoice": stripe_invoice_id,
                    "total": amount,
                    "currency": currency,
                    "metadata": {
                        "realmId": realm_id
                    }
                }
            }
        })
    }

    /// Insert a Stripe-provider paid invoice directly via SQL.
    ///
    /// Sets provider=stripe, status=paid, and external_invoice_id so the
    /// credit_note.created handler can resolve it via `find_by_external_invoice_id`.
    /// The invoice.source column is constrained to
    /// ('admin_manual', 'user_application', 'external_sync'); Stripe-synced
    /// invoices use 'external_sync' (the provider column carries 'stripe').
    async fn insert_stripe_paid_invoice(
        ctx: &CreditNoteWebhookTestContext,
        realm_id: &str,
        external_invoice_id: &str,
        total: i64,
    ) -> Uuid {
        let invoice_id = Uuid::new_v4();
        let invoice_number = format!("STR-{}", Uuid::now_v7().simple());
        let account_id = Uuid::new_v4();

        sqlx::query(
            "INSERT INTO invoice (
                id, realm_id, invoice_number, source, account_id,
                status, currency, subtotal, total,
                amount_refunded, amount_remaining,
                billing_name, seller_name,
                billing_address, seller_address,
                provider, payment_provider, external_invoice_id,
                due_date, paid_at, issued_at
            ) VALUES (
                $1, $2, $3, 'external_sync', $4,
                'paid', 'usd', $5, $5,
                0, $5,
                'Test Billing', 'Test Seller',
                NULL, NULL,
                'stripe', 'stripe', $6,
                NULL, NOW(), NOW()
            )",
        )
        .bind(invoice_id)
        .bind(realm_id)
        .bind(&invoice_number)
        .bind(account_id)
        .bind(total)
        .bind(external_invoice_id)
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to insert Stripe paid invoice");

        invoice_id
    }

    /// Insert a manual-provider paid invoice directly via SQL.
    ///
    /// Used by the provider-mismatch scenario where the credit_note.created
    /// webhook references an invoice that is NOT provider=stripe.
    async fn insert_manual_paid_invoice(
        ctx: &CreditNoteWebhookTestContext,
        realm_id: &str,
        external_invoice_id: &str,
        total: i64,
    ) -> Uuid {
        let invoice_id = Uuid::new_v4();
        let invoice_number = format!("MAN-{}", Uuid::now_v7().simple());
        let account_id = Uuid::new_v4();

        sqlx::query(
            "INSERT INTO invoice (
                id, realm_id, invoice_number, source, account_id,
                status, currency, subtotal, total,
                amount_refunded, amount_remaining,
                billing_name, seller_name,
                billing_address, seller_address,
                provider, payment_provider, external_invoice_id,
                due_date, paid_at, issued_at
            ) VALUES (
                $1, $2, $3, 'admin_manual', $4,
                'paid', 'usd', $5, $5,
                0, $5,
                'Test Billing', 'Test Seller',
                NULL, NULL,
                'manual', NULL, $6,
                NULL, NOW(), NOW()
            )",
        )
        .bind(invoice_id)
        .bind(realm_id)
        .bind(&invoice_number)
        .bind(account_id)
        .bind(total)
        .bind(external_invoice_id)
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to insert manual paid invoice");

        invoice_id
    }

    /// Return (amount_refunded, amount_remaining) for the given invoice.
    async fn get_invoice_refund_fields(
        ctx: &CreditNoteWebhookTestContext,
        invoice_id: Uuid,
    ) -> (i64, i64) {
        let row: (i64, i64) =
            sqlx::query_as("SELECT amount_refunded, amount_remaining FROM invoice WHERE id = $1")
                .bind(invoice_id)
                .fetch_one(&ctx.app_state.pool)
                .await
                .unwrap();
        row
    }

    /// Count credit notes matching the given external_credit_note_id.
    async fn count_credit_notes_by_external_id(
        ctx: &CreditNoteWebhookTestContext,
        external_credit_note_id: &str,
    ) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM credit_note WHERE external_credit_note_id = $1")
            .bind(external_credit_note_id)
            .fetch_one(&ctx.app_state.pool)
            .await
            .unwrap()
    }

    /// Count all credit notes for the given invoice.
    async fn count_credit_notes_for_invoice(
        ctx: &CreditNoteWebhookTestContext,
        invoice_id: Uuid,
    ) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM credit_note WHERE invoice_id = $1")
            .bind(invoice_id)
            .fetch_one(&ctx.app_state.pool)
            .await
            .unwrap()
    }

    /// Count payment_event rows with event_type = 'charge.refunded'.
    ///
    /// Used to verify that processing a credit_note.created webhook does not
    /// accidentally trigger a charge.refunded side effect (point clawback is
    /// driven by a separate charge.refunded webhook, not credit_note.created).
    async fn count_charge_refunded_events(ctx: &CreditNoteWebhookTestContext) -> i64 {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM payment_event WHERE event_type = 'charge.refunded'",
        )
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap()
    }

    /// Return the invoice status for the given invoice id.
    async fn get_invoice_status(ctx: &CreditNoteWebhookTestContext, invoice_id: Uuid) -> String {
        sqlx::query_scalar("SELECT status FROM invoice WHERE id = $1")
            .bind(invoice_id)
            .fetch_one(&ctx.app_state.pool)
            .await
            .unwrap()
    }

    // =========================================================================
    // Test: Normal Sync (US-IF-007 scenario 1)
    // =========================================================================
    // User Story: docs/user-stories/billing/invoice-fallback.md
    // Covers: US-IF-007 scenario 1, US-IF-008 scenario 1
    //
    // Given: A Stripe paid invoice in DB, total = 10000
    // When: Send credit_note.created webhook with amount = 3000, currency = "usd"
    // Then: Webhook returns 200
    // And: A credit note record exists with source = "stripe" and the
    //      external_credit_note_id set
    // And: Invoice amount_refunded = 3000, amount_remaining = 7000

    #[test_context(CreditNoteWebhookTestContext)]
    #[tokio::test]
    async fn test_stripe_credit_note_created_syncs_to_herald(
        ctx: &mut CreditNoteWebhookTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_test_cn_sync";
        let realm_id = ctx._realm_id.clone();
        let stripe_invoice_id = format!("in_test_cn_sync_{}", Uuid::now_v7());
        let stripe_credit_note_id = format!("cn_test_sync_{}", Uuid::now_v7());

        setup_stripe_config(ctx, &realm_id, "sk_test_key", webhook_secret).await;

        // Given: a Stripe paid invoice with total = 10000
        let invoice_id =
            insert_stripe_paid_invoice(ctx, &realm_id, &stripe_invoice_id, 10000).await;

        // When: send credit_note.created with amount = 3000
        let payload = build_stripe_credit_note_event(
            &stripe_credit_note_id,
            &stripe_invoice_id,
            &realm_id,
            3000,
            "usd",
        );

        let response = crate::tests::helpers::webhook_helpers::send_stripe_webhook_with_signature(
            &app,
            &realm_id,
            payload,
            webhook_secret,
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);

        // Then: a credit note record exists for the external_credit_note_id
        assert_eq!(
            count_credit_notes_by_external_id(ctx, &stripe_credit_note_id).await,
            1,
            "Expected exactly one credit note for the external_credit_note_id"
        );

        // And: the credit note source is stripe
        let source: String =
            sqlx::query_scalar("SELECT source FROM credit_note WHERE external_credit_note_id = $1")
                .bind(&stripe_credit_note_id)
                .fetch_one(&ctx.app_state.pool)
                .await
                .unwrap();
        assert_eq!(source, "stripe", "credit note source should be stripe");

        // And: invoice refund totals updated correctly
        let (amount_refunded, amount_remaining) = get_invoice_refund_fields(ctx, invoice_id).await;
        assert_eq!(
            amount_refunded, 3000,
            "amount_refunded should be 3000 after a 3000 refund"
        );
        assert_eq!(
            amount_remaining, 7000,
            "amount_remaining should be 7000 (= 10000 - 3000)"
        );
    }

    // =========================================================================
    // Test: Idempotency (US-IF-007 scenario 5)
    // =========================================================================
    // User Story: docs/user-stories/billing/invoice-fallback.md
    // Covers: US-IF-007 scenario 5
    //
    // Given: A Stripe paid invoice, already processed one credit_note.created
    // When: Send the same credit_note.created event again (same stripe_credit_note_id)
    // Then: Webhook returns 200
    // And: Still only 1 credit note record for that external_credit_note_id
    // And: amount_refunded unchanged (not double-counted)

    #[test_context(CreditNoteWebhookTestContext)]
    #[tokio::test]
    async fn test_stripe_credit_note_idempotent(ctx: &mut CreditNoteWebhookTestContext) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_test_cn_idem";
        let realm_id = ctx._realm_id.clone();
        let stripe_invoice_id = format!("in_test_cn_idem_{}", Uuid::now_v7());
        let stripe_credit_note_id = format!("cn_test_idem_{}", Uuid::now_v7());

        setup_stripe_config(ctx, &realm_id, "sk_test_key", webhook_secret).await;

        let invoice_id =
            insert_stripe_paid_invoice(ctx, &realm_id, &stripe_invoice_id, 10000).await;

        // First send: normal processing
        let payload1 = build_stripe_credit_note_event(
            &stripe_credit_note_id,
            &stripe_invoice_id,
            &realm_id,
            3000,
            "usd",
        );

        let response1 = crate::tests::helpers::webhook_helpers::send_stripe_webhook_with_signature(
            &app,
            &realm_id,
            payload1,
            webhook_secret,
        )
        .await;
        assert_eq!(response1.status(), StatusCode::OK);

        // Verify one record and amount_refunded = 3000
        assert_eq!(
            count_credit_notes_by_external_id(ctx, &stripe_credit_note_id).await,
            1
        );
        let (refunded_after_first, _) = get_invoice_refund_fields(ctx, invoice_id).await;
        assert_eq!(refunded_after_first, 3000);

        // Second send: same stripe_credit_note_id (idempotency)
        let payload2 = build_stripe_credit_note_event(
            &stripe_credit_note_id,
            &stripe_invoice_id,
            &realm_id,
            3000,
            "usd",
        );

        let response2 = crate::tests::helpers::webhook_helpers::send_stripe_webhook_with_signature(
            &app,
            &realm_id,
            payload2,
            webhook_secret,
        )
        .await;
        assert_eq!(response2.status(), StatusCode::OK);

        // Then: still only 1 record and amount_refunded unchanged
        assert_eq!(
            count_credit_notes_by_external_id(ctx, &stripe_credit_note_id).await,
            1,
            "Idempotent send must not create a second credit note"
        );

        let (refunded_after_second, _) = get_invoice_refund_fields(ctx, invoice_id).await;
        assert_eq!(
            refunded_after_second, 3000,
            "amount_refunded must not be double-counted on idempotent send"
        );
    }

    // =========================================================================
    // Test: Missing Invoice Warn + Skip (US-IF-007 scenario 6, edge case)
    // =========================================================================
    // User Story: docs/user-stories/billing/invoice-fallback.md
    // Covers: US-IF-007 scenario 6 (edge case)
    //
    // Given: No matching invoice for the stripe_invoice_id in the webhook
    // When: Send credit_note.created webhook
    // Then: Webhook returns 200 (does not fail)
    // And: No credit note record created

    #[test_context(CreditNoteWebhookTestContext)]
    #[tokio::test]
    async fn test_stripe_credit_note_missing_invoice_warns_and_skips(
        ctx: &mut CreditNoteWebhookTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_test_cn_missing";
        let realm_id = ctx._realm_id.clone();
        // No invoice inserted — this stripe_invoice_id has no match in DB.
        let stripe_invoice_id = format!("in_test_cn_missing_{}", Uuid::now_v7());
        let stripe_credit_note_id = format!("cn_test_missing_{}", Uuid::now_v7());

        setup_stripe_config(ctx, &realm_id, "sk_test_key", webhook_secret).await;

        let payload = build_stripe_credit_note_event(
            &stripe_credit_note_id,
            &stripe_invoice_id,
            &realm_id,
            3000,
            "usd",
        );

        let response = crate::tests::helpers::webhook_helpers::send_stripe_webhook_with_signature(
            &app,
            &realm_id,
            payload,
            webhook_secret,
        )
        .await;

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Missing invoice must not fail the webhook (warn + skip)"
        );

        // And: no credit note record was created
        assert_eq!(
            count_credit_notes_by_external_id(ctx, &stripe_credit_note_id).await,
            0,
            "No credit note should be created when the invoice is missing"
        );
    }

    // =========================================================================
    // Test: Provider Mismatch Warn + Skip (US-IF-007 edge case)
    // =========================================================================
    // User Story: docs/user-stories/billing/invoice-fallback.md
    // Covers: US-IF-007 edge case -- provider mismatch
    //
    // Given: An invoice with provider = manual, status = paid (not stripe),
    //        and external_invoice_id matching the value in the webhook
    // When: Send credit_note.created webhook referencing that invoice's ID
    // Then: Webhook returns 200
    // And: No credit note record created for that invoice

    #[test_context(CreditNoteWebhookTestContext)]
    #[tokio::test]
    async fn test_stripe_credit_note_provider_mismatch_warns_and_skips(
        ctx: &mut CreditNoteWebhookTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_test_cn_mismatch";
        let realm_id = ctx._realm_id.clone();
        let external_invoice_id = format!("in_test_cn_mismatch_{}", Uuid::now_v7());
        let stripe_credit_note_id = format!("cn_test_mismatch_{}", Uuid::now_v7());

        setup_stripe_config(ctx, &realm_id, "sk_test_key", webhook_secret).await;

        // Given: a manual-provider paid invoice with external_invoice_id set
        let invoice_id =
            insert_manual_paid_invoice(ctx, &realm_id, &external_invoice_id, 10000).await;

        let payload = build_stripe_credit_note_event(
            &stripe_credit_note_id,
            &external_invoice_id,
            &realm_id,
            3000,
            "usd",
        );

        let response = crate::tests::helpers::webhook_helpers::send_stripe_webhook_with_signature(
            &app,
            &realm_id,
            payload,
            webhook_secret,
        )
        .await;

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Provider mismatch must not fail the webhook (warn + skip)"
        );

        // And: no credit note created for this invoice
        assert_eq!(
            count_credit_notes_for_invoice(ctx, invoice_id).await,
            0,
            "No credit note should be created when invoice provider is not stripe"
        );
        assert_eq!(
            count_credit_notes_by_external_id(ctx, &stripe_credit_note_id).await,
            0,
            "No credit note should be created for the external_credit_note_id"
        );
    }

    // =========================================================================
    // Test: Partial Refunds Accumulate (US-IF-007 scenario 2)
    // =========================================================================
    // User Story: docs/user-stories/billing/invoice-fallback.md
    // Covers: US-IF-007 scenario 2
    //
    // Given: A Stripe paid invoice, total = 10000
    // When: Send first credit_note.created with amount = 3000, then second
    //       with amount = 2000 (different stripe_credit_note_id)
    // Then: Both return 200
    // And: Two credit note records exist
    // And: Invoice amount_refunded = 5000, amount_remaining = 5000

    #[test_context(CreditNoteWebhookTestContext)]
    #[tokio::test]
    async fn test_stripe_credit_note_partial_refunds_accumulate(
        ctx: &mut CreditNoteWebhookTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_test_cn_partial";
        let realm_id = ctx._realm_id.clone();
        let stripe_invoice_id = format!("in_test_cn_partial_{}", Uuid::now_v7());
        let stripe_credit_note_id_1 = format!("cn_test_partial_1_{}", Uuid::now_v7());
        let stripe_credit_note_id_2 = format!("cn_test_partial_2_{}", Uuid::now_v7());

        setup_stripe_config(ctx, &realm_id, "sk_test_key", webhook_secret).await;

        let invoice_id =
            insert_stripe_paid_invoice(ctx, &realm_id, &stripe_invoice_id, 10000).await;

        // First credit note: 3000
        let payload1 = build_stripe_credit_note_event(
            &stripe_credit_note_id_1,
            &stripe_invoice_id,
            &realm_id,
            3000,
            "usd",
        );

        let response1 = crate::tests::helpers::webhook_helpers::send_stripe_webhook_with_signature(
            &app,
            &realm_id,
            payload1,
            webhook_secret,
        )
        .await;
        assert_eq!(response1.status(), StatusCode::OK);

        // Second credit note: 2000 (different stripe_credit_note_id)
        let payload2 = build_stripe_credit_note_event(
            &stripe_credit_note_id_2,
            &stripe_invoice_id,
            &realm_id,
            2000,
            "usd",
        );

        let response2 = crate::tests::helpers::webhook_helpers::send_stripe_webhook_with_signature(
            &app,
            &realm_id,
            payload2,
            webhook_secret,
        )
        .await;
        assert_eq!(response2.status(), StatusCode::OK);

        // Then: two credit note records exist on this invoice
        assert_eq!(
            count_credit_notes_for_invoice(ctx, invoice_id).await,
            2,
            "Expected two credit notes after two partial refund webhooks"
        );

        // And: invoice refund totals accumulated
        let (amount_refunded, amount_remaining) = get_invoice_refund_fields(ctx, invoice_id).await;
        assert_eq!(
            amount_refunded, 5000,
            "amount_refunded should accumulate to 5000"
        );
        assert_eq!(
            amount_remaining, 5000,
            "amount_remaining should be 5000 (= 10000 - 5000)"
        );
    }

    // =========================================================================
    // Test: Invoice Status Unchanged (US-IF-007 scenario 3)
    // =========================================================================
    // User Story: docs/user-stories/billing/invoice-fallback.md
    // Covers: US-IF-007 scenario 3
    //
    // Given: A Stripe paid invoice (status = paid)
    // When: Send credit_note.created webhook
    // Then: Invoice status remains "paid"

    #[test_context(CreditNoteWebhookTestContext)]
    #[tokio::test]
    async fn test_stripe_credit_note_does_not_change_invoice_status(
        ctx: &mut CreditNoteWebhookTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_test_cn_status";
        let realm_id = ctx._realm_id.clone();
        let stripe_invoice_id = format!("in_test_cn_status_{}", Uuid::now_v7());
        let stripe_credit_note_id = format!("cn_test_status_{}", Uuid::now_v7());

        setup_stripe_config(ctx, &realm_id, "sk_test_key", webhook_secret).await;

        let invoice_id =
            insert_stripe_paid_invoice(ctx, &realm_id, &stripe_invoice_id, 10000).await;

        // Sanity: invoice starts as paid
        assert_eq!(get_invoice_status(ctx, invoice_id).await, "paid");

        let payload = build_stripe_credit_note_event(
            &stripe_credit_note_id,
            &stripe_invoice_id,
            &realm_id,
            3000,
            "usd",
        );

        let response = crate::tests::helpers::webhook_helpers::send_stripe_webhook_with_signature(
            &app,
            &realm_id,
            payload,
            webhook_secret,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);

        // Then: invoice status remains paid
        assert_eq!(
            get_invoice_status(ctx, invoice_id).await,
            "paid",
            "credit_note.created must not change invoice status"
        );
    }

    // =========================================================================
    // Test: No Point Clawback Side Effect (US-IF-007 scenario 4)
    // =========================================================================
    // User Story: docs/user-stories/billing/invoice-fallback.md
    // Covers: US-IF-007 scenario 4
    //
    // Given: A Stripe paid invoice
    // When: Send credit_note.created webhook
    // Then: Credit note is created successfully
    // And: No new payment_event record with event_type = 'charge.refunded'
    //      was created as a side effect of processing this credit_note.created
    //      event (the count before and after is identical).
    //
    // This validates that credit_note.created processing is independent from
    // charge.refunded and does not duplicate point clawback.

    #[test_context(CreditNoteWebhookTestContext)]
    #[tokio::test]
    async fn test_stripe_credit_note_does_not_trigger_point_clawback(
        ctx: &mut CreditNoteWebhookTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_test_cn_no_clawback";
        let realm_id = ctx._realm_id.clone();
        let stripe_invoice_id = format!("in_test_cn_no_clawback_{}", Uuid::now_v7());
        let stripe_credit_note_id = format!("cn_test_no_clawback_{}", Uuid::now_v7());

        setup_stripe_config(ctx, &realm_id, "sk_test_key", webhook_secret).await;

        let _invoice_id =
            insert_stripe_paid_invoice(ctx, &realm_id, &stripe_invoice_id, 10000).await;

        // Capture charge.refunded event count BEFORE the webhook
        let charge_refunded_before = count_charge_refunded_events(ctx).await;

        let payload = build_stripe_credit_note_event(
            &stripe_credit_note_id,
            &stripe_invoice_id,
            &realm_id,
            3000,
            "usd",
        );

        let response = crate::tests::helpers::webhook_helpers::send_stripe_webhook_with_signature(
            &app,
            &realm_id,
            payload,
            webhook_secret,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);

        // Then: credit note is created successfully
        assert_eq!(
            count_credit_notes_by_external_id(ctx, &stripe_credit_note_id).await,
            1,
            "Credit note should be created successfully"
        );

        // And: no new charge.refunded payment_event was created as a side effect
        let charge_refunded_after = count_charge_refunded_events(ctx).await;
        assert_eq!(
            charge_refunded_after, charge_refunded_before,
            "credit_note.created must not trigger a charge.refunded payment_event side effect"
        );
    }
}
