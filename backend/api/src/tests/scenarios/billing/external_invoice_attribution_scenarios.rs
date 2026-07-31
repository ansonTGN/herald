// =============================================================================
// External Invoice Attribution & Regression Scenario Tests
// =============================================================================
//
// Tests for the payment-invoice-mapping attribution backfill (Design §5.4) and
// regression safety of the existing sync flows after the attribution columns
// (`invoice.subscription_id` / `invoice.payment_attempt_id`) became writable.
//
// Scope:
// 1. One-time checkout attribution — both Creem and Stripe one-time checkout
//    invoices must carry `payment_attempt_id` pointing at the one-time payment
//    attempt and `subscription_id = None` (Design §5.4 attribution table:
//    "Creem checkout 一次性" / "Stripe checkout 一次性").
// 2. COALESCE non-clobber — `upsert_external_invoice` ON CONFLICT UPDATE must
//    use `COALESCE(EXCLUDED.x, invoice.x)` so a re-upsert that omits an
//    attribution slot preserves the prior value rather than nulling it
//    (Design §5.4 / §6.3).
// 3. Regression — Creem first-period checkout invoice sync, Stripe `invoice.*`
//    status mapping, and the self-issued (manual) invoice flow are NOT broken
//    by the attribution extension. These cases intentionally do NOT duplicate
//    `invoice_external_sync_scenarios.rs`; they only assert the attribution
//    dimension or cross-reference the existing strong assertions there.
//
// User Story: US-PM-003 (external invoices attributed to a local payment or
//             subscription; missing attribution is discoverable, not silent)
// Covers: Design §5.4 (ExternalInvoiceData attribution + upsert COALESCE),
//                  §6.1 (`external_invoice_attribution_scenarios` + regression),
//                  §6.3 (COALESCE must not clobber existing attribution)
//
// Cross-reference (regression, do not duplicate):
//   - `invoice_external_sync_scenarios.rs::test_creem_payment_syncs_tax_data_as_invoice`
//     strongly asserts Creem first-period checkout invoice provider/status.
//   - `invoice_external_sync_scenarios.rs::test_stripe_invoice_*_updates_to_*`
//     strongly asserts the Stripe `invoice.*` status mapping (draft/issued/paid/void).
//   The regression cases here only add the attribution-dimension assertion or a
//   minimal "no regression" check.
// =============================================================================

#[cfg(test)]
#[allow(dead_code)]
mod tests {
    use crate::tests::helpers::billing_helpers::{
        setup_stripe_config, setup_test_entitlement_mapping_full,
    };
    use crate::tests::helpers::webhook_helpers::{
        generate_test_event_id, send_stripe_webhook_with_signature, send_webhook_with_signature,
    };
    use crate::tests::schema_test_context::SchemaTestContext;
    use axum::http::StatusCode;
    // Bring the `InvoiceRepository` trait into scope so the
    // `upsert_external_invoice` method is callable on the repo handle stored
    // in `app_state`. The COALESCE test cases construct `ExternalInvoiceData`
    // and call the repo directly (focused assertion of the SQL COALESCE
    // semantics, per Design §5.4/§6.3).
    use herald_core::domain::billing::InvoiceRepository;
    use herald_core::domain::billing::invoice::{
        ExternalInvoiceData, InvoiceProvider, InvoiceStatus,
    };
    use serde_json::json;
    use test_context::test_context;
    use uuid::Uuid;

    use SchemaTestContext as AttributionTestContext;

    // =========================================================================
    // Private query helpers (attribution dimension)
    // =========================================================================

    /// Read the attribution triple for an invoice matched by `external_invoice_id`.
    /// Returns `(id, subscription_id, payment_attempt_id)`.
    async fn find_invoice_attribution(
        ctx: &AttributionTestContext,
        realm_id: &str,
        external_invoice_id: &str,
    ) -> Option<(Uuid, Option<Uuid>, Option<Uuid>)> {
        sqlx::query_as(
            "SELECT id, subscription_id, payment_attempt_id
             FROM invoice
             WHERE realm_id = $1 AND external_invoice_id = $2",
        )
        .bind(realm_id)
        .bind(external_invoice_id)
        .fetch_optional(&ctx.app_state.pool)
        .await
        .unwrap()
    }

    /// Read the attribution triple for an invoice matched by `external_order_id`
    /// (the Branch B upsert path).
    async fn find_invoice_attribution_by_order(
        ctx: &AttributionTestContext,
        realm_id: &str,
        external_order_id: &str,
    ) -> Option<(Uuid, Option<Uuid>, Option<Uuid>)> {
        sqlx::query_as(
            "SELECT id, subscription_id, payment_attempt_id
             FROM invoice
             WHERE realm_id = $1 AND external_order_id = $2",
        )
        .bind(realm_id)
        .bind(external_order_id)
        .fetch_optional(&ctx.app_state.pool)
        .await
        .unwrap()
    }

    /// Read the buyer attribution/snapshot fields for an invoice matched by
    /// `external_invoice_id`. Returns
    /// `(account_id, applicant_user_id, billing_name, billing_email)`.
    async fn find_invoice_buyer(
        ctx: &AttributionTestContext,
        realm_id: &str,
        external_invoice_id: &str,
    ) -> Option<(Option<Uuid>, Option<Uuid>, Option<String>, Option<String>)> {
        sqlx::query_as(
            "SELECT account_id, applicant_user_id, billing_name, billing_email
             FROM invoice
             WHERE realm_id = $1 AND external_invoice_id = $2",
        )
        .bind(realm_id)
        .bind(external_invoice_id)
        .fetch_optional(&ctx.app_state.pool)
        .await
        .unwrap()
    }

    /// Set the Creem webhook secret for the test realm.
    async fn set_creem_webhook_secret(ctx: &AttributionTestContext, webhook_secret: &str) {
        ctx.with_creem_config(
            &ctx._realm_id,
            Some("test_api_key"),
            Some(webhook_secret),
            Some(30),
        )
        .await;
    }

    /// Create a one-time entitlement mapping for the given provider and return
    /// its id (reuses the shared billing helper).
    async fn create_one_time_mapping(
        ctx: &mut AttributionTestContext,
        realm_id: &str,
        provider: &str,
        entitlement_key: &str,
    ) -> Uuid {
        setup_test_entitlement_mapping_full(
            ctx,
            realm_id,
            provider,
            &format!("prod_{provider}_{entitlement_key}"),
            None, // external_price_id
            entitlement_key,
            Some("one_time"), // billing_type
            None,             // billing_period
            Some(1000),       // points_per_period
            None,             // grant_period_type
            None,             // validity_days
            false,            // grant_on_subscribe
            None,             // max_periods
            true,             // enabled
            Some(json!({
                "price": 1000,
                "currency": "usd",
                "name": format!("One-time {}", entitlement_key)
            })),
        )
        .await
    }

    /// Create a test user in the realm plus a points wallet and return the user id.
    async fn create_test_user_and_wallet(
        ctx: &AttributionTestContext,
        realm_id: &str,
        email: &str,
    ) -> Uuid {
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

        let bucket_id = crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm(
            &ctx.app_state.pool,
            realm_id,
        )
        .await;

        sqlx::query(
            "INSERT INTO points_wallets
                (id, user_id, realm_id, bucket_id,
                 total_topup_granted, total_subscription_granted,
                 total_recharged, total_consumed, status, created_at, updated_at)
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

        user_id
    }

    /// Create a pending one-time payment attempt targeting `mapping_id`.
    /// Returns the attempt id.
    async fn create_pending_payment_attempt(
        ctx: &AttributionTestContext,
        realm_id: &str,
        user_id: Uuid,
        mapping_id: Uuid,
        provider: &str,
    ) -> Uuid {
        let attempt_id = Uuid::now_v7();
        // `payment_attempts.bucket_id` was removed by the distribution-rules
        // refactor (grant routing lives on distribution rules / attempt rule
        // snapshots). These tests assert invoice attribution, not points
        // balances, so no rule snapshot is seeded here.
        sqlx::query(
            "INSERT INTO payment_attempts
                (id, realm_id, user_id, payment_provider, target_type, target_id,
                 amount, currency, status, expires_at, created_at, updated_at)
             VALUES ($1, $2, $3, $4, 'entitlement_mapping', $5,
                 1000, 'usd', 'Pending', NOW() + INTERVAL '1 hour', NOW(), NOW())",
        )
        .bind(attempt_id)
        .bind(realm_id)
        .bind(user_id)
        .bind(provider)
        .bind(mapping_id)
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to create pending payment attempt");
        attempt_id
    }

    // =========================================================================
    // One-time checkout attribution
    // =========================================================================

    /// Build a Creem `checkout.completed` payload for a one-time purchase that
    /// carries the originating payment `attemptId` in metadata (mirrors the
    /// real Creem webhook shape used by the one-time dispatch path).
    fn build_creem_one_time_checkout_completed(
        event_id: &str,
        realm_id: &str,
        user_id: Uuid,
        client_app_id: Uuid,
        entitlement_key: &str,
        external_product_id: &str,
        attempt_id: Uuid,
    ) -> serde_json::Value {
        json!({
            "id": event_id,
            "eventType": "checkout.completed",
            "object": {
                "id": format!("ch_creem_{}", Uuid::now_v7()),
                "status": "completed",
                "amount": 2500,
                "currency": "USD",
                "product": {
                    "id": external_product_id,
                    "name": "One-time Pack"
                },
                "customer": {
                    "email": "creem-attribution@test.com"
                },
                "metadata": {
                    "realmId": realm_id,
                    "clientAppId": client_app_id.to_string(),
                    "userId": user_id.to_string(),
                    "entitlementKey": entitlement_key,
                    "herald_entitlement_key": entitlement_key,
                    "herald_realm_id": realm_id,
                    "herald_user_id": user_id.to_string(),
                    "herald_client_app_id": client_app_id.to_string(),
                    "herald_billing_kind": "one_time",
                    "billingPeriod": "monthly",
                    "attemptId": attempt_id.to_string()
                }
            }
        })
    }

    /// Build a Stripe `checkout.session.completed` payload for a one-time
    /// purchase (`mode=payment`) that carries `attemptId`, `payment_intent`,
    /// and `invoice` on the session object — the exact fields
    /// `handle_checkout_session_completed` reads to drive
    /// `link_one_time_payment_intent_to_invoice`.
    fn build_stripe_one_time_checkout_completed(
        event_id: &str,
        realm_id: &str,
        user_id: Uuid,
        client_app_id: Uuid,
        entitlement_key: &str,
        attempt_id: Uuid,
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
                    "amount_total": 2500,
                    "currency": "usd",
                    "customer": format!("cus_test_{}", Uuid::now_v7()),
                    "payment_intent": payment_intent,
                    "invoice": stripe_invoice_id,
                    "metadata": {
                        "herald_realm_id": realm_id,
                        "herald_user_id": user_id.to_string(),
                        "herald_client_app_id": client_app_id.to_string(),
                        "herald_entitlement_key": entitlement_key,
                        "clientAppId": client_app_id.to_string(),
                        "userId": user_id.to_string(),
                        "attemptId": attempt_id.to_string()
                    },
                    "display_items": [{
                        "price": {
                            "product": format!("prod_stripe_{}", entitlement_key)
                        }
                    }]
                }
            }
        })
    }

    // -------------------------------------------------------------------------
    // Test: Creem one-time checkout invoice attributed to the one-time attempt
    // -------------------------------------------------------------------------
    // User Story: US-PM-003 (one-time invoice → payment_attempt_id)
    // Covers: Design §5.4 attribution table ("Creem checkout 一次性"),
    //         §6.1 (`external_invoice_attribution_scenarios`: one-time checkout
    //         invoice → payment_attempt_id hits the one-time attempt)
    //
    // Given: A realm with Creem webhook configured and a one-time entitlement
    //        mapping, a user with a wallet, and a pending one-time payment
    //        attempt for that mapping
    // When: Creem sends checkout.completed carrying that attemptId in metadata
    // Then: The synced invoice's `payment_attempt_id` equals the one-time
    //       attempt id, and `subscription_id` is None (one-time has no
    //       subscription). The invoice is keyed by the checkout id
    //       (`external_invoice_id`).

    #[test_context(AttributionTestContext)]
    #[tokio::test]
    async fn test_creem_one_time_checkout_invoice_attributed_to_attempt(
        ctx: &mut AttributionTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_creem_onetime_attr";
        let realm_id = ctx._realm_id.clone();
        let client_app_id = Uuid::parse_str(&ctx._client_app_id).unwrap();
        let entitlement_key = "creem-onetime-attr";

        set_creem_webhook_secret(ctx, webhook_secret).await;
        let mapping_id = create_one_time_mapping(ctx, &realm_id, "creem", entitlement_key).await;
        let user_id =
            create_test_user_and_wallet(ctx, &realm_id, "creem-onetime-attr@test.com").await;
        let attempt_id =
            create_pending_payment_attempt(ctx, &realm_id, user_id, mapping_id, "creem").await;

        let event_id = generate_test_event_id();
        let payload = build_creem_one_time_checkout_completed(
            &event_id,
            &realm_id,
            user_id,
            client_app_id,
            entitlement_key,
            &format!("prod_creem_{entitlement_key}"),
            attempt_id,
        );

        // Capture the checkout id embedded in the payload so we can look the
        // invoice up by external_invoice_id (the handler keys on object.id).
        let checkout_id = payload["object"]["id"].as_str().unwrap().to_string();

        let response = send_webhook_with_signature(&app, &realm_id, payload, webhook_secret).await;
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Creem one-time checkout.completed should be accepted"
        );

        let attribution = find_invoice_attribution(ctx, &realm_id, &checkout_id)
            .await
            .unwrap_or_else(|| {
                panic!("Expected invoice row keyed by external_invoice_id={checkout_id}")
            });

        assert_eq!(
            attribution.2,
            Some(attempt_id),
            "Creem one-time checkout invoice must be attributed to the one-time payment_attempt (Design §5.4)"
        );
        assert_eq!(
            attribution.1, None,
            "Creem one-time checkout invoice must NOT carry a subscription_id (no subscription for one-time)"
        );
    }

    // -------------------------------------------------------------------------
    // Test: Stripe one-time checkout invoice attributed to the one-time attempt
    // -------------------------------------------------------------------------
    // User Story: US-PM-003 (one-time invoice → payment_attempt_id)
    // Covers: Design §5.4 attribution table ("Stripe checkout 一次性"),
    //         §6.1 (one-time checkout invoice → payment_attempt_id)
    //
    // Given: A realm with Stripe webhook configured and a one-time entitlement
    //        mapping, a user with a wallet, and a pending one-time payment
    //        attempt for that mapping
    // When: Stripe sends checkout.session.completed with mode=payment, the
    //       attemptId in metadata, and both `payment_intent` + `invoice` on
    //       the session (so `link_one_time_payment_intent_to_invoice` fires)
    // Then: The linked invoice's `payment_attempt_id` equals the one-time
    //       attempt id and `subscription_id` is None.

    #[test_context(AttributionTestContext)]
    #[tokio::test]
    async fn test_stripe_one_time_checkout_invoice_attributed_to_attempt(
        ctx: &mut AttributionTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_stripe_onetime_attr";
        let realm_id = ctx._realm_id.clone();
        let client_app_id = Uuid::parse_str(&ctx._client_app_id).unwrap();
        let entitlement_key = "stripe-onetime-attr";

        setup_stripe_config(ctx, &realm_id, "sk_test_onetime", webhook_secret).await;
        let mapping_id = create_one_time_mapping(ctx, &realm_id, "stripe", entitlement_key).await;
        let user_id =
            create_test_user_and_wallet(ctx, &realm_id, "stripe-onetime-attr@test.com").await;
        let attempt_id =
            create_pending_payment_attempt(ctx, &realm_id, user_id, mapping_id, "stripe").await;

        let stripe_invoice_id = format!("in_onetime_attr_{}", Uuid::now_v7());
        let payment_intent = format!("pi_onetime_attr_{}", Uuid::now_v7());

        let payload = build_stripe_one_time_checkout_completed(
            &generate_test_event_id(),
            &realm_id,
            user_id,
            client_app_id,
            entitlement_key,
            attempt_id,
            &payment_intent,
            &stripe_invoice_id,
        );

        let response =
            send_stripe_webhook_with_signature(&app, &realm_id, payload, webhook_secret).await;
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Stripe one-time checkout.session.completed should be accepted"
        );

        let attribution = find_invoice_attribution(ctx, &realm_id, &stripe_invoice_id)
            .await
            .unwrap_or_else(|| {
                panic!("Expected invoice row keyed by external_invoice_id={stripe_invoice_id}")
            });

        assert_eq!(
            attribution.2,
            Some(attempt_id),
            "Stripe one-time checkout invoice must be attributed to the one-time payment_attempt via link_one_time_payment_intent_to_invoice (Design §5.4)"
        );
        assert_eq!(
            attribution.1, None,
            "Stripe one-time checkout invoice must NOT carry a subscription_id (no subscription for one-time)"
        );
    }

    // =========================================================================
    // COALESCE non-clobber (Design §5.4 / §6.3)
    // =========================================================================
    //
    // These two cases call `InvoiceRepository::upsert_external_invoice` directly
    // via `app_state.invoice_repository`. The COALESCE semantics are a property
    // of the SQL ON CONFLICT UPDATE clause, so a focused repo-level call is the
    // clearest way to assert "re-upsert with None preserves the prior value"
    // without entangling two unrelated webhook event types.

    /// User Story: US-PM-003 (attribution must not be silently lost)
    /// Covers: Design §5.4 (upsert COALESCE), §6.3 (COALESCE does not clobber
    ///         existing attribution)
    ///
    /// Given: An invoice row exists for external_invoice_id=in_C with
    ///        subscription_id = Some(s1) (and no payment_attempt_id)
    /// When: The same external_invoice_id is re-upserted with
    ///        subscription_id = None AND payment_attempt_id = Some(a1)
    /// Then: subscription_id is STILL s1 (COALESCE preserved it; the None did
    ///       NOT clobber), AND payment_attempt_id was filled to a1 (COALESCE
    ///       filled the previously-empty slot).
    #[test_context(AttributionTestContext)]
    #[tokio::test]
    async fn test_upsert_update_path_coalesces_subscription_id(ctx: &mut AttributionTestContext) {
        let realm_id = ctx._realm_id.clone();
        let external_invoice_id = format!("in_coal_sub_{}", Uuid::now_v7());
        let subscription_id = Uuid::now_v7();
        let payment_attempt_id = Uuid::now_v7();

        // Seed an external_sync invoice with subscription attribution but no
        // payment_attempt attribution (mirrors a Stripe invoice.* sync row
        // before invoice.payment_succeeded backfills the renewal attempt).
        let seed = ExternalInvoiceData {
            realm_id: realm_id.clone(),
            provider: InvoiceProvider::Stripe,
            payment_provider: Some("stripe".to_string()),
            external_invoice_id: Some(external_invoice_id.clone()),
            external_order_id: None,
            external_status: None,
            external_hosted_url: None,
            external_pdf_url: None,
            external_payload: None,
            tax_details: None,
            account_id: None,
            applicant_user_id: None,
            billing_name: None,
            billing_email: None,
            billing_phone: None,
            billing_address: None,
            currency: "usd".to_string(),
            total: 2500,
            status: InvoiceStatus::Issued,
            subscription_id: Some(subscription_id),
            payment_attempt_id: None,
        };
        ctx.app_state
            .invoice_repository
            .upsert_external_invoice(seed)
            .await
            .expect("seed upsert must succeed");

        // Re-upsert the SAME external_invoice_id, this time omitting the
        // subscription attribution (None) while supplying a payment_attempt_id.
        let re_upsert = ExternalInvoiceData {
            realm_id: realm_id.clone(),
            provider: InvoiceProvider::Stripe,
            payment_provider: Some("stripe".to_string()),
            external_invoice_id: Some(external_invoice_id.clone()),
            external_order_id: None,
            external_status: None,
            external_hosted_url: None,
            external_pdf_url: None,
            external_payload: None,
            tax_details: None,
            account_id: None,
            applicant_user_id: None,
            billing_name: None,
            billing_email: None,
            billing_phone: None,
            billing_address: None,
            currency: "usd".to_string(),
            total: 2500,
            status: InvoiceStatus::Paid,
            subscription_id: None, // <-- must NOT clobber the existing s1
            payment_attempt_id: Some(payment_attempt_id),
        };
        ctx.app_state
            .invoice_repository
            .upsert_external_invoice(re_upsert)
            .await
            .expect("re-upsert must succeed");

        let attribution = find_invoice_attribution(ctx, &realm_id, &external_invoice_id)
            .await
            .expect("row must exist");

        assert_eq!(
            attribution.1,
            Some(subscription_id),
            "COALESCE must preserve the existing subscription_id when the re-upsert passes None (Design §6.3)"
        );
        assert_eq!(
            attribution.2,
            Some(payment_attempt_id),
            "COALESCE must fill the previously-empty payment_attempt_id slot"
        );
    }

    /// User Story: US-PM-003 (attribution must not be silently lost)
    /// Covers: Design §5.4 (upsert COALESCE), §6.3 (COALESCE does not clobber
    ///         existing attribution) — symmetric to the subscription_id case
    ///
    /// Given: An invoice row exists for external_invoice_id=in_C with
    ///        payment_attempt_id = Some(a1) (and no subscription_id)
    /// When: The same external_invoice_id is re-upserted with
    ///        payment_attempt_id = None
    /// Then: payment_attempt_id is STILL a1 (COALESCE preserved it; the None
    ///       did NOT clobber).
    #[test_context(AttributionTestContext)]
    #[tokio::test]
    async fn test_upsert_update_path_coalesces_payment_attempt_id(
        ctx: &mut AttributionTestContext,
    ) {
        let realm_id = ctx._realm_id.clone();
        let external_invoice_id = format!("in_coal_pa_{}", Uuid::now_v7());
        let payment_attempt_id = Uuid::now_v7();

        let seed = ExternalInvoiceData {
            realm_id: realm_id.clone(),
            provider: InvoiceProvider::Stripe,
            payment_provider: Some("stripe".to_string()),
            external_invoice_id: Some(external_invoice_id.clone()),
            external_order_id: None,
            external_status: None,
            external_hosted_url: None,
            external_pdf_url: None,
            external_payload: None,
            tax_details: None,
            account_id: None,
            applicant_user_id: None,
            billing_name: None,
            billing_email: None,
            billing_phone: None,
            billing_address: None,
            currency: "usd".to_string(),
            total: 2500,
            status: InvoiceStatus::Paid,
            subscription_id: None,
            payment_attempt_id: Some(payment_attempt_id),
        };
        ctx.app_state
            .invoice_repository
            .upsert_external_invoice(seed)
            .await
            .expect("seed upsert must succeed");

        // Re-upsert the SAME external_invoice_id omitting the payment_attempt
        // attribution (None). Mirrors a subsequent invoice.* sync event that
        // does not know about the renewal attempt.
        let re_upsert = ExternalInvoiceData {
            realm_id: realm_id.clone(),
            provider: InvoiceProvider::Stripe,
            payment_provider: Some("stripe".to_string()),
            external_invoice_id: Some(external_invoice_id.clone()),
            external_order_id: None,
            external_status: None,
            external_hosted_url: None,
            external_pdf_url: None,
            external_payload: None,
            tax_details: None,
            account_id: None,
            applicant_user_id: None,
            billing_name: None,
            billing_email: None,
            billing_phone: None,
            billing_address: None,
            currency: "usd".to_string(),
            total: 2500,
            status: InvoiceStatus::Paid,
            subscription_id: None,
            payment_attempt_id: None, // <-- must NOT clobber the existing a1
        };
        ctx.app_state
            .invoice_repository
            .upsert_external_invoice(re_upsert)
            .await
            .expect("re-upsert must succeed");

        let attribution = find_invoice_attribution(ctx, &realm_id, &external_invoice_id)
            .await
            .expect("row must exist");

        assert_eq!(
            attribution.2,
            Some(payment_attempt_id),
            "COALESCE must preserve the existing payment_attempt_id when the re-upsert passes None (Design §6.3)"
        );
    }

    // =========================================================================
    // Regression: attribution extension does not break existing flows
    // =========================================================================
    //
    // These cases deliberately avoid duplicating the strong assertions already
    // present in `invoice_external_sync_scenarios.rs`. Each one either
    // cross-references the existing case and adds only the attribution-dimension
    // assertion, or performs a minimal "still works" check.

    /// User Story: US-PM-003 (regression — attribution extension must not
    ///              break existing Creem first-period checkout invoice sync)
    /// Covers: Design §6.1 (regression: Creem first-period checkout invoice
    ///         sync unchanged), §5.4 attribution table (Creem checkout
    ///         first-period → subscription_id per BE-D02 design, may be None
    ///         at checkout time and filled by subscription.paid)
    ///
    /// Cross-reference: `invoice_external_sync_scenarios.rs::
    ///   test_creem_payment_syncs_tax_data_as_invoice` strongly asserts the
    ///   provider=creem / status=paid behavior for a Creem checkout.completed
    ///   invoice. This case does NOT re-assert those; it only asserts the
    ///   attribution dimension after the §5.4 extension: a first-period
    ///   checkout invoice must exist (still synced) and its attribution is
    ///   queryable without error.
    #[test_context(AttributionTestContext)]
    #[tokio::test]
    async fn test_creem_first_period_checkout_invoice_synced_unchanged(
        ctx: &mut AttributionTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_creem_first_period_attr";
        let realm_id = ctx._realm_id.clone();
        let plan_id = Uuid::now_v7();
        let client_app_id = Uuid::now_v7();
        let event_id = format!("evt_creem_first_period_{}", Uuid::now_v7());

        set_creem_webhook_secret(ctx, webhook_secret).await;
        crate::tests::helpers::webhook_helpers::setup_test_plan_config(ctx, &realm_id, plan_id)
            .await;

        let payload = json!({
            "id": event_id,
            "eventType": "checkout.completed",
            "object": {
                "id": format!("ch_first_period_{}", Uuid::now_v7()),
                "status": "completed",
                "amount": 2500,
                "currency": "USD",
                "product": {
                    "id": format!("prod_test_{}", plan_id),
                    "name": "Test Plan"
                },
                "customer": {
                    "email": "creem-first-period@test.com"
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

        let response = send_webhook_with_signature(&app, &realm_id, payload, webhook_secret).await;
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Creem first-period checkout.completed should still be accepted (no regression)"
        );

        // Regression: the invoice row must still exist (sync unchanged). We
        // only assert the attribution dimension is queryable; the strong
        // provider/status assertions live in invoice_external_sync_scenarios.
        let attribution = find_invoice_attribution_by_order(ctx, &realm_id, &event_id)
            .await
            .expect("Creem first-period checkout invoice must still be synced (regression)");
        // First-period checkout invoice has no originating one-time attempt in
        // this legacy payload shape, so payment_attempt_id is expected to be
        // None here (attribution is filled by subscription.paid for recurring).
        // We assert queryability + that the row exists; we do NOT assert a
        // specific attribution value to avoid duplicating/contradicting BE-D02.
        let _ = attribution; // row presence is the regression assertion
    }

    /// User Story: US-PM-003 (regression — attribution extension must not
    ///              break Stripe invoice.* status mapping)
    /// Covers: Design §6.1 (regression: Stripe invoice.* status mapping
    ///         unchanged), §6.3 (attribution field extension must not regress)
    ///
    /// Cross-reference: `invoice_external_sync_scenarios.rs` strongly asserts
    ///   the full draft→issued→paid→void status mapping for Stripe invoice.*
    ///   events (test_stripe_invoice_created_syncs_to_herald,
    ///   test_stripe_invoice_finalized_updates_to_issued,
    ///   test_stripe_invoice_paid_updates_to_paid,
    ///   test_stripe_invoice_voided_updates_to_void). This case does NOT
    ///   re-assert those transitions; it performs a minimal "an invoice.created
    ///   event still produces a provider=stripe row whose attribution columns
    ///   are queryable" check to guard against the attribution extension
    ///   accidentally breaking the sync write path.
    #[test_context(AttributionTestContext)]
    #[tokio::test]
    async fn test_stripe_invoice_status_mapping_unchanged(ctx: &mut AttributionTestContext) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "whsec_stripe_status_attr";
        let realm_id = ctx._realm_id.clone();
        let stripe_invoice_id = format!("in_status_attr_{}", Uuid::now_v7());

        setup_stripe_config(ctx, &realm_id, "sk_test_status", webhook_secret).await;

        let payload = json!({
            "id": format!("evt_{}", Uuid::now_v7()),
            "object": "event",
            "type": "invoice.created",
            "api_version": "2020-08-27",
            "created": chrono::Utc::now().timestamp(),
            "data": {
                "object": {
                    "id": stripe_invoice_id,
                    "object": "invoice",
                    "status": "draft",
                    "total": 3000,
                    "currency": "usd",
                    "metadata": { "realmId": realm_id }
                }
            }
        });

        let response =
            send_stripe_webhook_with_signature(&app, &realm_id, payload, webhook_secret).await;
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Stripe invoice.created should still be accepted (no regression)"
        );

        // Regression: the row exists and attribution is queryable. Strong
        // status-transition assertions live in invoice_external_sync_scenarios.
        let attribution = find_invoice_attribution(ctx, &realm_id, &stripe_invoice_id)
            .await
            .expect("Stripe invoice row must still be created from invoice.created (regression)");
        let _ = attribution; // row presence is the regression assertion
    }

    /// User Story: US-PM-003 (regression — self-issued manual invoice flow is
    ///              unaffected by the external-attribution extension)
    /// Covers: Design §1.3 ("Manual 自研发票的归属约束 — 维持现状，允许无对应支付"),
    ///         §6.1 (regression: manual invoice flow unaffected)
    ///
    /// Given: The realm has no external provider configured
    /// When: A manual (self-issued) invoice row is created directly
    /// Then: It has provider='manual', no external_invoice_id, and nullable
    ///       attribution columns — i.e. the attribution extension did not add
    ///       any NOT NULL / constraint that would break the manual flow.
    #[test_context(AttributionTestContext)]
    #[tokio::test]
    async fn test_manual_invoice_flow_unaffected(ctx: &mut AttributionTestContext) {
        let realm_id = ctx._realm_id.clone();
        let invoice_id = Uuid::now_v7();

        // Insert a manual invoice directly, the way the manual-creation path
        // would. provider='manual', no external ids, attribution columns NULL.
        sqlx::query(
            "INSERT INTO invoice
                (id, realm_id, invoice_number, source, provider, status, currency,
                 subtotal, discount_amount, tax_amount, shipping_amount, total,
                 amount_refunded, amount_remaining, created_at, updated_at)
             VALUES ($1, $2, $3, 'admin_manual', 'manual', 'draft', 'USD',
                     10000, 0, 0, 0, 10000, 0, 10000, NOW(), NOW())",
        )
        .bind(invoice_id)
        .bind(&realm_id)
        .bind(format!("INV-MANUAL-{}", Uuid::now_v7()))
        .execute(&ctx.app_state.pool)
        .await
        .expect("manual invoice insert must succeed (attribution extension must not constrain manual flow)");

        let row: (String, Option<String>, Option<Uuid>, Option<Uuid>) = sqlx::query_as(
            "SELECT provider, external_invoice_id, subscription_id, payment_attempt_id
             FROM invoice WHERE id = $1",
        )
        .bind(invoice_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap();

        assert_eq!(row.0, "manual", "manual invoice provider must be 'manual'");
        assert!(
            row.1.is_none(),
            "manual invoice must have no external_invoice_id"
        );
        assert!(
            row.2.is_none() && row.3.is_none(),
            "manual invoice attribution columns must be nullable (Design §1.3: manual invoices may have no corresponding payment)"
        );
    }

    /// User Story: US-PM-003 (buyer identity must be attributed to the purchaser)
    /// Covers: one-time Checkout flow — `account_id` + buyer snapshot must land
    ///         on the invoice row even though the buyer info arrives across two
    ///         webhook events (checkout.session.completed then invoice.*).
    ///
    /// Given: An external invoice is seeded (mirrors checkout.session.completed
    ///        landing first) with account_id resolved but NO buyer snapshot.
    /// When: The same external_invoice_id is re-upserted (mirrors the
    ///        subsequent invoice.* event) carrying the buyer snapshot.
    /// Then: account_id is preserved AND the buyer snapshot (billing_name/email)
    ///       is filled — both via COALESCE.
    #[test_context(AttributionTestContext)]
    #[tokio::test]
    async fn test_upsert_backfills_buyer_snapshot(ctx: &mut AttributionTestContext) {
        let realm_id = ctx._realm_id.clone();
        let external_invoice_id = format!("in_buyer_{}", Uuid::now_v7());
        let user_id = Uuid::now_v7();

        // Seed: account_id resolved, buyer snapshot absent (exactly the
        // checkout.session.completed state before invoice.* arrives).
        let seed = ExternalInvoiceData {
            realm_id: realm_id.clone(),
            provider: InvoiceProvider::Stripe,
            payment_provider: Some("stripe".to_string()),
            external_invoice_id: Some(external_invoice_id.clone()),
            external_order_id: None,
            external_status: None,
            external_hosted_url: None,
            external_pdf_url: None,
            external_payload: None,
            tax_details: None,
            account_id: Some(user_id),
            applicant_user_id: None,
            billing_name: None,
            billing_email: None,
            billing_phone: None,
            billing_address: None,
            currency: "usd".to_string(),
            total: 5000,
            status: InvoiceStatus::Paid,
            subscription_id: None,
            payment_attempt_id: None,
        };
        ctx.app_state
            .invoice_repository
            .upsert_external_invoice(seed)
            .await
            .expect("seed upsert must succeed");

        // Re-upsert: buyer snapshot arrives (mirrors invoice.* event carrying
        // customer_name/customer_email), account_id now None on this carrier.
        let re_upsert = ExternalInvoiceData {
            realm_id: realm_id.clone(),
            provider: InvoiceProvider::Stripe,
            payment_provider: Some("stripe".to_string()),
            external_invoice_id: Some(external_invoice_id.clone()),
            external_order_id: None,
            external_status: None,
            external_hosted_url: None,
            external_pdf_url: None,
            external_payload: None,
            tax_details: None,
            account_id: None, // must NOT clobber the seeded user_id
            applicant_user_id: None,
            billing_name: Some("Alice".to_string()),
            billing_email: Some("alice@example.com".to_string()),
            billing_phone: None,
            billing_address: None,
            currency: "usd".to_string(),
            total: 5000,
            status: InvoiceStatus::Paid,
            subscription_id: None,
            payment_attempt_id: None,
        };
        ctx.app_state
            .invoice_repository
            .upsert_external_invoice(re_upsert)
            .await
            .expect("re-upsert must succeed");

        let buyer = find_invoice_buyer(ctx, &realm_id, &external_invoice_id)
            .await
            .expect("row must exist");

        assert_eq!(
            buyer.0,
            Some(user_id),
            "account_id must be preserved by COALESCE when the re-upsert carrier is None"
        );
        assert_eq!(
            buyer.2.as_deref(),
            Some("Alice"),
            "billing_name must be backfilled from the re-upsert (previously-NULL slot)"
        );
        assert_eq!(
            buyer.3.as_deref(),
            Some("alice@example.com"),
            "billing_email must be backfilled from the re-upsert (previously-NULL slot)"
        );
    }

    /// User Story: US-PM-003 (buyer snapshot must not be silently overwritten)
    /// Covers: COALESCE must preserve an existing buyer snapshot when a
    ///         re-upsert passes None — symmetric to the attribution COALESCE.
    #[test_context(AttributionTestContext)]
    #[tokio::test]
    async fn test_upsert_preserves_existing_buyer_snapshot(ctx: &mut AttributionTestContext) {
        let realm_id = ctx._realm_id.clone();
        let external_invoice_id = format!("in_buyer_keep_{}", Uuid::now_v7());

        // Seed WITH a buyer snapshot.
        let seed = ExternalInvoiceData {
            realm_id: realm_id.clone(),
            provider: InvoiceProvider::Stripe,
            payment_provider: Some("stripe".to_string()),
            external_invoice_id: Some(external_invoice_id.clone()),
            external_order_id: None,
            external_status: None,
            external_hosted_url: None,
            external_pdf_url: None,
            external_payload: None,
            tax_details: None,
            account_id: None,
            applicant_user_id: None,
            billing_name: Some("Bob".to_string()),
            billing_email: Some("bob@example.com".to_string()),
            billing_phone: None,
            billing_address: None,
            currency: "usd".to_string(),
            total: 5000,
            status: InvoiceStatus::Paid,
            subscription_id: None,
            payment_attempt_id: None,
        };
        ctx.app_state
            .invoice_repository
            .upsert_external_invoice(seed)
            .await
            .expect("seed upsert must succeed");

        // Re-upsert with buyer snapshot = None (mirrors an event that lacks
        // customer_name/email). Must NOT clobber the seeded values.
        let re_upsert = ExternalInvoiceData {
            realm_id: realm_id.clone(),
            provider: InvoiceProvider::Stripe,
            payment_provider: Some("stripe".to_string()),
            external_invoice_id: Some(external_invoice_id.clone()),
            external_order_id: None,
            external_status: None,
            external_hosted_url: None,
            external_pdf_url: None,
            external_payload: None,
            tax_details: None,
            account_id: None,
            applicant_user_id: None,
            billing_name: None,
            billing_email: None,
            billing_phone: None,
            billing_address: None,
            currency: "usd".to_string(),
            total: 5000,
            status: InvoiceStatus::Paid,
            subscription_id: None,
            payment_attempt_id: None,
        };
        ctx.app_state
            .invoice_repository
            .upsert_external_invoice(re_upsert)
            .await
            .expect("re-upsert must succeed");

        let buyer = find_invoice_buyer(ctx, &realm_id, &external_invoice_id)
            .await
            .expect("row must exist");

        assert_eq!(
            buyer.2.as_deref(),
            Some("Bob"),
            "billing_name must survive a None re-upsert (COALESCE preserves it)"
        );
        assert_eq!(
            buyer.3.as_deref(),
            Some("bob@example.com"),
            "billing_email must survive a None re-upsert (COALESCE preserves it)"
        );
    }
}
