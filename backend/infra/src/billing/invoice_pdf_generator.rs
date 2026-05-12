use std::future::Future;

use herald_domain::billing::invoice::{InvoiceDetail, InvoicePdfGenerator};
use herald_domain::common::entities::app_errors::CoreError;

use crate::billing::invoice_pdf_template::render_invoice_html;

/// PDF generator that uses IronPress to convert rendered HTML into PDF bytes.
/// Stateless -- can be created inline without any configuration.
pub struct IronPressInvoicePdfGenerator;

impl InvoicePdfGenerator for IronPressInvoicePdfGenerator {
    fn generate(
        &self,
        detail: &InvoiceDetail,
    ) -> impl Future<Output = Result<Vec<u8>, CoreError>> + Send {
        let html = render_invoice_html(detail);
        async move { generate_pdf_bytes(&html) }
    }
}

fn generate_pdf_bytes(html: &str) -> Result<Vec<u8>, CoreError> {
    ironpress::HtmlConverter::new()
        .page_size(ironpress::PageSize::A4)
        .margin(ironpress::Margin::uniform(36.0))
        .convert(html)
        .map_err(|e| CoreError::BillingError(format!("PDF generation failed: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use herald_domain::billing::invoice::{AdjustmentMode, Invoice, InvoiceSource, InvoiceStatus};
    use uuid::Uuid;

    fn sample_detail() -> InvoiceDetail {
        InvoiceDetail {
            invoice: Invoice {
                id: Uuid::now_v7(),
                realm_id: "test-realm".to_string(),
                invoice_number: "INV-2026-0001".to_string(),
                source: InvoiceSource::AdminManual,
                account_id: Uuid::now_v7(),
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
                shipping_amount: 0,
                total: 10450,
                discount_mode: Some(AdjustmentMode::Fixed),
                discount_value: Some("5.00".to_string()),
                tax_mode: Some(AdjustmentMode::Percent),
                tax_value: Some("10".to_string()),
                shipping_mode: None,
                shipping_value: None,
                billing_name: "Acme Corp".to_string(),
                billing_address: Some("123 Main St".to_string()),
                billing_email: Some("billing@acme.com".to_string()),
                billing_phone: None,
                seller_name: "Seller Inc".to_string(),
                seller_address: Some("456 Oak Ave".to_string()),
                seller_email: Some("seller@example.com".to_string()),
                seller_phone: None,
                notes: Some("Thank you!".to_string()),
                payment_terms: Some("Net 30".to_string()),
                void_reason: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            line_items: vec![],
            history: vec![],
        }
    }

    #[tokio::test]
    async fn generate_pdf_produces_bytes() {
        let detail = sample_detail();
        let generator = IronPressInvoicePdfGenerator;
        let result = generator.generate(&detail).await;
        assert!(result.is_ok(), "PDF generation should succeed");
        let bytes = result.unwrap();
        // PDF files start with %PDF
        assert!(bytes.len() > 5, "PDF should have content");
        assert_eq!(&bytes[0..4], b"%PDF", "Should be a valid PDF");
    }
}
