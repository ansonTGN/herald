use anyhow::Result;
use chrono::Duration;
use clap::Parser;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::signal;
use tracing::info;
use tracing_subscriber::{
    filter::{Directive, EnvFilter},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

use herald_api::WebhookEventProcessorImpl;
use herald_api::config::ApiConfig;
use herald_api::observability;
use herald_core::domain::billing::compensation::WebhookEventProcessor;
use herald_core::domain::points::{ExpirationService, GrantPeriodType, GrantScheduler};
use herald_worker::PointsPreGrantJob;
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

    // Initialize observability: meter provider is always built (metrics on);
    // traces layer is built only when `traces_enabled=true` (baseline = off,
    // P0 back-pressure mitigation). The traces provider is
    // spliced into the handles so `shutdown` can flush it at exit.
    let mut handles = observability::build_meter_provider(&config.observability);
    let traces = observability::build_traces_layer(&config.observability);
    handles = handles.with_tracer_provider(traces.provider);

    // EnvFilter: honor RUST_LOG when present, otherwise fall back to the
    // configured `server.log_level`. Then silence high-volume transport
    // crates so they never spam stdout regardless of the base level. These
    // `=off` directives override the base level for the named targets
    // (tracing-subscriber: more specific directives win).
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(config.server.log_level.clone()))
        .add_directive("hyper=off".parse::<Directive>().expect("static directive"))
        .add_directive("tonic=off".parse::<Directive>().expect("static directive"))
        .add_directive("h2=off".parse::<Directive>().expect("static directive"))
        .add_directive(
            "reqwest=off"
                .parse::<Directive>()
                .expect("static directive"),
        );

    // Conditionally attach the OTel traces layer.
    //
    // The traces layer is `Box<dyn Layer<Registry>>`. Two composability
    // constraints drive the structure:
    //   1. It is only composable on a *bare* `Registry` (not generic over
    //      the subscriber type), so it must be layered on first.
    //   2. `fmt::Layer<S>` captures the subscriber type `S` at construction
    //      via inference, so each arm builds its own fmt layer — the two
    //      arms have different concrete subscriber types and must not share
    //      a single fmt layer value.
    //
    // Under the baseline (`traces_enabled=false`) `traces.layer` is `None`
    // and NOTHING is installed — traces do not leave the process. Branching
    // here (rather than `.with(Option<...>)`) is what makes the baseline a
    // structural guarantee rather than a runtime toggle.
    match traces.layer {
        Some(otel_layer) => tracing_subscriber::registry()
            .with(otel_layer)
            .with(env_filter)
            .with(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_ansi(false)
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_level(true)
                    .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE),
            )
            .init(),
        None => tracing_subscriber::registry()
            .with(env_filter)
            .with(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_ansi(false)
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_level(true)
                    .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE),
            )
            .init(),
    }

    info!("Starting Herald Application");
    info!("Configuration loaded from: {}", config_path);
    info!("Bind address: {}", config.server.bind_address);
    info!("Frontend URL: {}", config.frontend.url);

    // Build shared application state (database, Redis, all services)
    let state = herald_api::build_app_state(&config).await?;

    // Initialize services for worker. Reuse AppState's already-constructed
    // points repository / points service (policy-bound) so the worker's
    // GrantScheduler shares the same policy as the API path.
    let expiration_service = Arc::new(ExpirationService::new(state.points_repository.clone()));
    let invoice_repo = Arc::new(
        herald_core::infrastructure::billing::PostgresInvoiceRepository::new((*state.db).clone()),
    );

    // Construct the pre-grant lead_time_map. Defaults: Daily=1h,
    // Weekly=12h, Monthly=24h, Once=0. Env-overridable for ops tuning; not
    // exposed to Realm/frontend. Subscription schedules share
    // the Monthly entry (their grant_period_type placeholder is monthly).
    let lead_time_map = build_lead_time_map();

    let grant_scheduler = Arc::new(GrantScheduler::new(
        state.points_repository.clone(),
        state.points_service.clone(),
        lead_time_map,
    ));
    let pre_grant_job = Arc::new(PointsPreGrantJob::new(
        grant_scheduler,
        state.points_repository.clone(),
    ));

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
        .with_event_processor(event_processor)
        .with_pre_grant(pre_grant_job);
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

    // Flush + shut down OTel providers (meter always; tracer only when
    // traces were enabled). Takes handles by value and never panics — a
    // failing exporter cannot crash the process on exit.
    observability::shutdown(handles);

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

/// Build the pre-grant `lead_time_map` for the
/// `GrantScheduler`. Defaults: Daily=1h, Weekly=12h, Monthly=24h, Once=0.
/// Ops can override via env; the map is a backend scheduling parameter and is
/// NOT exposed to Realm/frontend. Subscription schedules reuse
/// the Monthly entry (their `grant_period_type` placeholder is monthly).
fn build_lead_time_map() -> HashMap<GrantPeriodType, Duration> {
    let daily = env_hours("WORKER_FREE_GRANT_LEAD_HOURLY", 1);
    let weekly = env_hours("WORKER_FREE_GRANT_LEAD_WEEKLY_HOURS", 12);
    let monthly = env_hours("WORKER_FREE_GRANT_LEAD_MONTHLY_HOURS", 24);
    let mut map = HashMap::new();
    map.insert(GrantPeriodType::Daily, Duration::hours(daily));
    map.insert(GrantPeriodType::Weekly, Duration::hours(weekly));
    map.insert(GrantPeriodType::Monthly, Duration::hours(monthly));
    // Once = 0 (no lead). Omitted from the map so GrantScheduler falls back
    // to its default_lead_time(Once) = 0; this keeps the "no lead" semantics
    // explicit even if defaults change upstream.
    map.insert(GrantPeriodType::Once, Duration::zero());
    map
}

/// Read an hours-valued env var with a default (in whole hours).
fn env_hours(name: &str, default: i64) -> i64 {
    env::var(name)
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(default)
}
