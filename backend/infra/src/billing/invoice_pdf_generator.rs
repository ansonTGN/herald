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
