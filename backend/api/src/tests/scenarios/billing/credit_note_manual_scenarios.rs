// =============================================================================
// Manual Credit Note Scenario Tests
// =============================================================================
//
// Tests for the Manual Credit Note creation endpoint
// (POST /api/bill/{realmId}/invoices/{invoiceId}/credit-notes) covering:
//   - Normal creation (US-IF-010 scenario 1)
//   - Amount overflow rejection (US-IF-010 scenario 3)
//   - Provider mismatch rejection (US-IF-010 scenario 4)
//   - Non-paid status rejection (US-IF-010 scenario 6)
//   - Partial refund accumulation (US-IF-010 scenario 2)
//   - Invoice not found (edge case)
//
// Refund visibility (US-IF-008) is verified through the invoice detail
// response's `amountRefunded` / `amountRemaining` fields populated by
// dev item BE-D06.
//
// User Story: docs/user-stories/billing/invoice-fallback.md
// Covers: US-IF-010, US-IF-008
//
// =============================================================================

#[cfg(test)]
mod tests {
    use crate::tests::helpers::*;
    use crate::tests::schema_test_context::SchemaTestContext;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use chrono::Datelike;
    use serde_json::json;
    use test_context::test_context;
    use tower::ServiceExt;
    use uuid::Uuid;

    use SchemaTestContext as CreditNoteTestContext;

    // Helper: parse response body to JSON
    async fn parse_body(body: axum::body::Body) -> serde_json::Value {
        let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    // Helper: create a test account row (invoice FK target)
    async fn ensure_test_account(ctx: &CreditNoteTestContext, realm_id: &str) -> Uuid {
        let account_id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO account (id, realm_id, email, password, status)
             VALUES ($1, $2, $3, $4, 1)
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(account_id)
        .bind(realm_id)
        .bind(format!("credit-note-test-{}@example.com", account_id))
        .bind("$2a$12$dummy_hash")
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();
        account_id
    }

    // Helper: create a draft invoice via API, return the full response JSON.
    async fn create_draft_invoice(
        app: &axum::Router,
        token: &str,
        realm_id: &str,
        account_id: Uuid,
        unit_price: i64,
    ) -> serde_json::Value {
        let payload = json!({
            "accountId": account_id.to_string(),
            "currency": "USD",
            "lineItems": [
                {
                    "name": "Refundable Service",
                    "quantity": "1",
                    "unitPrice": unit_price,
                }
            ],
            "billingName": "Credit Note Client",
            "billingAddress": "123 Test St",
            "billingEmail": "billing@test.com",
            "billingTaxId": "TAX-001",
            "sellerName": "Test Seller Inc.",
            "sellerAddress": "456 Seller Ave",
            "sellerTaxId": "SELLER-TAX-001",
            "dueDate": "2026-06-30",
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/bill/{}/invoices", realm_id))
                    .header("content-type", "application/json")
                    .header("cookie", format!("X-Auth={}", token))
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        parse_body(response.into_body()).await
    }

    // Helper: create a paid manual invoice through the API (draft -> issue -> mark-paid).
    // Returns the mark-paid response JSON (contains `id`, `status: "paid"`, `total`).
    async fn create_paid_manual_invoice(
        app: &axum::Router,
        token: &str,
        realm_id: &str,
        account_id: Uuid,
        total_amount: i64,
    ) -> serde_json::Value {
        let draft = create_draft_invoice(app, token, realm_id, account_id, total_amount).await;
        let invoice_id = draft["id"].as_str().unwrap();

        // Issue the invoice
        let issue_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/bill/{}/invoices/{}/issue",
                        realm_id, invoice_id
                    ))
                    .header("content-type", "application/json")
                    .header("cookie", format!("X-Auth={}", token))
                    .body(Body::from(json!({}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(issue_response.status(), StatusCode::OK);

        // Mark as paid
        let paid_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/bill/{}/invoices/{}/mark-paid",
                        realm_id, invoice_id
                    ))
                    .header("content-type", "application/json")
                    .header("cookie", format!("X-Auth={}", token))
                    .body(Body::from(json!({}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(paid_response.status(), StatusCode::OK);
        parse_body(paid_response.into_body()).await
    }

    // Helper: POST a credit note against the given invoice.
    async fn create_credit_note(
        app: &axum::Router,
        token: &str,
        realm_id: &str,
        invoice_id: &str,
        amount: i64,
        memo: &str,
    ) -> axum::response::Response {
        let payload = json!({
            "amount": amount,
            "memo": memo,
        });

        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/bill/{}/invoices/{}/credit-notes",
                        realm_id, invoice_id
                    ))
                    .header("content-type", "application/json")
                    .header("cookie", format!("X-Auth={}", token))
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    // Helper: insert a Stripe-provider paid invoice directly via SQL. Used for the
    // provider-mismatch scenario where the API-created invoices always have provider=manual.
    async fn insert_stripe_paid_invoice_via_sql(
        ctx: &CreditNoteTestContext,
        realm_id: &str,
        account_id: Uuid,
        total: i64,
    ) -> Uuid {
        let invoice_id = Uuid::now_v7();
        let year = chrono::Utc::now().year();
        let seq: i64 = sqlx::query_scalar(
            "INSERT INTO invoice_number_counter (realm_id, year, next_seq, updated_at)
             VALUES ($1, $2, 2, NOW())
             ON CONFLICT (realm_id, year) DO UPDATE SET next_seq = invoice_number_counter.next_seq + 1, updated_at = NOW()
             RETURNING next_seq - 1",
        )
        .bind(realm_id)
        .bind(year)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap();

        let invoice_number = format!("INV-{}-{:04}", year, seq);

        sqlx::query(
            "INSERT INTO invoice (
                id, realm_id, invoice_number, source, account_id,
                status, currency, subtotal, discount_amount, tax_amount, shipping_amount, total,
                amount_refunded, amount_remaining,
                provider, payment_provider, external_invoice_id,
                billing_name, billing_address, billing_tax_id,
                seller_name, seller_address, seller_tax_id,
                due_date, paid_at, created_at, updated_at
            ) VALUES (
                $1, $2, $3, 'external_sync', $4,
                'paid', 'USD', $5, 0, 0, 0, $5,
                0, $5,
                'stripe', 'stripe', $6,
                'Stripe Client', '123 Stripe St', 'TAX-STRIPE-001',
                'Seller Inc', '456 Seller Ave', 'SELLER-TAX-001',
                CURRENT_DATE + INTERVAL '30 days', NOW(), NOW(), NOW()
            )",
        )
        .bind(invoice_id)
        .bind(realm_id)
        .bind(&invoice_number)
        .bind(account_id)
        .bind(total)
        .bind(format!("in_stripe_{}", invoice_id))
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();

        invoice_id
    }

    // =========================================================================
    // Test: Normal Manual Credit Note Creation (US-IF-010 scenario 1)
    // =========================================================================
    // User Story: docs/user-stories/billing/invoice-fallback.md
    // Covers: US-IF-010 scenario 1, US-IF-008 scenario 1
    //
    // Given: A paid manual invoice, total = 10000
    // When: POST credit note with amount = 3000, memo = "Partial refund"
    // Then: Returns 201 with source=manual, amount=3000, memo, created_by_user_id,
    //       currency matching the invoice
    // And: GET invoice detail shows amountRefunded=3000, amountRemaining=7000

    #[test_context(CreditNoteTestContext)]
    #[tokio::test]
    async fn test_manual_credit_note_normal_creation(ctx: &mut CreditNoteTestContext) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();
        let (admin_token, admin_user_id) =
            setup_billing_admin_session_with_user(ctx, "cn-normal@test.com").await;
        let account_id = ensure_test_account(ctx, &realm_id).await;

        // Create a paid manual invoice with total = 10000
        let paid_invoice =
            create_paid_manual_invoice(&app, &admin_token, &realm_id, account_id, 10000).await;
        assert_eq!(paid_invoice["status"], "paid");
        assert_eq!(paid_invoice["total"], 10000);
        let invoice_id = paid_invoice["id"].as_str().unwrap();

        // Create a credit note for amount = 3000
        let response = create_credit_note(
            &app,
            &admin_token,
            &realm_id,
            invoice_id,
            3000,
            "Partial refund",
        )
        .await;

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = parse_body(response.into_body()).await;

        // Verify credit note fields
        assert_eq!(body["amount"], 3000);
        assert_eq!(body["source"], "manual");
        assert_eq!(body["memo"], "Partial refund");
        assert_eq!(body["currency"], "USD");
        assert_eq!(body["createdByUserId"], admin_user_id.to_string());
        assert!(
            body["id"].is_string(),
            "credit note id should be a string UUID"
        );

        // GET invoice detail and verify refund dimension fields
        let detail_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/bill/{}/invoices/{}", realm_id, invoice_id))
                    .header("cookie", format!("X-Auth={}", admin_token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(detail_response.status(), StatusCode::OK);
        let detail = parse_body(detail_response.into_body()).await;

        assert_eq!(detail["amountRefunded"], 3000);
        assert_eq!(detail["amountRemaining"], 7000);

        // Verify the credit note is attached to the detail response
        let credit_notes = detail["creditNotes"].as_array().unwrap();
        assert_eq!(credit_notes.len(), 1);
        assert_eq!(credit_notes[0]["amount"], 3000);
        assert_eq!(credit_notes[0]["source"], "manual");
    }

    // =========================================================================
    // Test: Amount Overflow Rejected (US-IF-010 scenario 3)
    // =========================================================================
    // User Story: docs/user-stories/billing/invoice-fallback.md
    // Covers: US-IF-010 scenario 3
    //
    // Given: A paid manual invoice, total = 10000, with an existing credit note of 8000
    // When: POST credit note with amount = 3000
    // Then: Returns 400 because amount (3000) > amount_remaining (2000)

    #[test_context(CreditNoteTestContext)]
    #[tokio::test]
    async fn test_manual_credit_note_amount_overflow_rejected(ctx: &mut CreditNoteTestContext) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();
        let admin_token = setup_billing_admin_session(ctx, "cn-overflow@test.com").await;
        let account_id = ensure_test_account(ctx, &realm_id).await;

        let paid_invoice =
            create_paid_manual_invoice(&app, &admin_token, &realm_id, account_id, 10000).await;
        let invoice_id = paid_invoice["id"].as_str().unwrap();

        // First credit note for 8000 (leaves amount_remaining = 2000)
        let first = create_credit_note(
            &app,
            &admin_token,
            &realm_id,
            invoice_id,
            8000,
            "First partial refund",
        )
        .await;
        assert_eq!(first.status(), StatusCode::CREATED);

        // Second credit note for 3000 should be rejected (3000 > 2000 remaining)
        let response = create_credit_note(
            &app,
            &admin_token,
            &realm_id,
            invoice_id,
            3000,
            "Overflow attempt",
        )
        .await;

        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "Expected 400 when refund amount exceeds remaining payable"
        );

        // Verify the message indicates the amount guard
        let body = parse_body(response.into_body()).await;
        let message = body["message"]
            .as_str()
            .unwrap_or_else(|| body["error"].as_str().unwrap_or(""));
        assert!(
            message.contains("exceeds")
                || message.contains("remaining")
                || message.contains("payable"),
            "Expected error message about exceeding remaining payable, got: {}",
            message
        );
    }

    // =========================================================================
    // Test: Provider Mismatch Rejected (US-IF-010 scenario 4)
    // =========================================================================
    // User Story: docs/user-stories/billing/invoice-fallback.md
    // Covers: US-IF-010 scenario 4
    //
    // Given: A Stripe invoice (provider=stripe, status=paid, external_invoice_id set)
    // When: POST credit note
    // Then: Returns 403 indicating refunds are managed externally

    #[test_context(CreditNoteTestContext)]
    #[tokio::test]
    async fn test_manual_credit_note_provider_mismatch_rejected(ctx: &mut CreditNoteTestContext) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();
        let admin_token = setup_billing_admin_session(ctx, "cn-mismatch@test.com").await;
        let account_id = ensure_test_account(ctx, &realm_id).await;

        // Insert a Stripe-provider paid invoice directly via SQL.
        // (The admin create-invoice API always produces provider=manual,
        //  so we cannot use it to set up this scenario.)
        let stripe_invoice_id =
            insert_stripe_paid_invoice_via_sql(ctx, &realm_id, account_id, 10000).await;

        // Attempt to record a manual credit note on the Stripe invoice
        let response = create_credit_note(
            &app,
            &admin_token,
            &realm_id,
            &stripe_invoice_id.to_string(),
            1000,
            "Attempt on Stripe invoice",
        )
        .await;

        assert_eq!(
            response.status(),
            StatusCode::FORBIDDEN,
            "Expected 403 when recording manual credit note on a non-manual provider invoice"
        );

        let body = parse_body(response.into_body()).await;
        let message = body["message"]
            .as_str()
            .unwrap_or_else(|| body["error"].as_str().unwrap_or(""));
        assert!(
            message.contains("externally") || message.contains("provider"),
            "Expected error message about external refund management, got: {}",
            message
        );
    }

    // =========================================================================
    // Test: Non-Paid Status Rejected (US-IF-010 scenario 6)
    // =========================================================================
    // User Story: docs/user-stories/billing/invoice-fallback.md
    // Covers: US-IF-010 scenario 6
    //
    // Given: A draft manual invoice
    // When: POST credit note
    // Then: Returns 400 indicating only paid invoices support refund recording

    #[test_context(CreditNoteTestContext)]
    #[tokio::test]
    async fn test_manual_credit_note_non_paid_rejected(ctx: &mut CreditNoteTestContext) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();
        let admin_token = setup_billing_admin_session(ctx, "cn-nonpaid@test.com").await;
        let account_id = ensure_test_account(ctx, &realm_id).await;

        // Create a draft invoice (no issue / mark-paid)
        let draft_invoice =
            create_draft_invoice(&app, &admin_token, &realm_id, account_id, 10000).await;
        assert_eq!(draft_invoice["status"], "draft");
        let invoice_id = draft_invoice["id"].as_str().unwrap();

        let response = create_credit_note(
            &app,
            &admin_token,
            &realm_id,
            invoice_id,
            1000,
            "Attempt on draft",
        )
        .await;

        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "Expected 400 when recording credit note on a non-paid invoice"
        );

        let body = parse_body(response.into_body()).await;
        let message = body["message"]
            .as_str()
            .unwrap_or_else(|| body["error"].as_str().unwrap_or(""));
        assert!(
            message.contains("paid") || message.contains("refund"),
            "Expected error message about paid-only refund recording, got: {}",
            message
        );
    }

    // =========================================================================
    // Test: Partial Refund Accumulation (US-IF-010 scenario 2)
    // =========================================================================
    // User Story: docs/user-stories/billing/invoice-fallback.md
    // Covers: US-IF-010 scenario 2, US-IF-008 scenario 2
    //
    // Given: A paid manual invoice, total = 10000
    // When: Create two credit notes: 3000 then 2000
    // Then: Both return 201
    // And: GET invoice detail shows amountRefunded = 5000, amountRemaining = 5000

    #[test_context(CreditNoteTestContext)]
    #[tokio::test]
    async fn test_manual_credit_note_partial_refund_accumulation(ctx: &mut CreditNoteTestContext) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();
        let admin_token = setup_billing_admin_session(ctx, "cn-accumulation@test.com").await;
        let account_id = ensure_test_account(ctx, &realm_id).await;

        let paid_invoice =
            create_paid_manual_invoice(&app, &admin_token, &realm_id, account_id, 10000).await;
        let invoice_id = paid_invoice["id"].as_str().unwrap();

        // First credit note: 3000
        let first_response = create_credit_note(
            &app,
            &admin_token,
            &realm_id,
            invoice_id,
            3000,
            "First partial",
        )
        .await;
        assert_eq!(first_response.status(), StatusCode::CREATED);

        // Second credit note: 2000
        let second_response = create_credit_note(
            &app,
            &admin_token,
            &realm_id,
            invoice_id,
            2000,
            "Second partial",
        )
        .await;
        assert_eq!(second_response.status(), StatusCode::CREATED);

        // GET invoice detail and verify accumulated totals
        let detail_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/bill/{}/invoices/{}", realm_id, invoice_id))
                    .header("cookie", format!("X-Auth={}", admin_token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(detail_response.status(), StatusCode::OK);
        let detail = parse_body(detail_response.into_body()).await;

        assert_eq!(detail["amountRefunded"], 5000);
        assert_eq!(detail["amountRemaining"], 5000);

        // Verify both credit notes are attached
        let credit_notes = detail["creditNotes"].as_array().unwrap();
        assert_eq!(credit_notes.len(), 2);
    }

    // =========================================================================
    // Test: Invoice Not Found (US-IF-010 edge case)
    // =========================================================================
    // User Story: docs/user-stories/billing/invoice-fallback.md
    // Covers: US-IF-010 edge case -- posting against a nonexistent invoice
    //
    // Given: A nonexistent invoice ID
    // When: POST credit note
    // Then: Returns 404

    #[test_context(CreditNoteTestContext)]
    #[tokio::test]
    async fn test_manual_credit_note_invoice_not_found(ctx: &mut CreditNoteTestContext) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();
        let admin_token = setup_billing_admin_session(ctx, "cn-notfound@test.com").await;

        // Use a freshly generated UUID that cannot exist in the database
        let nonexistent_invoice_id = Uuid::now_v7();

        let response = create_credit_note(
            &app,
            &admin_token,
            &realm_id,
            &nonexistent_invoice_id.to_string(),
            1000,
            "Attempt on missing invoice",
        )
        .await;

        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "Expected 404 when posting a credit note to a nonexistent invoice"
        );
    }
}
