use axum::extract::State;
use std::sync::Arc;

use herald_core::application::ApplicationService;

/// Helper type for extracting AppState from Axum requests
pub type AppStateExtractor = State<Arc<ApplicationService>>;
