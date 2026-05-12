use std::sync::Arc;

use herald_core::domain::billing::invoice::InvoiceRepository;
use herald_core::domain::billing::invoice_service::{InvoiceService, OverdueMarkResult};

/// Background job that scans for overdue invoices and marks them.
///
/// Delegates to `InvoiceService::mark_overdue_by_system`.
pub struct InvoiceOverdueJob<R> {
    service: InvoiceService<Arc<R>>,
}

impl<R> InvoiceOverdueJob<R>
where
    R: InvoiceRepository,
{
    pub fn new(repo: Arc<R>) -> Self {
        Self {
            service: InvoiceService::new(repo),
        }
    }

    pub async fn run(&self) -> anyhow::Result<OverdueMarkResult> {
        self.service
            .mark_overdue_by_system(chrono::Utc::now(), 500)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to mark overdue invoices: {}", e))
    }
}
