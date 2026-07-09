//! Herald Worker - Background job processing library
//!
//! This library provides background job processing services for Herald.
//! It should be used by the app crate to run workers alongside the API server.

pub mod jobs;

use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::info;

use herald_core::domain::billing::compensation::WebhookEventProcessor;
use herald_core::domain::billing::invoice::InvoiceRepository;
use herald_core::domain::points::ExpirationService;
use herald_core::infrastructure::points::PostgresPointsRepository;
use sqlx::PgPool;

pub use jobs::InvoiceOverdueJob;
pub use jobs::PaymentEventRetryJob;
pub use jobs::PointsExpirationJob;
pub use jobs::PointsQuotaExpirationJob;
pub use jobs::WebhookCompensationJob;

/// Configuration for the worker
#[derive(Clone)]
pub struct WorkerConfig<R>
where
    R: InvoiceRepository,
{
    /// Expiration service for processing expired points
    pub expiration_service: Arc<ExpirationService<PostgresPointsRepository>>,

    pub invoice_repo: Arc<R>,

    pub pg_pool: PgPool,

    /// Interval for running background jobs (in seconds)
    pub expiration_interval_secs: u64,

    /// Optional webhook compensation processor.
    /// When Some, the compensation job runs alongside other background jobs.
    pub event_processor: Option<Arc<dyn WebhookEventProcessor>>,

    /// Interval (and lookback window) for webhook compensation in seconds.
    pub compensation_interval_secs: u64,

    /// Optional points quota-entitlement expiry cleanup job. When Some, the
    /// job runs on its own interval sweeping already-lapsed
    /// `points_quota_entitlements` rows. This is NOT a correctness boundary
    /// (a lapsed-but-unswept entitlement contributes nothing to availability);
    /// it only keeps the active set small.
    pub quota_expiration: Option<Arc<PointsQuotaExpirationJob>>,

    /// Interval for the quota-entitlement expiry cleanup job (in seconds).
    pub quota_expiration_interval_secs: u64,

    /// Optional payment-event retry sweep job. When Some, the job sweeps
    /// `payment_event WHERE processed = false` and re-runs each missed event
    /// through the `WebhookEventProcessor` (design §5.5.1 / BE-D05). This IS a
    /// correctness boundary: it is the backstop that guarantees a webhook the
    /// API layer failed to process is eventually re-run, so a cancel/expire/
    /// refund can never permanently miss its role revoke.
    pub payment_event_retry: Option<Arc<PaymentEventRetryJob>>,

    /// Interval for the payment-event retry sweep job (in seconds).
    pub payment_event_retry_interval_secs: u64,
}

impl<R> WorkerConfig<R>
where
    R: InvoiceRepository,
{
    /// Create a new worker config with default values
    pub fn new(
        expiration_service: Arc<ExpirationService<PostgresPointsRepository>>,
        invoice_repo: Arc<R>,
        pg_pool: PgPool,
    ) -> Self {
        // TODO: 应从 AppConfig 统一读取，而非单独从环境变量获取。
        // 当前默认 1h 间隔意味着积分过期最多有 1h 的懒过期窗口。
        let expiration_interval_secs = std::env::var("WORKER_EXPIRATION_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(3600);
        let compensation_interval_secs = std::env::var("WORKER_COMPENSATION_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(1800);
        let quota_expiration_interval_secs = std::env::var("WORKER_QUOTA_EXPIRATION_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(300);
        // Default 300s (5 min) — shorter than the 30-min WebhookCompensationJob
        // (design §5.5.1): this sweep is the reliability backstop for missed
        // payment events, so a tighter cadence limits the revoke-grant gap.
        let payment_event_retry_interval_secs =
            std::env::var("WORKER_PAYMENT_EVENT_RETRY_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(300);
        Self {
            expiration_service,
            invoice_repo,
            pg_pool,
            expiration_interval_secs,
            event_processor: None,
            compensation_interval_secs,
            quota_expiration: None,
            quota_expiration_interval_secs,
            payment_event_retry: None,
            payment_event_retry_interval_secs,
        }
    }

    /// Set the webhook compensation event processor.
    pub fn with_event_processor(mut self, processor: Arc<dyn WebhookEventProcessor>) -> Self {
        self.event_processor = Some(processor);
        self
    }

    /// Attach the quota-entitlement expiry cleanup job. The job runs on
    /// `quota_expiration_interval_secs`; correctness is NOT gated on it firing
    /// on time (it only reaps already-lapsed rows).
    pub fn with_quota_expiration(mut self, job: Arc<PointsQuotaExpirationJob>) -> Self {
        self.quota_expiration = Some(job);
        self
    }

    /// Attach the payment-event retry sweep job (BE-D05). The job runs on
    /// `payment_event_retry_interval_secs` (default 300s) and is a correctness
    /// boundary: it guarantees missed payment events are eventually re-run.
    pub fn with_payment_event_retry(mut self, job: Arc<PaymentEventRetryJob>) -> Self {
        self.payment_event_retry = Some(job);
        self
    }
}

/// Worker service that runs background jobs
pub struct WorkerService<R>
where
    R: InvoiceRepository,
{
    config: WorkerConfig<R>,
}

impl<R> WorkerService<R>
where
    R: InvoiceRepository + 'static,
{
    /// Create a new worker service
    pub fn new(config: WorkerConfig<R>) -> Self {
        Self { config }
    }

    /// Start the worker service in the background
    ///
    /// Returns a handle that can be used to wait for the worker to complete
    pub fn start(self) -> Result<WorkerHandle> {
        let expiration_service = self.config.expiration_service.clone();
        let invoice_repo = self.config.invoice_repo.clone();
        let pg_pool = self.config.pg_pool.clone();
        let expiration_interval = Duration::from_secs(self.config.expiration_interval_secs);
        let compensation_interval = Duration::from_secs(self.config.compensation_interval_secs);
        let event_processor = self.config.event_processor.clone();
        let compensation_lookback_secs = self.config.compensation_interval_secs;
        let quota_expiration = self.config.quota_expiration.clone();
        let quota_expiration_interval =
            Duration::from_secs(self.config.quota_expiration_interval_secs);
        let payment_event_retry = self.config.payment_event_retry.clone();
        let payment_event_retry_interval =
            Duration::from_secs(self.config.payment_event_retry_interval_secs);

        // Spawn the worker loop
        let handle = tokio::spawn(async move {
            Self::worker_loop(
                expiration_service,
                invoice_repo,
                pg_pool,
                expiration_interval,
                compensation_interval,
                event_processor,
                compensation_lookback_secs,
                quota_expiration,
                quota_expiration_interval,
                payment_event_retry,
                payment_event_retry_interval,
            )
            .await
        });

        Ok(WorkerHandle { handle })
    }

    /// Main worker loop
    #[allow(clippy::too_many_arguments)]
    async fn worker_loop(
        expiration_service: Arc<ExpirationService<PostgresPointsRepository>>,
        invoice_repo: Arc<R>,
        pg_pool: PgPool,
        expiration_interval: Duration,
        compensation_interval: Duration,
        event_processor: Option<Arc<dyn WebhookEventProcessor>>,
        compensation_lookback_secs: u64,
        quota_expiration: Option<Arc<PointsQuotaExpirationJob>>,
        quota_expiration_interval: Duration,
        payment_event_retry: Option<Arc<PaymentEventRetryJob>>,
        payment_event_retry_interval: Duration,
    ) {
        info!("Starting worker service");

        // Create jobs
        let expiration_job = PointsExpirationJob::new(expiration_service);
        let invoice_overdue_job = InvoiceOverdueJob::new(invoice_repo);

        // Create compensation job only if processor is provided
        let compensation_job = event_processor.map(|processor| {
            WebhookCompensationJob::new(pg_pool.clone(), processor, compensation_lookback_secs)
        });

        let mut expiration_timer = tokio::time::interval(expiration_interval);
        let mut compensation_timer = tokio::time::interval(if compensation_job.is_some() {
            compensation_interval
        } else {
            Duration::MAX
        });
        // Quota-entitlement expiry cleanup runs on its own interval; when no
        // job is attached the timer is parked at Duration::MAX so the arm
        // never fires.
        let mut quota_expiration_timer = tokio::time::interval(if quota_expiration.is_some() {
            quota_expiration_interval
        } else {
            Duration::MAX
        });
        // Payment-event retry sweep runs on its own interval; when no job is
        // attached the timer is parked at Duration::MAX so the arm never fires.
        let mut payment_event_retry_timer =
            tokio::time::interval(if payment_event_retry.is_some() {
                payment_event_retry_interval
            } else {
                Duration::MAX
            });

        loop {
            tokio::select! {
                _ = expiration_timer.tick() => {
                    info!("Running background jobs...");

                    // Run expiration job
                    match expiration_job.run().await {
                        Ok(summary) => {
                            info!(
                                expired_count = summary.expired_count,
                                total_expired = summary.total_expired,
                                "Points expiration completed"
                            );
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "Points expiration failed");
                        }
                    }

                    // Run invoice overdue marking job
                    match invoice_overdue_job.run().await {
                        Ok(result) => {
                            info!(
                                candidates = result.candidates,
                                marked = result.marked,
                                errors = result.errors,
                                "Invoice overdue marking completed"
                            );
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "Invoice overdue marking failed");
                        }
                    }
                }

                // Run webhook compensation job on its own schedule
                _ = compensation_timer.tick(), if compensation_job.is_some() => {
                    if let Some(ref job) = compensation_job {
                        match job.run().await {
                            Ok(result) => {
                                info!(
                                    realms_scanned = result.realms_scanned,
                                    events_fetched = result.events_fetched,
                                    events_compensated = result.events_compensated,
                                    events_failed = result.events_failed,
                                    "Webhook compensation completed"
                                );
                            }
                            Err(e) => {
                                tracing::error!(error = %e, "Webhook compensation failed");
                            }
                        }
                    }
                }

                // Run quota-entitlement expiry cleanup on its own schedule.
                // Hygiene only — not a correctness boundary.
                _ = quota_expiration_timer.tick(), if quota_expiration.is_some() => {
                    if let Some(ref job) = quota_expiration {
                        match job.run().await {
                            Ok(summary) => {
                                info!(
                                    expired_count = summary.expired_count,
                                    "Points quota-entitlement expiry cleanup completed"
                                );
                            }
                            Err(e) => {
                                tracing::error!(error = %e, "Points quota-entitlement expiry cleanup failed");
                            }
                        }
                    }
                }

                // Run the payment-event retry sweep on its own schedule.
                // Correctness boundary — guarantees missed payment events are
                // eventually re-run (design §5.5.1 / BE-D05).
                _ = payment_event_retry_timer.tick(), if payment_event_retry.is_some() => {
                    if let Some(ref job) = payment_event_retry {
                        match job.run().await {
                            Ok(stats) => {
                                info!(
                                    scanned = stats.scanned,
                                    reprocessed = stats.reprocessed,
                                    failed = stats.failed,
                                    "Payment event retry sweep completed"
                                );
                            }
                            Err(e) => {
                                tracing::error!(error = %e, "Payment event retry sweep failed");
                            }
                        }
                    }
                }

                _ = Self::shutdown_signal() => {
                    info!("Shutting down worker service");
                    return;
                }
            }
        }
    }

    /// Wait for shutdown signal
    async fn shutdown_signal() {
        let ctrl_c = async {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {},
            _ = terminate => {},
        }
    }
}

/// Handle for a running worker
pub struct WorkerHandle {
    handle: JoinHandle<()>,
}

impl WorkerHandle {
    /// Wait for the worker to complete
    pub async fn wait(self) -> Result<()> {
        self.handle.await?;
        Ok(())
    }
}

/// Start the worker with the given configuration
///
/// This is a convenience function that creates and starts the worker
pub fn start<R>(config: WorkerConfig<R>) -> Result<WorkerHandle>
where
    R: InvoiceRepository + 'static,
{
    let service = WorkerService::new(config);
    service.start()
}
