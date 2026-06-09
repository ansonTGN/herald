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
pub use jobs::PointsExpirationJob;
pub use jobs::WebhookCompensationJob;
pub use jobs::WechatOrderExpiryJob;

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
        Self {
            expiration_service,
            invoice_repo,
            pg_pool,
            expiration_interval_secs,
            event_processor: None,
            compensation_interval_secs,
        }
    }

    /// Set the webhook compensation event processor.
    pub fn with_event_processor(mut self, processor: Arc<dyn WebhookEventProcessor>) -> Self {
        self.event_processor = Some(processor);
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
            )
            .await
        });

        Ok(WorkerHandle { handle })
    }

    /// Main worker loop
    async fn worker_loop(
        expiration_service: Arc<ExpirationService<PostgresPointsRepository>>,
        invoice_repo: Arc<R>,
        pg_pool: PgPool,
        expiration_interval: Duration,
        compensation_interval: Duration,
        event_processor: Option<Arc<dyn WebhookEventProcessor>>,
        compensation_lookback_secs: u64,
    ) {
        info!("Starting worker service");

        // Create jobs
        let expiration_job = PointsExpirationJob::new(expiration_service);
        let invoice_overdue_job = InvoiceOverdueJob::new(invoice_repo);
        let wechat_order_expiry_job = WechatOrderExpiryJob::new(pg_pool.clone());

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

                    match wechat_order_expiry_job.run().await {
                        Ok(result) => {
                            info!(
                                candidates = result.candidates,
                                closed = result.closed,
                                paid = result.paid,
                                errors = result.errors,
                                "WeChat order expiry processing completed"
                            );
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "WeChat order expiry processing failed");
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
