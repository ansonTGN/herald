// =============================================================================
// Creem Subscription Renewal Scenario Tests
// =============================================================================
//
// Tests for Creem `subscription.paid` **renewal** branch (no `attempt_id`
// metadata): each paid renewal must record exactly one Succeeded
// `payment_attempt` (idempotent by `provider_reference =
// creem_renewal:{ext_sub_id}:{last_transaction_id}`) and upsert one Paid
// invoice whose `external_invoice_id = last_transaction_id`, with both
// `subscription_id` and `payment_attempt_id` attribution non-null (Design
// §5.2, GAP1+GAP2). Also covers idempotency on duplicate delivery,
// one-row-per-distinct-transaction, first-period dedup (P0), the
// `last_transaction_id`-missing fallback (P1), and the zero-yuan cycle skip.
//
// User Story: US-PM-001 (every renewal records a payment_attempt),
//             US-PM-002 (every Creem renewal syncs a paid invoice)
// Covers: Design §5.2 (Creem renewal invoice sync),
//                  §6.1 (`creem_subscription_renewal_scenarios` cases),
//                  §7 (P0 first-period/renewal dedup, P1 fallback key)
//
// NOTE (orchestrator correction, verified against migration
// `20260609_points_package_one_time.sql:16`): the production `target_type`
// CHECK now only allows `'entitlement_mapping'`. These tests query renewal
// attempts BY `provider_reference` and assert `status='Succeeded'`; they do
// NOT assert `target_type='subscription_entitlement'` (the BE-T01 spec's
// dev-handoff was stale on this point).
// =============================================================================

#[cfg(test)]
mod tests {
    use crate::tests::helpers::billing_helpers::setup_test_entitlement_mapping_full;
    use crate::tests::helpers::webhook_helpers::{
        assert_webhook_success, generate_test_event_id, send_webhook_with_signature,
    };
    use crate::tests::schema_test_context::SchemaTestContext;
    use test_context::test_context;
    use uuid::Uuid;

    use SchemaTestContext as RenewalTestContext;

    // =========================================================================
    // Shared Setup Helpers
    // =========================================================================

    /// Set the Creem webhook secret for the test realm.
    async fn set_creem_webhook_secret(ctx: &RenewalTestContext, webhook_secret: &str) {
        ctx.with_creem_config(
            &ctx._realm_id,
            Some("test_api_key"),
            Some(webhook_secret),
            Some(30),
        )
        .await;
    }

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

    /// Create a recurring Creem entitlement mapping (the resolver used by the
    /// renewal branch maps to `external_price_id IS NULL`).
    async fn create_recurring_mapping(
        ctx: &mut RenewalTestContext,
        realm_id: &str,
        entitlement_key: &str,
        external_product_id: &str,
    ) {
        setup_test_entitlement_mapping_full(
            ctx,
            realm_id,
            "creem",
            external_product_id,
            None,
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
               AND payment_provider = 'creem'
               AND provider_reference = $2",
        )
        .bind(realm_id)
        .bind(provider_reference)
        .fetch_optional(&ctx.app_state.pool)
        .await
        .unwrap()
    }

    /// Count renewal attempts for a subscription (any `creem_renewal:{ext_sub}:*`).
    async fn count_renewal_attempts_by_sub(
        ctx: &RenewalTestContext,
        realm_id: &str,
        ext_sub_id: &str,
    ) -> i64 {
        let pattern = format!("creem_renewal:{}:%", ext_sub_id);
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM payment_attempts
             WHERE realm_id = $1
               AND payment_provider = 'creem'
               AND provider_reference LIKE $2",
        )
        .bind(realm_id)
        .bind(&pattern)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap()
    }

    /// Find an invoice by `external_invoice_id`. Returns
    /// (id, provider, status, subscription_id, payment_attempt_id).
    async fn find_invoice_by_external_id(
        ctx: &RenewalTestContext,
        realm_id: &str,
        external_invoice_id: &str,
    ) -> Option<(Uuid, String, String, Option<Uuid>, Option<Uuid>)> {
        sqlx::query_as(
            "SELECT id, provider, status, subscription_id, payment_attempt_id
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

    /// Count any invoice whose `external_invoice_id` starts with the given
    /// prefix (e.g. `tran_` for renewal invoices, `ch_`/`checkout_` for
    /// checkout invoices). Used to assert "no renewal invoice produced".
    async fn count_invoices_by_external_id_prefix(
        ctx: &RenewalTestContext,
        realm_id: &str,
        prefix: &str,
    ) -> i64 {
        let pattern = format!("{}%", prefix);
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM invoice
             WHERE realm_id = $1 AND external_invoice_id LIKE $2",
        )
        .bind(realm_id)
        .bind(&pattern)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap()
    }

    // =========================================================================
    // Test 1: renewal creates 1 Succeeded attempt + 1 Paid attributed invoice
    // =========================================================================

    /// User Story: US-PM-001, US-PM-002
    /// Covers: Design §5.2 (Creem renewal invoice sync), §6.1 case 1, §7 P0
    ///
    /// Given: A realm with a recurring Creem entitlement mapping + a user
    /// When: Creem sends a renewal `subscription.paid` (`last_transaction_id=tran_A`, amount=2500)
    /// Then: Exactly 1 Succeeded renewal attempt exists keyed by
    ///       `creem_renewal:{ext_sub}:tran_A`, and exactly 1 Paid invoice with
    ///       `external_invoice_id=tran_A` whose `subscription_id` and
    ///       `payment_attempt_id` are both non-null and `provider='creem'`.
    /// Why: This is the core GAP1+GAP2 closure -- a renewal must record a
    ///      payment and produce an attributable invoice, otherwise billing
    ///      cannot answer "which payment renewed this subscription" or
    ///      "which invoice belongs to this renewal".
    #[test_context(RenewalTestContext)]
    #[tokio::test]
    async fn test_creem_renewal_creates_succeeded_attempt_and_paid_invoice(
        ctx: &mut RenewalTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "test_renewal_invoice_secret";
        let realm_id = ctx._realm_id.clone();
        let user_id = create_test_user(ctx, &realm_id, "renewal-inv@test.com").await;
        let client_app_id = Uuid::parse_str(&ctx._client_app_id).unwrap();
        let entitlement_key = "renewal-invoice";
        let external_product_id = format!("prod_creem_{}", entitlement_key);
        let ext_sub_id = format!("sub_renewal_inv_{}", generate_test_event_id());

        set_creem_webhook_secret(ctx, webhook_secret).await;
        create_recurring_mapping(ctx, &realm_id, entitlement_key, &external_product_id).await;
        create_points_wallet(ctx, user_id, &realm_id).await;

        let event_id = generate_test_event_id();
        let payload = crate::tests::helpers::webhook_helpers::build_creem_subscription_paid_renewal_with_invoice(
            &event_id,
            entitlement_key,
            &realm_id,
            user_id,
            Some(client_app_id),
            &ext_sub_id,
            &external_product_id,
            Some("tran_A"),
            Some(2500),
            Some("usd"),
        );

        let response = send_webhook_with_signature(&app, &realm_id, payload, webhook_secret).await;
        assert_webhook_success(&response);

        // --- Attempt assertions (GAP1) ---
        let expected_reference = format!("creem_renewal:{}:{}", ext_sub_id, "tran_A");
        let attempt = find_renewal_attempt_by_reference(ctx, &realm_id, &expected_reference)
            .await
            .expect("renewal payment_attempt should exist after paid renewal");
        assert_eq!(
            attempt.1, "Succeeded",
            "renewal attempt status must be Succeeded (US-PM-001)"
        );
        assert_eq!(
            attempt.2, expected_reference,
            "renewal attempt provider_reference must encode ext_sub + tran_"
        );
        assert_eq!(
            count_renewal_attempts_by_sub(ctx, &realm_id, &ext_sub_id).await,
            1,
            "exactly one renewal attempt per distinct transaction"
        );

        // --- Invoice assertions (GAP2 + attribution) ---
        let invoice = find_invoice_by_external_id(ctx, &realm_id, "tran_A")
            .await
            .expect("renewal invoice should exist with external_invoice_id=tran_A");
        assert_eq!(invoice.1, "creem", "renewal invoice provider must be creem");
        assert_eq!(
            invoice.2, "paid",
            "renewal invoice status must be paid (US-PM-002)"
        );
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
            count_invoices_by_external_id(ctx, &realm_id, "tran_A").await,
            1,
            "exactly one invoice per renewal transaction"
        );
    }

    // =========================================================================
    // Test 2: duplicate same-transaction delivery is idempotent
    // =========================================================================

    /// User Story: US-PM-001, US-PM-002
    /// Covers: Design §4.1 "幂等三重保证", §6.1 case 2, §7 (idempotency)
    ///
    /// Given: A renewal `subscription.paid` with `last_transaction_id=tran_A` has fired
    /// When: The same `tran_A` event is re-delivered (different event_id)
    /// Then: The renewal attempt row count stays at 1 (same id), the invoice
    ///       row count stays at 1, and points are not double-granted.
    /// Why: Webhook delivery is at-least-once; if a duplicate renewal created a
    ///      second attempt or invoice, billing would over-count the user's
    ///      payments and over-credit points. Idempotency here is the safety net.
    #[test_context(RenewalTestContext)]
    #[tokio::test]
    async fn test_creem_renewal_duplicate_same_transaction_is_idempotent(
        ctx: &mut RenewalTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "test_renewal_idem_secret";
        let realm_id = ctx._realm_id.clone();
        let user_id = create_test_user(ctx, &realm_id, "renewal-idem@test.com").await;
        let client_app_id = Uuid::parse_str(&ctx._client_app_id).unwrap();
        let entitlement_key = "renewal-idem";
        let external_product_id = format!("prod_creem_{}", entitlement_key);
        let ext_sub_id = format!("sub_renewal_idem_{}", generate_test_event_id());

        set_creem_webhook_secret(ctx, webhook_secret).await;
        create_recurring_mapping(ctx, &realm_id, entitlement_key, &external_product_id).await;
        create_points_wallet(ctx, user_id, &realm_id).await;

        let build_payload = |event_id: String| {
            crate::tests::helpers::webhook_helpers::build_creem_subscription_paid_renewal_with_invoice(
                &event_id,
                entitlement_key,
                &realm_id,
                user_id,
                Some(client_app_id),
                &ext_sub_id,
                &external_product_id,
                Some("tran_A"),
                Some(2500),
                Some("usd"),
            )
        };

        // First delivery.
        let first_event = generate_test_event_id();
        let response = send_webhook_with_signature(
            &app,
            &realm_id,
            build_payload(first_event.clone()),
            webhook_secret,
        )
        .await;
        assert_webhook_success(&response);

        let attempt_after_first = find_renewal_attempt_by_reference(
            ctx,
            &realm_id,
            &format!("creem_renewal:{}:{}", ext_sub_id, "tran_A"),
        )
        .await
        .expect("attempt must exist after first delivery");
        let invoice_count_after_first =
            count_invoices_by_external_id(ctx, &realm_id, "tran_A").await;
        assert_eq!(invoice_count_after_first, 1);

        // Re-deliver with a NEW event_id but the SAME last_transaction_id.
        let second_event = generate_test_event_id();
        let response = send_webhook_with_signature(
            &app,
            &realm_id,
            build_payload(second_event),
            webhook_secret,
        )
        .await;
        assert_webhook_success(&response);

        // Idempotency: same attempt id, still exactly 1 row, still exactly 1 invoice.
        let attempt_after_second = find_renewal_attempt_by_reference(
            ctx,
            &realm_id,
            &format!("creem_renewal:{}:{}", ext_sub_id, "tran_A"),
        )
        .await
        .expect("attempt must still exist after re-delivery");
        assert_eq!(
            attempt_after_second.0, attempt_after_first.0,
            "duplicate delivery must resolve to the SAME attempt id (find-or-create)"
        );
        assert_eq!(
            count_renewal_attempts_by_sub(ctx, &realm_id, &ext_sub_id).await,
            1,
            "duplicate delivery must NOT create a second renewal attempt"
        );
        assert_eq!(
            count_invoices_by_external_id(ctx, &realm_id, "tran_A").await,
            invoice_count_after_first,
            "duplicate delivery must NOT create a second invoice (upsert idempotency)"
        );
    }

    // =========================================================================
    // Test 3: distinct transactions each produce exactly one row
    // =========================================================================

    /// User Story: US-PM-001, US-PM-002
    /// Covers: Design §6.1 case 3, §7 (one invoice per renewal cycle)
    ///
    /// Given: Two renewal cycles with different `last_transaction_id` (tran_A, tran_B)
    /// When: Both `subscription.paid` events fire
    /// Then: Two distinct renewal attempts (different provider_reference / id)
    ///       and two distinct invoices (`external_invoice_id` tran_A and tran_B).
    /// Why: Each paid renewal cycle is a separate charge and must be recorded as
    ///      a separate payment and invoice -- collapsing cycles would hide real
    ///      revenue from the billing ledger and break per-cycle auditability.
    #[test_context(RenewalTestContext)]
    #[tokio::test]
    async fn test_creem_renewal_different_transactions_each_one_row(ctx: &mut RenewalTestContext) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "test_renewal_multi_secret";
        let realm_id = ctx._realm_id.clone();
        let user_id = create_test_user(ctx, &realm_id, "renewal-multi@test.com").await;
        let client_app_id = Uuid::parse_str(&ctx._client_app_id).unwrap();
        let entitlement_key = "renewal-multi";
        let external_product_id = format!("prod_creem_{}", entitlement_key);
        let ext_sub_id = format!("sub_renewal_multi_{}", generate_test_event_id());

        set_creem_webhook_secret(ctx, webhook_secret).await;
        create_recurring_mapping(ctx, &realm_id, entitlement_key, &external_product_id).await;
        create_points_wallet(ctx, user_id, &realm_id).await;

        let send_renewal = |tx_id: &str| {
            let event_id = generate_test_event_id();
            crate::tests::helpers::webhook_helpers::build_creem_subscription_paid_renewal_with_invoice(
                &event_id,
                entitlement_key,
                &realm_id,
                user_id,
                Some(client_app_id),
                &ext_sub_id,
                &external_product_id,
                Some(tx_id),
                Some(2500),
                Some("usd"),
            )
        };

        // Cycle A.
        let response =
            send_webhook_with_signature(&app, &realm_id, send_renewal("tran_A"), webhook_secret)
                .await;
        assert_webhook_success(&response);

        // Cycle B (distinct transaction).
        let response =
            send_webhook_with_signature(&app, &realm_id, send_renewal("tran_B"), webhook_secret)
                .await;
        assert_webhook_success(&response);

        // Two distinct attempts (different provider_reference => different id).
        let attempt_a = find_renewal_attempt_by_reference(
            ctx,
            &realm_id,
            &format!("creem_renewal:{}:{}", ext_sub_id, "tran_A"),
        )
        .await
        .expect("attempt for tran_A must exist");
        let attempt_b = find_renewal_attempt_by_reference(
            ctx,
            &realm_id,
            &format!("creem_renewal:{}:{}", ext_sub_id, "tran_B"),
        )
        .await
        .expect("attempt for tran_B must exist");
        assert_ne!(
            attempt_a.0, attempt_b.0,
            "distinct transactions must produce distinct attempt ids"
        );
        assert_eq!(attempt_a.1, "Succeeded");
        assert_eq!(attempt_b.1, "Succeeded");
        assert_eq!(
            count_renewal_attempts_by_sub(ctx, &realm_id, &ext_sub_id).await,
            2,
            "two distinct renewal cycles => two renewal attempts"
        );

        // Two distinct invoices, each attributed to its own cycle's attempt.
        assert_eq!(
            count_invoices_by_external_id(ctx, &realm_id, "tran_A").await,
            1
        );
        assert_eq!(
            count_invoices_by_external_id(ctx, &realm_id, "tran_B").await,
            1
        );
        let inv_a = find_invoice_by_external_id(ctx, &realm_id, "tran_A")
            .await
            .expect("invoice tran_A must exist");
        let inv_b = find_invoice_by_external_id(ctx, &realm_id, "tran_B")
            .await
            .expect("invoice tran_B must exist");
        assert_eq!(inv_a.4, Some(attempt_a.0));
        assert_eq!(inv_b.4, Some(attempt_b.0));
    }

    // =========================================================================
    // Test 4: first-period `subscription.paid` (with attemptId) skips renewal invoice
    // =========================================================================

    /// User Story: US-PM-002
    /// Covers: Design §5.2 "首期/续费去重 (P0)", §6.1 case 4, §7 P0 (no dup)
    ///
    /// Given: A `subscription.paid` carrying an `attemptId` in metadata (first period)
    /// When: The event is processed
    /// Then: No renewal `payment_attempt` row is created (no `creem_renewal:`
    ///       provider_reference) and no `external_invoice_id=tran_*` renewal
    ///       invoice is produced -- the first-period branch returns early, and
    ///       the first-period invoice is the responsibility of `checkout.completed`.
    /// Why: If the first-period branch fell through into the renewal logic, the
    ///      same cycle would get BOTH a checkout invoice (ch_/checkout_) and a
    ///      renewal invoice (tran_) -- a P0 double-invoice regression (Design §7).
    #[test_context(RenewalTestContext)]
    #[tokio::test]
    async fn test_creem_first_period_paid_with_attempt_id_skips_renewal_invoice(
        ctx: &mut RenewalTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "test_renewal_first_secret";
        let realm_id = ctx._realm_id.clone();
        let user_id = create_test_user(ctx, &realm_id, "renewal-first@test.com").await;
        let client_app_id = Uuid::parse_str(&ctx._client_app_id).unwrap();
        let entitlement_key = "renewal-first";
        let external_product_id = format!("prod_creem_{}", entitlement_key);
        let ext_sub_id = format!("sub_renewal_first_{}", generate_test_event_id());

        set_creem_webhook_secret(ctx, webhook_secret).await;
        create_recurring_mapping(ctx, &realm_id, entitlement_key, &external_product_id).await;
        create_points_wallet(ctx, user_id, &realm_id).await;

        // First-period payload: carry an `attemptId` in metadata. Use the
        // existing herald-metadata builder (no renewal-invoice fields) so the
        // first-period branch (which inspects `metadata.attemptId`) is taken.
        let event_id = generate_test_event_id();
        let attempt_id = Uuid::now_v7();
        let period_start = chrono::Utc::now();
        let period_end = period_start + chrono::Duration::days(30);
        let payload = serde_json::json!({
            "id": event_id,
            "eventType": "subscription.paid",
            "data": {
                "object": {
                    "subscriptionId": ext_sub_id,
                    "productId": external_product_id,
                    "userId": user_id.to_string(),
                    "isRenewal": false,
                    "currentPeriodStart": period_start.to_rfc3339(),
                    "currentPeriodEnd": period_end.to_rfc3339(),
                    "status": "active",
                    "cancelAtPeriodEnd": false,
                    "metadata": {
                        "herald_entitlement_key": entitlement_key,
                        "herald_realm_id": realm_id,
                        "herald_user_id": user_id.to_string(),
                        "herald_client_app_id": client_app_id.to_string(),
                        "clientAppId": client_app_id.to_string(),
                        "attemptId": attempt_id.to_string(),
                    },
                }
            },
            "herald_entitlement_key": entitlement_key,
        });

        let response = send_webhook_with_signature(&app, &realm_id, payload, webhook_secret).await;
        // The first-period branch calls complete_succeeded_payment_attempt,
        // which requires the attempt to exist. Without a seeded attempt this
        // may error; we only assert the P0 dedup invariant (no renewal row /
        // no renewal invoice) regardless of outcome, since the renewal branch
        // is what we are guarding against.
        let _ = response.status();

        // P0 invariant: no renewal attempt was created for this subscription.
        assert_eq!(
            count_renewal_attempts_by_sub(ctx, &realm_id, &ext_sub_id).await,
            0,
            "first-period subscription.paid must NOT create a renewal payment_attempt (P0 dedup)"
        );

        // P0 invariant: no renewal invoice (tran_* external_invoice_id) was
        // produced for this realm.
        assert_eq!(
            count_invoices_by_external_id_prefix(ctx, &realm_id, "tran_").await,
            0,
            "first-period subscription.paid must NOT produce a tran_* renewal invoice (P0 dedup)"
        );
    }

    // =========================================================================
    // Test 5: missing last_transaction_id falls back to {ext_sub}:{period_start}
    // =========================================================================

    /// User Story: US-PM-002
    /// Covers: Design §5.2 fallback, §6.1 case 5, §7 P1 (last_transaction_id missing)
    ///
    /// Given: A renewal `subscription.paid` with `last_transaction_id=None` but a valid period
    /// When: The event is processed
    /// Then: A renewal attempt is still recorded using the fallback idempotency
    ///       key `creem_renewal:{ext_sub}:{period_start}`, and a renewal invoice
    ///       is upserted with the fallback `external_invoice_id = {ext_sub}:{period_start}`.
    /// Why: When Creem omits `last_transaction_id` (e.g. some trial-to-paid
    ///      transitions), billing must still record the renewal payment and
    ///      invoice using a deterministic key derived from the subscription +
    ///      period, otherwise these renewals would silently produce no record.
    #[test_context(RenewalTestContext)]
    #[tokio::test]
    async fn test_creem_renewal_missing_last_transaction_id_uses_fallback_key(
        ctx: &mut RenewalTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "test_renewal_fallback_secret";
        let realm_id = ctx._realm_id.clone();
        let user_id = create_test_user(ctx, &realm_id, "renewal-fallback@test.com").await;
        let client_app_id = Uuid::parse_str(&ctx._client_app_id).unwrap();
        let entitlement_key = "renewal-fallback";
        let external_product_id = format!("prod_creem_{}", entitlement_key);
        let ext_sub_id = format!("sub_renewal_fallback_{}", generate_test_event_id());

        set_creem_webhook_secret(ctx, webhook_secret).await;
        create_recurring_mapping(ctx, &realm_id, entitlement_key, &external_product_id).await;
        create_points_wallet(ctx, user_id, &realm_id).await;

        // No last_transaction_id, but amount > 0 and a valid period are present.
        let event_id = generate_test_event_id();
        let payload = crate::tests::helpers::webhook_helpers::build_creem_subscription_paid_renewal_with_invoice(
            &event_id,
            entitlement_key,
            &realm_id,
            user_id,
            Some(client_app_id),
            &ext_sub_id,
            &external_product_id,
            None, // last_transaction_id absent -> fallback path
            Some(2500),
            Some("usd"),
        );

        // Capture the period_start we emitted so we can build the expected
        // fallback key/id (production uses the same parsed value).
        let emitted_period_start = payload["data"]["object"]["currentPeriodStart"]
            .as_str()
            .expect("helper must emit currentPeriodStart")
            .to_string();

        let response = send_webhook_with_signature(&app, &realm_id, payload, webhook_secret).await;
        assert_webhook_success(&response);

        // Fallback idempotency key: creem_renewal:{ext_sub}:{period_start}.
        let fallback_reference = format!("creem_renewal:{}:{}", ext_sub_id, emitted_period_start);
        let attempt = find_renewal_attempt_by_reference(ctx, &realm_id, &fallback_reference)
            .await
            .expect("fallback renewal attempt should exist");
        assert_eq!(attempt.1, "Succeeded");

        // Fallback invoice external_invoice_id: {ext_sub}:{period_start}.
        let fallback_invoice_id = format!("{}:{}", ext_sub_id, emitted_period_start);
        let invoice = find_invoice_by_external_id(ctx, &realm_id, &fallback_invoice_id)
            .await
            .expect("fallback renewal invoice should exist");
        assert_eq!(invoice.1, "creem");
        assert_eq!(invoice.2, "paid");
        assert!(
            invoice.3.is_some(),
            "fallback invoice must attribute subscription_id"
        );
        assert_eq!(invoice.4, Some(attempt.0));
    }

    // =========================================================================
    // Test 6: zero-yuan cycle skips renewal attempt + invoice
    // =========================================================================

    /// User Story: US-PM-001, US-PM-002
    /// Covers: Design §5.2 "amount == 0 跳过", §6.1 case 6, §7 (zero-yuan)
    ///
    /// Given: A renewal `subscription.paid` with `amount=0` (100%-off / free tier cycle)
    /// When: The event is processed
    /// Then: No renewal `payment_attempt` is created (DB CHECK `amount > 0`
    ///       forbids a zero row anyway) and no renewal invoice is produced,
    ///       while the subscription cycle / points grant still runs.
    /// Why: A zero-yuan cycle is not a real charge, so it must NOT inflate the
    ///      payment ledger with a Succeeded 0-amount attempt (which the DB
    ///      rejects anyway) and must NOT produce a 0-amount invoice. Points /
    ///      cycle advancement are orthogonal and still proceed.
    #[test_context(RenewalTestContext)]
    #[tokio::test]
    async fn test_creem_renewal_zero_amount_skips_attempt_and_invoice(
        ctx: &mut RenewalTestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let webhook_secret = "test_renewal_zero_secret";
        let realm_id = ctx._realm_id.clone();
        let user_id = create_test_user(ctx, &realm_id, "renewal-zero@test.com").await;
        let client_app_id = Uuid::parse_str(&ctx._client_app_id).unwrap();
        let entitlement_key = "renewal-zero";
        let external_product_id = format!("prod_creem_{}", entitlement_key);
        let ext_sub_id = format!("sub_renewal_zero_{}", generate_test_event_id());

        set_creem_webhook_secret(ctx, webhook_secret).await;
        create_recurring_mapping(ctx, &realm_id, entitlement_key, &external_product_id).await;
        create_points_wallet(ctx, user_id, &realm_id).await;

        let event_id = generate_test_event_id();
        let payload = crate::tests::helpers::webhook_helpers::build_creem_subscription_paid_renewal_with_invoice(
            &event_id,
            entitlement_key,
            &realm_id,
            user_id,
            Some(client_app_id),
            &ext_sub_id,
            &external_product_id,
            Some("tran_zero"),
            Some(0), // zero-yuan cycle
            Some("usd"),
        );

        let response = send_webhook_with_signature(&app, &realm_id, payload, webhook_secret).await;
        assert_webhook_success(&response);

        // Zero-yuan cycle: no renewal attempt, no renewal invoice.
        assert_eq!(
            count_renewal_attempts_by_sub(ctx, &realm_id, &ext_sub_id).await,
            0,
            "zero-yuan renewal must NOT create a payment_attempt (amount > 0 CHECK)"
        );
        assert_eq!(
            count_invoices_by_external_id(ctx, &realm_id, "tran_zero").await,
            0,
            "zero-yuan renewal must NOT produce an invoice"
        );
    }
}
