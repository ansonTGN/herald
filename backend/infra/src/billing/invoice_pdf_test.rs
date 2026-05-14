// =============================================================================
// Invoice PDF Generation and Template Tests
// =============================================================================
//
// Tests for HTML template rendering, XSS prevention via HTML escaping,
// IronPress PDF output validation, and edge cases (unicode, amounts).
//
// User Story: docs/user-stories/13-invoice-user-stories.md
// Covers: US-IV-009 (PDF Download) validation criteria
//
// =============================================================================

use chrono::Utc;
use herald_domain::billing::invoice::{
    AdjustmentMode, Invoice, InvoiceDetail, InvoiceLineItem, InvoiceSource, InvoiceStatus,
};

use super::invoice_pdf_generator::IronPressInvoicePdfGenerator;
use herald_domain::billing::invoice::InvoicePdfGenerator;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn sample_invoice() -> Invoice {
    Invoice {
        id: uuid::Uuid::now_v7(),
        realm_id: "test-realm".to_string(),
        invoice_number: "INV-2026-0001".to_string(),
        source: InvoiceSource::AdminManual,
        account_id: uuid::Uuid::now_v7(),
        applicant_user_id: None,
        subscription_id: None,
        payment_attempt_id: None,
        status: InvoiceStatus::Issued,
        currency: "USD".to_string(),
        issue_date: Some(chrono::NaiveDate::from_ymd_opt(2026, 5, 1).unwrap()),
        due_date: Some(chrono::NaiveDate::from_ymd_opt(2026, 6, 1).unwrap()),
        issued_at: Some(Utc::now()),
        paid_at: None,
        voided_at: None,
        subtotal: 10000,
        discount_amount: 500,
        tax_amount: 950,
        shipping_amount: 200,
        total: 10650,
        discount_mode: Some(AdjustmentMode::Fixed),
        discount_value: Some("5.00".to_string()),
        tax_mode: Some(AdjustmentMode::Percent),
        tax_value: Some("10".to_string()),
        shipping_mode: Some(AdjustmentMode::Fixed),
        shipping_value: Some("2.00".to_string()),
        billing_name: "Acme Corp".to_string(),
        billing_address: Some("123 Main St".to_string()),
        billing_email: Some("billing@acme.com".to_string()),
        billing_phone: None,
        seller_name: "Seller Inc".to_string(),
        seller_address: Some("456 Oak Ave".to_string()),
        seller_email: Some("seller@example.com".to_string()),
        seller_phone: None,
        notes: Some("Thank you for your business!".to_string()),
        payment_terms: Some("Net 30".to_string()),
        void_reason: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn sample_detail_with_items() -> InvoiceDetail {
    let mut invoice = sample_invoice();
    invoice.billing_name = "Test Buyer".to_string();
    invoice.notes = Some("Standard notes".to_string());

    InvoiceDetail {
        invoice,
        line_items: vec![
            InvoiceLineItem {
                id: uuid::Uuid::now_v7(),
                invoice_id: uuid::Uuid::now_v7(),
                sort_order: 1,
                name: "Consulting Hours".to_string(),
                description: Some("10 hours of dev work".to_string()),
                quantity: "10".to_string(),
                unit_price: 15000,
                subtotal: 150000,
            },
            InvoiceLineItem {
                id: uuid::Uuid::now_v7(),
                invoice_id: uuid::Uuid::now_v7(),
                sort_order: 2,
                name: "Server Hosting".to_string(),
                description: None,
                quantity: "1".to_string(),
                unit_price: 5000,
                subtotal: 5000,
            },
        ],
        history: vec![],
    }
}
// =========================================================================
// PDF Generation Tests
// =========================================================================

// User Story: docs/user-stories/13-invoice-user-stories.md
// Covers: US-IV-009 -- handles non-ASCII characters

#[tokio::test]
async fn test_generate_pdf_with_unicode() {
    let mut detail = sample_detail_with_items();
    // Use Chinese characters in buyer name and notes to test unicode handling
    detail.invoice.billing_name = "\u{5317}\u{4eac}\u{516c}\u{53f8}".to_string(); // Beijing Company in Chinese
    detail.invoice.notes = Some("\u{611f}\u{8c22}\u{60a8}\u{7684}\u{4e1a}\u{52a1}".to_string()); // Thank you for your business

    let generator = IronPressInvoicePdfGenerator;
    let result = generator.generate(&detail).await;

    assert!(
        result.is_ok(),
        "PDF generation should succeed with unicode content: {:?}",
        result.err()
    );
    let bytes = result.unwrap();

    // Must still be a valid PDF
    assert_eq!(
        &bytes[0..4],
        b"%PDF",
        "PDF with unicode must start with %PDF"
    );
    assert!(
        bytes.len() > 1024,
        "PDF with unicode should have reasonable size"
    );
}
