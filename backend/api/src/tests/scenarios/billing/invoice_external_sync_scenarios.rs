// =============================================================================
// External Invoice Sync Scenario Tests
// =============================================================================
//
// Tests for Stripe invoice.* webhook sync and Creem checkout.completed
// invoice sync, covering status mapping, external data recording, and
// idempotent upsert behavior.
//
// User Story: docs/user-stories/billing/invoice-fallback.md
// Covers: US-IF-002 (Stripe sync), US-IF-003 (Creem sync)
//
// =============================================================================

#[cfg(test)]
#[allow(dead_code)]
mod tests {
    use crate::tests::helpers::billing_helpers::setup_stripe_config;
    use crate::tests::helpers::webhook_helpers::{
        generate_test_event_id, send_stripe_webhook_with_signature,
    };
    use crate::tests::schema_test_context::SchemaTestContext;
    use axum::http::StatusCode;
    use serde_json::json;
    use test_context::test_context;
    use uuid::Uuid;

    use SchemaTestContext as SyncTestContext;

    // ---------------------------------------------------------------------------
    // Helpers
    // ---------------------------------------------------------------------------

    /// Set Creem webhook secret for a realm in the database.
    async fn set_creem_webhook_secret(ctx: &SyncTestContext, webhook_secret: &str) {
        ctx.with_creem_config(
            &ctx._realm_id,
            Some("test_api_key"),
            Some(webhook_secret),
            Some(30),
        )
        .await;
    }

    /// Build a Stripe invoice.* webhook event payload.
    ///
    /// Constructs a minimal but valid Stripe event with the given invoice data.
    /// The `event_type` should be one of: invoice.created, invoice.finalized,
    /// invoice.paid, invoice.voided.
    fn build_stripe_invoice_event(
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

    /// Find an invoice by external_invoice_id in the database.
    async fn find_invoice_by_external_id(
        ctx: &SyncTestContext,
        realm_id: &str,
        external_invoice_id: &str,
    ) -> Option<(Uuid, String, String, Option<String>, Option<String>)> {
        sqlx::query_as(
            "SELECT id, provider, status, external_hosted_url, external_pdf_url
             FROM invoice
             WHERE realm_id = $1 AND external_invoice_id = $2",
        )
        .bind(realm_id)
        .bind(external_invoice_id)
        .fetch_optional(&ctx.app_state.pool)
        .await
        .unwrap()
    }

    /// Find an invoice by external_order_id in the database.
    async fn find_invoice_by_external_order_id(
        ctx: &SyncTestContext,
        realm_id: &str,
        external_order_id: &str,
    ) -> Option<(Uuid, String, String)> {
        sqlx::query_as(
            "SELECT id, provider, status
             FROM invoice
             WHERE realm_id = $1 AND external_order_id = $2",
        )
        .bind(realm_id)
        .bind(external_order_id)
        .fetch_optional(&ctx.app_state.pool)
        .await
        .unwrap()
    }

    /// Count invoices matching a given external_invoice_id.
    async fn count_invoices_by_external_id(
        ctx: &SyncTestContext,
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

    /// Count invoices matching a given external_order_id.
    async fn count_invoices_by_external_order_id(
        ctx: &SyncTestContext,
        realm_id: &str,
        external_order_id: &str,
    ) -> i64 {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM invoice
             WHERE realm_id = $1 AND external_order_id = $2",
        )
        .bind(realm_id)
        .bind(external_order_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap()
    }

    /// Get invoice status by id.
    async fn get_invoice_status(ctx: &SyncTestContext, invoice_id: Uuid) -> String {
        sqlx::query_scalar("SELECT status FROM invoice WHERE id = $1")
            .bind(invoice_id)
            .fetch_one(&ctx.app_state.pool)
            .await
            .unwrap()
    }

    // =========================================================================
    // Stripe invoice.created sync (US-IF-002 scenario 1)
    // =========================================================================
    // User Story: docs/user-stories/billing/invoice-fallback.md
    // Covers: US-IF-002 scenario 1 -- Stripe invoice.created creates provider=stripe invoice
    //
    // Given: A realm with Stripe webhook configured
    // When: Stripe sends invoice.created event
    // Then: Herald creates an invoice record with provider=stripe, status=draft,
    //       correct external_invoice_id, and non-empty external_payload

    #[test_context(SyncTestContext)]
    #[tokio::test]
    async fn test_stripe_invoice_created_syncs_to_herald(ctx: &mut SyncTestContext) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_test_sync_created";
        let stripe_invoice_id = format!("in_test_{}", Uuid::now_v7());
        let realm_id = ctx._realm_id.clone();

        setup_stripe_config(ctx, &realm_id, "sk_test_key", webhook_secret).await;

        let payload = build_stripe_invoice_event(
            "invoice.created",
            &stripe_invoice_id,
            &realm_id,
            2500,
            "draft",
            None,
            None,
        );

        let response = crate::tests::helpers::webhook_helpers::send_stripe_webhook_with_signature(
            &app,
            &realm_id,
            payload,
            webhook_secret,
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);

        // Verify invoice was created
        let invoice = find_invoice_by_external_id(ctx, &realm_id, &stripe_invoice_id)
            .await
            .expect("Expected invoice to be created for invoice.created event");

        assert_eq!(invoice.1, "stripe", "provider should be stripe");
        assert_eq!(invoice.2, "draft", "status should be draft");
    }

    // =========================================================================
    // Stripe invoice.finalized sync (US-IF-002 scenario 2)
    // =========================================================================
    // User Story: docs/user-stories/billing/invoice-fallback.md
    // Covers: US-IF-002 scenario 2 -- Stripe invoice.finalized updates status to issued
    //
    // Given: An existing Stripe invoice record in draft
    // When: Stripe sends invoice.finalized event
    // Then: Status updates to issued, external_hosted_url and external_pdf_url are recorded

    #[test_context(SyncTestContext)]
    #[tokio::test]
    async fn test_stripe_invoice_finalized_updates_to_issued(ctx: &mut SyncTestContext) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_test_sync_finalized";
        let stripe_invoice_id = format!("in_test_{}", Uuid::now_v7());
        let realm_id = ctx._realm_id.clone();

        setup_stripe_config(ctx, &realm_id, "sk_test_key", webhook_secret).await;

        // Step 1: Send invoice.created to create the initial record
        let created_payload = build_stripe_invoice_event(
            "invoice.created",
            &stripe_invoice_id,
            &realm_id,
            5000,
            "draft",
            None,
            None,
        );

        let response = crate::tests::helpers::webhook_helpers::send_stripe_webhook_with_signature(
            &app,
            &realm_id,
            created_payload,
            webhook_secret,
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);

        // Step 2: Send invoice.finalized to update status
        let finalized_payload = build_stripe_invoice_event(
            "invoice.finalized",
            &stripe_invoice_id,
            &realm_id,
            5000,
            "open",
            Some("https://invoice.stripe.com/hosted/test123"),
            Some("https://invoice.stripe.com/pdf/test123"),
        );

        let response = crate::tests::helpers::webhook_helpers::send_stripe_webhook_with_signature(
            &app,
            &realm_id,
            finalized_payload,
            webhook_secret,
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);

        // Verify status updated to issued
        let invoice = find_invoice_by_external_id(ctx, &realm_id, &stripe_invoice_id)
            .await
            .expect("Expected invoice to exist after finalized event");

        assert_eq!(
            invoice.2, "issued",
            "status should be issued after finalized"
        );
        assert_eq!(
            invoice.3,
            Some("https://invoice.stripe.com/hosted/test123".to_string()),
            "external_hosted_url should be recorded"
        );
        assert_eq!(
            invoice.4,
            Some("https://invoice.stripe.com/pdf/test123".to_string()),
            "external_pdf_url should be recorded"
        );
    }

    // =========================================================================
    // Stripe invoice.paid sync (US-IF-002 scenario 4)
    // =========================================================================
    // User Story: docs/user-stories/billing/invoice-fallback.md
    // Covers: US-IF-002 scenario 4 -- Stripe invoice.paid updates status to paid
    //
    // Given: An existing Stripe invoice record in issued state
    // When: Stripe sends invoice.paid event
    // Then: Status updates to paid

    #[test_context(SyncTestContext)]
    #[tokio::test]
    async fn test_stripe_invoice_paid_updates_to_paid(ctx: &mut SyncTestContext) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_test_sync_paid";
        let stripe_invoice_id = format!("in_test_{}", Uuid::now_v7());
        let realm_id = ctx._realm_id.clone();

        setup_stripe_config(ctx, &realm_id, "sk_test_key", webhook_secret).await;

        // Step 1: Create the invoice via invoice.created
        let created_payload = build_stripe_invoice_event(
            "invoice.created",
            &stripe_invoice_id,
            &realm_id,
            3000,
            "draft",
            None,
            None,
        );

        let response = crate::tests::helpers::webhook_helpers::send_stripe_webhook_with_signature(
            &app,
            &realm_id,
            created_payload,
            webhook_secret,
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);

        // Step 2: Send invoice.paid
        let paid_payload = build_stripe_invoice_event(
            "invoice.paid",
            &stripe_invoice_id,
            &realm_id,
            3000,
            "paid",
            Some("https://invoice.stripe.com/hosted/paid"),
            Some("https://invoice.stripe.com/pdf/paid"),
        );

        let response = crate::tests::helpers::webhook_helpers::send_stripe_webhook_with_signature(
            &app,
            &realm_id,
            paid_payload,
            webhook_secret,
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);

        // Verify status updated to paid
        let invoice = find_invoice_by_external_id(ctx, &realm_id, &stripe_invoice_id)
            .await
            .expect("Expected invoice to exist after paid event");

        assert_eq!(
            invoice.2, "paid",
            "status should be paid after invoice.paid event"
        );
    }

    // =========================================================================
    // Stripe invoice.voided sync (US-IF-002 scenario 3)
    // =========================================================================
    // User Story: docs/user-stories/billing/invoice-fallback.md
    // Covers: US-IF-002 scenario 3 -- Stripe invoice.voided updates status to void
    //
    // Given: An existing Stripe invoice record
    // When: Stripe sends invoice.voided event
    // Then: Status updates to void

    #[test_context(SyncTestContext)]
    #[tokio::test]
    async fn test_stripe_invoice_voided_updates_to_void(ctx: &mut SyncTestContext) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_test_sync_void";
        let stripe_invoice_id = format!("in_test_{}", Uuid::now_v7());
        let realm_id = ctx._realm_id.clone();

        setup_stripe_config(ctx, &realm_id, "sk_test_key", webhook_secret).await;

        // Step 1: Create the invoice via invoice.created
        let created_payload = build_stripe_invoice_event(
            "invoice.created",
            &stripe_invoice_id,
            &realm_id,
            4000,
            "draft",
            None,
            None,
        );

        let response = crate::tests::helpers::webhook_helpers::send_stripe_webhook_with_signature(
            &app,
            &realm_id,
            created_payload,
            webhook_secret,
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);

        // Step 2: Send invoice.voided
        let voided_payload = build_stripe_invoice_event(
            "invoice.voided",
            &stripe_invoice_id,
            &realm_id,
            4000,
            "void",
            None,
            None,
        );

        let response = crate::tests::helpers::webhook_helpers::send_stripe_webhook_with_signature(
            &app,
            &realm_id,
            voided_payload,
            webhook_secret,
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);

        // Verify status updated to void
        let invoice = find_invoice_by_external_id(ctx, &realm_id, &stripe_invoice_id)
            .await
            .expect("Expected invoice to exist after voided event");

        assert_eq!(
            invoice.2, "void",
            "status should be void after invoice.voided event"
        );
    }

    // =========================================================================
    // Stripe idempotency (US-IF-002 scenario 5)
    // =========================================================================
    // User Story: docs/user-stories/billing/invoice-fallback.md
    // Covers: US-IF-002 scenario 5 -- duplicate Stripe webhook updates existing record
    //
    // Given: A Stripe invoice record exists from a previous webhook
    // When: Stripe sends the same invoice event again (same external_invoice_id)
    // Then: The existing record is updated, not duplicated;
    //       only ONE invoice record exists for that external_invoice_id

    #[test_context(SyncTestContext)]
    #[tokio::test]
    async fn test_stripe_duplicate_webhook_updates_existing(ctx: &mut SyncTestContext) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_test_sync_idem";
        let stripe_invoice_id = format!("in_test_idem_{}", Uuid::now_v7());
        let realm_id = ctx._realm_id.clone();

        setup_stripe_config(ctx, &realm_id, "sk_test_key", webhook_secret).await;

        // Step 1: Send invoice.created
        let created_payload = build_stripe_invoice_event(
            "invoice.created",
            &stripe_invoice_id,
            &realm_id,
            6000,
            "draft",
            None,
            None,
        );

        // Use a distinct event_id for the first send (idempotency is on Stripe event ID)
        let response1 = crate::tests::helpers::webhook_helpers::send_stripe_webhook_with_signature(
            &app,
            &realm_id,
            created_payload,
            webhook_secret,
        )
        .await;

        assert_eq!(response1.status(), StatusCode::OK);

        // Verify one record exists
        assert_eq!(
            count_invoices_by_external_id(ctx, &realm_id, &stripe_invoice_id).await,
            1,
            "Expected exactly one invoice after first webhook"
        );

        // Step 2: Send invoice.finalized with a DIFFERENT event ID but same invoice ID
        // This simulates Stripe sending a new event for the same invoice
        let finalized_payload = build_stripe_invoice_event(
            "invoice.finalized",
            &stripe_invoice_id,
            &realm_id,
            6000,
            "open",
            Some("https://invoice.stripe.com/hosted/dup"),
            Some("https://invoice.stripe.com/pdf/dup"),
        );

        let response2 = crate::tests::helpers::webhook_helpers::send_stripe_webhook_with_signature(
            &app,
            &realm_id,
            finalized_payload,
            webhook_secret,
        )
        .await;

        assert_eq!(response2.status(), StatusCode::OK);

        // Verify still only one record, but status updated
        assert_eq!(
            count_invoices_by_external_id(ctx, &realm_id, &stripe_invoice_id).await,
            1,
            "Expected exactly one invoice after upsert (idempotent)"
        );

        let invoice = find_invoice_by_external_id(ctx, &realm_id, &stripe_invoice_id)
            .await
            .expect("Expected invoice to exist");

        assert_eq!(invoice.2, "issued", "status should be updated to issued");
    }

    // =========================================================================
    // Creem checkout.completed invoice sync (US-IF-003 scenario 1)
    // =========================================================================
    // User Story: docs/user-stories/billing/invoice-fallback.md
    // Covers: US-IF-003 scenario 1 -- Creem payment syncs tax data as invoice
    //
    // Given: A realm with Creem webhook configured and a valid plan
    // When: Creem sends checkout.completed webhook with amount and currency
    // Then: Herald creates a provider=creem invoice record with status=paid,
    //       correct external_order_id, and the event data in external_payload

    #[test_context(SyncTestContext)]
    #[tokio::test]
    async fn test_creem_payment_syncs_tax_data_as_invoice(ctx: &mut SyncTestContext) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "test_webhook_secret_creem";
        let plan_id = Uuid::now_v7();
        let client_app_id = Uuid::now_v7();
        let realm_id = ctx._realm_id.clone();
        let event_id = format!("evt_creem_invoice_{}", Uuid::now_v7());

        set_creem_webhook_secret(ctx, webhook_secret).await;

        // Setup plan config so the webhook handler can validate the plan
        crate::tests::helpers::webhook_helpers::setup_test_plan_config(ctx, &realm_id, plan_id)
            .await;

        // Build checkout.completed payload with amount and currency
        let payload = json!({
            "id": event_id,
            "eventType": "checkout.completed",
            "object": {
                "id": format!("checkout_test_{}", Uuid::now_v7()),
                "status": "completed",
                "amount": 2500,
                "currency": "USD",
                "product": {
                    "id": format!("prod_test_{}", plan_id),
                    "name": "Test Plan"
                },
                "customer": {
                    "email": "creem-invoice-test@example.com"
                },
                "metadata": {
                    "realmId": realm_id,
                    "clientAppId": client_app_id.to_string(),
                    "planId": plan_id.to_string(),
                    "billingPeriod": "monthly",
                    "entitlementKey": plan_id.to_string()
                }
            }
        });

        let response = crate::tests::helpers::webhook_helpers::send_webhook_with_signature(
            &app,
            &realm_id,
            payload,
            webhook_secret,
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);

        // Verify Creem invoice was created with provider=creem
        let invoice = find_invoice_by_external_order_id(ctx, &realm_id, &event_id).await;

        assert!(
            invoice.is_some(),
            "Expected invoice to be created for Creem checkout.completed event"
        );

        let invoice = invoice.unwrap();
        assert_eq!(invoice.1, "creem", "provider should be creem");
        assert_eq!(
            invoice.2, "paid",
            "status should be paid for Creem checkout"
        );
    }

    // =========================================================================
    // Creem idempotency (US-IF-003 scenario 1 extension)
    // =========================================================================
    // User Story: docs/user-stories/billing/invoice-fallback.md
    // Covers: US-IF-003 scenario 1 extension -- duplicate Creem callback no duplicate invoice
    //
    // Given: A Creem invoice record already exists from a previous checkout.completed
    // When: Creem sends checkout.completed again with the same event_id
    // Then: The Creem webhook handler returns OK via payment_event idempotency
    //       (first webhook is deduplicated by external_event_id;
    //        second send with a new event_id but same external_order_id does an upsert)
    //       and only ONE invoice record exists for that external_order_id

    #[test_context(SyncTestContext)]
    #[tokio::test]
    async fn test_creem_duplicate_callback_no_duplicate_invoice(ctx: &mut SyncTestContext) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "test_webhook_secret_creem_idem";
        let plan_id = Uuid::now_v7();
        let client_app_id = Uuid::now_v7();
        let realm_id = ctx._realm_id.clone();

        set_creem_webhook_secret(ctx, webhook_secret).await;

        crate::tests::helpers::webhook_helpers::setup_test_plan_config(ctx, &realm_id, plan_id)
            .await;

        // Use the event_id as the external_order_id in the Creem sync
        let event_id_1 = format!("evt_creem_idem_{}", Uuid::now_v7());

        // Step 1: First checkout.completed
        let payload1 = json!({
            "id": event_id_1,
            "eventType": "checkout.completed",
            "object": {
                "id": format!("checkout_idem_{}", Uuid::now_v7()),
                "status": "completed",
                "amount": 5000,
                "currency": "USD",
                "product": {
                    "id": format!("prod_test_{}", plan_id),
                    "name": "Test Plan"
                },
                "metadata": {
                    "realmId": realm_id,
                    "clientAppId": client_app_id.to_string(),
                    "planId": plan_id.to_string(),
                    "billingPeriod": "monthly",
                    "entitlementKey": plan_id.to_string()
                }
            }
        });

        let response1 = crate::tests::helpers::webhook_helpers::send_webhook_with_signature(
            &app,
            &realm_id,
            payload1,
            webhook_secret,
        )
        .await;

        assert_eq!(response1.status(), StatusCode::OK);

        // Verify one invoice exists
        assert_eq!(
            count_invoices_by_external_order_id(ctx, &realm_id, &event_id_1).await,
            1,
            "Expected exactly one invoice after first Creem webhook"
        );

        // Step 2: Send the SAME event again (same event_id).
        // The payment_event dedup will catch this and return OK without re-processing.
        let payload2 = json!({
            "id": event_id_1,
            "eventType": "checkout.completed",
            "object": {
                "id": format!("checkout_idem_{}", Uuid::now_v7()),
                "status": "completed",
                "amount": 5000,
                "currency": "USD",
                "product": {
                    "id": format!("prod_test_{}", plan_id),
                    "name": "Test Plan"
                },
                "metadata": {
                    "realmId": realm_id,
                    "clientAppId": client_app_id.to_string(),
                    "planId": plan_id.to_string(),
                    "billingPeriod": "monthly",
                    "entitlementKey": plan_id.to_string()
                }
            }
        });

        let response2 = crate::tests::helpers::webhook_helpers::send_webhook_with_signature(
            &app,
            &realm_id,
            payload2,
            webhook_secret,
        )
        .await;

        assert_eq!(response2.status(), StatusCode::OK);

        // Verify still only one invoice record
        assert_eq!(
            count_invoices_by_external_order_id(ctx, &realm_id, &event_id_1).await,
            1,
            "Expected exactly one invoice after duplicate Creem webhook (idempotent)"
        );
    }

    // =========================================================================
    // One-time checkout PI -> invoice.external_order_id linkage regression
    // =========================================================================
    //
    // Background:
    //   For Stripe Checkout Sessions in `mode=payment` with `invoice_creation`
    //   enabled, Stripe emits the PaymentIntent ID on the Checkout Session
    //   object but NEVER on the resulting `invoice.*` webhook event payloads.
    //   Herald's `handle_stripe_invoice_event` reads `object["payment_intent"]`,
    //   so without explicit linkage `invoice.external_order_id` stays null for
    //   every one-time purchase.
    //
    // Fix:
    //   `handle_checkout_session_completed` captures `payment_intent` and
    //   `invoice` from the session and upserts the invoice with
    //   `external_order_id = payment_intent`. Subsequent `invoice.*` events
    //   reuse Branch A's COALESCE so the PaymentIntent already stored is
    //   preserved.
    //
    // These tests guard that linkage end-to-end through the webhook surface
    // that the live demo test (us-pu-006-stripe-one-time-invoice-live.e2e.ts)
    // exercises against real Stripe.

    /// Build a `checkout.session.completed` payload for a one-time (mode=payment)
    /// Checkout Session with `invoice_creation` enabled. Mirrors the real Stripe
    /// payload shape: the session carries both `payment_intent` and `invoice`.
    fn build_one_time_checkout_completed(
        event_id: &str,
        realm_id: &str,
        user_id: Uuid,
        client_app_id: Uuid,
        payment_intent: &str,
        stripe_invoice_id: &str,
    ) -> serde_json::Value {
        json!({
            "id": event_id,
            "object": "event",
            "type": "checkout.session.completed",
            "api_version": "2020-08-27",
            "created": chrono::Utc::now().timestamp(),
            "data": {
                "object": {
                    "id": format!("cs_test_{}", Uuid::now_v7()),
                    "object": "checkout.session",
                    "status": "complete",
                    "payment_status": "paid",
                    "mode": "payment",
                    "customer": format!("cus_test_{}", Uuid::now_v7()),
                    "payment_intent": payment_intent,
                    "invoice": stripe_invoice_id,
                    "metadata": {
                        "herald_realm_id": realm_id,
                        "herald_user_id": user_id.to_string(),
                        "herald_client_app_id": client_app_id.to_string(),
                        "userId": user_id.to_string(),
                        "clientAppId": client_app_id.to_string(),
                    }
                }
            }
        })
    }

    /// Build an `invoice.*` event payload where the invoice object intentionally
    /// OMITS `payment_intent` — this is what Stripe actually sends for invoices
    /// produced from a one-time Checkout Session.
    fn build_stripe_invoice_event_without_pi(
        event_type: &str,
        stripe_invoice_id: &str,
        realm_id: &str,
        amount: i64,
        stripe_status: &str,
    ) -> serde_json::Value {
        json!({
            "id": format!("evt_{}", Uuid::now_v7()),
            "object": "event",
            "type": event_type,
            "api_version": "2020-08-27",
            "created": chrono::Utc::now().timestamp(),
            "data": {
                "object": {
                    "id": stripe_invoice_id,
                    "object": "invoice",
                    "status": stripe_status,
                    "total": amount,
                    "currency": "usd",
                    // NOTE: no payment_intent field — this is the bug condition.
                    "metadata": {
                        "realmId": realm_id
                    }
                }
            }
        })
    }

    /// Regression: checkout.session.completed in payment mode links the
    /// PaymentIntent to invoice.external_order_id even when the handler cannot
    /// fulfill (no attemptId in metadata — the legacy checkout endpoint path).
    ///
    /// Given: Stripe webhook configured; no payment attempt was created
    /// When: checkout.session.completed arrives with mode=payment, payment_intent,
    ///       and invoice IDs but no attemptId
    /// Then: A provider=stripe invoice row exists with external_order_id equal
    ///       to the PaymentIntent id
    #[test_context(SyncTestContext)]
    #[tokio::test]
    async fn test_one_time_checkout_links_payment_intent_to_invoice_external_order_id(
        ctx: &mut SyncTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_test_one_time_pi_link";
        let realm_id = ctx._realm_id.clone();
        let stripe_invoice_id = format!("in_onetime_{}", Uuid::now_v7());
        let payment_intent = format!("pi_onetime_{}", Uuid::now_v7());

        setup_stripe_config(ctx, &realm_id, "sk_test_key", webhook_secret).await;

        let user_id = Uuid::now_v7();
        let client_app_id = Uuid::now_v7();
        let payload = build_one_time_checkout_completed(
            &generate_test_event_id(),
            &realm_id,
            user_id,
            client_app_id,
            &payment_intent,
            &stripe_invoice_id,
        );

        let response =
            send_stripe_webhook_with_signature(&app, &realm_id, payload, webhook_secret).await;
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "checkout.session.completed (mode=payment) should be accepted"
        );

        // The fix must populate external_order_id with the PaymentIntent id.
        let invoice = find_invoice_by_external_id(ctx, &realm_id, &stripe_invoice_id)
            .await
            .expect("Expected invoice row to be created from checkout.session.completed linkage");

        let stored_external_order_id: Option<String> =
            sqlx::query_scalar("SELECT external_order_id FROM invoice WHERE id = $1")
                .bind(invoice.0)
                .fetch_one(&ctx.app_state.pool)
                .await
                .expect("Failed to query external_order_id");

        assert_eq!(
            stored_external_order_id.as_deref(),
            Some(payment_intent.as_str()),
            "invoice.external_order_id must equal the PaymentIntent id captured from \
             checkout.session.completed (got {:?})",
            stored_external_order_id
        );
    }

    /// Regression: a subsequent invoice.* event that omits payment_intent must
    /// NOT clobber the PaymentIntent already stored by the checkout.session.completed
    /// linkage (Branch A COALESCE preserves it).
    ///
    /// Given: An invoice row exists with external_order_id=pi_xxx (from
    ///        checkout.session.completed linkage)
    /// When: invoice.created arrives with the same stripe_invoice_id and NO
    ///       payment_intent field
    /// Then: external_order_id is still pi_xxx (preserved by COALESCE)
    #[test_context(SyncTestContext)]
    #[tokio::test]
    async fn test_subsequent_invoice_event_preserves_payment_intent_linkage(
        ctx: &mut SyncTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_test_one_time_pi_preserve";
        let realm_id = ctx._realm_id.clone();
        let stripe_invoice_id = format!("in_preserve_{}", Uuid::now_v7());
        let payment_intent = format!("pi_preserve_{}", Uuid::now_v7());

        setup_stripe_config(ctx, &realm_id, "sk_test_key", webhook_secret).await;

        // Step 1: checkout.session.completed establishes the PI linkage.
        let user_id = Uuid::now_v7();
        let client_app_id = Uuid::now_v7();
        let checkout_payload = build_one_time_checkout_completed(
            &generate_test_event_id(),
            &realm_id,
            user_id,
            client_app_id,
            &payment_intent,
            &stripe_invoice_id,
        );
        let response =
            send_stripe_webhook_with_signature(&app, &realm_id, checkout_payload, webhook_secret)
                .await;
        assert_eq!(response.status(), StatusCode::OK);

        // Step 2: invoice.created arrives without payment_intent (the bug condition).
        let invoice_created_payload = build_stripe_invoice_event_without_pi(
            "invoice.created",
            &stripe_invoice_id,
            &realm_id,
            5000,
            "draft",
        );
        let response = send_stripe_webhook_with_signature(
            &app,
            &realm_id,
            invoice_created_payload,
            webhook_secret,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);

        // Step 3: invoice.paid arrives without payment_intent.
        let invoice_paid_payload = build_stripe_invoice_event_without_pi(
            "invoice.paid",
            &stripe_invoice_id,
            &realm_id,
            5000,
            "paid",
        );
        let response = send_stripe_webhook_with_signature(
            &app,
            &realm_id,
            invoice_paid_payload,
            webhook_secret,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);

        // Verify external_order_id is still the original PaymentIntent.
        let stored_external_order_id: Option<String> =
            sqlx::query_scalar("SELECT external_order_id FROM invoice WHERE realm_id = $1 AND external_invoice_id = $2")
                .bind(&realm_id)
                .bind(&stripe_invoice_id)
                .fetch_one(&ctx.app_state.pool)
                .await
                .expect("Failed to query external_order_id");

        assert_eq!(
            stored_external_order_id.as_deref(),
            Some(payment_intent.as_str()),
            "Subsequent invoice.* events without payment_intent must not clobber the \
             PaymentIntent stored by checkout.session.completed linkage (got {:?})",
            stored_external_order_id
        );
    }
}
