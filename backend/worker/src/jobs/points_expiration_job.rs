use anyhow::Result;
use herald_core::domain::points::{ExpirationService, ExpirationSummary};
use std::sync::Arc;

const DEFAULT_BATCH_SIZE: usize = 1000;

pub struct PointsExpirationJob<R> {
    expiration_service: Arc<ExpirationService<R>>,
    batch_size: usize,
}

impl<R> PointsExpirationJob<R>
where
    R: herald_core::domain::points::ports::PointsRepository + Send + Sync,
{
    pub fn new(expiration_service: Arc<ExpirationService<R>>) -> Self {
        let batch_size = std::env::var("WORKER_EXPIRATION_BATCH_SIZE")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(DEFAULT_BATCH_SIZE);
        Self {
            expiration_service,
            batch_size,
        }
    }

    #[tracing::instrument(
        // Governance: root span — no inbound request context.
        // `self` holds the ExpirationService (repository / DB handles).
        // Only the low-cardinality job name is recorded.
        skip(self),
        fields(job.name = "points_expiration")
    )]
    pub async fn run(&self) -> Result<ExpirationSummary> {
        self.expiration_service
            .scan_and_expire_points(self.batch_size)
            .await
            .map_err(|e| anyhow::anyhow!("Expiration job failed: {}", e))
    }
}
