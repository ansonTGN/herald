use anyhow::Result;
use clap::Parser;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::signal;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use herald_api::WebhookEventProcessorImpl;
use herald_api::config::ApiConfig;
use herald_core::domain::billing::compensation::WebhookEventProcessor;
use herald_core::domain::points::ExpirationService;
use herald_core::infrastructure::points::PostgresPointsRepository;
use herald_worker::WorkerConfig;

/// Herald Application
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Export OpenAPI JSON to the specified file and exit
    #[arg(long)]
    export_openapi: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Export OpenAPI JSON if requested
    if let Some(output_path) = args.export_openapi {
        return herald_api::export_openapi_to_file(&output_path);
    }

    // Load configuration
    let config_path = env::var("HERALD_CONFIG").unwrap_or("config/config.toml".to_owned());
    let config = ApiConfig::load(&config_path)?;

    // Initialize tracing with config from file
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| config.server.log_level.clone().into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_target(true)
                .with_thread_ids(true)
                .with_level(true)
                .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE),
        )
        .init();

    info!("Starting Herald Application");
    info!("Configuration loaded from: {}", config_path);
    info!("Bind address: {}", config.server.bind_address);
    info!("Frontend URL: {}", config.frontend.url);

    // Build shared application state (database, Redis, all services)
    let state = herald_api::build_app_state(&config).await?;

    // Initialize services for worker
    let points_repo = Arc::new(PostgresPointsRepository::new(
        state.db.clone(),
        state.pool.clone(),
    ));
    let expiration_service = Arc::new(ExpirationService::new(points_repo));
    let invoice_repo = Arc::new(
        herald_core::infrastructure::billing::PostgresInvoiceRepository::new((*state.db).clone()),
    );

    // Construct webhook compensation processor
    let event_processor: Arc<dyn WebhookEventProcessor> =
        Arc::new(WebhookEventProcessorImpl::new(state.as_ref().clone()));

    // Start API server
    info!("Starting API server on {}", config.server.bind_address);
    let api_config = config.clone();
    let api_state = state.clone();
    let api_handle =
        tokio::spawn(async move { herald_api::start_server(api_state, api_config).await });

    // Start Worker
    info!("Starting Worker service");
    let worker_config = WorkerConfig::new(expiration_service, invoice_repo, state.pool.clone())
        .with_event_processor(event_processor);
    let worker_handle = herald_worker::start(worker_config)?;

    // Wait for either service to complete or shutdown signal
    tokio::select! {
        result = api_handle => {
            match result {
                Ok(Ok(())) => info!("API server completed successfully"),
                Ok(Err(e)) => info!("API server exited with error: {:?}", e),
                Err(e) => info!("API server task failed: {:?}", e),
            }
        }
        result = worker_handle.wait() => {
            match result {
                Ok(()) => info!("Worker completed successfully"),
                Err(e) => info!("Worker exited with error: {:?}", e),
            }
        }
        _ = shutdown_signal() => {
            info!("Received shutdown signal");
        }
    }

    info!("Herald Application shutdown complete");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C");
        }
        _ = terminate => {
            info!("Received SIGTERM");
        }
    }
}
