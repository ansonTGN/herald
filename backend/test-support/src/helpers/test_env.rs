// =============================================================================
// Test Environment Utilities
// =============================================================================
//
// Shared utilities for test environment configuration and cleanup.
// Reduces duplication across test context implementations.
//
// =============================================================================

use sqlx::PgPool;
use tracing::debug;

/// Drop a schema if configured to do so.
///
/// This helper centralizes the schema cleanup logic and provides
/// consistent logging across all test context implementations.
///
/// # Arguments
/// * `schema_name` - The name of the schema to drop
/// * `cleanup_pool` - The database connection pool to use for cleanup
///
/// # Returns
/// * `Ok(())` if the schema was dropped successfully or configured to skip
/// * `Err(sqlx::Error)` if the drop operation failed
pub async fn cleanup_schema_if_needed(
    schema_name: &str,
    cleanup_pool: &PgPool,
) -> Result<(), sqlx::Error> {
    debug!("🗑️  Dropping test schema: {}", schema_name);
    sqlx::query(&format!(
        r#"DROP SCHEMA IF EXISTS "{}" CASCADE"#,
        schema_name
    ))
    .execute(cleanup_pool)
    .await?;
    Ok(())
}
