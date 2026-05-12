use anyhow::Result;
use herald_core::domain::points::{ExpirationService, ExpirationSummary};
use std::sync::Arc;

pub struct PointsExpirationJob<R> {
    expiration_service: Arc<ExpirationService<R>>,
}

impl<R> PointsExpirationJob<R>
where
    R: herald_core::domain::points::ports::PointsRepository + Send + Sync,
{
    pub fn new(expiration_service: Arc<ExpirationService<R>>) -> Self {
        Self { expiration_service }
    }

    pub async fn run(&self) -> Result<ExpirationSummary> {
        self.expiration_service
            .scan_and_expire_points(1000)
            .await
            .map_err(|e| anyhow::anyhow!("Expiration job failed: {}", e))
    }
}
