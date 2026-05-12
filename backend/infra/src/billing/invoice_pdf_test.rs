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
use super::invoice_pdf_template::render_invoice_html;
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
// Template Rendering Tests
// =========================================================================

// User Story: docs/user-stories/13-invoice-user-stories.md
// Covers: US-IV-009 -- valid HTML output with invoice data

#[test]
fn test_render_invoice_html_basic() {
    let detail = sample_detail_with_items();
    let html = render_invoice_html(&detail);

    // Must be a valid HTML document
    assert!(html.contains("<!DOCTYPE html>"), "Expected DOCTYPE");
    assert!(html.contains("<html>"), "Expected <html> tag");
    assert!(html.contains("</html>"), "Expected closing </html>");

    // Must contain invoice number
    assert!(
        html.contains("INV-2026-0001"),
        "Expected invoice number in HTML"
    );

    // Must contain status
    assert!(html.contains("issued"), "Expected status in HTML");

    // Must contain date
    assert!(html.contains("2026-05-01"), "Expected issue date in HTML");
}

// User Story: docs/user-stories/13-invoice-user-stories.md
// Covers: US-IV-009 -- XSS prevention in buyer name

#[test]
fn test_render_invoice_html_escapes_buyer_name() {
    let mut detail = sample_detail_with_items();
    detail.invoice.billing_name = "<script>alert('xss')</script>".to_string();

    let html = render_invoice_html(&detail);

    // The raw script tag must NOT appear -- it must be escaped
    assert!(
        !html.contains("<script>alert('xss')</script>"),
        "Raw script tag must not appear in HTML output"
    );

    // The escaped version must be present
    assert!(
        html.contains("&lt;script&gt;alert(&#39;xss&#39;)&lt;/script&gt;"),
        "Escaped script tag should appear in HTML output"
    );
}

// User Story: docs/user-stories/13-invoice-user-stories.md
// Covers: US-IV-009 -- notes field HTML escape

#[test]
fn test_render_invoice_html_escapes_notes() {
    let mut detail = sample_detail_with_items();
    detail.invoice.notes = Some("<img src=x onerror=alert(1)>".to_string());

    let html = render_invoice_html(&detail);

    // Raw img tag must NOT appear
    assert!(
        !html.contains("<img src=x onerror=alert(1)>"),
        "Raw img tag must not appear in notes"
    );

    // Escaped version must be present
    assert!(
        html.contains("&lt;img src=x onerror=alert(1)&gt;"),
        "Escaped img tag should appear in notes"
    );
}

// User Story: docs/user-stories/13-invoice-user-stories.md
// Covers: US-IV-009 -- table rows for each line item

#[test]
fn test_render_invoice_html_line_items() {
    let detail = sample_detail_with_items();
    let html = render_invoice_html(&detail);

    // Both line items must appear in table rows
    assert!(
        html.contains("Consulting Hours"),
        "Expected first line item name"
    );
    assert!(
        html.contains("10 hours of dev work"),
        "Expected first line item description"
    );
    assert!(
        html.contains("Server Hosting"),
        "Expected second line item name"
    );

    // Must have <tr> elements for line items
    assert!(
        html.contains("<tr><td>"),
        "Expected table rows for line items"
    );
}

// User Story: docs/user-stories/13-invoice-user-stories.md
// Covers: US-IV-009 -- subtotal, discount, tax, shipping, total

#[test]
fn test_render_invoice_html_amounts() {
    let detail = sample_detail_with_items();
    let html = render_invoice_html(&detail);

    // Verify formatted amounts (in cents -> "XX.XX" format)
    // subtotal 10000 -> "100.00", discount 500 -> "5.00", tax 950 -> "9.50",
    // shipping 200 -> "2.00", total 10650 -> "106.50"
    assert!(html.contains("100.00"), "Expected subtotal amount in HTML");
    assert!(html.contains("5.00"), "Expected discount amount in HTML");
    assert!(html.contains("9.50"), "Expected tax amount in HTML");
    assert!(html.contains("2.00"), "Expected shipping amount in HTML");
    assert!(html.contains("106.50"), "Expected total amount in HTML");

    // Currency label should be present
    assert!(
        html.contains("Total (USD)"),
        "Expected currency label next to total"
    );
}

// =========================================================================
// PDF Generation Tests
// =========================================================================

// User Story: docs/user-stories/13-invoice-user-stories.md
// Covers: US-IV-009 -- output starts with %PDF header

#[tokio::test]
async fn test_generate_pdf_starts_with_pdf_header() {
    let detail = sample_detail_with_items();
    let generator = IronPressInvoicePdfGenerator;
    let result = generator.generate(&detail).await;

    assert!(result.is_ok(), "PDF generation should succeed");
    let bytes = result.unwrap();

    // PDF files always start with "%PDF"
    assert!(bytes.len() >= 5, "PDF should have at least 5 bytes");
    assert_eq!(
        &bytes[0..4],
        b"%PDF",
        "PDF output must start with %PDF header"
    );
}

// User Story: docs/user-stories/13-invoice-user-stories.md
// Covers: US-IV-009 -- output larger than minimum threshold

#[tokio::test]
async fn test_generate_pdf_minimum_size() {
    let detail = sample_detail_with_items();
    let generator = IronPressInvoicePdfGenerator;
    let result = generator.generate(&detail).await;

    assert!(result.is_ok(), "PDF generation should succeed");
    let bytes = result.unwrap();

    // A minimal PDF with our template should be at least 1KB
    // (even the simplest PDF with text content will be several KB)
    assert!(
        bytes.len() > 1024,
        "PDF should be larger than 1KB, got {} bytes",
        bytes.len()
    );
}

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
