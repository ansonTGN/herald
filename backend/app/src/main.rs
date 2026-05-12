use anyhow::Result;
use clap::Parser;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::signal;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use herald_api::config::ApiConfig;
use herald_core::PostgresInvoiceRepository;
use herald_core::domain::points::ExpirationService;
use herald_core::infrastructure::points::PostgresPointsRepository;
use herald_core::infrastructure::redis::{ManagerConfig, RedisConnectionManager};
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
    let config_path = env::var("CAS_CONFIG").unwrap_or("config.toml".to_owned());
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

    // Connect to database with connection pool configuration
    let mut db_opts = sea_orm::ConnectOptions::new(&config.database.url);
    db_opts
        .max_connections(config.database.max_connections)
        .acquire_timeout(std::time::Duration::from_secs(
            config.database.acquire_timeout_secs,
        ))
        .idle_timeout(std::time::Duration::from_secs(
            config.database.idle_timeout_secs,
        ))
        .max_lifetime(std::time::Duration::from_secs(
            config.database.max_lifetime_secs,
        ))
        .connect_timeout(std::time::Duration::from_secs(
            config.database.connect_timeout_secs,
        ));
    let db = sea_orm::Database::connect(db_opts).await?;
    info!(
        "Connected to database (pool size: {})",
        config.database.max_connections
    );

    // Run migrations
    let pg_pool = db.get_postgres_connection_pool();
    {
        use sqlx::PgPool;
        let pool: PgPool = pg_pool.clone();
        sqlx::migrate!("./migrations").run(&pool).await?;
    }
    info!("Database migrations completed");

    // Connect to Redis
    let redis_config = ManagerConfig {
        url: config.redis.url.clone(),
        default_db: 0,
        test_mode: config.server.app_env == "test",
        test_db: 1,
    };
    let redis_manager = RedisConnectionManager::new(redis_config)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create Redis manager: {}", e))?;

    // Health check
    redis_manager
        .health_check()
        .await
        .map_err(|e| anyhow::anyhow!("Redis health check failed: {}", e))?;

    info!("Connected to Redis");

    // Initialize services
    // Create points repository and expiration service
    let points_repo = Arc::new(PostgresPointsRepository::new(
        Arc::new(db.clone()),
        pg_pool.clone(),
    ));
    let expiration_service = Arc::new(ExpirationService::new(points_repo));

    // Start API server
    info!("Starting API server on {}", config.server.bind_address);
    let api_config = config.clone();
    let api_handle = tokio::spawn(async move { herald_api::run_with_config(api_config).await });

    // Start Worker
    info!("Starting Worker service");
    let invoice_repo = Arc::new(PostgresInvoiceRepository::new(db.clone()));
    let worker_config = WorkerConfig::new(expiration_service, invoice_repo);
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
