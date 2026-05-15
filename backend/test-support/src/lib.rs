// =============================================================================
// Herald Test Support Library
// =============================================================================
//
// Shared test infrastructure for Herald backend testing
// Provides schema-isolated test contexts and common test helpers
//
// =============================================================================

pub mod auth_schema_test_context;
pub mod bare_schema_test_context;
pub mod fixtures;
pub mod helpers;
pub mod mock_oauth_urls;
pub mod schema_test_context;
pub mod shared;

pub const TEST_JWT_SECRET: &str = "test-jwt-secret-key-for-integration-tests-32b";

// Re-export common types for convenience
pub use auth_schema_test_context::AuthSchemaTestContext;
pub use bare_schema_test_context::BareSchemaTestContext;
pub use fixtures::{TestRealmFixture, TestRedisFixture, TestUserFixture};
pub use schema_test_context::SchemaTestContext;
