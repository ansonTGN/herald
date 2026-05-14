use std::sync::Arc;

use herald_core::domain::billing::invoice::InvoiceRepository;
use herald_core::domain::billing::invoice_service::{InvoiceService, OverdueMarkResult};

const DEFAULT_BATCH_SIZE: i64 = 500;

/// Background job that scans for overdue invoices and marks them.
///
/// Delegates to `InvoiceService::mark_overdue_by_system`.
pub struct InvoiceOverdueJob<R> {
    service: InvoiceService<Arc<R>>,
    batch_size: i64,
}

impl<R> InvoiceOverdueJob<R>
where
    R: InvoiceRepository,
{
    pub fn new(repo: Arc<R>) -> Self {
        let batch_size = std::env::var("WORKER_OVERDUE_BATCH_SIZE")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(DEFAULT_BATCH_SIZE);
        Self {
            service: InvoiceService::new(repo),
            batch_size,
        }
    }

    pub async fn run(&self) -> anyhow::Result<OverdueMarkResult> {
        self.service
            .mark_overdue_by_system(chrono::Utc::now(), self.batch_size)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to mark overdue invoices: {}", e))
    }
}
