// PostgreSQL implementation of Purchase repository
// PurchaseRepository trait was removed (points_package_purchases table deprecated).
// This struct is retained for API compatibility and may be removed in a future cleanup.

use std::sync::Arc;

use sqlx::PgPool;

/// PostgreSQL implementation of PurchaseRepository
/// Retained as a placeholder; all points_package_purchase methods have been removed.
pub struct PostgresPurchaseRepository {
    #[allow(dead_code)]
    pool: Arc<PgPool>,
}

impl PostgresPurchaseRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}
