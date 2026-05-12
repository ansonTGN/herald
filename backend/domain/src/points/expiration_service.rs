// Points expiration service

use crate::common::entities::app_errors::CoreError;
use crate::points::ports::PointsRepository;
use chrono::{DateTime, Utc};
use std::sync::Arc;

/// Service for handling points expiration
pub struct ExpirationService<R> {
    repo: Arc<R>,
}

impl<R> ExpirationService<R>
where
    R: PointsRepository + Send + Sync,
{
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    /// Scan and expire points that have passed their expiration date
    ///
    /// This method finds all ledgers that have expired and marks them as expired,
    /// creates revocation records, and updates account balances accordingly.
    ///
    /// # Arguments
    /// * `batch_size` - Maximum number of ledgers to process in one batch
    ///
    /// # Returns
    /// Summary of expiration operation
    pub async fn scan_and_expire_points(
        &self,
        batch_size: usize,
    ) -> Result<ExpirationSummary, CoreError> {
        let summary = self.repo.scan_and_expire_points_atomic(batch_size).await?;

        tracing::info!(
            expired_count = summary.expired_count,
            total_expired = summary.total_expired,
            "Points expiration completed"
        );

        Ok(summary)
    }
}

/// Summary of expiration operation
#[derive(Debug, Clone)]
pub struct ExpirationSummary {
    pub expired_count: usize,
    pub total_expired: i64,
    pub expired_at: DateTime<Utc>,
}
